use crate::settings::AppSettings;
use serde::{Deserialize, Serialize};
use tokio::time::{timeout, Duration};

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

pub async fn translate(settings: &AppSettings, text: &str) -> Result<String, AiError> {
    if settings.api_key.trim().is_empty() {
        return Err(AiError::MissingApiKey);
    }

    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let prompt = format!(
        "请把下面文本翻译成{}。只输出译文，必要时保留专有名词：\n\n{}",
        settings.target_language, text
    );

    let request = ChatCompletionRequest {
        model: settings.model.clone(),
        temperature: settings.temperature,
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "你是一个准确、简洁的翻译助手。".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
    };

    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(30),
        client
            .post(endpoint)
            .bearer_auth(settings.api_key.trim())
            .json(&request)
            .send(),
    )
    .await
    .map_err(|_| AiError::Timeout)??;

    if !response.status().is_success() {
        let status = response.status();
        let message = response.text().await.unwrap_or_default();
        return Err(AiError::HttpStatus(status.as_u16(), message));
    }

    let completion: ChatCompletionResponse = response.json().await?;
    completion
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content.trim().to_string())
        .filter(|content| !content.is_empty())
        .ok_or(AiError::EmptyResponse)
}

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("请先配置 OpenAI-compatible API Key")]
    MissingApiKey,
    #[error("AI 翻译请求超时")]
    Timeout,
    #[error("AI 服务返回错误 {0}: {1}")]
    HttpStatus(u16, String),
    #[error("AI 服务响应为空")]
    EmptyResponse,
    #[error("网络请求失败：{0}")]
    Request(#[from] reqwest::Error),
}
