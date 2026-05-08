use super::error::DictionaryError;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

pub(super) fn dictionary_db_path(app: &AppHandle) -> Result<PathBuf, DictionaryError> {
    Ok(app.path().app_data_dir()?.join("dictionary.sqlite"))
}

pub(super) fn normalize_word(word: &str) -> String {
    word.trim()
        .trim_matches(|char: char| !char.is_ascii_alphanumeric() && char != '\'' && char != '-')
        .to_ascii_lowercase()
}

pub(super) fn trim_to_chars(text: &str, max_chars: usize) -> String {
    let mut output: String = text.chars().take(max_chars).collect();
    if text.chars().count() > max_chars {
        output.push('…');
    }
    output
}

pub(super) fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}
