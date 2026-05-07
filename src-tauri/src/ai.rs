use crate::{dictionary, settings::AppSettings};
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordAnalysis {
    pub translated: String,
    pub lemma: String,
}

pub async fn analyze_word(settings: &AppSettings, word: &str) -> Result<WordAnalysis, AiError> {
    let response = complete_with_prompt(
        settings,
        "你是一个准确的英语词形还原和翻译助手。只输出 JSON，不要输出 Markdown。",
        &format!(
            "分析下面这个英文单词，把它翻译成{}，并给出用于查词典的英文原型/词元。\
             只输出严格 JSON，格式为 {{\"translated\":\"译文\",\"lemma\":\"英文原型\"}}。\
             如果它已经是原型，lemma 就返回它本身。\n\n单词：{}",
            settings.target_language, word
        ),
        Duration::from_secs(30),
    )
    .await?;
    let analysis = parse_word_analysis(&response)?;
    log::debug!(
        "AI word analysis parsed; word_chars={} lemma={} translated_chars={}",
        word.chars().count(),
        analysis.lemma,
        analysis.translated.chars().count()
    );
    Ok(analysis)
}

pub async fn enrich_word_card(
    settings: &AppSettings,
    source: &str,
    lemma: &str,
    local_dictionary_summary: Option<&str>,
) -> Result<dictionary::AiWordCard, AiError> {
    let local_context = local_dictionary_summary
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("本地 MDX 词典安全文本摘要：\n{value}"))
        .unwrap_or_else(|| "本地 MDX 词典未命中。".to_string());

    let response = complete_with_prompt(
        settings,
        "你是一个严谨的英语学习词典卡片生成助手。只输出 JSON，不要输出 Markdown。",
        &format!(
            "请基于事实数据为英文单词生成学习卡片。目标语言：{}。\
             不要编造专有词典来源；不确定的同反义词可以留空数组。\
             只输出严格 JSON，格式为：\
             {{\"shortDefinition\":\"简明释义\",\"memoryHint\":\"记忆提示\",\
             \"phrases\":[{{\"phrase\":\"短语\",\"meaning\":\"含义\"}}],\
             \"examples\":[{{\"en\":\"英文例句\",\"zh\":\"中文翻译\",\"source\":\"AI\"}}],\
             \"synonyms\":[\"同义词\"],\"antonyms\":[\"反义词\"]}}。\n\n\
             原输入：{}\n词典原型：{}\n{}",
            settings.target_language, source, lemma, local_context
        ),
        Duration::from_secs(30),
    )
    .await?;

    parse_ai_word_card(&response)
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
    let prompt = format!(
        "请把下面文本翻译成{}。只输出译文，必要时保留专有名词：\n\n{}",
        settings.target_language, text
    );

    complete_with_prompt(settings, system_prompt, &prompt, Duration::from_secs(30)).await
}

async fn complete_with_prompt(
    settings: &AppSettings,
    system_prompt: &str,
    user_prompt: &str,
    request_timeout: Duration,
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
        user_prompt.chars().count(),
        settings.temperature
    );

    let config = OpenAIConfig::new()
        .with_api_key(settings.api_key.trim())
        .with_api_base(base_url.clone());
    let client = Client::with_config(config);

    let request = CreateChatCompletionRequestArgs::default()
        .model(settings.model.trim())
        .temperature(settings.temperature)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt)
                .build()?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_prompt)
                .build()?
                .into(),
        ])
        .build()?;

    let response: CompatibleChatCompletionResponse =
        timeout(request_timeout, client.chat().create_byot(request))
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

fn parse_word_analysis(content: &str) -> Result<WordAnalysis, AiError> {
    let json = extract_json_object(content).ok_or_else(|| {
        log::warn!("AI word analysis response did not contain JSON object");
        AiError::InvalidStructuredResponse
    })?;
    let mut analysis: WordAnalysis = serde_json::from_str(json).map_err(|error| {
        log::warn!("AI word analysis JSON parse failed; error={error}");
        AiError::InvalidStructuredResponse
    })?;
    analysis.translated = analysis.translated.trim().to_string();
    analysis.lemma = analysis.lemma.trim().to_ascii_lowercase();

    if analysis.translated.is_empty() || analysis.lemma.is_empty() {
        log::warn!("AI word analysis response missing translated or lemma");
        return Err(AiError::InvalidStructuredResponse);
    }

    Ok(analysis)
}

fn parse_ai_word_card(content: &str) -> Result<dictionary::AiWordCard, AiError> {
    let json = extract_json_object(content).ok_or_else(|| {
        log::warn!("AI word card response did not contain JSON object");
        AiError::InvalidStructuredResponse
    })?;
    serde_json::from_str(json).map_err(|error| {
        log::warn!("AI word card JSON parse failed; error={error}");
        AiError::InvalidStructuredResponse
    })
}

fn extract_json_object(content: &str) -> Option<&str> {
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if start > end {
        return None;
    }
    Some(&content[start..=end])
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
    use super::{
        extract_translation, normalize_base_url, parse_ai_word_card, parse_word_analysis,
        CompatibleChatCompletionResponse,
    };

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

    #[test]
    fn parses_word_analysis_from_plain_json() {
        let analysis =
            parse_word_analysis(r#"{"translated":"运行","lemma":"run"}"#).expect("valid json");

        assert_eq!(analysis.translated, "运行");
        assert_eq!(analysis.lemma, "run");
    }

    #[test]
    fn parses_word_analysis_from_markdown_wrapped_json() {
        let analysis =
            parse_word_analysis("```json\n{\"translated\":\"翻译\",\"lemma\":\"translate\"}\n```")
                .expect("wrapped json");

        assert_eq!(analysis.translated, "翻译");
        assert_eq!(analysis.lemma, "translate");
    }

    #[test]
    fn parses_ai_word_card_json() {
        let card = parse_ai_word_card(
            r#"{"shortDefinition":"实施；执行","memoryHint":"im + ple + ment","phrases":[{"phrase":"implement a plan","meaning":"实施计划"}],"examples":[{"en":"We implement the plan.","zh":"我们执行计划。","source":"AI"}],"synonyms":["execute"],"antonyms":[]}"#,
        )
        .unwrap();

        assert_eq!(card.short_definition.as_deref(), Some("实施；执行"));
        assert_eq!(card.phrases[0].phrase, "implement a plan");
        assert_eq!(card.examples[0].source, "AI");
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
    #[error("AI 单词分析响应格式不正确")]
    InvalidStructuredResponse,
    #[error("AI 调用失败：{0}")]
    OpenAi(#[from] OpenAIError),
}
