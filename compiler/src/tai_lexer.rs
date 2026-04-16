//! Legacy transitional textual `.tai` lexer.
//!
//! This module reflects an older block-oriented `.tai` experiment and is no
//! longer the source of truth for `.tai v0.3`. The approved v0.3 syntax is
//! primarily parsed by `tai_parser.rs` and `tai_exec.rs`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaiTokenKind {
    Meta,
    Target,
    Module,
    Doc,
    Function,
    Validate,
    Implement,
    Code,
    Unresolved,
    Begin,
    End,
    Identifier(String),
    String(String),
    Assign,
    Comma,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaiToken {
    pub kind: TaiTokenKind,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaiLexError {
    pub message: String,
    pub offset: usize,
}

pub struct TaiLexer<'a> {
    input: &'a str,
    chars: Vec<char>,
    index: usize,
}

impl<'a> TaiLexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().collect(),
            index: 0,
        }
    }

    pub fn lex(mut self) -> Result<Vec<TaiToken>, TaiLexError> {
        let mut tokens = Vec::new();

        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.bump();
                continue;
            }

            let offset = self.index;
            let token = match ch {
                '.' => self.lex_dotted_keyword_or_identifier()?,
                '(' => {
                    self.bump();
                    TaiTokenKind::LeftParen
                }
                ')' => {
                    self.bump();
                    TaiTokenKind::RightParen
                }
                ',' => {
                    self.bump();
                    TaiTokenKind::Comma
                }
                '=' => {
                    self.bump();
                    TaiTokenKind::Assign
                }
                '"' => TaiTokenKind::String(self.lex_string()?),
                _ if is_identifier_start(ch) => self.lex_identifier_or_keyword(),
                _ => {
                    return Err(TaiLexError {
                        message: format!("unexpected character '{}'", ch),
                        offset,
                    })
                }
            };

            tokens.push(TaiToken { kind: token, offset });
        }

        tokens.push(TaiToken {
            kind: TaiTokenKind::Eof,
            offset: self.index,
        });

        Ok(tokens)
    }

    fn lex_identifier_or_keyword(&mut self) -> TaiTokenKind {
        let start = self.index;
        while let Some(ch) = self.peek() {
            if is_identifier_continue(ch) {
                self.bump();
            } else {
                break;
            }
        }

        let ident = self.input[start..self.index].to_string();
        match ident.as_str() {
            "元信息" => TaiTokenKind::Meta,
            "目标" => TaiTokenKind::Target,
            "模块" => TaiTokenKind::Module,
            "说明" => TaiTokenKind::Doc,
            "函数" => TaiTokenKind::Function,
            "校验" => TaiTokenKind::Validate,
            "代码" => TaiTokenKind::Code,
            "待定" => TaiTokenKind::Unresolved,
            _ => TaiTokenKind::Identifier(ident),
        }
    }

    fn lex_dotted_keyword_or_identifier(&mut self) -> Result<TaiTokenKind, TaiLexError> {
        let offset = self.index;
        self.bump();

        let next = self.peek().ok_or(TaiLexError {
            message: "dot-prefixed keyword is incomplete".to_string(),
            offset,
        })?;

        if !is_identifier_start(next) {
            return Err(TaiLexError {
                message: "'.' must be followed by a Chinese keyword".to_string(),
                offset,
            });
        }

        let start = self.index;
        while let Some(ch) = self.peek() {
            if is_identifier_continue(ch) {
                self.bump();
            } else {
                break;
            }
        }

        let ident = self.input[start..self.index].to_string();
        let dotted = format!(".{ident}");
        Ok(match dotted.as_str() {
            ".元信息" => TaiTokenKind::Meta,
            ".目标平台" => TaiTokenKind::Target,
            ".程序集" => TaiTokenKind::Module,
            ".说明" => TaiTokenKind::Doc,
            ".子程序" => TaiTokenKind::Function,
            ".校验" => TaiTokenKind::Validate,
            ".实现" => TaiTokenKind::Implement,
            ".代码" => TaiTokenKind::Code,
            ".待定" => TaiTokenKind::Unresolved,
            ".开始" => TaiTokenKind::Begin,
            ".结束" => TaiTokenKind::End,
            _ => TaiTokenKind::Identifier(dotted),
        })
    }

    fn lex_string(&mut self) -> Result<String, TaiLexError> {
        let start = self.index;
        self.bump();
        let mut value = String::new();

        while let Some(ch) = self.peek() {
            match ch {
                '"' => {
                    self.bump();
                    return Ok(value);
                }
                '\\' => {
                    self.bump();
                    let escaped = self.peek().ok_or(TaiLexError {
                        message: "unterminated escape sequence".to_string(),
                        offset: self.index,
                    })?;
                    self.bump();
                    value.push(match escaped {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        other => other,
                    });
                }
                other => {
                    self.bump();
                    value.push(other);
                }
            }
        }

        Err(TaiLexError {
            message: "unterminated string literal".to_string(),
            offset: start,
        })
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn bump(&mut self) {
        self.index += 1;
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit() || ch == '-' || ch == '.'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_keywords_and_strings() {
        let tokens = TaiLexer::new(r#".程序集 认证 .开始 .说明 "认证流程" .结束"#)
            .lex()
            .expect("lex should succeed");

        assert!(matches!(tokens[0].kind, TaiTokenKind::Module));
        assert!(matches!(tokens[1].kind, TaiTokenKind::Identifier(_)));
        assert!(matches!(tokens[3].kind, TaiTokenKind::Doc));
        assert!(matches!(tokens[4].kind, TaiTokenKind::String(_)));
    }
}
