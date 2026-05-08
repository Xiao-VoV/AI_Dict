use super::{error::DictionaryError, models::DictionaryIndexProgress};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter};

pub(super) fn current_index_progress() -> Option<DictionaryIndexProgress> {
    lock_index_progress().clone()
}

pub(super) fn emit_index_progress(
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

pub(super) struct IndexTaskGuard;

impl Drop for IndexTaskGuard {
    fn drop(&mut self) {
        clear_index_progress();
    }
}

pub(super) fn begin_index_task(
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
