use crate::settings::AppSettings;
use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::chat::{
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use serde::Deserialize;
use tokio::time::{timeout, Duration};

pub async fn translate(settings: &AppSettings, text: &str) -> Result<String, AiError> {
    translate_with_system_prompt(settings, text, "你是一个准确、简洁的翻译助手。").await
}

pub async fn test_connection(settings: &AppSettings) -> Result<(), AiError> {
    let response = translate_with_system_prompt(
        settings,
        "connection test",
        "你是一个 API 连通性检测助手。请只回复 ok。",
    )
    .await?;

    log::info!(
        "AI connection test succeeded; response_chars={}",
        response.chars().count()
    );
    Ok(())
}

async fn translate_with_system_prompt(
    settings: &AppSettings,
    text: &str,
    system_prompt: &str,
) -> Result<String, AiError> {
    if settings.api_key.trim().is_empty() {
        log::warn!("AI translation rejected missing API key");
        return Err(AiError::MissingApiKey);
    }
    if settings.model.trim().is_empty() {
        log::warn!("AI translation rejected missing model");
        return Err(AiError::MissingModel);
    }

    let base_url = normalize_base_url(&settings.base_url);
    if base_url.is_empty() {
        log::warn!("AI translation rejected missing base URL");
        return Err(AiError::MissingBaseUrl);
    }

    log::debug!(
        "AI translation request preparing; base_url={} model={} target_language={} source_chars={} temperature={}",
        base_url,
        settings.model.trim(),
        settings.target_language,
        text.chars().count(),
        settings.temperature
    );

    let config = OpenAIConfig::new()
        .with_api_key(settings.api_key.trim())
        .with_api_base(base_url.clone());
    let client = Client::with_config(config);

    let prompt = format!(
        "请把下面文本翻译成{}。只输出译文，必要时保留专有名词：\n\n{}",
        settings.target_language, text
    );

    let request = CreateChatCompletionRequestArgs::default()
        .model(settings.model.trim())
        .temperature(settings.temperature)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(prompt)
                .build()?
                .into(),
        ])
        .build()?;

    let response: CompatibleChatCompletionResponse =
        timeout(Duration::from_secs(30), client.chat().create_byot(request))
            .await
            .map_err(|_| {
                log::warn!(
                    "AI translation request timed out; base_url={} model={}",
                    base_url,
                    settings.model.trim()
                );
                AiError::Timeout
            })?
            .map_err(|error| {
                log::warn!(
                    "AI translation request failed; base_url={} model={} error={error}",
                    base_url,
                    settings.model.trim()
                );
                error
            })?;

    let translated = extract_translation(response)?;
    log::debug!(
        "AI translation response parsed; translated_chars={}",
        translated.chars().count()
    );
    Ok(translated)
}

#[derive(Debug, Deserialize)]
struct CompatibleChatCompletionResponse {
    choices: Vec<CompatibleChoice>,
}

#[derive(Debug, Deserialize)]
struct CompatibleChoice {
    message: CompatibleMessage,
}

#[derive(Debug, Deserialize)]
struct CompatibleMessage {
    content: Option<String>,
}

fn extract_translation(response: CompatibleChatCompletionResponse) -> Result<String, AiError> {
    response
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .map(|content| content.trim().to_string())
        .filter(|content| !content.is_empty())
        .ok_or(AiError::EmptyResponse)
}

fn normalize_base_url(base_url: &str) -> String {
    let trimmed = base_url.trim().trim_end_matches('/');
    trimmed
        .strip_suffix("/chat/completions")
        .unwrap_or(trimmed)
        .trim_end_matches('/')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{extract_translation, normalize_base_url, CompatibleChatCompletionResponse};

    #[test]
    fn strips_chat_completions_endpoint_from_base_url() {
        assert_eq!(
            normalize_base_url("https://api.example.com/v1/chat/completions"),
            "https://api.example.com/v1"
        );
        assert_eq!(
            normalize_base_url("https://api.example.com/v1/"),
            "https://api.example.com/v1"
        );
    }

    #[test]
    fn accepts_compatible_response_with_empty_role() {
        let response: CompatibleChatCompletionResponse =
            serde_json::from_str(r#"{"choices":[{"message":{"role":"","content":"你好，测试"}}]}"#)
                .unwrap();

        assert_eq!(extract_translation(response).unwrap(), "你好，测试");
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("请先配置 OpenAI-compatible API Key")]
    MissingApiKey,
    #[error("请先配置模型名称")]
    MissingModel,
    #[error("请先配置 API Base URL，例如 https://api.openai.com/v1")]
    MissingBaseUrl,
    #[error("AI 翻译请求超时")]
    Timeout,
    #[error("AI 服务响应为空")]
    EmptyResponse,
    #[error("AI 调用失败：{0}")]
    OpenAi(#[from] OpenAIError),
}
