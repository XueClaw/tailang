use std::iter::Peekable;
use std::str::CharIndices;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Let,
    Function,
    If,
    Else,
    While,
    Return,
    True,
    False,
    Null,
    And,
    Or,
    Not,
    CodeBlockKeyword,
    Identifier(String),
    Number(String),
    String(String),
    CodeBlock(String),
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    Colon,
    Semicolon,
    Arrow,
    Assign,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Newline,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl LexError {
    fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

pub struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<CharIndices<'a>>,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            line: 1,
            column: 1,
        }
    }

    pub fn lex(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        while let Some((start, ch)) = self.peek() {
            if ch == '\n' {
                let span = self.make_span(start, start + ch.len_utf8());
                self.bump();
                tokens.push(Token {
                    kind: TokenKind::Newline,
                    span,
                });
                continue;
            }

            if ch.is_whitespace() {
                self.bump();
                continue;
            }

            if self.starts_with("```") {
                tokens.push(self.lex_fenced_code_block()?);
                continue;
            }

            if self.starts_with("//") {
                self.skip_line_comment();
                continue;
            }

            if self.starts_with("/*") {
                self.skip_block_comment()?;
                continue;
            }

            let token = match ch {
                '"' | '\'' => self.lex_string()?,
                '0'..='9' => self.lex_number()?,
                '(' => self.single_char(TokenKind::LeftParen),
                ')' => self.single_char(TokenKind::RightParen),
                '{' => self.single_char(TokenKind::LeftBrace),
                '}' => self.single_char(TokenKind::RightBrace),
                '[' => self.single_char(TokenKind::LeftBracket),
                ']' => self.single_char(TokenKind::RightBracket),
                ',' => self.single_char(TokenKind::Comma),
                '.' => self.single_char(TokenKind::Dot),
                ':' => self.single_char(TokenKind::Colon),
                ';' => self.single_char(TokenKind::Semicolon),
                '+' => self.single_char(TokenKind::Plus),
                '*' => self.single_char(TokenKind::Star),
                '%' => self.single_char(TokenKind::Percent),
                '-' => {
                    if self.starts_with("->") {
                        self.double_char(TokenKind::Arrow)
                    } else {
                        self.single_char(TokenKind::Minus)
                    }
                }
                '=' => {
                    if self.starts_with("==") {
                        self.double_char(TokenKind::Equal)
                    } else {
                        self.single_char(TokenKind::Assign)
                    }
                }
                '!' => {
                    if self.starts_with("!=") {
                        self.double_char(TokenKind::NotEqual)
                    } else {
                        return Err(self.error_here("非法字符 '!'", start));
                    }
                }
                '>' => {
                    if self.starts_with(">=") {
                        self.double_char(TokenKind::GreaterEqual)
                    } else {
                        self.single_char(TokenKind::Greater)
                    }
                }
                '<' => {
                    if self.starts_with("<=") {
                        self.double_char(TokenKind::LessEqual)
                    } else {
                        self.single_char(TokenKind::Less)
                    }
                }
                '/' => self.single_char(TokenKind::Slash),
                _ if is_identifier_start(ch) => self.lex_identifier_or_keyword()?,
                _ => return Err(self.error_here(format!("非法字符 '{ch}'"), start)),
            };

            tokens.push(token);
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            span: Span {
                start: self.source.len(),
                end: self.source.len(),
                line: self.line,
                column: self.column,
            },
        });

        Ok(tokens)
    }

    fn lex_identifier_or_keyword(&mut self) -> Result<Token, LexError> {
        let (start, _) = self.peek().expect("identifier must have a start");
        let start_line = self.line;
        let start_column = self.column;
        let mut ident = String::new();

        while let Some((_, ch)) = self.peek() {
            if !is_identifier_continue(ch) {
                break;
            }
            ident.push(ch);
            self.bump();
        }

        let kind = match ident.as_str() {
            "令" => TokenKind::Let,
            "函数" => TokenKind::Function,
            "若" | "如果" => TokenKind::If,
            "否则" => TokenKind::Else,
            "当" | "循环" => TokenKind::While,
            "返回" => TokenKind::Return,
            "真" => TokenKind::True,
            "假" => TokenKind::False,
            "空" => TokenKind::Null,
            "并且" => TokenKind::And,
            "或者" => TokenKind::Or,
            "非" => TokenKind::Not,
            "代码块" => {
                self.skip_inline_ws_and_comments()?;
                if matches!(self.peek(), Some((_, '{'))) {
                    let block = self.lex_braced_code_block()?;
                    return Ok(Token {
                        kind: TokenKind::CodeBlock(block),
                        span: Span {
                            start,
                            end: self.current_index(),
                            line: start_line,
                            column: start_column,
                        },
                    });
                }
                TokenKind::CodeBlockKeyword
            }
            _ => TokenKind::Identifier(ident),
        };

        Ok(Token {
            kind,
            span: Span {
                start,
                end: self.current_index(),
                line: start_line,
                column: start_column,
            },
        })
    }

    fn lex_number(&mut self) -> Result<Token, LexError> {
        let (start, _) = self.peek().expect("number must have a start");
        let start_line = self.line;
        let start_column = self.column;
        let mut number = String::new();
        let mut seen_dot = false;

        while let Some((_, ch)) = self.peek() {
            if ch.is_ascii_digit() {
                number.push(ch);
                self.bump();
                continue;
            }

            if ch == '.' && !seen_dot {
                seen_dot = true;
                number.push(ch);
                self.bump();
                continue;
            }

            break;
        }

        if number.ends_with('.') {
            return Err(LexError::new(
                "数字字面量不能以 '.' 结尾",
                Span {
                    start,
                    end: self.current_index(),
                    line: start_line,
                    column: start_column,
                },
            ));
        }

        Ok(Token {
            kind: TokenKind::Number(number),
            span: Span {
                start,
                end: self.current_index(),
                line: start_line,
                column: start_column,
            },
        })
    }

    fn lex_string(&mut self) -> Result<Token, LexError> {
        let (start, quote) = self.peek().expect("string must have a start");
        let start_line = self.line;
        let start_column = self.column;
        self.bump();
        let mut value = String::new();

        while let Some((idx, ch)) = self.peek() {
            if ch == quote {
                self.bump();
                return Ok(Token {
                    kind: TokenKind::String(value),
                    span: Span {
                        start,
                        end: self.current_index(),
                        line: start_line,
                        column: start_column,
                    },
                });
            }

            if ch == '\\' {
                self.bump();
                let escaped = match self.peek() {
                    Some((_, 'n')) => '\n',
                    Some((_, 'r')) => '\r',
                    Some((_, 't')) => '\t',
                    Some((_, '\\')) => '\\',
                    Some((_, '"')) => '"',
                    Some((_, '\'')) => '\'',
                    Some((_, other)) => other,
                    None => {
                        return Err(LexError::new(
                            "字符串转义不完整",
                            Span {
                                start: idx,
                                end: self.current_index(),
                                line: self.line,
                                column: self.column,
                            },
                        ))
                    }
                };
                self.bump();
                value.push(escaped);
                continue;
            }

            value.push(ch);
            self.bump();
        }

        Err(LexError::new(
            "未闭合的字符串字面量",
            Span {
                start,
                end: self.current_index(),
                line: start_line,
                column: start_column,
            },
        ))
    }

    fn lex_fenced_code_block(&mut self) -> Result<Token, LexError> {
        let start = self.current_index();
        let start_line = self.line;
        let start_column = self.column;
        self.consume_exact("```")?;

        while let Some((_, ch)) = self.peek() {
            self.bump();
            if ch == '\n' {
                break;
            }
        }

        let body_start = self.current_index();
        while self.peek().is_some() && !self.starts_with("```") {
            self.bump();
        }

        if !self.starts_with("```") {
            return Err(LexError::new(
                "未闭合的三反引号代码块",
                Span {
                    start,
                    end: self.current_index(),
                    line: start_line,
                    column: start_column,
                },
            ));
        }

        let body = self.source[body_start..self.current_index()].to_string();
        self.consume_exact("```")?;

        Ok(Token {
            kind: TokenKind::CodeBlock(body),
            span: Span {
                start,
                end: self.current_index(),
                line: start_line,
                column: start_column,
            },
        })
    }

    fn lex_braced_code_block(&mut self) -> Result<String, LexError> {
        let (start, _) = self.peek().expect("code block must start with '{'");
        let mut depth = 0usize;
        let mut body_start = None;

        while let Some((_, ch)) = self.peek() {
            match ch {
                '{' => {
                    depth += 1;
                    self.bump();
                    if depth == 1 {
                        body_start = Some(self.current_index());
                    }
                }
                '}' => {
                    if depth == 0 {
                        return Err(self.error_here("代码块括号不匹配", start));
                    }
                    let end = self.current_index();
                    self.bump();
                    depth -= 1;
                    if depth == 0 {
                        let start_idx = body_start.unwrap_or(end);
                        return Ok(self.source[start_idx..end].to_string());
                    }
                }
                '"' | '\'' => {
                    self.lex_string()?;
                }
                _ => {
                    self.bump();
                }
            }
        }

        Err(LexError::new(
            "未闭合的代码块",
            Span {
                start,
                end: self.current_index(),
                line: self.line,
                column: self.column,
            },
        ))
    }

    fn skip_inline_ws_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            let Some((_, ch)) = self.peek() else {
                return Ok(());
            };

            if ch == '\n' || !ch.is_whitespace() {
                if self.starts_with("//") {
                    self.skip_line_comment();
                    continue;
                }
                if self.starts_with("/*") {
                    self.skip_block_comment()?;
                    continue;
                }
                return Ok(());
            }

            self.bump();
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some((_, ch)) = self.peek() {
            self.bump();
            if ch == '\n' {
                break;
            }
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), LexError> {
        let start = self.current_index();
        let start_line = self.line;
        let start_column = self.column;
        self.consume_exact("/*")?;

        while self.peek().is_some() {
            if self.starts_with("*/") {
                self.consume_exact("*/")?;
                return Ok(());
            }
            self.bump();
        }

        Err(LexError::new(
            "未闭合的块注释",
            Span {
                start,
                end: self.current_index(),
                line: start_line,
                column: start_column,
            },
        ))
    }

    fn single_char(&mut self, kind: TokenKind) -> Token {
        let (start, ch) = self.peek().expect("single-char token must exist");
        let span = self.make_span(start, start + ch.len_utf8());
        self.bump();
        Token { kind, span }
    }

    fn double_char(&mut self, kind: TokenKind) -> Token {
        let (start, _) = self.peek().expect("double-char token must exist");
        self.bump();
        self.bump();
        Token {
            kind,
            span: Span {
                start,
                end: self.current_index(),
                line: self.line,
                column: self.column.saturating_sub(2),
            },
        }
    }

    fn consume_exact(&mut self, expected: &str) -> Result<(), LexError> {
        for expected_ch in expected.chars() {
            match self.peek() {
                Some((idx, actual)) if actual == expected_ch => {
                    self.bump();
                    if idx > self.source.len() {
                        return Err(self.error_here("内部索引错误", idx));
                    }
                }
                Some((idx, _)) => return Err(self.error_here(format!("期望 '{expected}'"), idx)),
                None => return Err(self.error_here(format!("期望 '{expected}'"), self.source.len())),
            }
        }
        Ok(())
    }

    fn starts_with(&mut self, expected: &str) -> bool {
        self.source[self.current_index()..].starts_with(expected)
    }

    fn peek(&mut self) -> Option<(usize, char)> {
        self.chars.peek().copied()
    }

    fn bump(&mut self) -> Option<(usize, char)> {
        let next = self.chars.next();
        if let Some((_, ch)) = next {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        next
    }

    fn current_index(&mut self) -> usize {
        self.peek().map(|(idx, _)| idx).unwrap_or(self.source.len())
    }

    fn make_span(&self, start: usize, end: usize) -> Span {
        Span {
            start,
            end,
            line: self.line,
            column: self.column,
        }
    }

    fn error_here(&self, message: impl Into<String>, start: usize) -> LexError {
        LexError::new(
            message,
            Span {
                start,
                end: start,
                line: self.line,
                column: self.column,
            },
        )
    }
}

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Lexer::new(source).lex()
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_alphabetic() || is_cjk(ch)
}

fn is_identifier_continue(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_chinese_keywords() {
        let tokens = lex("令 变量 = 真\n如果 变量 返回 假").unwrap();
        assert!(matches!(tokens[0].kind, TokenKind::Let));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(_)));
        assert!(matches!(tokens[2].kind, TokenKind::Assign));
        assert!(matches!(tokens[3].kind, TokenKind::True));
        assert!(matches!(tokens[5].kind, TokenKind::If));
        assert!(matches!(tokens[7].kind, TokenKind::Return));
        assert!(matches!(tokens[8].kind, TokenKind::False));
    }

    #[test]
    fn lexes_braced_code_block_after_keyword() {
        let tokens = lex("代码块 { print(1); if (x) { y(); } }").unwrap();
        assert_eq!(
            tokens[0].kind,
            TokenKind::CodeBlock(" print(1); if (x) { y(); } ".to_string())
        );
    }

    #[test]
    fn lexes_fenced_code_block() {
        let source = "```rust\nfn main() {}\n```";
        let tokens = lex(source).unwrap();
        assert_eq!(tokens[0].kind, TokenKind::CodeBlock("fn main() {}\n".into()));
    }
}
