use super::{
    constants::INDEX_PROGRESS_BATCH_SIZE,
    error::DictionaryError,
    models::ImportSummary,
    progress::emit_index_progress,
    store::{DictionaryKind, DictionaryStore},
    text::{html_to_safe_text, is_indexable_mdx_headword, mdx_name_from_path},
    util::normalize_word,
};
use rusqlite::{params, Transaction};
use rust_mdict::Mdx;
use std::path::Path;
use tauri::AppHandle;

impl DictionaryStore {
    pub(super) fn import_mdx_dictionary(
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
