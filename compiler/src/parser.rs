use crate::lexer::{Token, TokenKind, Span};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    FunctionDecl(FunctionDecl),
    VarDecl(VarDecl),
    IfStmt(IfStmt),
    WhileStmt(WhileStmt),
    ReturnStmt(ReturnStmt),
    ExprStmt(ExprStmt),
    BlockStmt(BlockStmt),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub name: String,
    pub value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_branch: BlockStmt,
    pub else_branch: Option<BlockStmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub condition: Expr,
    pub body: BlockStmt,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    pub expr: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockStmt {
    pub statements: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Identifier(String),
    Number(String),
    String(String),
    Bool(bool),
    Null,
    CodeBlock(String),
    Array(Vec<Expr>),
    Object(Vec<ObjectField>),
    Grouping(Box<Expr>),
    Unary {
        op: UnaryOp,
        right: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
    },
    Member {
        object: Box<Expr>,
        property: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectField {
    pub key: String,
    pub value: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Not,
    Negate,
    Positive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Or,
    And,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {:?}", self.message, self.span)
    }
}

impl std::error::Error for ParseError {}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    pub fn parse(mut self) -> Result<Program, ParseError> {
        self.parse_program()
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut statements = Vec::new();
        self.skip_separators();

        while !self.is_at_end() {
            statements.push(self.parse_declaration()?);
            self.skip_separators();
        }

        Ok(Program { statements })
    }

    fn parse_declaration(&mut self) -> Result<Stmt, ParseError> {
        self.skip_separators();

        if self.matches(|k| matches!(k, TokenKind::Function)) {
            return Ok(Stmt::FunctionDecl(self.parse_function_decl()?));
        }

        if self.matches(|k| matches!(k, TokenKind::Let)) {
            return Ok(Stmt::VarDecl(self.parse_var_decl()?));
        }

        self.parse_statement()
    }

    fn parse_function_decl(&mut self) -> Result<FunctionDecl, ParseError> {
        let name = self.consume_identifier("expected function name")?;
        self.consume(
            |k| matches!(k, TokenKind::LeftParen),
            "expected '(' after function name",
        )?;

        let mut params = Vec::new();
        self.skip_newlines();

        if !self.check(|k| matches!(k, TokenKind::RightParen)) {
            loop {
                params.push(self.consume_identifier("expected parameter name")?);
                self.skip_newlines();

                if !self.matches(|k| matches!(k, TokenKind::Comma)) {
                    break;
                }

                self.skip_newlines();
            }
        }

        self.consume(
            |k| matches!(k, TokenKind::RightParen),
            "expected ')' after parameters",
        )?;
        self.skip_newlines();

        let body = self.parse_block_stmt()?;
        Ok(FunctionDecl { name, params, body })
    }

    fn parse_var_decl(&mut self) -> Result<VarDecl, ParseError> {
        let name = self.consume_identifier("expected variable name")?;
        self.skip_newlines();

        let value = if self.matches(|k| matches!(k, TokenKind::Assign)) {
            self.skip_newlines();
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.consume_statement_terminator();
        Ok(VarDecl { name, value })
    }

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        self.skip_separators();

        if self.matches(|k| matches!(k, TokenKind::If)) {
            return Ok(Stmt::IfStmt(self.parse_if_stmt()?));
        }

        if self.matches(|k| matches!(k, TokenKind::While)) {
            return Ok(Stmt::WhileStmt(self.parse_while_stmt()?));
        }

        if self.matches(|k| matches!(k, TokenKind::Return)) {
            return Ok(Stmt::ReturnStmt(self.parse_return_stmt()?));
        }

        if self.check(|k| matches!(k, TokenKind::LeftBrace)) {
            return Ok(Stmt::BlockStmt(self.parse_block_stmt()?));
        }

        self.parse_expr_stmt().map(Stmt::ExprStmt)
    }

    fn parse_if_stmt(&mut self) -> Result<IfStmt, ParseError> {
        self.skip_newlines();
        let condition = self.parse_expression()?;
        self.skip_newlines();
        let then_branch = self.parse_block_stmt()?;
        self.skip_newlines();

        let else_branch = if self.matches(|k| matches!(k, TokenKind::Else)) {
            self.skip_newlines();

            if self.matches(|k| matches!(k, TokenKind::If)) {
                let nested = self.parse_if_stmt()?;
                Some(BlockStmt {
                    statements: vec![Stmt::IfStmt(nested)],
                })
            } else {
                Some(self.parse_block_stmt()?)
            }
        } else {
            None
        };

        Ok(IfStmt {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_while_stmt(&mut self) -> Result<WhileStmt, ParseError> {
        self.skip_newlines();
        let condition = self.parse_expression()?;
        self.skip_newlines();
        let body = self.parse_block_stmt()?;

        Ok(WhileStmt { condition, body })
    }

    fn parse_return_stmt(&mut self) -> Result<ReturnStmt, ParseError> {
        self.skip_newlines();

        let value = if self.check(|k| {
            matches!(
                k,
                TokenKind::Semicolon
                    | TokenKind::Newline
                    | TokenKind::RightBrace
                    | TokenKind::Eof
            )
        }) {
            None
        } else {
            Some(self.parse_expression()?)
        };

        self.consume_statement_terminator();
        Ok(ReturnStmt { value })
    }

    fn parse_expr_stmt(&mut self) -> Result<ExprStmt, ParseError> {
        let expr = self.parse_expression()?;
        self.consume_statement_terminator();
        Ok(ExprStmt { expr })
    }

    fn parse_block_stmt(&mut self) -> Result<BlockStmt, ParseError> {
        self.consume(
            |k| matches!(k, TokenKind::LeftBrace),
            "expected '{' to start block",
        )?;

        let mut statements = Vec::new();
        self.skip_separators();

        while !self.check(|k| matches!(k, TokenKind::RightBrace)) && !self.is_at_end() {
            statements.push(self.parse_declaration()?);
            self.skip_separators();
        }

        self.consume(
            |k| matches!(k, TokenKind::RightBrace),
            "expected '}' after block",
        )?;

        Ok(BlockStmt { statements })
    }

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_or()?;
        self.skip_newlines();

        if self.matches(|k| matches!(k, TokenKind::Assign)) {
            self.skip_newlines();
            let value = self.parse_assignment()?;

            match expr {
                Expr::Identifier(_) | Expr::Member { .. } | Expr::Index { .. } => Ok(Expr::Assign {
                    target: Box::new(expr),
                    value: Box::new(value),
                }),
                _ => Err(self.error_current("invalid assignment target")),
            }
        } else {
            Ok(expr)
        }
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_and()?;

        loop {
            self.skip_newlines();

            if self.matches(|k| matches!(k, TokenKind::Or)) {
                let right = self.parse_and()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::Or,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_equality()?;

        loop {
            self.skip_newlines();

            if self.matches(|k| matches!(k, TokenKind::And)) {
                let right = self.parse_equality()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op: BinaryOp::And,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;

        loop {
            self.skip_newlines();

            let op = if self.matches(|k| matches!(k, TokenKind::Equal)) {
                Some(BinaryOp::Equal)
            } else if self.matches(|k| matches!(k, TokenKind::NotEqual)) {
                Some(BinaryOp::NotEqual)
            } else {
                None
            };

            if let Some(op) = op {
                let right = self.parse_comparison()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_term()?;

        loop {
            self.skip_newlines();

            let op = if self.matches(|k| matches!(k, TokenKind::Greater)) {
                Some(BinaryOp::Greater)
            } else if self.matches(|k| matches!(k, TokenKind::GreaterEqual)) {
                Some(BinaryOp::GreaterEqual)
            } else if self.matches(|k| matches!(k, TokenKind::Less)) {
                Some(BinaryOp::Less)
            } else if self.matches(|k| matches!(k, TokenKind::LessEqual)) {
                Some(BinaryOp::LessEqual)
            } else {
                None
            };

            if let Some(op) = op {
                let right = self.parse_term()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_factor()?;

        loop {
            self.skip_newlines();

            let op = if self.matches(|k| matches!(k, TokenKind::Plus)) {
                Some(BinaryOp::Add)
            } else if self.matches(|k| matches!(k, TokenKind::Minus)) {
                Some(BinaryOp::Subtract)
            } else {
                None
            };

            if let Some(op) = op {
                let right = self.parse_factor()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;

        loop {
            self.skip_newlines();

            let op = if self.matches(|k| matches!(k, TokenKind::Star)) {
                Some(BinaryOp::Multiply)
            } else if self.matches(|k| matches!(k, TokenKind::Slash)) {
                Some(BinaryOp::Divide)
            } else if self.matches(|k| matches!(k, TokenKind::Percent)) {
                Some(BinaryOp::Modulo)
            } else {
                None
            };

            if let Some(op) = op {
                let right = self.parse_unary()?;
                expr = Expr::Binary {
                    left: Box::new(expr),
                    op,
                    right: Box::new(right),
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        self.skip_newlines();

        if self.matches(|k| matches!(k, TokenKind::Not)) {
            let right = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                right: Box::new(right),
            });
        }

        if self.matches(|k| matches!(k, TokenKind::Minus)) {
            let right = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Negate,
                right: Box::new(right),
            });
        }

        if self.matches(|k| matches!(k, TokenKind::Plus)) {
            let right = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Positive,
                right: Box::new(right),
            });
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            self.skip_newlines();

            if self.matches(|k| matches!(k, TokenKind::LeftParen)) {
                let mut arguments = Vec::new();
                self.skip_newlines();

                if !self.check(|k| matches!(k, TokenKind::RightParen)) {
                    loop {
                        arguments.push(self.parse_expression()?);
                        self.skip_newlines();

                        if !self.matches(|k| matches!(k, TokenKind::Comma)) {
                            break;
                        }

                        self.skip_newlines();
                    }
                }

                self.consume(
                    |k| matches!(k, TokenKind::RightParen),
                    "expected ')' after arguments",
                )?;

                expr = Expr::Call {
                    callee: Box::new(expr),
                    arguments,
                };
                continue;
            }

            if self.matches(|k| matches!(k, TokenKind::Dot)) {
                let property = self.consume_identifier("expected property name after '.'")?;
                expr = Expr::Member {
                    object: Box::new(expr),
                    property,
                };
                continue;
            }

            if self.matches(|k| matches!(k, TokenKind::LeftBracket)) {
                let index = self.parse_expression()?;
                self.consume(
                    |k| matches!(k, TokenKind::RightBracket),
                    "expected ']' after index",
                )?;

                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
                continue;
            }

            break;
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        self.skip_newlines();

        if self.matches(|k| matches!(k, TokenKind::True)) {
            return Ok(Expr::Bool(true));
        }

        if self.matches(|k| matches!(k, TokenKind::False)) {
            return Ok(Expr::Bool(false));
        }

        if self.matches(|k| matches!(k, TokenKind::Null)) {
            return Ok(Expr::Null);
        }

        if self.matches(|k| matches!(k, TokenKind::CodeBlockKeyword)) {
            let token = self.consume(
                |k| matches!(k, TokenKind::CodeBlock(_)),
                "expected code block literal after code block keyword",
            )?;

            if let TokenKind::CodeBlock(value) = &token.kind {
                return Ok(Expr::CodeBlock(value.clone()));
            }
        }

        if let Some(name) = self.match_identifier() {
            return Ok(Expr::Identifier(name));
        }

        if let Some(value) = self.match_number() {
            return Ok(Expr::Number(value));
        }

        if let Some(value) = self.match_string() {
            return Ok(Expr::String(value));
        }

        if let Some(value) = self.match_code_block() {
            return Ok(Expr::CodeBlock(value));
        }

        if self.matches(|k| matches!(k, TokenKind::LeftParen)) {
            let expr = self.parse_expression()?;
            self.consume(
                |k| matches!(k, TokenKind::RightParen),
                "expected ')' after expression",
            )?;
            return Ok(Expr::Grouping(Box::new(expr)));
        }

        if self.matches(|k| matches!(k, TokenKind::LeftBracket)) {
            let mut items = Vec::new();
            self.skip_newlines();

            if !self.check(|k| matches!(k, TokenKind::RightBracket)) {
                loop {
                    items.push(self.parse_expression()?);
                    self.skip_newlines();

                    if !self.matches(|k| matches!(k, TokenKind::Comma)) {
                        break;
                    }

                    self.skip_newlines();
                }
            }

            self.consume(
                |k| matches!(k, TokenKind::RightBracket),
                "expected ']' after array literal",
            )?;
            return Ok(Expr::Array(items));
        }

        if self.matches(|k| matches!(k, TokenKind::LeftBrace)) {
            let mut fields = Vec::new();
            self.skip_newlines();

            if !self.check(|k| matches!(k, TokenKind::RightBrace)) {
                loop {
                    let key = match &self.peek().kind {
                        TokenKind::Identifier(name) => {
                            let name = name.clone();
                            self.advance();
                            name
                        }
                        TokenKind::String(value) => {
                            let value = value.clone();
                            self.advance();
                            value
                        }
                        _ => {
                            return Err(self.error_current(
                                "expected identifier or string as object key",
                            ))
                        }
                    };

                    self.consume(
                        |k| matches!(k, TokenKind::Colon),
                        "expected ':' after object key",
                    )?;
                    self.skip_newlines();

                    let value = self.parse_expression()?;
                    fields.push(ObjectField { key, value });
                    self.skip_newlines();

                    if !self.matches(|k| matches!(k, TokenKind::Comma)) {
                        break;
                    }

                    self.skip_newlines();
                }
            }

            self.consume(
                |k| matches!(k, TokenKind::RightBrace),
                "expected '}' after object literal",
            )?;
            return Ok(Expr::Object(fields));
        }

        Err(self.error_current("expected expression"))
    }

    fn consume_statement_terminator(&mut self) {
        while self.matches(|k| matches!(k, TokenKind::Semicolon | TokenKind::Newline)) {}
    }

    fn skip_separators(&mut self) {
        while self.matches(|k| matches!(k, TokenKind::Semicolon | TokenKind::Newline)) {}
    }

    fn skip_newlines(&mut self) {
        while self.matches(|k| matches!(k, TokenKind::Newline)) {}
    }

    fn consume<F>(&mut self, predicate: F, message: &str) -> Result<Token, ParseError>
    where
        F: Fn(&TokenKind) -> bool,
    {
        if self.check(&predicate) {
            Ok(self.advance().clone())
        } else {
            Err(self.error_current(message))
        }
    }

    fn consume_identifier(&mut self, message: &str) -> Result<String, ParseError> {
        match &self.peek().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(self.error_current(message)),
        }
    }

    fn match_identifier(&mut self) -> Option<String> {
        match &self.peek().kind {
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Some(name)
            }
            _ => None,
        }
    }

    fn match_number(&mut self) -> Option<String> {
        match &self.peek().kind {
            TokenKind::Number(value) => {
                let value = value.clone();
                self.advance();
                Some(value)
            }
            _ => None,
        }
    }

    fn match_string(&mut self) -> Option<String> {
        match &self.peek().kind {
            TokenKind::String(value) => {
                let value = value.clone();
                self.advance();
                Some(value)
            }
            _ => None,
        }
    }

    fn match_code_block(&mut self) -> Option<String> {
        match &self.peek().kind {
            TokenKind::CodeBlock(value) => {
                let value = value.clone();
                self.advance();
                Some(value)
            }
            _ => None,
        }
    }

    fn matches<F>(&mut self, predicate: F) -> bool
    where
        F: Fn(&TokenKind) -> bool,
    {
        if self.check(&predicate) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check<F>(&self, predicate: F) -> bool
    where
        F: Fn(&TokenKind) -> bool,
    {
        if self.is_at_end() {
            predicate(&TokenKind::Eof)
        } else {
            predicate(&self.peek().kind)
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }

        self.previous()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current.saturating_sub(1)]
    }

    fn error_current(&self, message: &str) -> ParseError {
        ParseError {
            message: message.to_string(),
            span: self.peek().span.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span() -> Span {
        Span {
            start: 0,
            end: 0,
            line: 1,
            column: 1,
        }
    }

    fn token(kind: TokenKind) -> Token {
        Token { kind, span: span() }
    }

    fn parse(tokens: Vec<Token>) -> Program {
        Parser::new(tokens).parse().expect("parse should succeed")
    }

    #[test]
    fn parse_variable_declaration() {
        let program = parse(vec![
            token(TokenKind::Let),
            token(TokenKind::Identifier("x".into())),
            token(TokenKind::Assign),
            token(TokenKind::Number("42".into())),
            token(TokenKind::Semicolon),
            token(TokenKind::Eof),
        ]);

        assert_eq!(
            program,
            Program {
                statements: vec![Stmt::VarDecl(VarDecl {
                    name: "x".into(),
                    value: Some(Expr::Number("42".into())),
                })],
            }
        );
    }

    #[test]
    fn parse_function_declaration() {
        let program = parse(vec![
            token(TokenKind::Function),
            token(TokenKind::Identifier("add".into())),
            token(TokenKind::LeftParen),
            token(TokenKind::Identifier("a".into())),
            token(TokenKind::Comma),
            token(TokenKind::Identifier("b".into())),
            token(TokenKind::RightParen),
            token(TokenKind::LeftBrace),
            token(TokenKind::Return),
            token(TokenKind::Identifier("a".into())),
            token(TokenKind::Plus),
            token(TokenKind::Identifier("b".into())),
            token(TokenKind::Semicolon),
            token(TokenKind::RightBrace),
            token(TokenKind::Eof),
        ]);

        assert_eq!(
            program,
            Program {
                statements: vec![Stmt::FunctionDecl(FunctionDecl {
                    name: "add".into(),
                    params: vec!["a".into(), "b".into()],
                    body: BlockStmt {
                        statements: vec![Stmt::ReturnStmt(ReturnStmt {
                            value: Some(Expr::Binary {
                                left: Box::new(Expr::Identifier("a".into())),
                                op: BinaryOp::Add,
                                right: Box::new(Expr::Identifier("b".into())),
                            }),
                        })],
                    },
                })],
            }
        );
    }

    #[test]
    fn parse_if_else_statement() {
        let program = parse(vec![
            token(TokenKind::If),
            token(TokenKind::True),
            token(TokenKind::LeftBrace),
            token(TokenKind::Return),
            token(TokenKind::Number("1".into())),
            token(TokenKind::Semicolon),
            token(TokenKind::RightBrace),
            token(TokenKind::Else),
            token(TokenKind::LeftBrace),
            token(TokenKind::Return),
            token(TokenKind::Number("2".into())),
            token(TokenKind::Semicolon),
            token(TokenKind::RightBrace),
            token(TokenKind::Eof),
        ]);

        assert_eq!(
            program,
            Program {
                statements: vec![Stmt::IfStmt(IfStmt {
                    condition: Expr::Bool(true),
                    then_branch: BlockStmt {
                        statements: vec![Stmt::ReturnStmt(ReturnStmt {
                            value: Some(Expr::Number("1".into())),
                        })],
                    },
                    else_branch: Some(BlockStmt {
                        statements: vec![Stmt::ReturnStmt(ReturnStmt {
                            value: Some(Expr::Number("2".into())),
                        })],
                    }),
                })],
            }
        );
    }

    #[test]
    fn parse_while_statement() {
        let program = parse(vec![
            token(TokenKind::While),
            token(TokenKind::Identifier("running".into())),
            token(TokenKind::LeftBrace),
            token(TokenKind::Identifier("tick".into())),
            token(TokenKind::LeftParen),
            token(TokenKind::RightParen),
            token(TokenKind::Semicolon),
            token(TokenKind::RightBrace),
            token(TokenKind::Eof),
        ]);

        assert_eq!(
            program,
            Program {
                statements: vec![Stmt::WhileStmt(WhileStmt {
                    condition: Expr::Identifier("running".into()),
                    body: BlockStmt {
                        statements: vec![Stmt::ExprStmt(ExprStmt {
                            expr: Expr::Call {
                                callee: Box::new(Expr::Identifier("tick".into())),
                                arguments: vec![],
                            },
                        })],
                    },
                })],
            }
        );
    }

    #[test]
    fn parse_expression_precedence() {
        let program = parse(vec![
            token(TokenKind::Number("1".into())),
            token(TokenKind::Plus),
            token(TokenKind::Number("2".into())),
            token(TokenKind::Star),
            token(TokenKind::Number("3".into())),
            token(TokenKind::Semicolon),
            token(TokenKind::Eof),
        ]);

        assert_eq!(
            program,
            Program {
                statements: vec![Stmt::ExprStmt(ExprStmt {
                    expr: Expr::Binary {
                        left: Box::new(Expr::Number("1".into())),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Binary {
                            left: Box::new(Expr::Number("2".into())),
                            op: BinaryOp::Multiply,
                            right: Box::new(Expr::Number("3".into())),
                        }),
                    },
                })],
            }
        );
    }

    #[test]
    fn parse_assignment_expression() {
        let program = parse(vec![
            token(TokenKind::Identifier("x".into())),
            token(TokenKind::Assign),
            token(TokenKind::Number("10".into())),
            token(TokenKind::Semicolon),
            token(TokenKind::Eof),
        ]);

        assert_eq!(
            program,
            Program {
                statements: vec![Stmt::ExprStmt(ExprStmt {
                    expr: Expr::Assign {
                        target: Box::new(Expr::Identifier("x".into())),
                        value: Box::new(Expr::Number("10".into())),
                    },
                })],
            }
        );
    }

    #[test]
    fn invalid_assignment_target_should_fail() {
        let err = Parser::new(vec![
            token(TokenKind::Number("1".into())),
            token(TokenKind::Assign),
            token(TokenKind::Number("2".into())),
            token(TokenKind::Semicolon),
            token(TokenKind::Eof),
        ])
        .parse()
        .expect_err("parse should fail");

        assert!(err.message.contains("invalid assignment target"));
    }
}
