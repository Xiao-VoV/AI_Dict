use crate::{ai, settings};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use rust_mdict::Mdx;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager};

const BUILTIN_ECDICT_KEY: &str = "ecdict";
const BUILTIN_ECDICT_NAME: &str = "ECDICT";
const BUILTIN_ECDICT_RESOURCE: &str = "resources/dicts/ecdict.db";
const AI_CARD_PROMPT_VERSION: &str = "word-card-v3-ecdict-sqlite";
const MAX_CARD_TEXT_CHARS: usize = 1_200;
const MAX_IMPORTED_CARDS: usize = 5;
const INDEX_PROGRESS_BATCH_SIZE: usize = 2_000;

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
pub struct DictionaryIndexProgress {
    pub name: String,
    pub kind: String,
    pub phase: String,
    pub processed_entries: usize,
    pub total_entries: usize,
    pub imported_entries: usize,
    pub skipped_entries: usize,
    pub done: bool,
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

struct EcdictEntry {
    word: String,
    phonetic: Option<String>,
    definition: Option<String>,
    translation: Option<String>,
    pos: Option<String>,
    tag: Option<String>,
    exchange: Option<String>,
}

impl EcdictEntry {
    fn to_plain_text(&self) -> String {
        let mut lines = Vec::new();
        push_optional_lines(&mut lines, self.translation.as_deref());
        push_optional_lines(&mut lines, self.definition.as_deref());
        push_labeled_line(&mut lines, "phonetic", self.phonetic.as_deref());
        push_labeled_line(&mut lines, "exchange", self.exchange.as_deref());
        push_labeled_line(&mut lines, "tag", self.tag.as_deref());
        push_labeled_line(&mut lines, "pos", self.pos.as_deref());
        lines.join("\n")
    }
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
    let name = mdx_name_from_path(Path::new(mdx_path));
    let _index_task = begin_index_task(app, &name, DictionaryKind::UserMdx.as_str())?;
    let mut db = DictionaryStore::open(app)?;
    db.ensure_schema()?;
    db.import_mdx_dictionary(app, mdx_path, DictionaryKind::UserMdx)
}

pub fn reindex_builtin_dictionary(app: &AppHandle) -> Result<ImportSummary, DictionaryError> {
    let mut db = DictionaryStore::open(app)?;
    db.ensure_schema()?;
    db.reindex_builtin_dictionary(app)
}

pub fn current_index_progress() -> Option<DictionaryIndexProgress> {
    lock_index_progress().clone()
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
    BuiltinSqlite,
    UserMdx,
}

impl DictionaryKind {
    fn as_str(&self) -> &'static str {
        match self {
            Self::BuiltinSqlite => "builtin_sqlite",
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
        self.conn.execute(
            &format!("alter table {table} add column {column} {definition}"),
            [],
        )?;
        Ok(())
    }

    fn ensure_builtin_dictionary(&self, app: &AppHandle) -> Result<(), DictionaryError> {
        self.ensure_schema()?;
        let path = builtin_ecdict_db_path(app)?;
        let path_string = path.as_ref().map(|path| path.to_string_lossy().to_string());
        let existing = self
            .conn
            .query_row(
                "select id, path, entry_count from dictionaries where builtin_key = ?1",
                [BUILTIN_ECDICT_KEY],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, i64>(2)?,
                    ))
                },
            )
            .optional()?;

        let entry_count = match (&path, &path_string, &existing) {
            (Some(path), Some(path_string), Some((_, existing_path, existing_count)))
                if existing_path.as_deref() == Some(path_string.as_str())
                    && *existing_count > 0 =>
            {
                *existing_count
            }
            (Some(path), _, _) => ecdict_entry_count(path)?,
            (None, _, _) => {
                log::warn!(
                    "built-in ECDICT SQLite resource not found; expected {}",
                    BUILTIN_ECDICT_RESOURCE
                );
                0
            }
        };

        if let Some((dictionary_id, _, _)) = existing {
            self.conn.execute(
                "update dictionaries
                 set name = ?1, kind = ?2, path = ?3, entry_count = ?4
                 where id = ?5",
                params![
                    BUILTIN_ECDICT_NAME,
                    DictionaryKind::BuiltinSqlite.as_str(),
                    path_string.as_deref(),
                    entry_count,
                    dictionary_id
                ],
            )?;
        } else {
            self.create_dictionary(
                BUILTIN_ECDICT_NAME,
                DictionaryKind::BuiltinSqlite.as_str(),
                path_string.as_deref(),
                Some(BUILTIN_ECDICT_KEY),
                entry_count,
            )?;
        }
        Ok(())
    }

    fn reindex_builtin_dictionary(
        &mut self,
        app: &AppHandle,
    ) -> Result<ImportSummary, DictionaryError> {
        self.ensure_schema()?;
        let Some(path) = builtin_ecdict_db_path(app)? else {
            self.ensure_builtin_dictionary(app)?;
            return Err(DictionaryError::MissingBuiltinDb);
        };
        let entry_count = ecdict_entry_count(&path)?;
        let path_string = path.to_string_lossy().to_string();
        let dictionary_id = self.upsert_builtin_dictionary(&path_string, entry_count)?;
        Ok(ImportSummary {
            dictionary_id: Some(dictionary_id),
            name: BUILTIN_ECDICT_NAME.to_string(),
            kind: DictionaryKind::BuiltinSqlite.as_str().to_string(),
            imported_entries: entry_count as usize,
            skipped_entries: 0,
        })
    }

    fn import_mdx_dictionary(
        &mut self,
        app: &AppHandle,
        mdx_path: &str,
        kind: DictionaryKind,
    ) -> Result<ImportSummary, DictionaryError> {
        let path = Path::new(mdx_path);
        let name = mdx_name_from_path(path);
        self.import_mdx_dictionary_at_path(app, path, &name, kind, None)
    }

    fn import_mdx_dictionary_at_path(
        &mut self,
        app: &AppHandle,
        path: &Path,
        name: &str,
        kind: DictionaryKind,
        builtin_key: Option<&str>,
    ) -> Result<ImportSummary, DictionaryError> {
        let path_string = path.to_string_lossy().to_string();
        let kind_label = kind.as_str().to_string();

        emit_index_progress(app, name, &kind_label, "opening", 0, 0, 0, 0, false);
        let mut mdx = match Mdx::new(path) {
            Ok(mdx) => mdx,
            Err(error) => {
                emit_index_progress(app, name, &kind_label, "error", 0, 0, 0, 0, true);
                return Err(DictionaryError::Mdict(error.to_string()));
            }
        };

        let dictionary_id =
            self.create_dictionary(name, kind.as_str(), Some(&path_string), builtin_key, 0)?;

        let mut imported_entries = 0usize;
        let mut skipped_entries = 0usize;
        let total_entries = mdx.keyword_count();

        emit_index_progress(
            app,
            name,
            &kind_label,
            "indexing",
            0,
            total_entries,
            imported_entries,
            skipped_entries,
            false,
        );

        let transaction = self.conn.transaction()?;
        for index in 0..total_entries {
            let keyword = mdx.keyword_list()[index].clone();
            if !is_indexable_mdx_headword(&keyword.key_text) {
                skipped_entries += 1;
            } else if let Some(result) = mdx.fetch(&keyword) {
                let plain_text = html_to_safe_text(&result.definition);
                if plain_text.is_empty() {
                    skipped_entries += 1;
                } else {
                    insert_mdx_entry(&transaction, dictionary_id, &result.key_text, &plain_text)?;
                    imported_entries += 1;
                }
            } else {
                skipped_entries += 1;
            }

            let processed_entries = index + 1;
            if processed_entries == total_entries
                || processed_entries % INDEX_PROGRESS_BATCH_SIZE == 0
            {
                emit_index_progress(
                    app,
                    name,
                    &kind_label,
                    "indexing",
                    processed_entries,
                    total_entries,
                    imported_entries,
                    skipped_entries,
                    false,
                );
            }
        }
        transaction.execute(
            "update dictionaries set entry_count = ?1 where id = ?2",
            params![imported_entries as i64, dictionary_id],
        )?;
        transaction.commit()?;

        emit_index_progress(
            app,
            name,
            &kind_label,
            "done",
            total_entries,
            total_entries,
            imported_entries,
            skipped_entries,
            true,
        );

        Ok(ImportSummary {
            dictionary_id: Some(dictionary_id),
            name: name.to_string(),
            kind: kind_label,
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
        entry_count: i64,
    ) -> Result<i64, DictionaryError> {
        self.conn.execute(
            "insert into dictionaries (name, kind, path, entry_count, created_at, builtin_key)
             values (?1, ?2, ?3, ?4, ?5, ?6)",
            params![name, kind, path, entry_count, now_ts(), builtin_key],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    fn upsert_builtin_dictionary(
        &self,
        path: &str,
        entry_count: i64,
    ) -> Result<i64, DictionaryError> {
        if let Some(dictionary_id) = self
            .conn
            .query_row(
                "select id from dictionaries where builtin_key = ?1",
                [BUILTIN_ECDICT_KEY],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
        {
            self.conn.execute(
                "update dictionaries
                 set name = ?1, kind = ?2, path = ?3, entry_count = ?4
                 where id = ?5",
                params![
                    BUILTIN_ECDICT_NAME,
                    DictionaryKind::BuiltinSqlite.as_str(),
                    path,
                    entry_count,
                    dictionary_id
                ],
            )?;
            Ok(dictionary_id)
        } else {
            self.create_dictionary(
                BUILTIN_ECDICT_NAME,
                DictionaryKind::BuiltinSqlite.as_str(),
                Some(path),
                Some(BUILTIN_ECDICT_KEY),
                entry_count,
            )
        }
    }

    fn lookup_cards(&self, word: &str) -> Result<Vec<ImportedCard>, DictionaryError> {
        let normalized = normalize_word(word);
        if normalized.is_empty() {
            return Ok(Vec::new());
        }

        let mut cards = self.lookup_builtin_ecdict_card(&normalized)?;
        let remaining_limit = MAX_IMPORTED_CARDS.saturating_sub(cards.len());
        if remaining_limit == 0 {
            return Ok(cards);
        }

        let mut stmt = self.conn.prepare(
            r#"
            select e.dictionary_id, d.name, e.headword, e.plain_text
            from mdx_entries e
            join dictionaries d on d.id = e.dictionary_id
            where e.normalized_word = ?1
              and d.kind = 'user_mdx'
            order by d.created_at asc
            limit ?2
            "#,
        )?;
        let rows = stmt.query_map(params![normalized, remaining_limit as i64], |row| {
            Ok(ImportedCard {
                dictionary_id: row.get(0)?,
                dictionary_name: row.get(1)?,
                headword: row.get(2)?,
                plain_text: row.get(3)?,
            })
        })?;

        cards.extend(rows.collect::<Result<Vec<_>, _>>()?);
        Ok(cards)
    }

    fn lookup_builtin_ecdict_card(&self, word: &str) -> Result<Vec<ImportedCard>, DictionaryError> {
        let Some((dictionary_id, path)) = self
            .conn
            .query_row(
                "select id, path from dictionaries where builtin_key = ?1 and kind = ?2",
                params![BUILTIN_ECDICT_KEY, DictionaryKind::BuiltinSqlite.as_str()],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?
        else {
            return Ok(Vec::new());
        };
        let Some(path) = path else {
            return Ok(Vec::new());
        };

        let conn = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        let entry = conn
            .query_row(
                r#"
                select word, phonetic, definition, translation, pos, tag, exchange
                from stardict
                where word = ?1 collate nocase or sw = ?1 collate nocase
                order by case when word = ?1 collate nocase then 0 else 1 end
                limit 1
                "#,
                [word],
                |row| {
                    Ok(EcdictEntry {
                        word: row.get(0)?,
                        phonetic: row.get(1)?,
                        definition: row.get(2)?,
                        translation: row.get(3)?,
                        pos: row.get(4)?,
                        tag: row.get(5)?,
                        exchange: row.get(6)?,
                    })
                },
            )
            .optional()?;

        Ok(entry
            .map(|entry| {
                vec![ImportedCard {
                    dictionary_id,
                    dictionary_name: BUILTIN_ECDICT_NAME.to_string(),
                    headword: entry.word.clone(),
                    plain_text: entry.to_plain_text(),
                }]
            })
            .unwrap_or_default())
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
             where kind in ('builtin_sqlite', 'builtin_mdx', 'user_mdx')
             order by case when builtin_key is not null then 0 else 1 end, created_at asc",
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
        if kind.as_deref() == Some(DictionaryKind::BuiltinSqlite.as_str())
            || kind.as_deref() == Some("builtin_mdx")
        {
            return Err(DictionaryError::CannotDeleteBuiltin);
        }

        self.conn.execute(
            "delete from mdx_entries where dictionary_id = ?1",
            [dictionary_id],
        )?;
        self.conn
            .execute("delete from dictionaries where id = ?1", [dictionary_id])?;
        Ok(())
    }
}

fn insert_mdx_entry(
    transaction: &Transaction<'_>,
    dictionary_id: i64,
    headword: &str,
    plain_text: &str,
) -> Result<(), DictionaryError> {
    transaction.execute(
        r#"
        insert into mdx_entries
          (dictionary_id, headword, normalized_word, plain_text)
        values (?1, ?2, ?3, ?4)
        "#,
        params![
            dictionary_id,
            headword,
            normalize_word(headword),
            plain_text
        ],
    )?;
    Ok(())
}

fn emit_index_progress(
    app: &AppHandle,
    name: &str,
    kind: &str,
    phase: &str,
    processed_entries: usize,
    total_entries: usize,
    imported_entries: usize,
    skipped_entries: usize,
    done: bool,
) {
    let payload = build_index_progress(
        name,
        kind,
        phase,
        processed_entries,
        total_entries,
        imported_entries,
        skipped_entries,
        done,
    );
    set_index_progress(&payload);
    if let Err(error) = app.emit("dictionary-index-progress", payload) {
        log::warn!("failed to emit dictionary index progress: {error}");
    }
}

struct IndexTaskGuard;

impl Drop for IndexTaskGuard {
    fn drop(&mut self) {
        clear_index_progress();
    }
}

fn begin_index_task(
    app: &AppHandle,
    name: &str,
    kind: &str,
) -> Result<IndexTaskGuard, DictionaryError> {
    let payload = build_index_progress(name, kind, "opening", 0, 0, 0, 0, false);
    {
        let mut progress = lock_index_progress();
        if let Some(active) = progress.as_ref() {
            return Err(DictionaryError::IndexTaskAlreadyRunning(
                active.name.clone(),
            ));
        }
        *progress = Some(payload.clone());
    }
    if let Err(error) = app.emit("dictionary-index-progress", payload) {
        log::warn!("failed to emit dictionary index progress: {error}");
    }
    Ok(IndexTaskGuard)
}

fn build_index_progress(
    name: &str,
    kind: &str,
    phase: &str,
    processed_entries: usize,
    total_entries: usize,
    imported_entries: usize,
    skipped_entries: usize,
    done: bool,
) -> DictionaryIndexProgress {
    DictionaryIndexProgress {
        name: name.to_string(),
        kind: kind.to_string(),
        phase: phase.to_string(),
        processed_entries,
        total_entries,
        imported_entries,
        skipped_entries,
        done,
    }
}

fn set_index_progress(payload: &DictionaryIndexProgress) {
    let mut progress = lock_index_progress();
    if payload.done {
        *progress = None;
    } else {
        *progress = Some(payload.clone());
    }
}

fn clear_index_progress() {
    *lock_index_progress() = None;
}

fn lock_index_progress() -> std::sync::MutexGuard<'static, Option<DictionaryIndexProgress>> {
    static INDEX_PROGRESS: OnceLock<Mutex<Option<DictionaryIndexProgress>>> = OnceLock::new();
    INDEX_PROGRESS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn mdx_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("MDX Dictionary")
        .to_string()
}

fn is_indexable_mdx_headword(headword: &str) -> bool {
    let normalized = normalize_word(headword);
    !normalized.is_empty()
        && normalized.len() <= 80
        && normalized
            .chars()
            .all(|char| char.is_ascii_alphabetic() || char == '\'' || char == '-')
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
    let mut phonetics = Phonetics::default();
    let mut forms = WordForms::default();
    let mut exam_tags = Vec::new();

    for card in &imported_cards {
        sources.insert(card.dictionary_name.clone());
    }

    if let Some(first_card) = imported_cards.first() {
        if first_card.dictionary_name == BUILTIN_ECDICT_NAME {
            phonetics = parse_ecdict_phonetics(&first_card.plain_text);
            forms = parse_ecdict_forms(&first_card.plain_text);
            exam_tags = parse_ecdict_tags(&first_card.plain_text);
        }
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
        phonetics,
        definitions,
        forms,
        examples,
        phrases,
        synonyms,
        antonyms,
        memory_hint,
        exam_tags,
        imported_cards,
        sources: sources.into_iter().collect(),
    }
}

fn push_optional_lines(lines: &mut Vec<String>, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        lines.extend(
            value
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToString::to_string),
        );
    }
}

fn push_labeled_line(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) {
        lines.push(format!("{label}: {value}"));
    }
}

fn parse_plain_text_definitions(text: &str, source: &str) -> Vec<Definition> {
    text.lines()
        .flat_map(|line| line.split("；"))
        .filter_map(|chunk| {
            let trimmed = chunk.trim();
            if trimmed.is_empty() || is_ecdict_metadata_line(trimmed) {
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

fn is_ecdict_metadata_line(line: &str) -> bool {
    line.strip_prefix("phonetic:")
        .or_else(|| line.strip_prefix("exchange:"))
        .or_else(|| line.strip_prefix("tag:"))
        .or_else(|| line.strip_prefix("pos:"))
        .is_some()
}

fn parse_ecdict_phonetics(text: &str) -> Phonetics {
    let phonetic = metadata_line_value(text, "phonetic").map(ToString::to_string);
    Phonetics {
        uk: phonetic.clone(),
        us: phonetic,
        audio: None,
    }
}

fn parse_ecdict_forms(text: &str) -> WordForms {
    let mut forms = WordForms::default();
    let Some(exchange) = metadata_line_value(text, "exchange") else {
        return forms;
    };
    for item in exchange.split('/') {
        let Some((kind, value)) = item.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        match kind {
            "p" => forms.past = Some(value.to_string()),
            "d" => forms.past_participle = Some(value.to_string()),
            "i" => forms.present_participle = Some(value.to_string()),
            "3" => forms.third_person = Some(value.to_string()),
            "s" => forms.plural = Some(value.to_string()),
            "r" => forms.comparative = Some(value.to_string()),
            "t" => forms.superlative = Some(value.to_string()),
            _ => {}
        }
    }
    forms
}

fn parse_ecdict_tags(text: &str) -> Vec<String> {
    metadata_line_value(text, "tag")
        .map(|tags| {
            tags.split_whitespace()
                .filter(|tag| !tag.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn metadata_line_value<'a>(text: &'a str, label: &str) -> Option<&'a str> {
    let prefix = format!("{label}:");
    text.lines()
        .find_map(|line| line.trim().strip_prefix(&prefix).map(str::trim))
        .filter(|value| !value.is_empty())
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

fn builtin_ecdict_db_path(app: &AppHandle) -> Result<Option<PathBuf>, DictionaryError> {
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

fn ecdict_entry_count(path: &Path) -> Result<i64, DictionaryError> {
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    conn.query_row("select count(*) from stardict", [], |row| row.get(0))
        .map_err(DictionaryError::from)
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
    #[error("内置 ECDICT SQLite 未安装，请将 ecdict.db 放入应用资源目录")]
    MissingBuiltinDb,
    #[error("内置词典不能删除")]
    CannotDeleteBuiltin,
    #[error("词典索引任务正在运行：{0}")]
    IndexTaskAlreadyRunning(String),
    #[error("MDX 词典解析错误：{0}")]
    Mdict(String),
}

#[cfg(test)]
mod tests {
    use super::{
        html_to_safe_text, is_indexable_mdx_headword, parse_ecdict_forms, parse_ecdict_phonetics,
        parse_ecdict_tags, parse_plain_text_definitions, trim_to_chars, DictionaryStore,
    };
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
    fn filters_mdx_headwords_to_single_english_words() {
        assert!(is_indexable_mdx_headword("running"));
        assert!(is_indexable_mdx_headword("mother-in-law"));
        assert!(is_indexable_mdx_headword("don't"));
        assert!(!is_indexable_mdx_headword("look up"));
        assert!(!is_indexable_mdx_headword("hello2"));
        assert!(!is_indexable_mdx_headword("中文"));
    }

    #[test]
    fn parses_ecdict_metadata_without_polluting_definitions() {
        let text =
            "n. 跑；赛跑\nphonetic: rʌn\nexchange: p:ran/i:running/d:run/3:runs/s:runs\ntag: zk gk";
        let definitions = parse_plain_text_definitions(text, "ECDICT");
        let phonetics = parse_ecdict_phonetics(text);
        let forms = parse_ecdict_forms(text);
        let tags = parse_ecdict_tags(text);

        assert_eq!(definitions.len(), 2);
        assert_eq!(definitions[0].meaning, "跑");
        assert_eq!(phonetics.uk.as_deref(), Some("rʌn"));
        assert_eq!(forms.past.as_deref(), Some("ran"));
        assert_eq!(forms.present_participle.as_deref(), Some("running"));
        assert_eq!(forms.third_person.as_deref(), Some("runs"));
        assert_eq!(tags, vec!["zk", "gk"]);
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
