#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaiExecStmt {
    Let { name: String, ty: Option<String>, value: Option<TaiExecExpr> },
    Print(TaiExecExpr),
    Return(Option<TaiExecExpr>),
    Break,
    Continue,
    If {
        condition: TaiExecExpr,
        then_branch: Vec<TaiExecStmt>,
        else_branch: Option<Vec<TaiExecStmt>>,
    },
    Match {
        subject: TaiExecExpr,
        branches: Vec<(TaiExecExpr, Vec<TaiExecStmt>)>,
        default_branch: Option<Vec<TaiExecStmt>>,
    },
    While { condition: TaiExecExpr, body: Vec<TaiExecStmt> },
    Expr(TaiExecExpr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaiExecExpr {
    Identifier(String),
    Number(String),
    String(String),
    Bool(bool),
    Null,
    Array(Vec<TaiExecExpr>),
    Object(Vec<(String, TaiExecExpr)>),
    Unary { op: TaiExecUnaryOp, right: Box<TaiExecExpr> },
    Binary { left: Box<TaiExecExpr>, op: TaiExecBinaryOp, right: Box<TaiExecExpr> },
    Assign { target: Box<TaiExecExpr>, value: Box<TaiExecExpr> },
    Call { callee: Box<TaiExecExpr>, arguments: Vec<TaiExecExpr> },
    Member { object: Box<TaiExecExpr>, property: String },
    Index { object: Box<TaiExecExpr>, index: Box<TaiExecExpr> },
    Grouping(Box<TaiExecExpr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaiExecUnaryOp { Not, Negate, Positive }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaiExecBinaryOp {
    Or, And, Equal, NotEqual, Greater, GreaterEqual, Less, LessEqual, Add, Subtract, Multiply, Divide, Modulo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaiExecError {
    pub message: String,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Keyword { Let, Print, If, Else, ElseIf, While, Return, Break, Continue, MatchStart, Case, Default, True, False, Null, And, Or, Not, Begin, End }

#[derive(Debug, Clone, PartialEq, Eq)]
enum TokenKind {
    Keyword(Keyword),
    Identifier(String),
    Number(String),
    String(String),
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Comma,
    Colon,
    Dot,
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
    CmpEqual,
    CmpNotEqual,
    CmpGreater,
    CmpGreaterEqual,
    CmpLess,
    CmpLessEqual,
    Newline,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    kind: TokenKind,
    offset: usize,
}

pub fn parse_native_tai_exec(source: &str) -> Result<Vec<TaiExecStmt>, TaiExecError> {
    let tokens = TaiExecLexer::new(source).lex()?;
    TaiExecParser::new(tokens).parse()
}

pub fn render_native_tai_exec_to_rust(statements: &[TaiExecStmt]) -> String {
    let mut out = String::new();
    render_statements(statements, 0, &mut out);
    out.trim_end().to_string()
}

pub fn render_native_tai_expr_to_rust(source: &str) -> Result<String, TaiExecError> {
    let wrapped = format!(".返回 {}", source);
    let statements = parse_native_tai_exec(&wrapped)?;
    match statements.first() {
        Some(TaiExecStmt::Return(Some(expr))) => Ok(render_expr(expr)),
        _ => Err(TaiExecError {
            message: "无法解析表达式".to_string(),
            offset: 0,
        }),
    }
}

struct TaiExecLexer<'a> {
    input: &'a str,
    index: usize,
}

impl<'a> TaiExecLexer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, index: 0 }
    }

    fn lex(mut self) -> Result<Vec<Token>, TaiExecError> {
        let mut tokens = Vec::new();
        while let Some(ch) = self.peek() {
            if ch == '\n' {
                tokens.push(Token { kind: TokenKind::Newline, offset: self.index });
                self.bump();
                continue;
            }
            if ch.is_whitespace() {
                self.bump();
                continue;
            }
            let offset = self.index;
            let kind = match ch {
                '(' => { self.bump(); TokenKind::LeftParen }
                ')' => { self.bump(); TokenKind::RightParen }
                '[' => { self.bump(); TokenKind::LeftBracket }
                ']' => { self.bump(); TokenKind::RightBracket }
                '{' => { self.bump(); TokenKind::LeftBrace }
                '}' => { self.bump(); TokenKind::RightBrace }
                ',' => { self.bump(); TokenKind::Comma }
                ':' => { self.bump(); TokenKind::Colon }
                '+' => { self.bump(); TokenKind::Plus }
                '-' => { self.bump(); TokenKind::Minus }
                '*' => { self.bump(); TokenKind::Star }
                '/' => { self.bump(); TokenKind::Slash }
                '%' => { self.bump(); TokenKind::Percent }
                '.' => self.lex_dot_prefixed_or_member()?,
                '=' => {
                    self.bump();
                    if self.peek() == Some('=') { self.bump(); TokenKind::Equal } else { TokenKind::Assign }
                }
                '!' => {
                    self.bump();
                    if self.peek() == Some('=') { self.bump(); TokenKind::NotEqual } else {
                        return Err(TaiExecError { message: "非法字符 '!'".to_string(), offset });
                    }
                }
                '>' => {
                    self.bump();
                    if self.peek() == Some('=') { self.bump(); TokenKind::GreaterEqual } else { TokenKind::Greater }
                }
                '<' => {
                    self.bump();
                    if self.peek() == Some('=') { self.bump(); TokenKind::LessEqual } else { TokenKind::Less }
                }
                '"' => TokenKind::String(self.lex_string()?),
                _ if ch.is_ascii_digit() => self.lex_number(),
                _ if is_exec_identifier_start(ch) => self.lex_identifier_or_number(),
                _ => return Err(TaiExecError { message: format!("非法字符 '{}'", ch), offset }),
            };
            tokens.push(Token { kind, offset });
        }
        tokens.push(Token { kind: TokenKind::Eof, offset: self.index });
        Ok(tokens)
    }

    fn lex_dot_prefixed_or_member(&mut self) -> Result<TokenKind, TaiExecError> {
        let offset = self.index;
        self.bump();
        let Some(next) = self.peek() else { return Ok(TokenKind::Dot) };
        if !is_exec_identifier_start(next) { return Ok(TokenKind::Dot) }
        let start = self.index;
        while let Some(ch) = self.peek() {
            if is_exec_identifier_continue(ch) { self.bump(); } else { break; }
        }
        let ident = self.input[start..self.index].to_string();
        let keyword = match ident.as_str() {
            "令" => Some(Keyword::Let),
            "显示" => Some(Keyword::Print),
            "如果" | "若" => Some(Keyword::If),
            "否则如果" => Some(Keyword::ElseIf),
            "否则" => Some(Keyword::Else),
            "循环当" | "循环判断首" => Some(Keyword::While),
            "返回" => Some(Keyword::Return),
            "跳出循环" => Some(Keyword::Break),
            "到循环尾" => Some(Keyword::Continue),
            "判断开始" => Some(Keyword::MatchStart),
            "判断" => Some(Keyword::Case),
            "默认" => Some(Keyword::Default),
            "真" => Some(Keyword::True),
            "假" => Some(Keyword::False),
            "空" => Some(Keyword::Null),
            "并且" => Some(Keyword::And),
            "或者" => Some(Keyword::Or),
            "非" => Some(Keyword::Not),
            "开始" => Some(Keyword::Begin),
            "结束" => Some(Keyword::End),
            "如果结束" => Some(Keyword::End),
            "循环判断尾" => Some(Keyword::End),
            "判断结束" => Some(Keyword::End),
            "version" => Some(Keyword::Begin),
            "if" => Some(Keyword::If),
            "else" => Some(Keyword::Else),
            "while" => Some(Keyword::While),
            "return" => Some(Keyword::Return),
            "print" => Some(Keyword::Print),
            "break" => Some(Keyword::Break),
            "continue" => Some(Keyword::Continue),
            "match" => Some(Keyword::MatchStart),
            "case" => Some(Keyword::Case),
            "default" => Some(Keyword::Default),
            "true" => Some(Keyword::True),
            "false" => Some(Keyword::False),
            "null" => Some(Keyword::Null),
            "and" => Some(Keyword::And),
            "or" => Some(Keyword::Or),
            "not" => Some(Keyword::Not),
            "end" => Some(Keyword::End),
            _ => None,
        };
        match keyword {
            Some(keyword) => Ok(TokenKind::Keyword(keyword)),
            None => Err(TaiExecError { message: format!("未知的点号执行关键字 '.{}'", ident), offset }),
        }
    }

    fn lex_string(&mut self) -> Result<String, TaiExecError> {
        let start = self.index;
        self.bump();
        let mut value = String::new();
        while let Some(ch) = self.peek() {
            match ch {
                '"' => { self.bump(); return Ok(value); }
                '\\' => {
                    self.bump();
                    let escaped = self.peek().ok_or(TaiExecError { message: "未闭合的字符串转义".to_string(), offset: self.index })?;
                    self.bump();
                    value.push(match escaped { 'n' => '\n', 'r' => '\r', 't' => '\t', '"' => '"', '\\' => '\\', other => other });
                }
                other => { self.bump(); value.push(other); }
            }
        }
        Err(TaiExecError { message: "未闭合的字符串字面量".to_string(), offset: start })
    }

    fn lex_identifier_or_number(&mut self) -> TokenKind {
        let start = self.index;
        while let Some(ch) = self.peek() {
            if is_exec_identifier_continue(ch) { self.bump(); } else { break; }
        }
        let value = self.input[start..self.index].to_string();
        match value.as_str() {
            "令" => TokenKind::Keyword(Keyword::Let),
            "显示" => TokenKind::Keyword(Keyword::Print),
            "如果" | "若" => TokenKind::Keyword(Keyword::If),
            "否则如果" => TokenKind::Keyword(Keyword::ElseIf),
            "否则" => TokenKind::Keyword(Keyword::Else),
            "循环当" | "循环判断首" => TokenKind::Keyword(Keyword::While),
            "返回" => TokenKind::Keyword(Keyword::Return),
            "跳出循环" => TokenKind::Keyword(Keyword::Break),
            "到循环尾" => TokenKind::Keyword(Keyword::Continue),
            "判断开始" => TokenKind::Keyword(Keyword::MatchStart),
            "判断" => TokenKind::Keyword(Keyword::Case),
            "默认" => TokenKind::Keyword(Keyword::Default),
            "真" => TokenKind::Keyword(Keyword::True),
            "假" => TokenKind::Keyword(Keyword::False),
            "空" => TokenKind::Keyword(Keyword::Null),
            "并且" => TokenKind::Keyword(Keyword::And),
            "或者" => TokenKind::Keyword(Keyword::Or),
            "非" => TokenKind::Keyword(Keyword::Not),
            "开始" => TokenKind::Keyword(Keyword::Begin),
            "结束" | "如果结束" | "循环判断尾" | "判断结束" => TokenKind::Keyword(Keyword::End),
            "true" => TokenKind::Keyword(Keyword::True),
            "false" => TokenKind::Keyword(Keyword::False),
            "null" => TokenKind::Keyword(Keyword::Null),
            "and" => TokenKind::Keyword(Keyword::And),
            "or" => TokenKind::Keyword(Keyword::Or),
            "not" => TokenKind::Keyword(Keyword::Not),
            "等于" => TokenKind::CmpEqual,
            "不等于" => TokenKind::CmpNotEqual,
            "大于" => TokenKind::CmpGreater,
            "大于或等于" => TokenKind::CmpGreaterEqual,
            "小于" => TokenKind::CmpLess,
            "小于或等于" => TokenKind::CmpLessEqual,
            "==" => TokenKind::CmpEqual,
            "!=" => TokenKind::CmpNotEqual,
            _ if value.chars().all(|ch| ch.is_ascii_digit()) => TokenKind::Number(value),
            _ => TokenKind::Identifier(value),
        }
    }

    fn lex_number(&mut self) -> TokenKind {
        let start = self.index;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }
        TokenKind::Number(self.input[start..self.index].to_string())
    }

    fn peek(&self) -> Option<char> { self.input[self.index..].chars().next() }
    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            self.index += ch.len_utf8();
        }
    }
}

struct TaiExecParser {
    tokens: Vec<Token>,
    current: usize,
}

impl TaiExecParser {
    fn new(tokens: Vec<Token>) -> Self { Self { tokens, current: 0 } }
    fn parse(&mut self) -> Result<Vec<TaiExecStmt>, TaiExecError> { self.parse_statements(false, false) }

    fn parse_statements(&mut self, stop_on_end: bool, stop_on_else: bool) -> Result<Vec<TaiExecStmt>, TaiExecError> {
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() {
            if stop_on_end && self.check_keyword(Keyword::End) { break; }
            if stop_on_else && self.check_keyword(Keyword::Else) { break; }
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<TaiExecStmt, TaiExecError> {
        if self.check_identifier_local_decl() { return self.parse_typed_local_decl(); }
        if self.match_keyword(Keyword::Let) { return self.parse_let(); }
        if self.match_keyword(Keyword::Print) { return self.parse_print(); }
        if self.match_keyword(Keyword::If) { return self.parse_if(); }
        if self.match_keyword(Keyword::MatchStart) { return self.parse_match(); }
        if self.match_keyword(Keyword::While) { return self.parse_while(); }
        if self.match_keyword(Keyword::Return) { return self.parse_return(); }
        if self.match_keyword(Keyword::Break) { return Ok(TaiExecStmt::Break); }
        if self.match_keyword(Keyword::Continue) { return Ok(TaiExecStmt::Continue); }
        Ok(TaiExecStmt::Expr(self.parse_expression()?))
    }

    fn parse_let(&mut self) -> Result<TaiExecStmt, TaiExecError> {
        let name = self.consume_identifier("需要变量名")?;
        let value = if self.match_token(|kind| matches!(kind, TokenKind::Assign)) { Some(self.parse_expression()?) } else { None };
        Ok(TaiExecStmt::Let { name, ty: None, value })
    }

    fn parse_typed_local_decl(&mut self) -> Result<TaiExecStmt, TaiExecError> {
        let name = self.consume_identifier("需要变量名")?;
        self.consume_token(|kind| matches!(kind, TokenKind::Colon), "变量声明缺少 ':'")?;
        let ty = self.consume_identifier("变量声明缺少类型")?;
        let value = if self.match_token(|kind| matches!(kind, TokenKind::Assign)) {
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(TaiExecStmt::Let { name, ty: Some(ty), value })
    }

    fn parse_print(&mut self) -> Result<TaiExecStmt, TaiExecError> {
        Ok(TaiExecStmt::Print(self.parse_expression()?))
    }

    fn parse_if(&mut self) -> Result<TaiExecStmt, TaiExecError> {
        let condition = self.parse_expression()?;
        if self.match_keyword(Keyword::Begin) {
            // support transitional block style
        }
        let then_branch = self.parse_statements(true, true)?;
        let else_branch = if self.match_keyword(Keyword::ElseIf) {
            let nested_if = self.parse_if()?;
            Some(vec![nested_if])
        } else if self.match_keyword(Keyword::Else) {
            if self.match_keyword(Keyword::Begin) {
                // support transitional block style
            }
            let branch = self.parse_statements(true, false)?;
            Some(branch)
        } else { None };
        self.consume_keyword(Keyword::End, ".如果 分支后缺少 .如果结束")?;
        Ok(TaiExecStmt::If { condition, then_branch, else_branch })
    }

    fn parse_while(&mut self) -> Result<TaiExecStmt, TaiExecError> {
        let condition = self.parse_expression()?;
        if self.match_keyword(Keyword::Begin) {
            // support transitional block style
        }
        let body = self.parse_statements(true, false)?;
        self.consume_keyword(Keyword::End, ".循环判断首 结束时缺少 .循环判断尾")?;
        Ok(TaiExecStmt::While { condition, body })
    }

    fn parse_match(&mut self) -> Result<TaiExecStmt, TaiExecError> {
        let subject = self.parse_expression()?;
        let mut branches = Vec::new();
        let mut default_branch = None;

        loop {
            self.skip_newlines();
            if self.check_keyword(Keyword::End) {
                break;
            }

            if self.match_keyword(Keyword::Case) {
                let value = self.parse_expression()?;
                let branch = self.parse_match_branch_statements()?;
                branches.push((value, branch));
                continue;
            }

            if self.match_keyword(Keyword::Default) {
                default_branch = Some(self.parse_match_branch_statements()?);
                continue;
            }

            return Err(self.error_here(".判断开始 内部需要 .判断 / .默认 / .判断结束"));
        }

        self.consume_keyword(Keyword::End, ".判断开始 结束时缺少 .判断结束")?;
        Ok(TaiExecStmt::Match {
            subject,
            branches,
            default_branch,
        })
    }

    fn parse_match_branch_statements(&mut self) -> Result<Vec<TaiExecStmt>, TaiExecError> {
        let mut statements = Vec::new();
        self.skip_newlines();
        while !self.is_at_end() {
            if self.check_keyword(Keyword::End)
                || self.check_keyword(Keyword::Case)
                || self.check_keyword(Keyword::Default)
            {
                break;
            }
            statements.push(self.parse_statement()?);
            self.skip_newlines();
        }
        Ok(statements)
    }

    fn parse_return(&mut self) -> Result<TaiExecStmt, TaiExecError> {
        if self.check(|kind| matches!(kind, TokenKind::Newline | TokenKind::Eof | TokenKind::Keyword(Keyword::End))) {
            return Ok(TaiExecStmt::Return(None));
        }
        Ok(TaiExecStmt::Return(Some(self.parse_expression()?)))
    }

    fn parse_expression(&mut self) -> Result<TaiExecExpr, TaiExecError> { self.parse_assignment() }

    fn parse_assignment(&mut self) -> Result<TaiExecExpr, TaiExecError> {
        let expr = self.parse_or()?;
        if self.match_token(|kind| matches!(kind, TokenKind::Assign)) {
            let value = self.parse_assignment()?;
            return match expr {
                TaiExecExpr::Identifier(_) | TaiExecExpr::Member { .. } | TaiExecExpr::Index { .. } =>
                    Ok(TaiExecExpr::Assign { target: Box::new(expr), value: Box::new(value) }),
                _ => Err(self.error_here("赋值目标非法")),
            };
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<TaiExecExpr, TaiExecError> {
        let mut expr = self.parse_and()?;
        while self.match_keyword(Keyword::Or) {
            let right = self.parse_and()?;
            expr = TaiExecExpr::Binary { left: Box::new(expr), op: TaiExecBinaryOp::Or, right: Box::new(right) };
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<TaiExecExpr, TaiExecError> {
        let mut expr = self.parse_equality()?;
        while self.match_keyword(Keyword::And) {
            let right = self.parse_equality()?;
            expr = TaiExecExpr::Binary { left: Box::new(expr), op: TaiExecBinaryOp::And, right: Box::new(right) };
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<TaiExecExpr, TaiExecError> {
        let mut expr = self.parse_comparison()?;
        loop {
            let op = if self.match_token(|kind| matches!(kind, TokenKind::Equal | TokenKind::CmpEqual)) {
                Some(TaiExecBinaryOp::Equal)
            } else if self.match_token(|kind| matches!(kind, TokenKind::NotEqual | TokenKind::CmpNotEqual)) {
                Some(TaiExecBinaryOp::NotEqual)
            } else { None };
            if let Some(op) = op {
                let right = self.parse_comparison()?;
                expr = TaiExecExpr::Binary { left: Box::new(expr), op, right: Box::new(right) };
            } else { break; }
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<TaiExecExpr, TaiExecError> {
        let mut expr = self.parse_term()?;
        loop {
            let op = if self.match_token(|kind| matches!(kind, TokenKind::Greater | TokenKind::CmpGreater)) {
                Some(TaiExecBinaryOp::Greater)
            } else if self.match_token(|kind| matches!(kind, TokenKind::GreaterEqual | TokenKind::CmpGreaterEqual)) {
                Some(TaiExecBinaryOp::GreaterEqual)
            } else if self.match_token(|kind| matches!(kind, TokenKind::Less | TokenKind::CmpLess)) {
                Some(TaiExecBinaryOp::Less)
            } else if self.match_token(|kind| matches!(kind, TokenKind::LessEqual | TokenKind::CmpLessEqual)) {
                Some(TaiExecBinaryOp::LessEqual)
            } else { None };
            if let Some(op) = op {
                let right = self.parse_term()?;
                expr = TaiExecExpr::Binary { left: Box::new(expr), op, right: Box::new(right) };
            } else { break; }
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<TaiExecExpr, TaiExecError> {
        let mut expr = self.parse_factor()?;
        loop {
            let op = if self.match_token(|kind| matches!(kind, TokenKind::Plus)) {
                Some(TaiExecBinaryOp::Add)
            } else if self.match_token(|kind| matches!(kind, TokenKind::Minus)) {
                Some(TaiExecBinaryOp::Subtract)
            } else { None };
            if let Some(op) = op {
                let right = self.parse_factor()?;
                expr = TaiExecExpr::Binary { left: Box::new(expr), op, right: Box::new(right) };
            } else { break; }
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<TaiExecExpr, TaiExecError> {
        let mut expr = self.parse_unary()?;
        loop {
            let op = if self.match_token(|kind| matches!(kind, TokenKind::Star)) {
                Some(TaiExecBinaryOp::Multiply)
            } else if self.match_token(|kind| matches!(kind, TokenKind::Slash)) {
                Some(TaiExecBinaryOp::Divide)
            } else if self.match_token(|kind| matches!(kind, TokenKind::Percent)) {
                Some(TaiExecBinaryOp::Modulo)
            } else { None };
            if let Some(op) = op {
                let right = self.parse_unary()?;
                expr = TaiExecExpr::Binary { left: Box::new(expr), op, right: Box::new(right) };
            } else { break; }
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<TaiExecExpr, TaiExecError> {
        if self.match_keyword(Keyword::Not) {
            return Ok(TaiExecExpr::Unary { op: TaiExecUnaryOp::Not, right: Box::new(self.parse_unary()?) });
        }
        if self.match_token(|kind| matches!(kind, TokenKind::Minus)) {
            return Ok(TaiExecExpr::Unary { op: TaiExecUnaryOp::Negate, right: Box::new(self.parse_unary()?) });
        }
        if self.match_token(|kind| matches!(kind, TokenKind::Plus)) {
            return Ok(TaiExecExpr::Unary { op: TaiExecUnaryOp::Positive, right: Box::new(self.parse_unary()?) });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<TaiExecExpr, TaiExecError> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.match_token(|kind| matches!(kind, TokenKind::LeftParen)) {
                let mut arguments = Vec::new();
                if !self.check(|kind| matches!(kind, TokenKind::RightParen)) {
                    loop {
                        arguments.push(self.parse_expression()?);
                        if !self.match_token(|kind| matches!(kind, TokenKind::Comma)) { break; }
                    }
                }
                self.consume_token(|kind| matches!(kind, TokenKind::RightParen), "参数列表后缺少 ')'")?;
                expr = TaiExecExpr::Call { callee: Box::new(expr), arguments };
                continue;
            }
            if self.match_token(|kind| matches!(kind, TokenKind::Dot)) {
                let property = self.consume_identifier("成员访问后需要属性名")?;
                expr = TaiExecExpr::Member { object: Box::new(expr), property };
                continue;
            }
            if self.match_token(|kind| matches!(kind, TokenKind::LeftBracket)) {
                let index = self.parse_expression()?;
                self.consume_token(|kind| matches!(kind, TokenKind::RightBracket), "下标表达式后缺少 ']'")?;
                expr = TaiExecExpr::Index { object: Box::new(expr), index: Box::new(index) };
                continue;
            }
            break;
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<TaiExecExpr, TaiExecError> {
        if self.match_keyword(Keyword::True) { return Ok(TaiExecExpr::Bool(true)); }
        if self.match_keyword(Keyword::False) { return Ok(TaiExecExpr::Bool(false)); }
        if self.match_keyword(Keyword::Null) { return Ok(TaiExecExpr::Null); }
        if let Some(value) = self.match_identifier() { return Ok(TaiExecExpr::Identifier(value)); }
        if let Some(value) = self.match_number() { return Ok(TaiExecExpr::Number(value)); }
        if let Some(value) = self.match_string() { return Ok(TaiExecExpr::String(value)); }
        if self.match_token(|kind| matches!(kind, TokenKind::LeftBracket)) {
            let mut items = Vec::new();
            if !self.check(|kind| matches!(kind, TokenKind::RightBracket)) {
                loop {
                    items.push(self.parse_expression()?);
                    if !self.match_token(|kind| matches!(kind, TokenKind::Comma)) {
                        break;
                    }
                }
            }
            self.consume_token(|kind| matches!(kind, TokenKind::RightBracket), "数组字面量后缺少 ']'")?;
            return Ok(TaiExecExpr::Array(items));
        }
        if self.match_token(|kind| matches!(kind, TokenKind::LeftBrace)) {
            let mut entries = Vec::new();
            if !self.check(|kind| matches!(kind, TokenKind::RightBrace)) {
                loop {
                    let key = if let Some(value) = self.match_string() {
                        value
                    } else if let Some(value) = self.match_identifier() {
                        value
                    } else {
                        return Err(self.error_here("对象字面量的键必须是字符串或标识符"));
                    };
                    self.consume_token(|kind| matches!(kind, TokenKind::Colon), "对象字面量键后缺少 ':'")?;
                    let value = self.parse_expression()?;
                    entries.push((key, value));
                    if !self.match_token(|kind| matches!(kind, TokenKind::Comma)) {
                        break;
                    }
                }
            }
            self.consume_token(|kind| matches!(kind, TokenKind::RightBrace), "对象字面量后缺少 '}'")?;
            return Ok(TaiExecExpr::Object(entries));
        }
        if self.match_token(|kind| matches!(kind, TokenKind::LeftParen)) {
            let expr = self.parse_expression()?;
            self.consume_token(|kind| matches!(kind, TokenKind::RightParen), "表达式后缺少 ')'")?;
            return Ok(TaiExecExpr::Grouping(Box::new(expr)));
        }
        Err(self.error_here("需要表达式"))
    }

    fn skip_newlines(&mut self) { while self.match_token(|kind| matches!(kind, TokenKind::Newline)) {} }
    fn match_keyword(&mut self, keyword: Keyword) -> bool { self.match_token(|kind| matches!(kind, TokenKind::Keyword(k) if *k == keyword)) }
    fn consume_keyword(&mut self, keyword: Keyword, message: &str) -> Result<(), TaiExecError> {
        self.consume_token(|kind| matches!(kind, TokenKind::Keyword(k) if *k == keyword), message).map(|_| ())
    }
    fn match_identifier(&mut self) -> Option<String> {
        match &self.peek().kind { TokenKind::Identifier(v) => { let v=v.clone(); self.advance(); Some(v) }, _ => None }
    }
    fn match_number(&mut self) -> Option<String> {
        match &self.peek().kind { TokenKind::Number(v) => { let v=v.clone(); self.advance(); Some(v) }, _ => None }
    }
    fn match_string(&mut self) -> Option<String> {
        match &self.peek().kind { TokenKind::String(v) => { let v=v.clone(); self.advance(); Some(v) }, _ => None }
    }
    fn consume_identifier(&mut self, message: &str) -> Result<String, TaiExecError> {
        self.match_identifier().ok_or_else(|| self.error_here(message))
    }
    fn consume_token<F>(&mut self, predicate: F, message: &str) -> Result<Token, TaiExecError>
    where F: Fn(&TokenKind) -> bool {
        if self.check(predicate) { Ok(self.advance().clone()) } else { Err(self.error_here(message)) }
    }
    fn match_token<F>(&mut self, predicate: F) -> bool
    where F: Fn(&TokenKind) -> bool {
        if self.check(&predicate) { self.advance(); true } else { false }
    }
    fn check<F>(&self, predicate: F) -> bool
    where F: Fn(&TokenKind) -> bool { predicate(&self.peek().kind) }
    fn check_keyword(&self, keyword: Keyword) -> bool { self.check(|kind| matches!(kind, TokenKind::Keyword(k) if *k == keyword)) }
    fn advance(&mut self) -> &Token { if !self.is_at_end() { self.current += 1; } self.previous() }
    fn peek(&self) -> &Token { &self.tokens[self.current] }
    fn previous(&self) -> &Token { &self.tokens[self.current.saturating_sub(1)] }
    fn is_at_end(&self) -> bool { matches!(self.peek().kind, TokenKind::Eof) }
    fn error_here(&self, message: &str) -> TaiExecError { TaiExecError { message: message.to_string(), offset: self.peek().offset } }

    fn check_identifier_local_decl(&self) -> bool {
        matches!(&self.peek().kind, TokenKind::Identifier(_))
            && self
                .tokens
                .get(self.current + 1)
                .map(|token| matches!(token.kind, TokenKind::Colon))
                .unwrap_or(false)
    }
}

fn render_statements(statements: &[TaiExecStmt], indent: usize, out: &mut String) {
    for stmt in statements { render_statement(stmt, indent, out); }
}

fn render_statement(stmt: &TaiExecStmt, indent: usize, out: &mut String) {
    let padding = "    ".repeat(indent);
    match stmt {
        TaiExecStmt::Let { name, value, .. } => {
            if let Some(value) = value {
                out.push_str(&format!("{padding}let {} = {};\n", name, render_expr(value)));
            } else {
                out.push_str(&format!("{padding}let {} = ();\n", name));
            }
        }
        TaiExecStmt::Print(value) => {
            out.push_str(&format!("{padding}println!(\"{{}}\", {});\n", render_expr(value)));
        }
        TaiExecStmt::Return(value) => {
            if let Some(value) = value { out.push_str(&format!("{padding}return {};\n", render_expr(value))); }
            else { out.push_str(&format!("{padding}return;\n")); }
        }
        TaiExecStmt::Break => out.push_str(&format!("{padding}break;\n")),
        TaiExecStmt::Continue => out.push_str(&format!("{padding}continue;\n")),
        TaiExecStmt::Expr(expr) => out.push_str(&format!("{padding}{};\n", render_expr(expr))),
        TaiExecStmt::If { condition, then_branch, else_branch } => {
            out.push_str(&format!("{padding}if {} {{\n", render_expr(condition)));
            render_statements(then_branch, indent + 1, out);
            if let Some(else_branch) = else_branch {
                out.push_str(&format!("{padding}}} else {{\n"));
                render_statements(else_branch, indent + 1, out);
            }
            out.push_str(&format!("{padding}}}\n"));
        }
        TaiExecStmt::While { condition, body } => {
            out.push_str(&format!("{padding}while {} {{\n", render_expr(condition)));
            render_statements(body, indent + 1, out);
            out.push_str(&format!("{padding}}}\n"));
        }
        TaiExecStmt::Match { subject, branches, default_branch } => {
            out.push_str(&format!("{padding}match {} {{\n", render_expr(subject)));
            for (value, branch) in branches {
                out.push_str(&format!("{padding}    {} => {{\n", render_expr(value)));
                render_statements(branch, indent + 2, out);
                out.push_str(&format!("{padding}    }},\n"));
            }
            if let Some(default_branch) = default_branch {
                out.push_str(&format!("{padding}    _ => {{\n"));
                render_statements(default_branch, indent + 2, out);
                out.push_str(&format!("{padding}    }},\n"));
            }
            out.push_str(&format!("{padding}}}\n"));
        }
    }
}

fn render_expr(expr: &TaiExecExpr) -> String {
    match expr {
        TaiExecExpr::Identifier(v) => v.clone(),
        TaiExecExpr::Number(v) => v.clone(),
        TaiExecExpr::String(v) => format!("{:?}", v),
        TaiExecExpr::Bool(v) => v.to_string(),
        TaiExecExpr::Null => "()".to_string(),
        TaiExecExpr::Array(items) => {
            let items = items.iter().map(render_expr).collect::<Vec<_>>().join(", ");
            format!("vec![{}]", items)
        }
        TaiExecExpr::Object(entries) => {
            let entries = entries
                .iter()
                .map(|(key, value)| format!("({:?}.to_string(), {})", key, render_expr(value)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("std::collections::BTreeMap::from([{}])", entries)
        }
        TaiExecExpr::Grouping(expr) => format!("({})", render_expr(expr)),
        TaiExecExpr::Unary { op, right } => {
            let op = match op { TaiExecUnaryOp::Not => "!", TaiExecUnaryOp::Negate => "-", TaiExecUnaryOp::Positive => "+" };
            format!("{op}{}", render_expr(right))
        }
        TaiExecExpr::Binary { left, op, right } => {
            let op = match op {
                TaiExecBinaryOp::Or => "||", TaiExecBinaryOp::And => "&&", TaiExecBinaryOp::Equal => "==",
                TaiExecBinaryOp::NotEqual => "!=", TaiExecBinaryOp::Greater => ">", TaiExecBinaryOp::GreaterEqual => ">=",
                TaiExecBinaryOp::Less => "<", TaiExecBinaryOp::LessEqual => "<=", TaiExecBinaryOp::Add => "+",
                TaiExecBinaryOp::Subtract => "-", TaiExecBinaryOp::Multiply => "*", TaiExecBinaryOp::Divide => "/",
                TaiExecBinaryOp::Modulo => "%",
            };
            format!("{} {} {}", render_expr(left), op, render_expr(right))
        }
        TaiExecExpr::Assign { target, value } => format!("{} = {}", render_expr(target), render_expr(value)),
        TaiExecExpr::Call { callee, arguments } => {
            let args = arguments.iter().map(render_expr).collect::<Vec<_>>().join(", ");
            format!("{}({})", render_expr(callee), args)
        }
        TaiExecExpr::Member { object, property } => format!("{}.{}", render_expr(object), property),
        TaiExecExpr::Index { object, index } => format!("{}[{}]", render_expr(object), render_expr(index)),
    }
}

fn is_exec_identifier_start(ch: char) -> bool { ch == '_' || ch.is_alphabetic() }
fn is_exec_identifier_continue(ch: char) -> bool { is_exec_identifier_start(ch) || ch.is_ascii_digit() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_native_exec_if_else() {
        let source = r#"
.令 结果 = 用户名
.显示 "开始检查"
.如果 结果 等于 ""
    .返回 "空"
.否则
    .返回 结果
.如果结束
"#;
        let statements = parse_native_tai_exec(source).expect("parse should succeed");
        assert_eq!(statements.len(), 3);
        assert!(matches!(statements[0], TaiExecStmt::Let { .. }));
        assert!(matches!(statements[1], TaiExecStmt::Print(..)));
        assert!(matches!(statements[2], TaiExecStmt::If { .. }));
    }

    #[test]
    fn renders_native_exec_to_rust() {
        let source = r#"
.令 结果 = 用户名
.显示 "准备返回"
.如果 结果 等于 ""
    .返回 "空"
.如果结束
.返回 结果
"#;
        let statements = parse_native_tai_exec(source).expect("parse should succeed");
        let rust = render_native_tai_exec_to_rust(&statements);
        assert!(rust.contains("let 结果 = 用户名;"));
        assert!(rust.contains("println!(\"{}\", \"准备返回\");"));
        assert!(rust.contains("if 结果 == \"\" {"));
        assert!(rust.contains("return 结果;"));
    }

    #[test]
    fn parses_print_statement() {
        let source = r#"
.显示 "Hello World"
.返回 0
"#;
        let statements = parse_native_tai_exec(source).expect("parse should succeed");
        assert!(matches!(statements[0], TaiExecStmt::Print(TaiExecExpr::String(_))));
        assert!(matches!(statements[1], TaiExecStmt::Return(Some(TaiExecExpr::Number(_)))));
    }

    #[test]
    fn parses_match_and_loop_jump() {
        let source = r#"
.判断开始 状态
.判断 "成功"
    .返回 "ok"
.默认
    .返回 "unknown"
.判断结束

.循环判断首 计数 小于 10
    .跳出循环
.循环判断尾
"#;
        let statements = parse_native_tai_exec(source).expect("parse should succeed");
        assert!(matches!(statements[0], TaiExecStmt::Match { .. }));
        assert!(matches!(statements[1], TaiExecStmt::While { .. }));
    }

    #[test]
    fn parses_array_object_and_index_assignment() {
        let source = r#"
.令 列表 = [1, 2, 3]
.令 配置 = {"名称": "结衣", 启用: 真}
列表[0] = 10
.返回 配置["名称"]
"#;
        let statements = parse_native_tai_exec(source).expect("parse should succeed");
        assert!(matches!(statements[0], TaiExecStmt::Let { .. }));
        assert!(matches!(statements[1], TaiExecStmt::Let { .. }));
        assert!(matches!(statements[2], TaiExecStmt::Expr(TaiExecExpr::Assign { .. })));
        assert!(matches!(statements[3], TaiExecStmt::Return(Some(TaiExecExpr::Index { .. }))));
    }

    #[test]
    fn renders_native_expr_to_rust() {
        let rust = render_native_tai_expr_to_rust(r#"{"名称": "结衣", "年龄": 1}"#)
            .expect("expr render should succeed");
        assert!(rust.contains("BTreeMap::from"));
        assert!(rust.contains("\"名称\".to_string()"));
    }

    #[test]
    fn parses_eyuyan_style_keywords_without_dot_prefix() {
        let source = r#"
令 名称 = "结衣"
如果 名称 等于 "结衣"
    返回 真
否则
    返回 假
如果结束
"#;
        let statements = parse_native_tai_exec(source).expect("parse should succeed");
        assert!(matches!(statements[0], TaiExecStmt::Let { .. }));
        assert!(matches!(statements[1], TaiExecStmt::If { .. }));
    }

    #[test]
    fn parses_typed_local_declaration() {
        let source = r#"
结果: 整数型 = 0
.返回 结果
"#;
        let statements = parse_native_tai_exec(source).expect("parse should succeed");
        match &statements[0] {
            TaiExecStmt::Let { name, ty, .. } => {
                assert_eq!(name, "结果");
                assert_eq!(ty.as_deref(), Some("整数型"));
            }
            _ => panic!("expected typed local declaration"),
        }
    }
}
