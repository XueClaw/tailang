//! .tai 文件格式 - Tailang 统一中间表示
//!
//! 该 schema 需要与 Go CLI 中的 `.tai` JSON 保持一致。

use serde::{Deserialize, Serialize};

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
    pub fn normalize(mut self) -> Result<Self, String> {
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

        for (module_index, module) in self.modules.iter_mut().enumerate() {
            if module.name.trim().is_empty() {
                return Err(format!("invalid .tai schema: modules[{module_index}].name is required"));
            }

            for (function_index, function) in module.functions.iter_mut().enumerate() {
                if function.name.trim().is_empty() {
                    return Err(format!(
                        "invalid .tai schema: modules[{module_index}].functions[{function_index}].name is required"
                    ));
                }
            }
        }

        for (code_index, code_block) in self.code_blocks.iter().enumerate() {
            if code_block.language.trim().is_empty() {
                return Err(format!(
                    "invalid .tai schema: code_blocks[{code_index}].language is required"
                ));
            }
            if code_block.code.trim().is_empty() {
                return Err(format!(
                    "invalid .tai schema: code_blocks[{code_index}].code is required"
                ));
            }
        }

        Ok(self)
    }

    pub fn from_json(content: &str) -> Result<Self, String> {
        let parsed: TaiFile =
            serde_json::from_str(content).map_err(|e| format!("反序列化失败：{}", e))?;
        parsed.normalize()
    }

    pub fn to_pretty_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self).map_err(|e| format!("序列化失败：{}", e))
    }
}

pub struct TaiTranslator {
    version: String,
}

impl TaiTranslator {
    pub fn new() -> Self {
        Self {
            version: "0.1.0".to_string(),
        }
    }

    pub fn empty(&self) -> TaiFile {
        TaiFile {
            version: self.version.clone(),
            source: TaiSource {
                provider: "compiler".to_string(),
                model: "internal".to_string(),
                temperature: "0".to_string(),
            },
            modules: vec![],
            code_blocks: vec![],
            unresolved_items: vec![],
        }
    }

    pub fn serialize(&self, tai: &TaiFile) -> Result<String, String> {
        tai.to_pretty_json()
    }

    pub fn deserialize(&self, content: &str) -> Result<TaiFile, String> {
        TaiFile::from_json(content)
    }

    pub fn translate(&self, ir: &crate::translator::IRProgram, name: &str) -> TaiFile {
        let mut module = TaiModule {
            name: sanitize_module_name(name),
            description: "Generated from Tailang IR".to_string(),
            functions: vec![],
        };

        for function in &ir.functions {
            module.functions.push(TaiFunction {
                name: function.name.clone(),
                params: function.params.clone(),
                description: format!("Function {}", function.name),
                validations: vec![],
            });
        }

        let mut code_blocks = Vec::new();
        for instruction in &ir.instructions {
            if let crate::translator::IRInstruction::CodeBlock(code) = instruction {
                code_blocks.push(TaiCodeBlock {
                    language: "raw".to_string(),
                    code: code.clone(),
                    linked_to: None,
                });
            }
        }

        TaiFile {
            version: self.version.clone(),
            source: TaiSource {
                provider: "compiler".to_string(),
                model: "internal".to_string(),
                temperature: "0".to_string(),
            },
            modules: vec![module],
            code_blocks,
            unresolved_items: vec![],
        }
    }
}

impl Default for TaiTranslator {
    fn default() -> Self {
        Self::new()
    }
}

fn sanitize_module_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        "main".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_defaults() {
        let tai = TaiFile {
            version: "".to_string(),
            source: TaiSource {
                provider: "".to_string(),
                model: "".to_string(),
                temperature: "".to_string(),
            },
            modules: vec![],
            code_blocks: vec![],
            unresolved_items: vec![],
        }
        .normalize()
        .unwrap();

        assert_eq!(tai.version, "0.1.0");
        assert_eq!(tai.source.provider, "unknown");
        assert_eq!(tai.source.model, "unknown");
        assert_eq!(tai.source.temperature, "0");
    }

    #[test]
    fn test_reject_empty_module_name() {
        let result = TaiFile {
            version: "0.1.0".to_string(),
            source: TaiSource {
                provider: "dashscope".to_string(),
                model: "qwen-plus".to_string(),
                temperature: "0".to_string(),
            },
            modules: vec![TaiModule {
                name: "".to_string(),
                description: "".to_string(),
                functions: vec![],
            }],
            code_blocks: vec![],
            unresolved_items: vec![],
        }
        .normalize();

        assert!(result.is_err());
    }

    #[test]
    fn test_serialize_deserialize() {
        let translator = TaiTranslator::new();
        let tai = TaiFile {
            version: "0.1.0".to_string(),
            source: TaiSource {
                provider: "dashscope".to_string(),
                model: "qwen-plus".to_string(),
                temperature: "0".to_string(),
            },
            modules: vec![TaiModule {
                name: "auth".to_string(),
                description: "用户认证".to_string(),
                functions: vec![TaiFunction {
                    name: "login".to_string(),
                    params: vec!["email".to_string(), "password".to_string()],
                    description: "邮箱密码登录".to_string(),
                    validations: vec!["邮箱格式正确".to_string()],
                }],
            }],
            code_blocks: vec![TaiCodeBlock {
                language: "python".to_string(),
                code: "print('hello')".to_string(),
                linked_to: Some("login".to_string()),
            }],
            unresolved_items: vec![TaiUnresolvedItem {
                kind: "missing-detail".to_string(),
                description: "token 策略未明确".to_string(),
            }],
        };

        let serialized = translator.serialize(&tai).unwrap();
        let deserialized = translator.deserialize(&serialized).unwrap();
        assert_eq!(tai, deserialized);
    }
}
