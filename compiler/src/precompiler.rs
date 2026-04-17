//! 预编译器 - 使用可插拔 Provider 将自然语言展开为结构化逻辑

use crate::tai::TaiFile;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

const DEFAULT_TEMPERATURE: f32 = 0.0;
const DEFAULT_TIMEOUT_SECS: u64 = 60;
const DASHSCOPE_DEFAULT_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";
const DASHSCOPE_DEFAULT_MODEL: &str = "qwen-plus";
const OLLAMA_DEFAULT_BASE_URL: &str = "http://localhost:11434/v1";
const OLLAMA_DEFAULT_MODEL: &str = "qwen2.5-coder:latest";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    DashScope,
    Ollama,
    Custom,
}

impl ProviderKind {
    fn from_env(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "dashscope" | "bailian" => Ok(Self::DashScope),
            "ollama" => Ok(Self::Ollama),
            "custom" => Ok(Self::Custom),
            other => Err(format!("unsupported provider: {}", other)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrecompilerConfig {
    pub provider: ProviderKind,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    pub temperature: f32,
    pub timeout_secs: u64,
    pub max_tokens: Option<u32>,
}

impl PrecompilerConfig {
    pub fn from_env() -> Result<Self, String> {
        dotenv::dotenv().ok();

        let provider = ProviderKind::from_env(
            &env::var("TAILANG_LLM_PROVIDER").unwrap_or_else(|_| "dashscope".to_string()),
        )?;

        let model = env::var("TAILANG_LLM_MODEL").unwrap_or_else(|_| match provider {
            ProviderKind::DashScope => DASHSCOPE_DEFAULT_MODEL.to_string(),
            ProviderKind::Ollama => OLLAMA_DEFAULT_MODEL.to_string(),
            ProviderKind::Custom => DASHSCOPE_DEFAULT_MODEL.to_string(),
        });

        let base_url = match provider {
            ProviderKind::DashScope => env::var("DASHSCOPE_BASE_URL")
                .or_else(|_| env::var("TAILANG_LLM_BASE_URL"))
                .unwrap_or_else(|_| DASHSCOPE_DEFAULT_BASE_URL.to_string()),
            ProviderKind::Ollama => env::var("OLLAMA_BASE_URL")
                .or_else(|_| env::var("TAILANG_LLM_BASE_URL"))
                .unwrap_or_else(|_| OLLAMA_DEFAULT_BASE_URL.to_string()),
            ProviderKind::Custom => env::var("TAILANG_LLM_BASE_URL")
                .map_err(|_| "TAILANG_LLM_BASE_URL not found".to_string())?,
        };

        let api_key = match provider {
            ProviderKind::DashScope => Some(
                env::var("DASHSCOPE_API_KEY")
                    .or_else(|_| env::var("TAILANG_LLM_API_KEY"))
                    .map_err(|_| "DASHSCOPE_API_KEY or TAILANG_LLM_API_KEY not found".to_string())?,
            ),
            ProviderKind::Ollama => env::var("OLLAMA_API_KEY")
                .or_else(|_| env::var("TAILANG_LLM_API_KEY"))
                .ok(),
            ProviderKind::Custom => env::var("TAILANG_LLM_API_KEY").ok(),
        };

        let temperature = env::var("TAILANG_LLM_TEMPERATURE")
            .ok()
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(DEFAULT_TEMPERATURE);

        let timeout_secs = env::var("TAILANG_LLM_TIMEOUT_SECS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let max_tokens = env::var("TAILANG_LLM_MAX_TOKENS")
            .ok()
            .and_then(|value| value.parse::<u32>().ok());

        Ok(Self {
            provider,
            base_url: base_url.trim_end_matches('/').to_string(),
            model,
            api_key,
            temperature,
            timeout_secs,
            max_tokens,
        })
    }
}

impl Default for PrecompilerConfig {
    fn default() -> Self {
        Self::from_env().unwrap_or(Self {
            provider: ProviderKind::DashScope,
            base_url: DASHSCOPE_DEFAULT_BASE_URL.to_string(),
            model: DASHSCOPE_DEFAULT_MODEL.to_string(),
            api_key: env::var("DASHSCOPE_API_KEY")
                .or_else(|_| env::var("TAILANG_LLM_API_KEY"))
                .ok(),
            temperature: DEFAULT_TEMPERATURE,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            max_tokens: None,
        })
    }
}

pub trait Provider {
    fn kind(&self) -> ProviderKind;
    fn precompile(&self, prompt: &str, config: &PrecompilerConfig) -> Result<String, String>;
}

pub struct Precompiler {
    config: PrecompilerConfig,
    provider: Box<dyn Provider + Send + Sync>,
}

impl Precompiler {
    pub fn new(config: PrecompilerConfig) -> Result<Self, String> {
        let provider = create_provider(&config)?;
        Ok(Self { config, provider })
    }

    pub fn precompile(&self, meng_content: &str) -> Result<String, String> {
        let prompt = build_prompt(meng_content);
        let raw = self.provider.precompile(&prompt, &self.config)?;
        normalize_tai_output(&raw, &self.config)
    }
}

struct DashScopeProvider {
    client: Client,
}

impl DashScopeProvider {
    fn new(timeout_secs: u64) -> Result<Self, String> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .map_err(|e| format!("failed to create HTTP client: {}", e))?,
        })
    }
}

impl Provider for DashScopeProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::DashScope
    }

    fn precompile(&self, prompt: &str, config: &PrecompilerConfig) -> Result<String, String> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| "missing DashScope API key".to_string())?;

        let request = ChatCompletionsRequest {
            model: config.model.clone(),
            messages: vec![ChatMessage::system(system_prompt()), ChatMessage::user(prompt)],
            temperature: config.temperature,
            stream: false,
            max_tokens: config.max_tokens,
        };

        let response = self.client
            .post(format!("{}/chat/completions", config.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| format!("调用 DashScope 失败：{}", e))?;

        parse_chat_completions_response(response)
    }
}

struct OllamaProvider {
    client: Client,
}

impl OllamaProvider {
    fn new(timeout_secs: u64) -> Result<Self, String> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .map_err(|e| format!("failed to create HTTP client: {}", e))?,
        })
    }
}

impl Provider for OllamaProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ollama
    }

    fn precompile(&self, prompt: &str, config: &PrecompilerConfig) -> Result<String, String> {
        let request = ChatCompletionsRequest {
            model: config.model.clone(),
            messages: vec![ChatMessage::system(system_prompt()), ChatMessage::user(prompt)],
            temperature: config.temperature,
            stream: false,
            max_tokens: config.max_tokens,
        };

        let mut builder = self.client
            .post(format!("{}/chat/completions", config.base_url))
            .header("Content-Type", "application/json");

        if let Some(api_key) = &config.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = builder
            .json(&request)
            .send()
            .map_err(|e| format!("调用 Ollama 失败：{}", e))?;

        parse_chat_completions_response(response)
    }
}

struct CustomProvider {
    client: Client,
}

impl CustomProvider {
    fn new(timeout_secs: u64) -> Result<Self, String> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()
                .map_err(|e| format!("failed to create HTTP client: {}", e))?,
        })
    }
}

impl Provider for CustomProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Custom
    }

    fn precompile(&self, prompt: &str, config: &PrecompilerConfig) -> Result<String, String> {
        let request = ChatCompletionsRequest {
            model: config.model.clone(),
            messages: vec![ChatMessage::system(system_prompt()), ChatMessage::user(prompt)],
            temperature: config.temperature,
            stream: false,
            max_tokens: config.max_tokens,
        };

        let mut builder = self.client
            .post(format!("{}/chat/completions", config.base_url))
            .header("Content-Type", "application/json");

        if let Some(api_key) = &config.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = builder
            .json(&request)
            .send()
            .map_err(|e| format!("调用自定义 Provider 失败：{}", e))?;

        parse_chat_completions_response(response)
    }
}

fn create_provider(config: &PrecompilerConfig) -> Result<Box<dyn Provider + Send + Sync>, String> {
    match config.provider {
        ProviderKind::DashScope => Ok(Box::new(DashScopeProvider::new(config.timeout_secs)?)),
        ProviderKind::Ollama => Ok(Box::new(OllamaProvider::new(config.timeout_secs)?)),
        ProviderKind::Custom => Ok(Box::new(CustomProvider::new(config.timeout_secs)?)),
    }
}

#[derive(Debug, Serialize)]
struct ChatCompletionsRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

impl ChatMessage {
    fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
        }
    }

    fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ChatCompletionsResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

fn parse_chat_completions_response(
    response: reqwest::blocking::Response,
) -> Result<String, String> {
    if !response.status().is_success() {
        return Err(format!("Provider 返回错误状态：{}", response.status()));
    }

    let api_response: ChatCompletionsResponse = response
        .json()
        .map_err(|e| format!("解析响应失败：{}", e))?;

    let first = api_response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| "empty completion response".to_string())?;

    Ok(first.message.content)
}

fn normalize_tai_output(raw: &str, config: &PrecompilerConfig) -> Result<String, String> {
    let trimmed = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let mut tai = TaiFile::from_json(trimmed)?;

    if tai.source.provider.trim().is_empty() || tai.source.provider == "unknown" {
        tai.source.provider = provider_name(config.provider).to_string();
    }
    if tai.source.model.trim().is_empty() || tai.source.model == "unknown" {
        tai.source.model = config.model.clone();
    }
    if tai.source.temperature.trim().is_empty() || tai.source.temperature == "0" {
        tai.source.temperature = config.temperature.to_string();
    }

    tai.normalize()?.to_pretty_json()
}

fn provider_name(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::DashScope => "dashscope",
        ProviderKind::Ollama => "ollama",
        ProviderKind::Custom => "custom",
    }
}

fn system_prompt() -> &'static str {
    r#"你是 Tailang 的预编译器，不是示例拼接器，也不是条件判断打印机。

任务目标：
- 将 `.meng` 中的自然语言意图整理为稳定、可审查、可编译的结构化表示
- 忠实保留用户语义，不降级为机械的“如果/验证/返回”样板
- 保留原有代码块、多语言补充和表达风格中的有效信息

硬性约束：
- 不得为了凑模板臆造模块、函数、参数、验证规则、分支逻辑
- 不得把没有出现的业务规则补成示例代码
- 不得把编程语言本体收缩成“输入 -> 条件判断 -> 打印输出”的玩具模型
- 结构化是为了审查和编译，不是为了丢失语义密度

输出原则：
- 只提取输入中真实存在或可以稳定推断的结构
- 推断时保持克制，避免补全式幻想
- 若信息不足，保留原描述并显式标记待明确项
- 保持输出稳定，便于缓存、版本控制和回归测试"#
}

fn build_prompt(meng_content: &str) -> String {
    format!(
        "请将以下 Tailang `.meng` 内容预编译为结构化 `.tai` JSON。\n\n输出 schema：\n{{\n  \"version\": \"string\",\n  \"source\": {{\n    \"provider\": \"string\",\n    \"model\": \"string\",\n    \"temperature\": \"string\"\n  }},\n  \"modules\": [\n    {{\n      \"name\": \"string\",\n      \"description\": \"string\",\n      \"functions\": [\n        {{\n          \"name\": \"string\",\n          \"params\": [\"string\"],\n          \"description\": \"string\",\n          \"validations\": [\"string\"]\n        }}\n      ]\n    }}\n  ],\n  \"code_blocks\": [\n    {{\n      \"language\": \"string\",\n      \"code\": \"string\",\n      \"linked_to\": \"string\"\n    }}\n  ],\n  \"unresolved_items\": [\n    {{\n      \"kind\": \"string\",\n      \"description\": \"string\"\n    }}\n  ]\n}}\n\n规则：\n1. 保持原意，不要套用示例工程模板\n2. 保留所有代码块和多语言补充\n3. 只提取真实存在或可稳定推断的结构\n4. 不要凭空补充条件分支、验证规则、打印语句或演示逻辑\n5. 输出必须是 JSON，不要带解释、前后缀或 Markdown 代码块\n\n用户输入：\n{}",
        meng_content
    )
}

pub fn precompile_meng_file(input_path: &str, output_path: Option<&str>) -> Result<String, String> {
    let content = std::fs::read_to_string(input_path)
        .map_err(|e| format!("读取文件失败：{}", e))?;

    let precompiler = Precompiler::new(PrecompilerConfig::from_env()?)?;
    let result = precompiler.precompile(&content)?;

    if let Some(output) = output_path {
        std::fs::write(output, &result)
            .map_err(|e| format!("写入文件失败：{}", e))?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_build_prompt() {
        let prompt = build_prompt("邮箱密码登录 qwq");
        assert!(prompt.contains("邮箱密码登录 qwq"));
    }

    #[test]
    fn test_default_config() {
        let _guard = env_lock().lock().unwrap();
        env::set_var("DASHSCOPE_API_KEY", "test-key");
        env::remove_var("TAILANG_LLM_PROVIDER");
        env::remove_var("TAILANG_LLM_MODEL");
        env::remove_var("TAILANG_LLM_BASE_URL");
        env::remove_var("TAILANG_LLM_API_KEY");
        env::remove_var("OLLAMA_BASE_URL");
        env::remove_var("OLLAMA_API_KEY");
        let config = PrecompilerConfig::default();
        assert_eq!(config.provider, ProviderKind::DashScope);
        assert_eq!(config.temperature, 0.0);
    }

    #[test]
    fn test_ollama_config() {
        let _guard = env_lock().lock().unwrap();
        env::set_var("TAILANG_LLM_PROVIDER", "ollama");
        env::remove_var("TAILANG_LLM_MODEL");
        env::remove_var("TAILANG_LLM_BASE_URL");
        env::remove_var("OLLAMA_BASE_URL");
        let config = PrecompilerConfig::from_env().unwrap();
        assert_eq!(config.provider, ProviderKind::Ollama);
        assert_eq!(config.base_url, "http://localhost:11434/v1");
    }

    #[test]
    fn test_normalize_tai_output() {
        let config = PrecompilerConfig {
            provider: ProviderKind::DashScope,
            base_url: "https://example.com".to_string(),
            model: "qwen-plus".to_string(),
            api_key: Some("test-key".to_string()),
            temperature: 0.0,
            timeout_secs: 60,
            max_tokens: None,
        };

        let raw = r#"{
          "version": "",
          "source": {
            "provider": "",
            "model": "",
            "temperature": ""
          },
          "modules": [],
          "code_blocks": [],
          "unresolved_items": []
        }"#;

        let normalized = normalize_tai_output(raw, &config).unwrap();
        assert!(normalized.contains("\"provider\": \"dashscope\""));
        assert!(normalized.contains("\"model\": \"qwen-plus\""));
    }
}
