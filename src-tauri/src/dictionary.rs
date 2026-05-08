use crate::{ai, settings};
use tauri::AppHandle;

mod builtin;
mod constants;
mod error;
mod import;
mod models;
mod profile;
mod progress;
mod store;
mod text;
mod util;

pub use error::DictionaryError;
pub use models::{
    AiWordCard, DictionaryIndexProgress, DictionaryMetadata, ImportSummary, WordLookupSeed,
    WordProfile,
};

use constants::AI_CARD_PROMPT_VERSION;
use profile::{build_profile, fallback_translated, summarize_cards_for_ai};
use progress::begin_index_task;
use store::{DictionaryKind, DictionaryStore};
use text::mdx_name_from_path;
use util::normalize_word;

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
    let name = mdx_name_from_path(std::path::Path::new(mdx_path));
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
    progress::current_index_progress()
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

#[cfg(test)]
mod tests {
    use super::{
        models::ImportedCard,
        profile::{
            fallback_translated, parse_ecdict_forms, parse_ecdict_phonetics, parse_ecdict_tags,
            parse_plain_text_definitions,
        },
        store::DictionaryStore,
        text::{html_to_safe_text, is_indexable_mdx_headword},
        util::trim_to_chars,
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
    fn translated_fallback_uses_only_local_chinese_definition() {
        let cards = vec![ImportedCard {
            dictionary_id: 1,
            dictionary_name: "ECDICT".to_string(),
            headword: "world".to_string(),
            plain_text:
                "n. 世界, 地球, 宇宙, 万物\nn. people in general; especially a distinctive group\nphonetic: wɜːld"
                    .to_string(),
        }];

        assert_eq!(
            fallback_translated("", &cards, "world"),
            "世界, 地球, 宇宙, 万物"
        );
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
