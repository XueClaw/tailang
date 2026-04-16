use crate::tai_ast::{
    TaiCodeDecl, TaiFunctionDecl, TaiMetaField, TaiModuleDecl, TaiProgram, TaiUnresolvedDecl, TaiVarDecl,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaiParseError {
    pub message: String,
    pub offset: usize,
}

pub struct TaiParser;

impl TaiParser {
    pub fn from_source(input: &str) -> Result<TaiProgram, TaiParseError> {
        V3LineParser::new(input).parse_program()
    }
}

struct V3LineParser<'a> {
    source: &'a str,
    lines: Vec<&'a str>,
    index: usize,
    offset: usize,
}

impl<'a> V3LineParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            lines: source.lines().collect(),
            index: 0,
            offset: 0,
        }
    }

    fn parse_program(&mut self) -> Result<TaiProgram, TaiParseError> {
        let mut program = TaiProgram {
            version: None,
            meta: vec![],
            target: None,
            modules: vec![],
            unresolved: vec![],
        };

        while let Some(line) = self.next_meaningful_line() {
            if let Some(version) = line.strip_prefix(".版本 ") {
                program.version = Some(version.trim().to_string());
                continue;
            }

            if let Some(target) = line.strip_prefix(".目标平台 ") {
                program.target = Some(target.trim().to_string());
                continue;
            }

            if let Some(rest) = line.strip_prefix(".待定 ") {
                program.unresolved.push(parse_unresolved_decl(rest, self.offset)?);
                continue;
            }

            if let Some(rest) = line.strip_prefix(".程序集 ") {
                program.modules.push(self.parse_module(rest.trim())?);
                continue;
            }

            if let Some(rest) = line.strip_prefix(".元信息 ") {
                program.meta.push(parse_meta_decl(rest.trim(), self.offset)?);
                continue;
            }

            if let Some(rest) = line.strip_prefix(".常量 ") {
                program.meta.push(TaiMetaField {
                    key: "常量".to_string(),
                    value: rest.trim().to_string(),
                });
                continue;
            }

            return Err(self.error_here("无法识别的顶层 .tai 声明"));
        }

        Ok(program)
    }

    fn parse_module(&mut self, name: &str) -> Result<TaiModuleDecl, TaiParseError> {
        let mut module = TaiModuleDecl {
            name: name.to_string(),
            globals: vec![],
            doc: None,
            functions: vec![],
        };

        while let Some(line) = self.peek_meaningful_line() {
            if line.starts_with(".程序集 ") || line.starts_with(".版本 ") || line.starts_with(".目标平台 ") || line.starts_with(".待定 ") || line.starts_with(".元信息 ") {
                break;
            }

            let line = self.next_meaningful_line().expect("peeked line must exist");
            if let Some(doc) = line.strip_prefix(".说明 ") {
                module.doc = Some(parse_string_literal(doc.trim(), self.offset)?);
                continue;
            }

            if let Some(rest) = line.strip_prefix(".程序集变量 ") {
                module.globals.push(parse_var_decl(rest.trim(), self.offset)?);
                continue;
            }

            if let Some(rest) = line.strip_prefix(".常量 ") {
                module.globals.push(parse_var_decl(rest.trim(), self.offset)?);
                continue;
            }

            if let Some(rest) = line.strip_prefix(".子程序 ") {
                module.functions.push(self.parse_function(rest.trim())?);
                continue;
            }

            break;
        }

        Ok(module)
    }

    fn parse_function(&mut self, header: &str) -> Result<TaiFunctionDecl, TaiParseError> {
        let (name, return_type) = split_name_and_type(header);
        let mut function = TaiFunctionDecl {
            name,
            return_type,
            params: vec![],
            param_decls: vec![],
            locals: vec![],
            doc: None,
            validations: vec![],
            implementation: None,
            code_blocks: vec![],
        };

        let mut implementation_lines = Vec::new();
        let mut current_code: Option<(String, Vec<String>)> = None;

        while let Some(line) = self.peek_meaningful_line() {
            if is_function_boundary(line) {
                break;
            }

            let line = self.next_meaningful_line().expect("peeked line must exist");

            if let Some((language, body)) = current_code.as_mut() {
                if line == ".代码结束" {
                    function.code_blocks.push(TaiCodeDecl {
                        language: language.clone(),
                        body: body.join("\n"),
                    });
                    current_code = None;
                    continue;
                }
                body.push(line.to_string());
                continue;
            }

            if let Some(rest) = line.strip_prefix(".参数 ") {
                let decl = parse_var_decl(rest.trim(), self.offset)?;
                function.params.push(decl.name.clone());
                function.param_decls.push(decl);
                continue;
            }

            if let Some(rest) = line.strip_prefix(".局部变量 ") {
                function.locals.push(parse_var_decl(rest.trim(), self.offset)?);
                continue;
            }

            if let Some(rest) = line.strip_prefix(".常量 ") {
                function.locals.push(parse_var_decl(rest.trim(), self.offset)?);
                continue;
            }

            if let Some(doc) = line.strip_prefix(".说明 ") {
                function.doc = Some(parse_string_literal(doc.trim(), self.offset)?);
                continue;
            }

            if let Some(validation) = line.strip_prefix(".校验 ") {
                function
                    .validations
                    .push(parse_string_literal(validation.trim(), self.offset)?);
                continue;
            }

            if let Some(language) = line.strip_prefix(".代码 ") {
                current_code = Some((language.trim().to_string(), Vec::new()));
                continue;
            }

            if let Some(rest) = line.strip_prefix(".待定 ") {
                implementation_lines.push(format!(".待定 {}", rest.trim()));
                continue;
            }

            implementation_lines.push(line.to_string());
        }

        if current_code.is_some() {
            return Err(self.error_here(".代码 块缺少 .代码结束"));
        }

        if !implementation_lines.is_empty() {
            function.implementation = Some(implementation_lines.join("\n"));
        }

        Ok(function)
    }

    fn next_meaningful_line(&mut self) -> Option<String> {
        while self.index < self.lines.len() {
            let raw = self.lines[self.index];
            let trimmed = raw.trim();
            let current_offset = self.offset;
            self.offset += raw.len() + 1;
            self.index += 1;
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }
            self.offset = current_offset;
            return Some(trimmed.to_string());
        }
        None
    }

    fn peek_meaningful_line(&self) -> Option<String> {
        let mut idx = self.index;
        while idx < self.lines.len() {
            let trimmed = self.lines[idx].trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                idx += 1;
                continue;
            }
            return Some(trimmed.to_string());
        }
        None
    }

    fn error_here(&self, message: &str) -> TaiParseError {
        TaiParseError {
            message: message.to_string(),
            offset: self.offset.min(self.source.len()),
        }
    }
}

fn parse_meta_decl(input: &str, offset: usize) -> Result<TaiMetaField, TaiParseError> {
    let Some((key, value)) = input.split_once('=') else {
        return Err(TaiParseError {
            message: ".元信息 需要 key = \"value\" 格式".to_string(),
            offset,
        });
    };
    Ok(TaiMetaField {
        key: key.trim().to_string(),
        value: parse_string_literal(value.trim(), offset)?,
    })
}

fn parse_unresolved_decl(input: &str, offset: usize) -> Result<TaiUnresolvedDecl, TaiParseError> {
    if let Some((kind, desc)) = input.split_once(',') {
        return Ok(TaiUnresolvedDecl {
            kind: kind.trim().to_string(),
            description: parse_string_literal(desc.trim(), offset)?,
        });
    }
    let mut parts = input.splitn(2, ' ');
    let kind = parts.next().unwrap_or("").trim();
    let desc = parts.next().unwrap_or("").trim();
    if kind.is_empty() || desc.is_empty() {
        return Err(TaiParseError {
            message: ".待定 需要类别与描述".to_string(),
            offset,
        });
    }
    Ok(TaiUnresolvedDecl {
        kind: kind.to_string(),
        description: parse_string_literal(desc, offset)?,
    })
}

fn parse_var_decl(input: &str, offset: usize) -> Result<TaiVarDecl, TaiParseError> {
    let (left, value) = if let Some((left, value)) = input.split_once('=') {
        (left.trim(), Some(value.trim().to_string()))
    } else {
        (input.trim(), None)
    };

    let mut parts = left.splitn(2, ',');
    let name = parts.next().unwrap_or("").trim();
    let ty = parts.next().map(|value| value.trim().to_string()).filter(|value| !value.is_empty());

    if name.is_empty() {
        return Err(TaiParseError {
            message: "变量声明缺少名称".to_string(),
            offset,
        });
    }

    Ok(TaiVarDecl {
        name: name.to_string(),
        ty,
        value,
    })
}

fn parse_string_literal(input: &str, offset: usize) -> Result<String, TaiParseError> {
    let trimmed = input.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        Ok(trimmed[1..trimmed.len() - 1].to_string())
    } else {
        Err(TaiParseError {
            message: "需要字符串字面量".to_string(),
            offset,
        })
    }
}

fn split_name_and_type(input: &str) -> (String, Option<String>) {
    let mut parts = input.splitn(2, ',');
    let name = parts.next().unwrap_or("").trim().to_string();
    let ty = parts.next().map(|value| value.trim().to_string()).filter(|value| !value.is_empty());
    (name, ty)
}

fn is_function_boundary(line: &str) -> bool {
    line.starts_with(".子程序 ")
        || line.starts_with(".程序集 ")
        || line.starts_with(".版本 ")
        || line.starts_with(".目标平台 ")
        || line.starts_with(".待定 ")
        || line.starts_with(".元信息 ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v03_style_program() {
        let source = r#"
.版本 3
.目标平台 视窗
.程序集 登录模块

.子程序 登录, 文本型
.参数 邮箱, 文本型
.参数 密码, 文本型
.局部变量 结果, 文本型
.校验 "邮箱不能为空"
.如果 邮箱 等于 ""
    .返回 "邮箱不能为空"
.如果结束
结果 = 邮箱
.返回 结果

.代码 Rust
println!("执行登录流程");
.代码结束

.待定 规则, "缺少密码复杂度规则"
"#;

        let program = TaiParser::from_source(source).expect("parse should succeed");
        assert_eq!(program.version.as_deref(), Some("3"));
        assert_eq!(program.target.as_deref(), Some("视窗"));
        assert_eq!(program.modules.len(), 1);
        assert_eq!(program.modules[0].functions.len(), 1);
        assert_eq!(program.modules[0].functions[0].params, vec!["邮箱", "密码"]);
        assert_eq!(program.modules[0].functions[0].locals.len(), 1);
        assert!(program.modules[0].functions[0].implementation.is_some());
        assert_eq!(program.modules[0].functions[0].code_blocks.len(), 1);
        assert_eq!(program.unresolved.len(), 1);
    }
}
