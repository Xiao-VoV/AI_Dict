use super::{
    constants::{
        BUILTIN_ECDICT_KEY, BUILTIN_ECDICT_NAME, BUILTIN_ECDICT_RESOURCE, MAX_IMPORTED_CARDS,
    },
    error::DictionaryError,
    models::{ImportSummary, ImportedCard},
    store::{DictionaryKind, DictionaryStore},
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

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

impl DictionaryStore {
    pub(super) fn ensure_builtin_dictionary(&self, app: &AppHandle) -> Result<(), DictionaryError> {
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

    pub(super) fn reindex_builtin_dictionary(
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

    pub(super) fn lookup_cards(&self, word: &str) -> Result<Vec<ImportedCard>, DictionaryError> {
        let normalized = super::util::normalize_word(word);
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
