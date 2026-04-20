use crate::tai_ast::{TaiFunctionDecl, TaiProgram, TaiVarDecl};
use crate::tai_exec::{parse_native_tai_exec, TaiExecExpr, TaiExecStmt};
use crate::types::TaiType;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirProgram {
    pub entry_label: String,
    pub functions: Vec<HirFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirFunction {
    pub name: String,
    pub return_type: TaiType,
    pub params: Vec<HirBinding>,
    pub locals: Vec<HirBinding>,
    pub body: Vec<HirStmt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirBinding {
    pub name: String,
    pub ty: TaiType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirStmt {
    Let {
        name: String,
        ty: TaiType,
        value: Option<HirExpr>,
    },
    Assign {
        name: String,
        value: HirExpr,
    },
    Print(HirExpr),
    Return(Option<HirExpr>),
    Break,
    Continue,
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
    Expr(HirExpr),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HirExpr {
    pub ty: TaiType,
    pub kind: HirExprKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HirExprKind {
    Identifier(String),
    Number(i64),
    String(String),
    Bool(bool),
    Null,
    Call {
        callee: String,
        arguments: Vec<HirExpr>,
    },
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionSignature {
    name: String,
    return_type: TaiType,
    params: Vec<HirBinding>,
}

pub fn lower_tai_to_hir(program: &TaiProgram) -> Result<HirProgram, String> {
    let signatures = collect_signatures(program)?;
    let entry_label = select_entry_label(&signatures)?;
    let signature_map = signatures
        .iter()
        .map(|item| (item.name.clone(), item.clone()))
        .collect::<BTreeMap<_, _>>();

    let mut functions = Vec::new();
    for module in &program.modules {
        for function in &module.functions {
            functions.push(lower_function(function, &signature_map)?);
        }
    }

    Ok(HirProgram {
        entry_label,
        functions,
    })
}

fn collect_signatures(program: &TaiProgram) -> Result<Vec<FunctionSignature>, String> {
    let mut names = BTreeMap::new();
    let mut signatures = Vec::new();
    for module in &program.modules {
        for function in &module.functions {
            if names.insert(function.name.clone(), ()).is_some() {
                return Err(format!("重复声明的子程序 '{}'", function.name));
            }
            let params = function
                .param_decls
                .iter()
                .map(|decl| binding_from_decl("参数", decl, true))
                .collect::<Result<Vec<_>, _>>()?;
            signatures.push(FunctionSignature {
                name: function.name.clone(),
                return_type: TaiType::parse_optional(function.return_type.as_deref())?,
                params,
            });
        }
    }
    Ok(signatures)
}

fn select_entry_label(signatures: &[FunctionSignature]) -> Result<String, String> {
    for signature in signatures {
        if matches!(signature.name.trim(), "主程序" | "主函数" | "main" | "Main") {
            return Ok(signature.name.clone());
        }
    }
    signatures
        .first()
        .map(|item| item.name.clone())
        .ok_or_else(|| "当前 .tai 程序没有可编译的 .子程序".to_string())
}

fn lower_function(
    function: &TaiFunctionDecl,
    signatures: &BTreeMap<String, FunctionSignature>,
) -> Result<HirFunction, String> {
    let signature = signatures
        .get(&function.name)
        .cloned()
        .ok_or_else(|| format!("未找到子程序 '{}' 的签名", function.name))?;
    let mut context = HirContext::for_function(function, signatures, signature)?;
    let body = match &function.implementation {
        Some(implementation) => {
            let stmts = parse_native_tai_exec(implementation)
                .map_err(|err| format!("原生 .tai 执行语法解析失败：{}", err.message))?;
            context.lower_stmts(&stmts)?
        }
        None => Vec::new(),
    };

    Ok(HirFunction {
        name: function.name.clone(),
        return_type: context.return_type.clone(),
        params: context.params.clone(),
        locals: context.locals.clone(),
        body,
    })
}

fn binding_from_decl(kind: &str, decl: &TaiVarDecl, require_type: bool) -> Result<HirBinding, String> {
    let ty = TaiType::from_var_decl(decl)?;
    let ty = match (require_type, ty) {
        (_, Some(value)) => value,
        (true, None) => {
            return Err(format!("{} '{}' 缺少类型声明", kind, decl.name));
        }
        (false, None) => TaiType::Integer,
    };
    Ok(HirBinding {
        name: decl.name.clone(),
        ty,
    })
}

struct HirContext<'a> {
    return_type: TaiType,
    params: Vec<HirBinding>,
    locals: Vec<HirBinding>,
    bindings: BTreeMap<String, TaiType>,
    signatures: &'a BTreeMap<String, FunctionSignature>,
}

impl<'a> HirContext<'a> {
    fn for_function(
        function: &TaiFunctionDecl,
        signatures: &'a BTreeMap<String, FunctionSignature>,
        signature: FunctionSignature,
    ) -> Result<Self, String> {
        let locals = function
            .locals
            .iter()
            .map(|decl| binding_from_decl("局部变量", decl, false))
            .collect::<Result<Vec<_>, _>>()?;

        let mut bindings = BTreeMap::new();
        for binding in &signature.params {
            bindings.insert(binding.name.clone(), binding.ty.clone());
        }
        for binding in &locals {
            if bindings.contains_key(&binding.name) {
                return Err(format!("重复声明的局部标识符 '{}'", binding.name));
            }
            bindings.insert(binding.name.clone(), binding.ty.clone());
        }

        Ok(Self {
            return_type: signature.return_type,
            params: signature.params,
            locals,
            bindings,
            signatures,
        })
    }

    fn lower_stmts(&mut self, stmts: &[TaiExecStmt]) -> Result<Vec<HirStmt>, String> {
        stmts.iter().map(|stmt| self.lower_stmt(stmt)).collect()
    }

    fn lower_stmt(&mut self, stmt: &TaiExecStmt) -> Result<HirStmt, String> {
        match stmt {
            TaiExecStmt::Let { name, value } => {
                let ty = self
                    .bindings
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| TaiType::Integer);
                let lowered = value.as_ref().map(|expr| self.lower_expr(expr)).transpose()?;
                if let Some(expr) = &lowered {
                    self.ensure_assignable(&ty, &expr.ty, &format!("变量 '{}'", name))?;
                }
                self.bindings.entry(name.clone()).or_insert_with(|| ty.clone());
                if !self.locals.iter().any(|item| item.name == *name)
                    && !self.params.iter().any(|item| item.name == *name)
                {
                    self.locals.push(HirBinding {
                        name: name.clone(),
                        ty: ty.clone(),
                    });
                }
                Ok(HirStmt::Let {
                    name: name.clone(),
                    ty,
                    value: lowered,
                })
            }
            TaiExecStmt::Print(expr) => Ok(HirStmt::Print(self.lower_expr(expr)?)),
            TaiExecStmt::Return(expr) => {
                let lowered = expr.as_ref().map(|item| self.lower_expr(item)).transpose()?;
                match (&self.return_type, &lowered) {
                    (TaiType::Void, None) => {}
                    (TaiType::Void, Some(_)) => {
                        return Err("当前子程序声明为空返回，但实际返回了值".to_string());
                    }
                    (expected, Some(value)) => {
                        self.ensure_assignable(expected, &value.ty, "返回值")?;
                    }
                    (_, None) => {
                        return Err(format!("当前子程序要求返回 {}", self.return_type));
                    }
                }
                Ok(HirStmt::Return(lowered))
            }
            TaiExecStmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.lower_expr(condition)?;
                self.ensure_boolean(&condition.ty, "如果条件")?;
                Ok(HirStmt::If {
                    condition,
                    then_branch: self.lower_stmts(then_branch)?,
                    else_branch: else_branch
                        .as_ref()
                        .map(|items| self.lower_stmts(items))
                        .transpose()?
                        .unwrap_or_default(),
                })
            }
            TaiExecStmt::While { condition, body } => {
                let condition = self.lower_expr(condition)?;
                self.ensure_boolean(&condition.ty, "循环条件")?;
                Ok(HirStmt::While {
                    condition,
                    body: self.lower_stmts(body)?,
                })
            }
            TaiExecStmt::Match {
                subject,
                branches,
                default_branch,
            } => {
                let subject = self.lower_expr(subject)?;
                let subject_ty = subject.ty.clone();
                let branches = branches
                    .iter()
                    .map(|(expr, body)| {
                        let case_expr = self.lower_expr(expr)?;
                        self.ensure_assignable(&subject_ty, &case_expr.ty, "判断分支值")?;
                        Ok((case_expr, self.lower_stmts(body)?))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let default_branch = default_branch
                    .as_ref()
                    .map(|items| self.lower_stmts(items))
                    .transpose()?
                    .unwrap_or_default();
                Ok(HirStmt::Match {
                    subject,
                    branches,
                    default_branch,
                })
            }
            TaiExecStmt::Expr(TaiExecExpr::Assign { target, value }) => match target.as_ref() {
                TaiExecExpr::Identifier(name) => {
                    let slot_ty = self
                        .bindings
                        .get(name)
                        .cloned()
                        .ok_or_else(|| format!("变量 '{}' 尚未声明类型", name))?;
                    let value = self.lower_expr(value)?;
                    self.ensure_assignable(&slot_ty, &value.ty, &format!("变量 '{}'", name))?;
                    Ok(HirStmt::Assign {
                        name: name.clone(),
                        value,
                    })
                }
                _ => Err("当前 HIR 只支持标识符赋值".to_string()),
            },
            TaiExecStmt::Break => Ok(HirStmt::Break),
            TaiExecStmt::Continue => Ok(HirStmt::Continue),
            TaiExecStmt::Expr(expr) => Ok(HirStmt::Expr(self.lower_expr(expr)?)),
        }
    }

    fn lower_expr(&mut self, expr: &TaiExecExpr) -> Result<HirExpr, String> {
        match expr {
            TaiExecExpr::Identifier(name) => {
                let ty = self
                    .bindings
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("标识符 '{}' 尚未声明类型", name))?;
                Ok(HirExpr {
                    ty,
                    kind: HirExprKind::Identifier(name.clone()),
                })
            }
            TaiExecExpr::Number(value) => value
                .parse::<i64>()
                .map(|parsed| HirExpr {
                    ty: TaiType::Integer,
                    kind: HirExprKind::Number(parsed),
                })
                .map_err(|_| format!("无法编译数字字面量 '{}'", value)),
            TaiExecExpr::String(value) => Ok(HirExpr {
                ty: TaiType::Text,
                kind: HirExprKind::String(value.clone()),
            }),
            TaiExecExpr::Bool(value) => Ok(HirExpr {
                ty: TaiType::Boolean,
                kind: HirExprKind::Bool(*value),
            }),
            TaiExecExpr::Null => Ok(HirExpr {
                ty: TaiType::Void,
                kind: HirExprKind::Null,
            }),
            TaiExecExpr::Grouping(inner) => self.lower_expr(inner),
            TaiExecExpr::Call { callee, arguments } => {
                let TaiExecExpr::Identifier(name) = callee.as_ref() else {
                    return Err("当前 HIR 只支持直接调用已命名子程序".to_string());
                };
                let signature = self
                    .signatures
                    .get(name)
                    .cloned()
                    .ok_or_else(|| format!("未找到子程序 '{}'", name))?;
                if signature.params.len() != arguments.len() {
                    return Err(format!(
                        "调用 '{}' 的参数数量不匹配：期望 {}，实际 {}",
                        name,
                        signature.params.len(),
                        arguments.len()
                    ));
                }
                let mut lowered_args = Vec::with_capacity(arguments.len());
                for (index, argument) in arguments.iter().enumerate() {
                    let lowered = self.lower_expr(argument)?;
                    self.ensure_assignable(
                        &signature.params[index].ty,
                        &lowered.ty,
                        &format!("调用 '{}' 的第 {} 个参数", name, index + 1),
                    )?;
                    lowered_args.push(lowered);
                }
                Ok(HirExpr {
                    ty: signature.return_type,
                    kind: HirExprKind::Call {
                        callee: name.clone(),
                        arguments: lowered_args,
                    },
                })
            }
            TaiExecExpr::Unary { op, right } => {
                let right = self.lower_expr(right)?;
                let ty = match op {
                    crate::tai_exec::TaiExecUnaryOp::Not => {
                        self.ensure_boolean(&right.ty, "非运算")?;
                        TaiType::Boolean
                    }
                    crate::tai_exec::TaiExecUnaryOp::Negate
                    | crate::tai_exec::TaiExecUnaryOp::Positive => {
                        self.ensure_integer(&right.ty, "数值一元运算")?;
                        TaiType::Integer
                    }
                };
                Ok(HirExpr {
                    ty,
                    kind: HirExprKind::Unary {
                        op: match op {
                            crate::tai_exec::TaiExecUnaryOp::Not => HirUnaryOp::Not,
                            crate::tai_exec::TaiExecUnaryOp::Negate => HirUnaryOp::Negate,
                            crate::tai_exec::TaiExecUnaryOp::Positive => HirUnaryOp::Positive,
                        },
                        right: Box::new(right),
                    },
                })
            }
            TaiExecExpr::Binary { left, op, right } => {
                let left = self.lower_expr(left)?;
                let right = self.lower_expr(right)?;
                let ty = match op {
                    crate::tai_exec::TaiExecBinaryOp::Or | crate::tai_exec::TaiExecBinaryOp::And => {
                        self.ensure_boolean(&left.ty, "逻辑运算左值")?;
                        self.ensure_boolean(&right.ty, "逻辑运算右值")?;
                        TaiType::Boolean
                    }
                    crate::tai_exec::TaiExecBinaryOp::Equal
                    | crate::tai_exec::TaiExecBinaryOp::NotEqual => {
                        self.ensure_assignable(&left.ty, &right.ty, "比较表达式")?;
                        TaiType::Boolean
                    }
                    crate::tai_exec::TaiExecBinaryOp::Greater
                    | crate::tai_exec::TaiExecBinaryOp::GreaterEqual
                    | crate::tai_exec::TaiExecBinaryOp::Less
                    | crate::tai_exec::TaiExecBinaryOp::LessEqual => {
                        self.ensure_integer(&left.ty, "数值比较左值")?;
                        self.ensure_integer(&right.ty, "数值比较右值")?;
                        TaiType::Boolean
                    }
                    crate::tai_exec::TaiExecBinaryOp::Add => match (&left.ty, &right.ty) {
                        (TaiType::Integer, TaiType::Integer) => TaiType::Integer,
                        (TaiType::Text, TaiType::Text) => TaiType::Text,
                        _ => {
                            return Err(format!(
                                "加法两侧类型不兼容：{} 与 {}",
                                left.ty, right.ty
                            ));
                        }
                    },
                    crate::tai_exec::TaiExecBinaryOp::Subtract
                    | crate::tai_exec::TaiExecBinaryOp::Multiply
                    | crate::tai_exec::TaiExecBinaryOp::Divide
                    | crate::tai_exec::TaiExecBinaryOp::Modulo => {
                        self.ensure_integer(&left.ty, "数值运算左值")?;
                        self.ensure_integer(&right.ty, "数值运算右值")?;
                        TaiType::Integer
                    }
                };
                Ok(HirExpr {
                    ty,
                    kind: HirExprKind::Binary {
                        left: Box::new(left),
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
                        right: Box::new(right),
                    },
                })
            }
            TaiExecExpr::Assign { .. } => Err("HIR 表达式中不支持嵌套赋值".to_string()),
            TaiExecExpr::Array(_) => Err("当前 HIR 暂不支持数组".to_string()),
            TaiExecExpr::Object(_) => Err("当前 HIR 暂不支持对象".to_string()),
            TaiExecExpr::Member { .. } => Err("当前 HIR 暂不支持成员访问".to_string()),
            TaiExecExpr::Index { .. } => Err("当前 HIR 暂不支持下标访问".to_string()),
        }
    }

    fn ensure_assignable(&self, expected: &TaiType, actual: &TaiType, context: &str) -> Result<(), String> {
        if expected == actual {
            Ok(())
        } else {
            Err(format!(
                "{} 类型不匹配：期望 {}，实际 {}",
                context, expected, actual
            ))
        }
    }

    fn ensure_boolean(&self, actual: &TaiType, context: &str) -> Result<(), String> {
        self.ensure_assignable(&TaiType::Boolean, actual, context)
    }

    fn ensure_integer(&self, actual: &TaiType, context: &str) -> Result<(), String> {
        self.ensure_assignable(&TaiType::Integer, actual, context)
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
.子程序 加一, 整数型
.参数 输入, 整数型
.返回 输入 + 1

.子程序 主程序, 整数型
.局部变量 计数, 整数型
计数 = 加一(1)
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
        assert_eq!(hir.functions.len(), 2);
        assert_eq!(hir.functions[0].return_type, TaiType::Integer);
        assert!(matches!(hir.functions[1].body[0], HirStmt::Assign { .. }));
        assert!(matches!(hir.functions[1].body[1], HirStmt::If { .. }));
    }

    #[test]
    fn lowers_loop_control_to_hir() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序, 整数型
.参数 计数, 整数型
.循环判断首 计数 小于 10
    .到循环尾
    .跳出循环
.循环判断尾
.返回 0
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should lower");
        let HirStmt::While { body, .. } = &hir.functions[0].body[0] else {
            panic!("expected while statement");
        };
        assert!(matches!(body[0], HirStmt::Continue));
        assert!(matches!(body[1], HirStmt::Break));
    }

    #[test]
    fn rejects_type_mismatch_returns() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序, 整数型
.返回 "错误"
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let err = lower_tai_to_hir(&program).expect_err("hir should reject mismatch");
        assert!(err.contains("返回值 类型不匹配"));
    }

    #[test]
    fn rejects_non_boolean_condition() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序, 整数型
.如果 1
    .返回 1
.如果结束
.返回 0
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let err = lower_tai_to_hir(&program).expect_err("hir should reject mismatch");
        assert!(err.contains("如果条件 类型不匹配"));
    }

    #[test]
    fn defaults_untyped_local_let_to_integer() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序, 整数型
.令 计数 = 1
.返回 计数 + 1
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should lower");
        let HirStmt::Let { ty, .. } = &hir.functions[0].body[0] else {
            panic!("expected let statement");
        };
        assert_eq!(*ty, TaiType::Integer);
    }

    #[test]
    fn lowers_function_call_expr() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 加一, 整数型
.参数 输入, 整数型
.返回 输入 + 1

.子程序 主程序, 整数型
.返回 加一(2)
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should lower");
        let HirStmt::Return(Some(expr)) = &hir.functions[1].body[0] else {
            panic!("expected return");
        };
        match &expr.kind {
            HirExprKind::Call { callee, arguments } => {
                assert_eq!(callee, "加一");
                assert_eq!(arguments.len(), 1);
                assert_eq!(expr.ty, TaiType::Integer);
            }
            _ => panic!("expected call expression"),
        }
    }

    #[test]
    fn lowers_text_equality_condition() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序, 整数型
.如果 "甲" 等于 "甲"
    .返回 1
.否则
    .返回 0
.如果结束
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should lower");
        let HirStmt::If { condition, .. } = &hir.functions[0].body[0] else {
            panic!("expected if statement");
        };
        assert_eq!(condition.ty, TaiType::Boolean);
        match &condition.kind {
            HirExprKind::Binary { op, .. } => assert_eq!(*op, HirBinaryOp::Equal),
            _ => panic!("expected binary condition"),
        }
    }
}
