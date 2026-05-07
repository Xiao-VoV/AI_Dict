use crate::{ai, settings};
use rust_mdict::Mdx;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

const BUILTIN_ECDICT_KEY: &str = "ecdict";
const BUILTIN_ECDICT_NAME: &str = "ECDICT";
const BUILTIN_ECDICT_RESOURCE: &str = "resources/dicts/ecdict.mdx";
const AI_CARD_PROMPT_VERSION: &str = "word-card-v2-mdx";
const MAX_CARD_TEXT_CHARS: usize = 1_200;
const MAX_IMPORTED_CARDS: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordProfile {
    pub source: String,
    pub lemma: String,
    pub translated: String,
    pub phonetics: Phonetics,
    pub definitions: Vec<Definition>,
    pub forms: WordForms,
    pub examples: Vec<Example>,
    pub phrases: Vec<Phrase>,
    pub synonyms: Vec<String>,
    pub antonyms: Vec<String>,
    pub memory_hint: Option<String>,
    pub exam_tags: Vec<String>,
    pub imported_cards: Vec<ImportedCard>,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Phonetics {
    pub uk: Option<String>,
    pub us: Option<String>,
    pub audio: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub part_of_speech: String,
    pub meaning: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub en: String,
    pub zh: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordForms {
    pub plural: Option<String>,
    pub third_person: Option<String>,
    pub past: Option<String>,
    pub past_participle: Option<String>,
    pub present_participle: Option<String>,
    pub comparative: Option<String>,
    pub superlative: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Phrase {
    pub phrase: String,
    pub meaning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportedCard {
    pub dictionary_id: i64,
    pub dictionary_name: String,
    pub headword: String,
    pub plain_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DictionaryMetadata {
    pub id: i64,
    pub name: String,
    pub kind: String,
    pub path: Option<String>,
    pub entry_count: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportSummary {
    pub dictionary_id: Option<i64>,
    pub name: String,
    pub kind: String,
    pub imported_entries: usize,
    pub skipped_entries: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiWordCard {
    pub short_definition: Option<String>,
    #[serde(default)]
    pub memory_hint: Option<String>,
    #[serde(default)]
    pub phrases: Vec<Phrase>,
    #[serde(default)]
    pub examples: Vec<Example>,
    #[serde(default)]
    pub synonyms: Vec<String>,
    #[serde(default)]
    pub antonyms: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WordLookupSeed {
    pub source: String,
    pub translated: String,
    pub lemma: String,
}

pub async fn lookup_word_profile(
    app: &AppHandle,
    settings: &settings::AppSettings,
    seed: WordLookupSeed,
) -> Result<WordProfile, DictionaryError> {
    let db = DictionaryStore::open(app)?;
    db.ensure_builtin_dictionary(app)?;

    let effective_lemma = normalize_word(&seed.lemma);
    let mut cards = db.lookup_cards(&effective_lemma)?;
    if cards.is_empty() {
        cards = db.lookup_cards(&seed.source)?;
    }

    let local_summary = summarize_cards_for_ai(&cards);
    let cached_card = db.get_ai_card(
        &effective_lemma,
        &settings.target_language,
        settings.model.trim(),
        AI_CARD_PROMPT_VERSION,
    )?;

    let ai_card = match cached_card {
        Some(card) => Some(card),
        None => match ai::enrich_word_card(
            settings,
            &seed.source,
            &effective_lemma,
            local_summary.as_deref(),
        )
        .await
        {
            Ok(card) => {
                db.save_ai_card(
                    &effective_lemma,
                    &settings.target_language,
                    settings.model.trim(),
                    AI_CARD_PROMPT_VERSION,
                    &card,
                )?;
                Some(card)
            }
            Err(error) => {
                log::warn!(
                    "AI word card enrichment failed; lemma={} error={error}",
                    effective_lemma
                );
                None
            }
        },
    };

    Ok(build_profile(
        seed.source,
        fallback_translated(&seed.translated, &cards, &seed.lemma),
        effective_lemma,
        ai_card,
        cards,
    ))
}

pub fn import_mdict(app: &AppHandle, mdx_path: &str) -> Result<ImportSummary, DictionaryError> {
    let db = DictionaryStore::open(app)?;
    db.ensure_schema()?;
    db.import_mdx_dictionary(mdx_path, DictionaryKind::UserMdx)
}

pub fn reindex_builtin_dictionary(app: &AppHandle) -> Result<ImportSummary, DictionaryError> {
    let db = DictionaryStore::open(app)?;
    db.ensure_schema()?;
    db.reindex_builtin_dictionary(app)
}

pub fn list_dictionaries(app: &AppHandle) -> Result<Vec<DictionaryMetadata>, DictionaryError> {
    let db = DictionaryStore::open(app)?;
    db.ensure_builtin_dictionary(app)?;
    db.list_dictionaries()
}

pub fn delete_dictionary(app: &AppHandle, dictionary_id: i64) -> Result<(), DictionaryError> {
    let db = DictionaryStore::open(app)?;
    db.delete_dictionary(dictionary_id)
}

enum DictionaryKind {
    BuiltinMdx,
    UserMdx,
}

impl DictionaryKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::BuiltinMdx => "builtin_mdx",
            Self::UserMdx => "user_mdx",
        }
    }
}

struct DictionaryStore {
    conn: Connection,
}

impl DictionaryStore {
    fn open(app: &AppHandle) -> Result<Self, DictionaryError> {
        let db_path = dictionary_db_path(app)?;
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        Ok(Self { conn })
    }

    fn ensure_schema(&self) -> Result<(), DictionaryError> {
        self.conn.execute_batch(
            r#"
            create table if not exists dictionaries (
              id integer primary key autoincrement,
              name text not null,
              kind text not null,
              path text,
              entry_count integer not null default 0,
              created_at integer not null,
              builtin_key text
            );

            create table if not exists mdx_entries (
              id integer primary key autoincrement,
              dictionary_id integer not null,
              headword text not null,
              normalized_word text not null,
              plain_text text not null,
              foreign key(dictionary_id) references dictionaries(id) on delete cascade
            );

            create index if not exists idx_mdx_entries_word
              on mdx_entries(normalized_word);

            create table if not exists ai_word_cards (
              lemma text not null,
              target_language text not null,
              model text not null,
              prompt_version text not null,
              card_json text not null,
              created_at integer not null,
              primary key (lemma, target_language, model, prompt_version)
            );
            "#,
        )?;

        self.add_column_if_missing("dictionaries", "builtin_key", "text")?;
        self.conn.execute_batch(
            r#"
            create unique index if not exists idx_dictionaries_builtin_key
              on dictionaries(builtin_key)
              where builtin_key is not null;
            "#,
        )?;
        Ok(())
    }

    fn add_column_if_missing(
        &self,
        table: &str,
        column: &str,
        definition: &str,
    ) -> Result<(), DictionaryError> {
        let mut stmt = self.conn.prepare(&format!("pragma table_info({table})"))?;
        let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for existing in columns {
            if existing? == column {
                return Ok(());
            }
        }
        self.conn
            .execute(&format!("alter table {table} add column {column} {definition}"), [])?;
        Ok(())
    }

    fn ensure_builtin_dictionary(&self, app: &AppHandle) -> Result<(), DictionaryError> {
        self.ensure_schema()?;
        let existing = self
            .conn
            .query_row(
                "select id from dictionaries where builtin_key = ?1",
                [BUILTIN_ECDICT_KEY],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;

        if existing.is_none() {
            if let Some(path) = builtin_mdx_path(app)? {
                self.import_builtin_dictionary(&path)?;
            } else {
                log::warn!(
                    "built-in ECDICT MDX resource not found; expected {}",
                    BUILTIN_ECDICT_RESOURCE
                );
                self.create_dictionary(
                    BUILTIN_ECDICT_NAME,
                    DictionaryKind::BuiltinMdx.as_str(),
                    None,
                    Some(BUILTIN_ECDICT_KEY),
                )?;
            }
        }
        Ok(())
    }

    fn reindex_builtin_dictionary(&self, app: &AppHandle) -> Result<ImportSummary, DictionaryError> {
        self.ensure_schema()?;
        let Some(path) = builtin_mdx_path(app)? else {
            self.ensure_builtin_dictionary(app)?;
            return Err(DictionaryError::MissingBuiltinMdx);
        };

        if let Some(dictionary_id) = self
            .conn
            .query_row(
                "select id from dictionaries where builtin_key = ?1",
                [BUILTIN_ECDICT_KEY],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
        {
            self.conn
                .execute("delete from mdx_entries where dictionary_id = ?1", [dictionary_id])?;
            self.conn
                .execute("delete from dictionaries where id = ?1", [dictionary_id])?;
        }

        self.import_builtin_dictionary(&path)
    }

    fn import_builtin_dictionary(&self, path: &Path) -> Result<ImportSummary, DictionaryError> {
        self.import_mdx_dictionary_at_path(
            path,
            BUILTIN_ECDICT_NAME,
            DictionaryKind::BuiltinMdx,
            Some(BUILTIN_ECDICT_KEY),
        )
    }

    fn import_mdx_dictionary(
        &self,
        mdx_path: &str,
        kind: DictionaryKind,
    ) -> Result<ImportSummary, DictionaryError> {
        let path = Path::new(mdx_path);
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("MDX Dictionary");
        self.import_mdx_dictionary_at_path(path, name, kind, None)
    }

    fn import_mdx_dictionary_at_path(
        &self,
        path: &Path,
        name: &str,
        kind: DictionaryKind,
        builtin_key: Option<&str>,
    ) -> Result<ImportSummary, DictionaryError> {
        let path_string = path.to_string_lossy().to_string();
        let mut mdx = Mdx::new(path).map_err(|error| DictionaryError::Mdict(error.to_string()))?;
        let dictionary_id =
            self.create_dictionary(name, kind.as_str(), Some(&path_string), builtin_key)?;

        let mut imported_entries = 0usize;
        let mut skipped_entries = 0usize;
        let keywords: Vec<String> = mdx.keywords().into_iter().map(str::to_string).collect();

        for keyword in keywords {
            if let Some(result) = mdx.lookup(&keyword) {
                let plain_text = html_to_safe_text(&result.definition);
                if plain_text.is_empty() {
                    skipped_entries += 1;
                    continue;
                }
                self.insert_mdx_entry(dictionary_id, &result.key_text, &plain_text)?;
                imported_entries += 1;
            } else {
                skipped_entries += 1;
            }
        }

        self.conn.execute(
            "update dictionaries set entry_count = ?1 where id = ?2",
            params![imported_entries as i64, dictionary_id],
        )?;

        Ok(ImportSummary {
            dictionary_id: Some(dictionary_id),
            name: name.to_string(),
            kind: kind.as_str().to_string(),
            imported_entries,
            skipped_entries,
        })
    }

    fn create_dictionary(
        &self,
        name: &str,
        kind: &str,
        path: Option<&str>,
        builtin_key: Option<&str>,
    ) -> Result<i64, DictionaryError> {
        self.conn.execute(
            "insert into dictionaries (name, kind, path, entry_count, created_at, builtin_key)
             values (?1, ?2, ?3, 0, ?4, ?5)",
            params![name, kind, path, now_ts(), builtin_key],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn insert_mdx_entry(
        &self,
        dictionary_id: i64,
        headword: &str,
        plain_text: &str,
    ) -> Result<(), DictionaryError> {
        self.conn.execute(
            r#"
            insert into mdx_entries
              (dictionary_id, headword, normalized_word, plain_text)
            values (?1, ?2, ?3, ?4)
            "#,
            params![dictionary_id, headword, normalize_word(headword), plain_text],
        )?;
        Ok(())
    }

    fn lookup_cards(&self, word: &str) -> Result<Vec<ImportedCard>, DictionaryError> {
        let normalized = normalize_word(word);
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        let mut stmt = self.conn.prepare(
            r#"
            select e.dictionary_id, d.name, e.headword, e.plain_text
            from mdx_entries e
            join dictionaries d on d.id = e.dictionary_id
            where e.normalized_word = ?1
            order by case d.kind when 'builtin_mdx' then 0 else 1 end, d.created_at asc
            limit ?2
            "#,
        )?;
        let rows = stmt.query_map(params![normalized, MAX_IMPORTED_CARDS as i64], |row| {
            Ok(ImportedCard {
                dictionary_id: row.get(0)?,
                dictionary_name: row.get(1)?,
                headword: row.get(2)?,
                plain_text: row.get(3)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(DictionaryError::from)
    }

    fn get_ai_card(
        &self,
        lemma: &str,
        target_language: &str,
        model: &str,
        prompt_version: &str,
    ) -> Result<Option<AiWordCard>, DictionaryError> {
        let card_json: Option<String> = self
            .conn
            .query_row(
                r#"
                select card_json from ai_word_cards
                where lemma = ?1 and target_language = ?2 and model = ?3 and prompt_version = ?4
                "#,
                params![lemma, target_language, model, prompt_version],
                |row| row.get(0),
            )
            .optional()?;
        card_json
            .map(|json| serde_json::from_str(&json).map_err(DictionaryError::from))
            .transpose()
    }

    fn save_ai_card(
        &self,
        lemma: &str,
        target_language: &str,
        model: &str,
        prompt_version: &str,
        card: &AiWordCard,
    ) -> Result<(), DictionaryError> {
        let card_json = serde_json::to_string(card)?;
        self.conn.execute(
            r#"
            insert or replace into ai_word_cards
              (lemma, target_language, model, prompt_version, card_json, created_at)
            values (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                lemma,
                target_language,
                model,
                prompt_version,
                card_json,
                now_ts()
            ],
        )?;
        Ok(())
    }

    fn list_dictionaries(&self) -> Result<Vec<DictionaryMetadata>, DictionaryError> {
        let mut stmt = self.conn.prepare(
            "select id, name, kind, path, entry_count, created_at
             from dictionaries
             where kind in ('builtin_mdx', 'user_mdx')
             order by case kind when 'builtin_mdx' then 0 else 1 end, created_at asc",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(DictionaryMetadata {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: row.get(2)?,
                path: row.get(3)?,
                entry_count: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(DictionaryError::from)
    }

    fn delete_dictionary(&self, dictionary_id: i64) -> Result<(), DictionaryError> {
        let kind: Option<String> = self
            .conn
            .query_row(
                "select kind from dictionaries where id = ?1",
                [dictionary_id],
                |row| row.get(0),
            )
            .optional()?;
        if kind.as_deref() == Some(DictionaryKind::BuiltinMdx.as_str()) {
            return Err(DictionaryError::CannotDeleteBuiltin);
        }

        self.conn
            .execute("delete from mdx_entries where dictionary_id = ?1", [dictionary_id])?;
        self.conn
            .execute("delete from dictionaries where id = ?1", [dictionary_id])?;
        Ok(())
    }
}

fn build_profile(
    source: String,
    translated: String,
    lemma: String,
    ai_card: Option<AiWordCard>,
    imported_cards: Vec<ImportedCard>,
) -> WordProfile {
    let mut definitions = Vec::new();
    let mut examples = Vec::new();
    let mut phrases = Vec::new();
    let mut synonyms = Vec::new();
    let mut antonyms = Vec::new();
    let mut sources = BTreeSet::new();
    let mut memory_hint = None;

    for card in &imported_cards {
        sources.insert(card.dictionary_name.clone());
    }

    if let Some(first_card) = imported_cards.first() {
        definitions.extend(parse_plain_text_definitions(
            &first_card.plain_text,
            &first_card.dictionary_name,
        ));
    }

    if let Some(card) = ai_card {
        sources.insert("AI".to_string());
        if let Some(short_definition) = card
            .short_definition
            .filter(|value| !value.trim().is_empty())
        {
            definitions.insert(
                0,
                Definition {
                    part_of_speech: "AI".to_string(),
                    meaning: short_definition,
                    source: "AI".to_string(),
                },
            );
        }
        examples.extend(card.examples);
        phrases.extend(card.phrases);
        synonyms.extend(card.synonyms);
        antonyms.extend(card.antonyms);
        memory_hint = card.memory_hint;
    }

    WordProfile {
        source,
        lemma,
        translated,
        phonetics: Phonetics::default(),
        definitions,
        forms: WordForms::default(),
        examples,
        phrases,
        synonyms,
        antonyms,
        memory_hint,
        exam_tags: Vec::new(),
        imported_cards,
        sources: sources.into_iter().collect(),
    }
}

fn parse_plain_text_definitions(text: &str, source: &str) -> Vec<Definition> {
    text.lines()
        .flat_map(|line| line.split("；"))
        .filter_map(|chunk| {
            let trimmed = chunk.trim();
            if trimmed.is_empty() {
                return None;
            }
            let (part_of_speech, meaning) = split_definition_line(trimmed);
            if meaning.is_empty() {
                return None;
            }
            Some(Definition {
                part_of_speech,
                meaning: trim_to_chars(&meaning, 180),
                source: source.to_string(),
            })
        })
        .take(6)
        .collect()
}

fn split_definition_line(line: &str) -> (String, String) {
    for marker in [
        "vt.", "vi.", "v.", "n.", "adj.", "adv.", "prep.", "conj.", "pron.", "abbr.", "a.",
    ] {
        if let Some(rest) = line.strip_prefix(marker) {
            return (marker.to_string(), rest.trim().to_string());
        }
    }
    ("".to_string(), line.to_string())
}

fn fallback_translated(translated: &str, cards: &[ImportedCard], lemma: &str) -> String {
    let translated = translated.trim();
    if !translated.is_empty() {
        return translated.to_string();
    }

    cards
        .first()
        .map(|card| trim_to_chars(&card.plain_text, 120))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| lemma.to_string())
}

fn summarize_cards_for_ai(cards: &[ImportedCard]) -> Option<String> {
    let summary = cards
        .iter()
        .map(|card| {
            format!(
                "{} / {}: {}",
                card.dictionary_name,
                card.headword,
                trim_to_chars(&card.plain_text, MAX_CARD_TEXT_CHARS)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if summary.trim().is_empty() {
        None
    } else {
        Some(summary)
    }
}

fn html_to_safe_text(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut tag = String::new();
    let mut skip_until: Option<&'static str> = None;

    for char in html.chars() {
        if let Some(end_tag) = skip_until {
            tag.push(char.to_ascii_lowercase());
            if tag.ends_with(end_tag) {
                skip_until = None;
                tag.clear();
            }
            continue;
        }

        match char {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' if in_tag => {
                let tag_name = tag
                    .trim()
                    .trim_start_matches('/')
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if tag_name == "script" {
                    skip_until = Some("</script>");
                } else if tag_name == "style" {
                    skip_until = Some("</style>");
                }
                in_tag = false;
                output.push(' ');
                tag.clear();
            }
            _ if in_tag => tag.push(char),
            _ => output.push(char),
        }
    }

    decode_basic_entities(&output)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_basic_entities(text: &str) -> String {
    text.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn builtin_mdx_path(app: &AppHandle) -> Result<Option<PathBuf>, DictionaryError> {
    let resource_path = app.path().resource_dir()?.join(BUILTIN_ECDICT_RESOURCE);
    if resource_path.is_file() {
        return Ok(Some(resource_path));
    }

    let dev_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(BUILTIN_ECDICT_RESOURCE);
    if dev_path.is_file() {
        return Ok(Some(dev_path));
    }

    Ok(None)
}

fn dictionary_db_path(app: &AppHandle) -> Result<PathBuf, DictionaryError> {
    Ok(app.path().app_data_dir()?.join("dictionary.sqlite"))
}

fn normalize_word(word: &str) -> String {
    word.trim()
        .trim_matches(|char: char| !char.is_ascii_alphanumeric() && char != '\'' && char != '-')
        .to_ascii_lowercase()
}

fn trim_to_chars(text: &str, max_chars: usize) -> String {
    let mut output: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        output.push('…');
    }
    output
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Debug, thiserror::Error)]
pub enum DictionaryError {
    #[error("词典数据库错误：{0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("词典 JSON 错误：{0}")]
    Json(#[from] serde_json::Error),
    #[error("词典文件读写错误：{0}")]
    Io(#[from] std::io::Error),
    #[error("词典路径错误：{0}")]
    Tauri(#[from] tauri::Error),
    #[error("内置 ECDICT MDX 未安装，请将 ecdict.mdx 放入应用资源目录后重建索引")]
    MissingBuiltinMdx,
    #[error("内置词典不能删除")]
    CannotDeleteBuiltin,
    #[error("MDX 词典解析错误：{0}")]
    Mdict(String),
}

#[cfg(test)]
mod tests {
    use super::{html_to_safe_text, parse_plain_text_definitions, trim_to_chars, DictionaryStore};
    use rusqlite::Connection;

    #[test]
    fn parses_plain_text_definitions() {
        let definitions = parse_plain_text_definitions("vt. 实施；执行\nn. 工具；器具", "ECDICT");
        assert_eq!(definitions.len(), 4);
        assert_eq!(definitions[0].part_of_speech, "vt.");
        assert_eq!(definitions[0].meaning, "实施");
    }

    #[test]
    fn strips_html_to_safe_text() {
        assert_eq!(
            html_to_safe_text("<div>Hello&nbsp;<b>world</b><script>alert(1)</script></div>"),
            "Hello world"
        );
    }

    #[test]
    fn trims_by_chars() {
        assert_eq!(trim_to_chars("abcdef", 3), "abc…");
        assert_eq!(trim_to_chars("abc", 3), "abc");
    }

    #[test]
    fn migrates_legacy_dictionaries_table_before_indexing_builtin_key() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            create table dictionaries (
              id integer primary key autoincrement,
              name text not null,
              kind text not null,
              path text,
              entry_count integer not null default 0,
              created_at integer not null
            );
            "#,
        )
        .unwrap();

        let store = DictionaryStore { conn };
        store.ensure_schema().unwrap();

        let builtin_key_count: i64 = store
            .conn
            .query_row(
                "select count(*) from pragma_table_info('dictionaries') where name = 'builtin_key'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(builtin_key_count, 1);
    }
}
