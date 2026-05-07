use crate::{ai, dictionary, settings};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum LookupResult {
    Dictionary {
        source: String,
        entry: dictionary::DictionaryEntry,
    },
    Translation {
        source: String,
        translated: String,
    },
    DictionaryMiss {
        source: String,
        message: String,
    },
}

pub async fn lookup_text(app: &AppHandle, text: String) -> Result<LookupResult, TranslateError> {
    let source = text.trim().to_string();
    if source.is_empty() {
        return Err(TranslateError::EmptyInput);
    }

    if is_single_word(&source) {
        if let Some(entry) = dictionary::lookup(&source) {
            return Ok(LookupResult::Dictionary { source, entry });
        }

        return Ok(LookupResult::DictionaryMiss {
            source,
            message: "本地词典暂未收录该词。".to_string(),
        });
    }

    let settings = settings::load_settings(app)?;
    let translated = ai::translate(&settings, &source).await?;
    Ok(LookupResult::Translation { source, translated })
}

fn is_single_word(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.len() > 48 || trimmed.split_whitespace().count() != 1 {
        return false;
    }
    trimmed
        .chars()
        .all(|char| char.is_ascii_alphabetic() || char == '\'' || char == '-')
}

#[derive(Debug, thiserror::Error)]
pub enum TranslateError {
    #[error("请输入或选中要翻译的文本")]
    EmptyInput,
    #[error(transparent)]
    Settings(#[from] settings::SettingsError),
    #[error(transparent)]
    Ai(#[from] ai::AiError),
}
