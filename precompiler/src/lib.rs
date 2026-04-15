//! Tailang Precompiler
//!
//! Converts `.meng` files to `.tai` using a pluggable LLM provider layer.

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use std::time::Duration;
use thiserror::Error;

const DEFAULT_TEMPERATURE: f32 = 0.0;
const DEFAULT_TIMEOUT_SECS: u64 = 60;
const DASHSCOPE_DEFAULT_BASE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode/v1";
const DASHSCOPE_DEFAULT_MODEL: &str = "qwen-plus";
const OLLAMA_DEFAULT_BASE_URL: &str = "http://localhost:11434/v1";
const OLLAMA_DEFAULT_MODEL: &str = "qwen2.5-coder:latest";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaiFile {
    pub version: String,
    pub source: TaiSource,
    pub modules: Vec<TaiModule>,
    pub code_blocks: Vec<TaiCodeBlock>,
    pub unresolved_items: Vec<TaiUnresolvedItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaiSource {
    pub provider: String,
    pub model: String,
    pub temperature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaiModule {
    pub name: String,
    pub description: String,
    pub functions: Vec<TaiFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaiFunction {
    pub name: String,
    pub params: Vec<String>,
    pub description: String,
    pub validations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaiCodeBlock {
    pub language: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_to: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaiUnresolvedItem {
    pub kind: String,
    pub description: String,
}

impl TaiFile {
    pub fn normalize(mut self) -> Result<Self, PrecompilerError> {
        if self.version.trim().is_empty() {
            self.version = "0.1.0".to_string();
        }
        if self.source.provider.trim().is_empty() {
            self.source.provider = "unknown".to_string();
        }
        if self.source.model.trim().is_empty() {
            self.source.model = "unknown".to_string();
        }
        if self.source.temperature.trim().is_empty() {
            self.source.temperature = "0".to_string();
        }

        for (module_index, module) in self.modules.iter().enumerate() {
            if module.name.trim().is_empty() {
                return Err(PrecompilerError::Config(format!(
                    "invalid .tai schema: modules[{module_index}].name is required"
                )));
            }

            for (function_index, function) in module.functions.iter().enumerate() {
                if function.name.trim().is_empty() {
                    return Err(PrecompilerError::Config(format!(
                        "invalid .tai schema: modules[{module_index}].functions[{function_index}].name is required"
                    )));
                }
            }
        }

        for (code_index, block) in self.code_blocks.iter().enumerate() {
            if block.language.trim().is_empty() {
                return Err(PrecompilerError::Config(format!(
                    "invalid .tai schema: code_blocks[{code_index}].language is required"
                )));
            }
            if block.code.trim().is_empty() {
                return Err(PrecompilerError::Config(format!(
                    "invalid .tai schema: code_blocks[{code_index}].code is required"
                )));
            }
        }

        Ok(self)
    }

    pub fn from_json(content: &str) -> Result<Self, PrecompilerError> {
        let parsed: TaiFile = serde_json::from_str(content)?;
        parsed.normalize()
    }

    pub fn to_pretty_json(&self) -> Result<String, PrecompilerError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    DashScope,
    Ollama,
    Custom,
}

impl ProviderKind {
    fn from_env(value: &str) -> Result<Self, PrecompilerError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "dashscope" | "bailian" => Ok(Self::DashScope),
            "ollama" => Ok(Self::Ollama),
            "custom" => Ok(Self::Custom),
            other => Err(PrecompilerError::Config(format!(
                "unsupported provider: {}",
                other
            ))),
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
    pub fn from_env() -> Result<Self, PrecompilerError> {
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
            ProviderKind::Custom => env::var("TAILANG_LLM_BASE_URL").map_err(|_| {
                PrecompilerError::EnvVar("TAILANG_LLM_BASE_URL not found".to_string())
            })?,
        };

        let api_key = match provider {
            ProviderKind::DashScope => Some(
                env::var("DASHSCOPE_API_KEY")
                    .or_else(|_| env::var("TAILANG_LLM_API_KEY"))
                    .map_err(|_| {
                        PrecompilerError::EnvVar(
                            "DASHSCOPE_API_KEY or TAILANG_LLM_API_KEY not found".to_string(),
                        )
                    })?,
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

#[derive(Error, Debug)]
pub enum PrecompilerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("API error: {0}")]
    Api(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Environment variable not found: {0}")]
    EnvVar(String),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),
}

pub trait Provider {
    fn kind(&self) -> ProviderKind;
    fn precompile(
        &self,
        prompt: &str,
        config: &PrecompilerConfig,
    ) -> Result<String, PrecompilerError>;
}

pub struct Precompiler {
    config: PrecompilerConfig,
    provider: Box<dyn Provider + Send + Sync>,
}

impl Precompiler {
    pub fn new(config: PrecompilerConfig) -> Result<Self, PrecompilerError> {
        let provider = create_provider(&config)?;
        Ok(Self { config, provider })
    }

    pub fn precompile_file(
        &self,
        input_path: &Path,
        output_path: &Path,
    ) -> Result<(), PrecompilerError> {
        let meng_content = fs::read_to_string(input_path)?;
        let tai_content = self.precompile(&meng_content)?;
        fs::write(output_path, tai_content)?;
        Ok(())
    }

    pub fn precompile(&self, meng_content: &str) -> Result<String, PrecompilerError> {
        let prompt = build_prompt(meng_content);
        let raw = self.provider.precompile(&prompt, &self.config)?;
        normalize_tai_output(&raw, &self.config)
    }
}

struct DashScopeProvider {
    client: Client,
}

impl DashScopeProvider {
    fn new(timeout_secs: u64) -> Result<Self, PrecompilerError> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()?,
        })
    }
}

impl Provider for DashScopeProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::DashScope
    }

    fn precompile(
        &self,
        prompt: &str,
        config: &PrecompilerConfig,
    ) -> Result<String, PrecompilerError> {
        let api_key = config
            .api_key
            .clone()
            .ok_or_else(|| PrecompilerError::EnvVar("missing DashScope API key".to_string()))?;

        let request = ChatCompletionsRequest {
            model: config.model.clone(),
            messages: vec![ChatMessage::system(system_prompt()), ChatMessage::user(prompt)],
            temperature: config.temperature,
            stream: false,
            max_tokens: config.max_tokens,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", config.base_url))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()?;

        parse_chat_completions_response(response)
    }
}

struct OllamaProvider {
    client: Client,
}

impl OllamaProvider {
    fn new(timeout_secs: u64) -> Result<Self, PrecompilerError> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()?,
        })
    }
}

impl Provider for OllamaProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Ollama
    }

    fn precompile(
        &self,
        prompt: &str,
        config: &PrecompilerConfig,
    ) -> Result<String, PrecompilerError> {
        let request = ChatCompletionsRequest {
            model: config.model.clone(),
            messages: vec![ChatMessage::system(system_prompt()), ChatMessage::user(prompt)],
            temperature: config.temperature,
            stream: false,
            max_tokens: config.max_tokens,
        };

        let mut builder = self
            .client
            .post(format!("{}/chat/completions", config.base_url))
            .header("Content-Type", "application/json");

        if let Some(api_key) = &config.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = builder.json(&request).send()?;
        parse_chat_completions_response(response)
    }
}

struct CustomProvider {
    client: Client,
}

impl CustomProvider {
    fn new(timeout_secs: u64) -> Result<Self, PrecompilerError> {
        Ok(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(timeout_secs))
                .build()?,
        })
    }
}

impl Provider for CustomProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Custom
    }

    fn precompile(
        &self,
        prompt: &str,
        config: &PrecompilerConfig,
    ) -> Result<String, PrecompilerError> {
        let request = ChatCompletionsRequest {
            model: config.model.clone(),
            messages: vec![ChatMessage::system(system_prompt()), ChatMessage::user(prompt)],
            temperature: config.temperature,
            stream: false,
            max_tokens: config.max_tokens,
        };

        let mut builder = self
            .client
            .post(format!("{}/chat/completions", config.base_url))
            .header("Content-Type", "application/json");

        if let Some(api_key) = &config.api_key {
            builder = builder.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = builder.json(&request).send()?;
        parse_chat_completions_response(response)
    }
}

fn create_provider(
    config: &PrecompilerConfig,
) -> Result<Box<dyn Provider + Send + Sync>, PrecompilerError> {
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
) -> Result<String, PrecompilerError> {
    if !response.status().is_success() {
        return Err(PrecompilerError::Api(format!(
            "API returned status: {}",
            response.status()
        )));
    }

    let api_response: ChatCompletionsResponse = response.json()?;
    let first = api_response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| PrecompilerError::Api("empty completion response".to_string()))?;

    Ok(first.message.content)
}

fn normalize_tai_output(raw: &str, config: &PrecompilerConfig) -> Result<String, PrecompilerError> {
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
        r#"请将以下 Tailang `.meng` 内容预编译为结构化 `.tai` JSON。

输出 schema：
{{
  "version": "string",
  "source": {{
    "provider": "string",
    "model": "string",
    "temperature": "string"
  }},
  "modules": [
    {{
      "name": "string",
      "description": "string",
      "functions": [
        {{
          "name": "string",
          "params": ["string"],
          "description": "string",
          "validations": ["string"]
        }}
      ]
    }}
  ],
  "code_blocks": [
    {{
      "language": "string",
      "code": "string",
      "linked_to": "string"
    }}
  ],
  "unresolved_items": [
    {{
      "kind": "string",
      "description": "string"
    }}
  ]
}}

规则：
1. 保持原意，不要套用示例工程模板
2. 保留所有代码块和多语言补充
3. 只提取真实存在或可稳定推断的结构
4. 不要凭空补充条件分支、验证规则、打印语句或演示逻辑
5. 输出必须是 JSON，不要带解释、前后缀或 Markdown 代码块

用户输入：
{}
"#,
        meng_content
    )
}

pub fn precompile_meng_file(
    input_path: &str,
    output_path: Option<&str>,
) -> Result<String, PrecompilerError> {
    let content = fs::read_to_string(input_path)?;
    let precompiler = Precompiler::new(PrecompilerConfig::from_env()?)?;
    let result = precompiler.precompile(&content)?;

    if let Some(output) = output_path {
        fs::write(output, &result)?;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_from_env_dashscope() {
        env::set_var("TAILANG_LLM_PROVIDER", "dashscope");
        env::set_var("DASHSCOPE_API_KEY", "test-key");
        let config = PrecompilerConfig::from_env().expect("failed to load config");
        assert_eq!(config.provider, ProviderKind::DashScope);
    }

    #[test]
    fn test_provider_from_env_ollama() {
        env::set_var("TAILANG_LLM_PROVIDER", "ollama");
        env::remove_var("DASHSCOPE_API_KEY");
        let config = PrecompilerConfig::from_env().expect("failed to load config");
        assert_eq!(config.provider, ProviderKind::Ollama);
        assert_eq!(config.base_url, "http://localhost:11434/v1");
    }

    #[test]
    fn test_build_prompt_contains_input() {
        let prompt = build_prompt("邮箱密码登录 qwq");
        assert!(prompt.contains("邮箱密码登录 qwq"));
    }

    #[test]
    fn test_normalize_tai_output() {
        let config = PrecompilerConfig {
            provider: ProviderKind::Ollama,
            base_url: "http://localhost:11434/v1".to_string(),
            model: "qwen2.5-coder:latest".to_string(),
            api_key: None,
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
        assert!(normalized.contains("\"provider\": \"ollama\""));
        assert!(normalized.contains("\"model\": \"qwen2.5-coder:latest\""));
    }
}
