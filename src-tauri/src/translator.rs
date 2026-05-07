use crate::{ai, dictionary, settings};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum LookupResult {
    Word { profile: dictionary::WordProfile },
    Translation { source: String, translated: String },
}

pub async fn lookup_text(app: &AppHandle, text: String) -> Result<LookupResult, TranslateError> {
    let source = text.trim().to_string();
    if source.is_empty() {
        log::warn!("lookup rejected empty input");
        return Err(TranslateError::EmptyInput);
    }

    log::debug!("lookup started; char_count={}", source.chars().count());
    if is_single_word(&source) {
        log::debug!("lookup routed to AI word analysis; word={source}");
        let settings = settings::load_settings(app)?;
        let analysis = match ai::analyze_word(&settings, &source).await {
            Ok(analysis) => {
                log::info!(
                    "AI word analysis completed; source={} lemma={}",
                    source,
                    analysis.lemma
                );
                analysis
            }
            Err(error) => {
                log::warn!(
                    "AI word analysis failed; falling back to local MDX lookup; source={} error={error}",
                    source
                );
                ai::WordAnalysis {
                    translated: String::new(),
                    lemma: source.to_ascii_lowercase(),
                }
            }
        };

        return Ok(LookupResult::Word {
            profile: dictionary::lookup_word_profile(
                app,
                &settings,
                dictionary::WordLookupSeed {
                    source,
                    translated: analysis.translated,
                    lemma: analysis.lemma,
                },
            )
            .await?,
        });
    }

    log::debug!("lookup routed to AI translation");
    let settings = settings::load_settings(app)?;
    let translated = ai::translate(&settings, &source).await?;
    log::info!(
        "AI translation completed; source_chars={} translated_chars={}",
        source.chars().count(),
        translated.chars().count()
    );
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
    #[error(transparent)]
    Dictionary(#[from] dictionary::DictionaryError),
}
