use crate::tai_ast::{TaiFunctionDecl, TaiProgram};
use crate::tai_exec::{parse_native_tai_exec, TaiExecExpr, TaiExecStmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirProgram {
    pub entry_label: String,
    pub body: Vec<HirStmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirStmt {
    Let { name: String, value: Option<HirExpr> },
    Assign { name: String, value: HirExpr },
    Print(HirExpr),
    Return(Option<HirExpr>),
    If {
        condition: HirExpr,
        then_branch: Vec<HirStmt>,
        else_branch: Vec<HirStmt>,
    },
    While {
        condition: HirExpr,
        body: Vec<HirStmt>,
    },
    Match {
        subject: HirExpr,
        branches: Vec<(HirExpr, Vec<HirStmt>)>,
        default_branch: Vec<HirStmt>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirExpr {
    Identifier(String),
    Number(i64),
    String(String),
    Bool(bool),
    Null,
    Unary {
        op: HirUnaryOp,
        right: Box<HirExpr>,
    },
    Binary {
        left: Box<HirExpr>,
        op: HirBinaryOp,
        right: Box<HirExpr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirUnaryOp {
    Not,
    Negate,
    Positive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirBinaryOp {
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

pub fn lower_tai_to_hir(program: &TaiProgram) -> Result<HirProgram, String> {
    let function = select_entry_function(program)?;
    let body = match &function.implementation {
        Some(implementation) => parse_native_tai_exec(implementation)
            .map_err(|err| format!("原生 .tai 执行语法解析失败：{}", err.message))?
            .iter()
            .map(lower_stmt)
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };

    Ok(HirProgram {
        entry_label: function.name.clone(),
        body,
    })
}

fn select_entry_function(program: &TaiProgram) -> Result<&TaiFunctionDecl, String> {
    for module in &program.modules {
        for function in &module.functions {
            if matches!(function.name.trim(), "主程序" | "主函数" | "main" | "Main") {
                return Ok(function);
            }
        }
    }

    program
        .modules
        .iter()
        .flat_map(|module| module.functions.iter())
        .next()
        .ok_or_else(|| "当前 .tai 程序没有可编译的 .子程序".to_string())
}

fn lower_stmt(stmt: &TaiExecStmt) -> Result<HirStmt, String> {
    match stmt {
        TaiExecStmt::Let { name, value } => Ok(HirStmt::Let {
            name: name.clone(),
            value: value.as_ref().map(lower_expr).transpose()?,
        }),
        TaiExecStmt::Print(expr) => Ok(HirStmt::Print(lower_expr(expr)?)),
        TaiExecStmt::Return(expr) => Ok(HirStmt::Return(expr.as_ref().map(lower_expr).transpose()?)),
        TaiExecStmt::If {
            condition,
            then_branch,
            else_branch,
        } => Ok(HirStmt::If {
            condition: lower_expr(condition)?,
            then_branch: then_branch.iter().map(lower_stmt).collect::<Result<Vec<_>, _>>()?,
            else_branch: else_branch
                .as_ref()
                .map(|items| items.iter().map(lower_stmt).collect::<Result<Vec<_>, _>>())
                .transpose()?
                .unwrap_or_default(),
        }),
        TaiExecStmt::While { condition, body } => Ok(HirStmt::While {
            condition: lower_expr(condition)?,
            body: body.iter().map(lower_stmt).collect::<Result<Vec<_>, _>>()?,
        }),
        TaiExecStmt::Match {
            subject,
            branches,
            default_branch,
        } => Ok(HirStmt::Match {
            subject: lower_expr(subject)?,
            branches: branches
                .iter()
                .map(|(expr, body)| {
                    Ok((
                        lower_expr(expr)?,
                        body.iter().map(lower_stmt).collect::<Result<Vec<_>, _>>()?,
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?,
            default_branch: default_branch
                .as_ref()
                .map(|items| items.iter().map(lower_stmt).collect::<Result<Vec<_>, _>>())
                .transpose()?
                .unwrap_or_default(),
        }),
        TaiExecStmt::Expr(TaiExecExpr::Assign { target, value }) => match target.as_ref() {
            TaiExecExpr::Identifier(name) => Ok(HirStmt::Assign {
                name: name.clone(),
                value: lower_expr(value)?,
            }),
            _ => Err("当前 HIR 只支持标识符赋值".to_string()),
        },
        TaiExecStmt::Break => Err("当前 HIR 暂不支持 .跳出循环".to_string()),
        TaiExecStmt::Continue => Err("当前 HIR 暂不支持 .到循环尾".to_string()),
        TaiExecStmt::Expr(_) => Err("当前 HIR 暂不支持独立表达式语句".to_string()),
    }
}

fn lower_expr(expr: &TaiExecExpr) -> Result<HirExpr, String> {
    match expr {
        TaiExecExpr::Identifier(name) => Ok(HirExpr::Identifier(name.clone())),
        TaiExecExpr::Number(value) => value
            .parse::<i64>()
            .map(HirExpr::Number)
            .map_err(|_| format!("无法编译数字字面量 '{}'", value)),
        TaiExecExpr::String(value) => Ok(HirExpr::String(value.clone())),
        TaiExecExpr::Bool(value) => Ok(HirExpr::Bool(*value)),
        TaiExecExpr::Null => Ok(HirExpr::Null),
        TaiExecExpr::Grouping(inner) => lower_expr(inner),
        TaiExecExpr::Unary { op, right } => Ok(HirExpr::Unary {
            op: match op {
                crate::tai_exec::TaiExecUnaryOp::Not => HirUnaryOp::Not,
                crate::tai_exec::TaiExecUnaryOp::Negate => HirUnaryOp::Negate,
                crate::tai_exec::TaiExecUnaryOp::Positive => HirUnaryOp::Positive,
            },
            right: Box::new(lower_expr(right)?),
        }),
        TaiExecExpr::Binary { left, op, right } => Ok(HirExpr::Binary {
            left: Box::new(lower_expr(left)?),
            op: match op {
                crate::tai_exec::TaiExecBinaryOp::Or => HirBinaryOp::Or,
                crate::tai_exec::TaiExecBinaryOp::And => HirBinaryOp::And,
                crate::tai_exec::TaiExecBinaryOp::Equal => HirBinaryOp::Equal,
                crate::tai_exec::TaiExecBinaryOp::NotEqual => HirBinaryOp::NotEqual,
                crate::tai_exec::TaiExecBinaryOp::Greater => HirBinaryOp::Greater,
                crate::tai_exec::TaiExecBinaryOp::GreaterEqual => HirBinaryOp::GreaterEqual,
                crate::tai_exec::TaiExecBinaryOp::Less => HirBinaryOp::Less,
                crate::tai_exec::TaiExecBinaryOp::LessEqual => HirBinaryOp::LessEqual,
                crate::tai_exec::TaiExecBinaryOp::Add => HirBinaryOp::Add,
                crate::tai_exec::TaiExecBinaryOp::Subtract => HirBinaryOp::Subtract,
                crate::tai_exec::TaiExecBinaryOp::Multiply => HirBinaryOp::Multiply,
                crate::tai_exec::TaiExecBinaryOp::Divide => HirBinaryOp::Divide,
                crate::tai_exec::TaiExecBinaryOp::Modulo => HirBinaryOp::Modulo,
            },
            right: Box::new(lower_expr(right)?),
        }),
        TaiExecExpr::Assign { .. } => Err("HIR 表达式中不支持嵌套赋值".to_string()),
        TaiExecExpr::Array(_) => Err("当前 HIR 暂不支持数组".to_string()),
        TaiExecExpr::Object(_) => Err("当前 HIR 暂不支持对象".to_string()),
        TaiExecExpr::Call { .. } => Err("当前 HIR 暂不支持函数调用".to_string()),
        TaiExecExpr::Member { .. } => Err("当前 HIR 暂不支持成员访问".to_string()),
        TaiExecExpr::Index { .. } => Err("当前 HIR 暂不支持下标访问".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tai_parser::TaiParser;

    #[test]
    fn lowers_program_to_hir() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序
.令 计数 = 1
.如果 计数 小于 3
    .显示 "ok"
.否则
    .显示 "bad"
.如果结束
.返回 0
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should lower");
        assert_eq!(hir.entry_label, "主程序");
        assert!(matches!(hir.body[0], HirStmt::Let { .. }));
        assert!(matches!(hir.body[1], HirStmt::If { .. }));
    }
}
