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
        let source = source.strip_prefix('\u{feff}').unwrap_or(source);
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
            if let Some(version) = strip_keyword_prefix(&line, &[".版本", ".version"]) {
                program.version = Some(version.trim().to_string());
                continue;
            }

            if let Some(target) = strip_keyword_prefix(&line, &[".目标平台", ".target"]) {
                program.target = Some(target.trim().to_string());
                continue;
            }

            if let Some(rest) = strip_keyword_prefix(&line, &[".待定", ".todo"]) {
                program.unresolved.push(parse_unresolved_decl(rest, self.offset)?);
                continue;
            }

            if let Some(rest) = strip_keyword_prefix(&line, &[".程序集", ".module"]) {
                program.modules.push(self.parse_module(rest.trim())?);
                continue;
            }

            if let Some(rest) = strip_keyword_prefix(&line, &[".元信息", ".meta"]) {
                program.meta.push(parse_meta_decl(rest.trim(), self.offset)?);
                continue;
            }

            if let Some(rest) = strip_keyword_prefix(&line, &[".常量", ".const"]) {
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
            if starts_with_keyword(&line, &[".程序集", ".module"])
                || starts_with_keyword(&line, &[".版本", ".version"])
                || starts_with_keyword(&line, &[".目标平台", ".target"])
                || starts_with_keyword(&line, &[".待定", ".todo"])
                || starts_with_keyword(&line, &[".元信息", ".meta"])
            {
                break;
            }

            let line = self.next_meaningful_line().expect("peeked line must exist");
            if let Some(doc) = strip_keyword_prefix(&line, &[".说明", ".doc"]) {
                module.doc = Some(parse_string_literal(doc.trim(), self.offset)?);
                continue;
            }

            if let Some(rest) = strip_keyword_prefix(&line, &[".程序集变量", ".global"]) {
                module.globals.push(parse_var_decl(rest.trim(), self.offset)?);
                continue;
            }

            if let Some(rest) = strip_keyword_prefix(&line, &[".常量", ".const"]) {
                module.globals.push(parse_var_decl(rest.trim(), self.offset)?);
                continue;
            }

            if let Some(rest) = strip_keyword_prefix(&line, &[".子程序", ".subprogram"]) {
                module.functions.push(self.parse_function(rest.trim())?);
                continue;
            }

            break;
        }

        Ok(module)
    }

    fn parse_function(&mut self, header: &str) -> Result<TaiFunctionDecl, TaiParseError> {
        let (name, params, return_type, slot3, slot4, binding) =
            split_function_signature(header).map_err(|message| TaiParseError {
                message,
                offset: self.offset,
            })?;
        let mut function = TaiFunctionDecl {
            name,
            return_type,
            slot3,
            slot4,
            binding,
            params: params.iter().map(|decl| decl.name.clone()).collect(),
            param_decls: params,
            locals: vec![],
            doc: None,
            validations: vec![],
            implementation: None,
            code_blocks: vec![],
        };

        let mut implementation_lines = Vec::new();
        let mut current_code: Option<(String, Vec<String>)> = None;

        while let Some(line) = self.peek_meaningful_line() {
            if is_function_boundary(&line) {
                break;
            }

            if starts_with_keyword(&line, &[".待定", ".todo"]) {
                break;
            }

            let line = self.next_meaningful_line().expect("peeked line must exist");

            if let Some((language, body)) = current_code.as_mut() {
                if matches_keyword(&line, &[".代码结束", ".endcode"]) {
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

            if starts_with_keyword(&line, &[".参数", ".param", ".局部变量", ".local", ".常量", ".const"]) {
                return Err(self.error_here("旧式分行参数/变量/常量声明已废弃，请改用新函数头和标准变量声明"));
            }

            if let Some(doc) = strip_keyword_prefix(&line, &[".说明", ".doc"]) {
                function.doc = Some(parse_string_literal(doc.trim(), self.offset)?);
                continue;
            }

            if let Some(validation) = strip_keyword_prefix(&line, &[".校验", ".validate"]) {
                function
                    .validations
                    .push(parse_string_literal(validation.trim(), self.offset)?);
                continue;
            }

            if let Some(language) = strip_keyword_prefix(&line, &[".代码", ".code"]) {
                current_code = Some((language.trim().to_string(), Vec::new()));
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
        self.peek_meaningful_line_from(self.index)
    }

    fn peek_meaningful_line_from(&self, start: usize) -> Option<String> {
        let mut idx = start;
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

fn split_function_signature(
    input: &str,
) -> Result<
    (
        String,
        Vec<TaiVarDecl>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ),
    String,
> {
    if let Some((signature, tail)) = input.split_once("->") {
        let (name, params) = parse_function_name_and_params(signature.trim())?;
        let mut slots = tail
            .split(',')
            .map(|value| value.trim().to_string())
            .collect::<Vec<_>>();
        if slots.is_empty() {
            return Err("子程序声明缺少返回类型".to_string());
        }
        while slots.len() < 4 {
            slots.push(String::new());
        }
        if slots.len() > 4 {
            return Err("子程序声明在返回类型后最多允许 3 个槽位".to_string());
        }
        let return_type = non_empty_string(&slots[0]);
        let slot3 = non_empty_string(&slots[1]);
        let slot4 = non_empty_string(&slots[2]);
        let binding = non_empty_string(&slots[3]);
        return Ok((name, params, return_type, slot3, slot4, binding));
    }

    Err("子程序声明必须使用新语法：.子程序 名称(参数) -> 返回类型, , , 绑定".to_string())
}

fn parse_function_name_and_params(input: &str) -> Result<(String, Vec<TaiVarDecl>), String> {
    let trimmed = input.trim();
    let Some(paren_start) = trimmed.find('(') else {
        return Ok((trimmed.to_string(), Vec::new()));
    };
    let Some(paren_end) = trimmed.rfind(')') else {
        return Err("子程序参数列表缺少 ')'".to_string());
    };
    if paren_end < paren_start {
        return Err("子程序参数列表格式非法".to_string());
    }
    let name = trimmed[..paren_start].trim();
    if name.is_empty() {
        return Err("子程序缺少名称".to_string());
    }
    let inside = trimmed[paren_start + 1..paren_end].trim();
    if trimmed[paren_end + 1..].trim().is_empty() == false {
        return Err("子程序参数列表后存在无法识别的内容".to_string());
    }
    if inside.is_empty() {
        return Ok((name.to_string(), Vec::new()));
    }
    let params = inside
        .split(',')
        .map(|item| parse_inline_param_decl(item.trim()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((name.to_string(), params))
}

fn parse_inline_param_decl(input: &str) -> Result<TaiVarDecl, String> {
    let Some((name, ty)) = input.split_once(':') else {
        return Err(format!("参数 '{}' 缺少 ':' 类型声明", input));
    };
    let name = name.trim();
    let ty = ty.trim();
    if name.is_empty() {
        return Err("参数缺少名称".to_string());
    }
    if ty.is_empty() {
        return Err(format!("参数 '{}' 缺少类型", name));
    }
    Ok(TaiVarDecl {
        name: name.to_string(),
        ty: Some(ty.to_string()),
        value: None,
    })
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn is_function_boundary(line: &str) -> bool {
    starts_with_keyword(line, &[".子程序", ".subprogram"])
        || starts_with_keyword(line, &[".程序集", ".module"])
        || starts_with_keyword(line, &[".版本", ".version"])
        || starts_with_keyword(line, &[".目标平台", ".target"])
        || starts_with_keyword(line, &[".元信息", ".meta"])
}

fn strip_keyword_prefix<'a>(line: &'a str, keywords: &[&str]) -> Option<&'a str> {
    keywords.iter().find_map(|keyword| {
        line.strip_prefix(keyword)
            .and_then(|rest| rest.strip_prefix(' '))
    })
}

fn starts_with_keyword(line: &str, keywords: &[&str]) -> bool {
    strip_keyword_prefix(line, keywords).is_some()
}

fn matches_keyword(line: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| line == *keyword)
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

.子程序 登录(邮箱: 文本型, 密码: 文本型) -> 文本型, , ,
.校验 "邮箱不能为空"
.如果 邮箱 等于 ""
    .返回 "邮箱不能为空"
.如果结束
结果: 文本型 = 邮箱
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
        assert_eq!(program.modules[0].functions[0].locals.len(), 0);
        assert!(program.modules[0].functions[0].implementation.is_some());
        assert_eq!(program.modules[0].functions[0].code_blocks.len(), 1);
        assert_eq!(program.unresolved.len(), 1);
    }

    #[test]
    fn parses_program_with_utf8_bom() {
        let source = "\u{feff}.版本 3\n.元信息 提供者 = \"custom\"\n.程序集 main\n";
        let program = TaiParser::from_source(source).expect("parse should succeed");
        assert_eq!(program.version.as_deref(), Some("3"));
        assert_eq!(program.meta.len(), 1);
        assert_eq!(program.modules.len(), 1);
    }

    #[test]
    fn parses_new_subprogram_signature_with_slots() {
        let source = r#"
.版本 2
.程序集 窗口示例

.子程序 启动程序(宽度: 整数型, 高度: 整数型) -> 整数型, , , 启动
返回 0
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let function = &program.modules[0].functions[0];
        assert_eq!(function.name, "启动程序");
        assert_eq!(function.return_type.as_deref(), Some("整数型"));
        assert_eq!(function.binding.as_deref(), Some("启动"));
        assert_eq!(function.params, vec!["宽度", "高度"]);
        assert_eq!(function.param_decls.len(), 2);
        assert_eq!(function.slot3, None);
        assert_eq!(function.slot4, None);
    }

    #[test]
    fn parses_english_subprogram_signature_with_slots() {
        let source = r#"
.version 2
.module window_demo

.subprogram startup(width: int, height: int) -> int, , , startup
return 0
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let function = &program.modules[0].functions[0];
        assert_eq!(function.name, "startup");
        assert_eq!(function.return_type.as_deref(), Some("int"));
        assert_eq!(function.binding.as_deref(), Some("startup"));
        assert_eq!(function.params, vec!["width", "height"]);
    }
}
