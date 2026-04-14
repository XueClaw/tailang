use crate::parser::{self, Program, Stmt, Expr, Span};

/// 中间表示 (Intermediate Representation)
#[derive(Debug, Clone, PartialEq)]
pub struct IRProgram {
    pub functions: Vec<IRFunction>,
    pub variables: Vec<IRVariable>,
    pub instructions: Vec<IRInstruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRFunction {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<IRInstruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IRVariable {
    pub name: String,
    pub value: Option<IRExpression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IRInstruction {
    /// 变量声明
    Declare(IRVariable),
    /// 赋值
    Assign {
        target: String,
        value: IRExpression,
    },
    /// 条件分支
    Conditional {
        condition: IRExpression,
        then_branch: Vec<IRInstruction>,
        else_branch: Option<Vec<IRInstruction>>,
    },
    /// 循环
    Loop {
        condition: IRExpression,
        body: Vec<IRInstruction>,
    },
    /// 返回
    Return(Option<IRExpression>),
    /// 表达式语句
    Expr(IRExpression),
    /// 函数调用
    Call {
        callee: String,
        arguments: Vec<IRExpression>,
    },
    /// 代码块引用
    CodeBlock(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IRExpression {
    Identifier(String),
    Number(String),
    String(String),
    Bool(bool),
    Null,
    Binary {
        left: Box<IRExpression>,
        op: String,
        right: Box<IRExpression>,
    },
    Unary {
        op: String,
        operand: Box<IRExpression>,
    },
    Assign {
        target: Box<IRExpression>,
        value: Box<IRExpression>,
    },
    Call {
        callee: Box<IRExpression>,
        arguments: Vec<IRExpression>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslateError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for TranslateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {:?}", self.message, self.span)
    }
}

impl std::error::Error for TranslateError {}

/// 翻译器：AST → IR
pub struct Translator {
    functions: Vec<IRFunction>,
    variables: Vec<IRVariable>,
    instructions: Vec<IRInstruction>,
}

impl Translator {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            variables: Vec::new(),
            instructions: Vec::new(),
        }
    }

    pub fn translate(mut self, program: Program) -> Result<IRProgram, TranslateError> {
        for stmt in program.statements {
            self.translate_stmt(stmt)?;
        }

        Ok(IRProgram {
            functions: self.functions,
            variables: self.variables,
            instructions: self.instructions,
        })
    }

    fn translate_stmt(&mut self, stmt: Stmt) -> Result<(), TranslateError> {
        match stmt {
            Stmt::FunctionDecl(func) => {
                let mut body_instructions = Vec::new();
                for stmt in func.body.statements {
                    // 递归翻译函数体
                    let mut sub_translator = Translator::new();
                    sub_translator.translate_stmt(stmt)?;
                    body_instructions.extend(sub_translator.instructions);
                }

                self.functions.push(IRFunction {
                    name: func.name,
                    params: func.params,
                    body: body_instructions,
                });
            }

            Stmt::VarDecl(var) => {
                let value = match var.value {
                    Some(expr) => Some(self.translate_expr(expr)?),
                    None => None,
                };

                self.variables.push(IRVariable {
                    name: var.name,
                    value,
                });
            }

            Stmt::IfStmt(if_stmt) => {
                let condition = self.translate_expr(if_stmt.condition)?;

                let mut then_instructions = Vec::new();
                for stmt in if_stmt.then_branch.statements {
                    let mut sub_translator = Translator::new();
                    sub_translator.translate_stmt(stmt)?;
                    then_instructions.extend(sub_translator.instructions);
                }

                let else_instructions = if let Some(else_branch) = if_stmt.else_branch {
                    let mut else_instructions = Vec::new();
                    for stmt in else_branch.statements {
                        let mut sub_translator = Translator::new();
                        sub_translator.translate_stmt(stmt)?;
                        else_instructions.extend(sub_translator.instructions);
                    }
                    Some(else_instructions)
                } else {
                    None
                };

                self.instructions.push(IRInstruction::Conditional {
                    condition,
                    then_branch: then_instructions,
                    else_branch: else_instructions,
                });
            }

            Stmt::WhileStmt(while_stmt) => {
                let condition = self.translate_expr(while_stmt.condition)?;

                let mut body_instructions = Vec::new();
                for stmt in while_stmt.body.statements {
                    let mut sub_translator = Translator::new();
                    sub_translator.translate_stmt(stmt)?;
                    body_instructions.extend(sub_translator.instructions);
                }

                self.instructions.push(IRInstruction::Loop {
                    condition,
                    body: body_instructions,
                });
            }

            Stmt::ReturnStmt(return_stmt) => {
                let value = match return_stmt.value {
                    Some(expr) => Some(self.translate_expr(expr)?),
                    None => None,
                };

                self.instructions.push(IRInstruction::Return(value));
            }

            Stmt::ExprStmt(expr_stmt) => {
                let ir_expr = self.translate_expr(expr_stmt.expr)?;
                self.instructions.push(IRInstruction::Expr(ir_expr));
            }

            Stmt::BlockStmt(block) => {
                for stmt in block.statements {
                    self.translate_stmt(stmt)?;
                }
            }
        }

        Ok(())
    }

    fn translate_expr(&self, expr: Expr) -> Result<IRExpression, TranslateError> {
        match expr {
            Expr::Identifier(name) => Ok(IRExpression::Identifier(name)),
            Expr::Number(value) => Ok(IRExpression::Number(value)),
            Expr::String(value) => Ok(IRExpression::String(value)),
            Expr::Bool(value) => Ok(IRExpression::Bool(value)),
            Expr::Null => Ok(IRExpression::Null),
            Expr::CodeBlock(code) => {
                // 代码块作为特殊表达式处理
                Ok(IRExpression::Identifier(format!("__codeblock_{}", code.len())))
            }

            Expr::Binary { left, op, right } => {
                let op_str = match op {
                    parser::BinaryOp::Or => "||",
                    parser::BinaryOp::And => "&&",
                    parser::BinaryOp::Equal => "==",
                    parser::BinaryOp::NotEqual => "!=",
                    parser::BinaryOp::Greater => ">",
                    parser::BinaryOp::GreaterEqual => ">=",
                    parser::BinaryOp::Less => "<",
                    parser::BinaryOp::LessEqual => "<=",
                    parser::BinaryOp::Add => "+",
                    parser::BinaryOp::Subtract => "-",
                    parser::BinaryOp::Multiply => "*",
                    parser::BinaryOp::Divide => "/",
                    parser::BinaryOp::Modulo => "%",
                };

                Ok(IRExpression::Binary {
                    left: Box::new(self.translate_expr(*left)?),
                    op: op_str.to_string(),
                    right: Box::new(self.translate_expr(*right)?),
                })
            }

            Expr::Unary { op, right } => {
                let op_str = match op {
                    parser::UnaryOp::Not => "!",
                    parser::UnaryOp::Negate => "-",
                    parser::UnaryOp::Positive => "+",
                };

                Ok(IRExpression::Unary {
                    op: op_str.to_string(),
                    operand: Box::new(self.translate_expr(*right)?),
                })
            }

            Expr::Assign { target, value } => Ok(IRExpression::Assign {
                target: Box::new(self.translate_expr(*target)?),
                value: Box::new(self.translate_expr(*value)?),
            }),

            Expr::Call { callee, arguments } => {
                let ir_args: Result<Vec<IRExpression>, TranslateError> =
                    arguments.into_iter().map(|arg| self.translate_expr(arg)).collect();

                Ok(IRExpression::Call {
                    callee: Box::new(self.translate_expr(*callee)?),
                    arguments: ir_args?,
                })
            }

            Expr::Grouping(expr) => self.translate_expr(*expr),
            Expr::Member { object, property } => {
                // 简化处理：成员访问转为 identifier
                Ok(IRExpression::Identifier(format!("{}.{}", 
                    match *object {
                        Expr::Identifier(name) => name,
                        _ => "object".to_string(),
                    },
                    property
                )))
            }
            Expr::Index { object, index } => {
                // 简化处理：索引访问转为 identifier
                Ok(IRExpression::Identifier(format!("{}[{}]",
                    match *object {
                        Expr::Identifier(name) => name,
                        _ => "array".to_string(),
                    },
                    match *index {
                        Expr::Number(n) => n,
                        _ => "index".to_string(),
                    }
                )))
            }
            Expr::Array(items) => {
                // 数组字面量暂不展开
                Ok(IRExpression::Identifier("__array_literal".to_string()))
            }
            Expr::Object(fields) => {
                // 对象字面量暂不展开
                Ok(IRExpression::Identifier("__object_literal".to_string()))
            }
        }
    }
}

impl Default for Translator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{Parser, lexer};

    fn parse_and_translate(source: &str) -> Result<IRProgram, TranslateError> {
        // 词法分析
        let tokens = lexer::Lexer::new(source).lex().expect("lex failed");
        
        // 语法分析
        let program = Parser::new(tokens).parse().expect("parse failed");
        
        // 翻译
        Translator::new().translate(program)
    }

    #[test]
    fn test_translate_variable_declaration() {
        let source = r#"令 x = 10"#;
        let ir = parse_and_translate(source).expect("translate failed");
        
        assert_eq!(ir.variables.len(), 1);
        assert_eq!(ir.variables[0].name, "x");
        assert!(matches!(
            &ir.variables[0].value,
            Some(IRExpression::Number(n)) if n == "10"
        ));
    }

    #[test]
    fn test_translate_if_else() {
        let source = r#"
如果 真 {
    返回 1
} 否则 {
    返回 2
}
"#;
        let ir = parse_and_translate(source).expect("translate failed");
        
        assert_eq!(ir.instructions.len(), 1);
        assert!(matches!(
            &ir.instructions[0],
            IRInstruction::Conditional { .. }
        ));
    }

    #[test]
    fn test_translate_while_loop() {
        let source = r#"
当 真 {
    返回 x
}
"#;
        let ir = parse_and_translate(source).expect("translate failed");
        
        assert_eq!(ir.instructions.len(), 1);
        assert!(matches!(
            &ir.instructions[0],
            IRInstruction::Loop { .. }
        ));
    }

    #[test]
    fn test_translate_function() {
        let source = r#"
函数 加法 (a, b) {
    返回 a + b
}
"#;
        let ir = parse_and_translate(source).expect("translate failed");
        
        assert_eq!(ir.functions.len(), 1);
        assert_eq!(ir.functions[0].name, "加法");
        assert_eq!(ir.functions[0].params, vec!["a", "b"]);
    }

    #[test]
    fn test_translate_binary_expression() {
        let source = r#"令 x = 1 + 2 * 3"#;
        let ir = parse_and_translate(source).expect("translate failed");
        
        assert_eq!(ir.variables.len(), 1);
        assert!(matches!(
            &ir.variables[0].value,
            Some(IRExpression::Binary { op, .. }) if op == "+"
        ));
    }
}
