use super::{
    error::DictionaryError,
    models::{AiWordCard, DictionaryMetadata},
    util::{dictionary_db_path, now_ts},
};
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use tauri::AppHandle;

pub(super) enum DictionaryKind {
    BuiltinSqlite,
    UserMdx,
}

impl DictionaryKind {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::BuiltinSqlite => "builtin_sqlite",
            Self::UserMdx => "user_mdx",
        }
    }
}

pub(super) struct DictionaryStore {
    pub(super) conn: Connection,
}

impl DictionaryStore {
    pub(super) fn open(app: &AppHandle) -> Result<Self, DictionaryError> {
        let db_path = dictionary_db_path(app)?;
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        Ok(Self { conn })
    }

    pub(super) fn ensure_schema(&self) -> Result<(), DictionaryError> {
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

    pub(super) fn create_dictionary(
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

    pub(super) fn get_ai_card(
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

    pub(super) fn save_ai_card(
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

    pub(super) fn list_dictionaries(&self) -> Result<Vec<DictionaryMetadata>, DictionaryError> {
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

    pub(super) fn delete_dictionary(&self, dictionary_id: i64) -> Result<(), DictionaryError> {
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
