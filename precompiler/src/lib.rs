//! Tailang Precompiler
//! 
//! Converts .meng files (casual natural language) to .tai files (structured natural language)
//! using LLM (DashScope/Qwen API)

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use thiserror::Error;

/// 预编译器错误
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
}

/// API 响应结构
#[derive(Debug, Serialize)]
struct ApiRequest {
    model: String,
    input: Input,
    parameters: Parameters,
}

#[derive(Debug, Serialize)]
struct Input {
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct Parameters {
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    output: Output,
}

#[derive(Debug, Deserialize)]
struct Output {
    text: String,
}

/// 预编译器配置
#[derive(Debug, Clone)]
pub struct Config {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl Config {
    /// 从环境变量加载配置
    pub fn from_env() -> Result<Self, PrecompilerError> {
        // 尝试加载 .env 文件
        dotenv::dotenv().ok();

        Ok(Self {
            api_key: env::var("DASHSCOPE_API_KEY")
                .map_err(|_| PrecompilerError::EnvVar("DASHSCOPE_API_KEY not found".to_string()))?,
            base_url: env::var("DASHSCOPE_BASE_URL")
                .unwrap_or_else(|_| "https://dashscope.aliyuncs.com/api/v1".to_string()),
            model: env::var("PRECOMPILER_MODEL")
                .unwrap_or_else(|_| "qwen-turbo".to_string()),
        })
    }
}

/// 预编译器
pub struct Precompiler {
    config: Config,
    client: Client,
}

impl Precompiler {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            client: Client::new(),
        }
    }

    /// 预编译 .meng 文件到 .tai 文件
    pub fn precompile(&self, input_path: &Path, output_path: &Path) -> Result<(), PrecompilerError> {
        // 读取 .meng 文件
        let meng_content = fs::read_to_string(input_path)?;

        // 调用 LLM API
        let tai_content = self.convert_to_tai(&meng_content)?;

        // 写入 .tai 文件
        fs::write(output_path, tai_content)?;

        Ok(())
    }

    /// 将随意自然语言转换为结构化格式
    fn convert_to_tai(&self, content: &str) -> Result<String, PrecompilerError> {
        let system_prompt = r#"你是一个 Tailang 预编译器。你的任务是将用户随意的自然语言描述转换为结构化的 Tailang 格式。

输入格式：随意的自然语言，可能包含表情、口语、代码块
输出格式：结构化的 Tailang 代码，包含：
- 模块定义：模块 名字：
- 功能定义：功能 名字 (参数):
- 结构化逻辑：验证...，如果...，返回...
- 保留原有的代码块

示例输入：
```
邮箱密码登录 qwq

```python
import bcrypt
...
```
```

示例输出：
```
模块 用户登录：
  功能 登录 (邮箱，密码):
    验证 邮箱格式正确
    验证 密码长度 >= 8
    验证 邮箱已注册
    
    ```python
    import bcrypt
    ...
    ```
```

请保持原意，但转换为结构化格式。保留所有代码块。"#;

        let request = ApiRequest {
            model: self.config.model.clone(),
            input: Input {
                messages: vec![
                    Message {
                        role: "system".to_string(),
                        content: system_prompt.to_string(),
                    },
                    Message {
                        role: "user".to_string(),
                        content: content.to_string(),
                    },
                ],
            },
            parameters: Parameters {
                temperature: 0.0, // 确定性输出
            },
        };

        let response = self.client
            .post(&self.config.base_url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()?;

        if !response.status().is_success() {
            return Err(PrecompilerError::Api(format!(
                "API returned status: {}",
                response.status()
            )));
        }

        let api_response: ApiResponse = response.json()?;
        Ok(api_response.output.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_env() {
        // 设置环境变量
        env::set_var("DASHSCOPE_API_KEY", "test-key");
        env::set_var("DASHSCOPE_BASE_URL", "https://test.com");
        env::set_var("PRECOMPILER_MODEL", "qwen-turbo");

        let config = Config::from_env().expect("Failed to load config");
        
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.base_url, "https://test.com");
        assert_eq!(config.model, "qwen-turbo");
    }

    #[test]
    fn test_config_default_values() {
        env::set_var("DASHSCOPE_API_KEY", "test-key");
        env::remove_var("DASHSCOPE_BASE_URL");
        env::remove_var("PRECOMPILER_MODEL");

        let config = Config::from_env().expect("Failed to load config");
        
        assert_eq!(config.base_url, "https://dashscope.aliyuncs.com/api/v1");
        assert_eq!(config.model, "qwen-turbo");
    }

    #[test]
    fn test_precompiler_creation() {
        env::set_var("DASHSCOPE_API_KEY", "test-key");
        let config = Config::from_env().unwrap();
        let _precompiler = Precompiler::new(config);
    }
}
