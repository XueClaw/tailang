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
    Noop,
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
    ArrayLiteral {
        elements: Vec<HirExpr>,
    },
    ArrayIndex {
        array: Box<HirExpr>,
        index: Box<HirExpr>,
    },
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
    constant_bindings: BTreeMap<String, ConstValue>,
    signatures: &'a BTreeMap<String, FunctionSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConstValue {
    Integer(i64),
    Boolean(bool),
    Text(String),
    Null,
    Array(Vec<ConstValue>),
    Object(BTreeMap<String, ConstValue>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ConstPathSegment {
    Member(String),
    Index(ConstValue),
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
            constant_bindings: BTreeMap::new(),
            signatures,
        })
    }

    fn lower_stmts(&mut self, stmts: &[TaiExecStmt]) -> Result<Vec<HirStmt>, String> {
        stmts.iter().map(|stmt| self.lower_stmt(stmt)).collect()
    }

    fn lower_stmt(&mut self, stmt: &TaiExecStmt) -> Result<HirStmt, String> {
        match stmt {
            TaiExecStmt::Let { name, ty, value } => {
                let const_value = value
                    .as_ref()
                    .map(|expr| self.eval_const(expr))
                    .transpose()?
                    .flatten();
                let lowered = value.as_ref().map(|expr| self.lower_expr(expr)).transpose()?;
                let ty = self
                    .bindings
                    .get(name)
                    .cloned()
                    .or_else(|| {
                        ty.as_deref()
                            .map(TaiType::from_decl_name)
                            .transpose()
                            .ok()
                            .flatten()
                    })
                    .or_else(|| lowered.as_ref().map(|expr| expr.ty.clone()))
                    .or_else(|| const_value.as_ref().and_then(ConstValue::scalar_type))
                    .unwrap_or_else(|| TaiType::Integer);
                if let Some(expr) = &lowered {
                    self.ensure_assignable(&ty, &expr.ty, &format!("变量 '{}'", name))?;
                }
                if let Some(value) = &const_value {
                    self.constant_bindings.insert(name.clone(), value.clone());
                    if matches!(value, ConstValue::Object(_)) {
                        return Ok(HirStmt::Noop);
                    }
                } else {
                    self.constant_bindings.remove(name);
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
                    let const_value = self.eval_const(value)?;
                    if let Some(value) = &const_value {
                        self.constant_bindings.insert(name.clone(), value.clone());
                        if value.is_collection() {
                            return Ok(HirStmt::Noop);
                        }
                    } else {
                        self.constant_bindings.remove(name);
                    }
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
                TaiExecExpr::Member { .. } | TaiExecExpr::Index { .. } => {
                    let const_value = self
                        .eval_const(value)?
                        .ok_or_else(|| "当前集合赋值需要可静态求值的右值".to_string())?;
                    if self.apply_const_target_assignment(target.as_ref(), const_value)? {
                        Ok(HirStmt::Noop)
                    } else {
                        Err("当前集合赋值需要可静态求值的集合目标".to_string())
                    }
                }
                _ => Err("当前 HIR 只支持标识符、成员或下标赋值".to_string()),
            },
            TaiExecStmt::Break => Ok(HirStmt::Break),
            TaiExecStmt::Continue => Ok(HirStmt::Continue),
            TaiExecStmt::Expr(expr) => Ok(HirStmt::Expr(self.lower_expr(expr)?)),
        }
    }

    fn lower_expr(&mut self, expr: &TaiExecExpr) -> Result<HirExpr, String> {
        if let Some(value) = self.eval_const(expr)? {
            if let Some(expr) = Self::const_to_scalar_hir(&value) {
                return Ok(expr);
            }
        }
        match expr {
            TaiExecExpr::Identifier(name) => {
                if let Some(value) = self.constant_bindings.get(name) {
                    let binding_ty = self.bindings.get(name);
                    let allow_runtime_array = matches!(binding_ty, Some(TaiType::Array(_)));
                    if value.is_collection() && !allow_runtime_array {
                        return Err(format!(
                            "常量集合 '{}' 目前只能通过成员访问或下标访问读取",
                            name
                        ));
                    }
                }
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
            TaiExecExpr::Array(items) => {
                if items.is_empty() {
                    return Err("当前运行时数组字面量暂不支持空数组".to_string());
                }
                let mut lowered = Vec::with_capacity(items.len());
                for item in items {
                    let value = self.lower_expr(item)?;
                    match value.ty {
                        TaiType::Integer | TaiType::Boolean | TaiType::Text => {}
                        _ => {
                            return Err(format!("当前运行时数组暂不支持元素类型 {}", value.ty));
                        }
                    }
                    lowered.push(value);
                }
                let element_ty = lowered[0].ty.clone();
                for item in lowered.iter().skip(1) {
                    self.ensure_assignable(&element_ty, &item.ty, "数组字面量元素")?;
                }
                Ok(HirExpr {
                    ty: TaiType::Array(Box::new(element_ty)),
                    kind: HirExprKind::ArrayLiteral { elements: lowered },
                })
            }
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
            TaiExecExpr::Object(_) => Err("当前对象值只能在可静态求值的上下文中使用".to_string()),
            TaiExecExpr::Member { .. } => Err("当前成员访问需要可静态求值的对象".to_string()),
            TaiExecExpr::Index { object, index } => {
                let array = self.lower_expr(object)?;
                let element_ty = match &array.ty {
                    TaiType::Array(inner) => inner.as_ref().clone(),
                    _ => return Err("当前运行时下标访问仅支持数组值".to_string()),
                };
                let index = self.lower_expr(index)?;
                self.ensure_integer(&index.ty, "数组下标")?;
                Ok(HirExpr {
                    ty: element_ty,
                    kind: HirExprKind::ArrayIndex {
                        array: Box::new(array),
                        index: Box::new(index),
                    },
                })
            }
        }
    }

    fn eval_const(&self, expr: &TaiExecExpr) -> Result<Option<ConstValue>, String> {
        match expr {
            TaiExecExpr::Identifier(name) => Ok(self.constant_bindings.get(name).cloned()),
            TaiExecExpr::Number(value) => value
                .parse::<i64>()
                .map(ConstValue::Integer)
                .map(Some)
                .map_err(|_| format!("无法编译数字字面量 '{}'", value)),
            TaiExecExpr::String(value) => Ok(Some(ConstValue::Text(value.clone()))),
            TaiExecExpr::Bool(value) => Ok(Some(ConstValue::Boolean(*value))),
            TaiExecExpr::Null => Ok(Some(ConstValue::Null)),
            TaiExecExpr::Array(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    let Some(value) = self.eval_const(item)? else {
                        return Ok(None);
                    };
                    values.push(value);
                }
                Ok(Some(ConstValue::Array(values)))
            }
            TaiExecExpr::Object(entries) => {
                let mut values = BTreeMap::new();
                for (key, value_expr) in entries {
                    let Some(value) = self.eval_const(value_expr)? else {
                        return Ok(None);
                    };
                    values.insert(key.clone(), value);
                }
                Ok(Some(ConstValue::Object(values)))
            }
            TaiExecExpr::Unary { op, right } => {
                let Some(value) = self.eval_const(right)? else {
                    return Ok(None);
                };
                match (op, value) {
                    (crate::tai_exec::TaiExecUnaryOp::Not, ConstValue::Boolean(value)) => {
                        Ok(Some(ConstValue::Boolean(!value)))
                    }
                    (crate::tai_exec::TaiExecUnaryOp::Negate, ConstValue::Integer(value)) => {
                        Ok(Some(ConstValue::Integer(-value)))
                    }
                    (crate::tai_exec::TaiExecUnaryOp::Positive, ConstValue::Integer(value)) => {
                        Ok(Some(ConstValue::Integer(value)))
                    }
                    _ => Ok(None),
                }
            }
            TaiExecExpr::Binary { left, op, right } => {
                let Some(left) = self.eval_const(left)? else {
                    return Ok(None);
                };
                let Some(right) = self.eval_const(right)? else {
                    return Ok(None);
                };
                Self::eval_const_binary(*op, left, right)
            }
            TaiExecExpr::Assign { .. } => Ok(None),
            TaiExecExpr::Call { .. } => Ok(None),
            TaiExecExpr::Member { object, property } => {
                let Some(object) = self.eval_const(object)? else {
                    return Ok(None);
                };
                match object {
                    ConstValue::Object(entries) => Ok(entries.get(property).cloned()),
                    _ => Ok(None),
                }
            }
            TaiExecExpr::Index { object, index } => {
                let Some(object) = self.eval_const(object)? else {
                    return Ok(None);
                };
                let Some(index) = self.eval_const(index)? else {
                    return Ok(None);
                };
                match (object, index) {
                    (ConstValue::Array(items), ConstValue::Integer(index)) => {
                        if index < 0 {
                            return Ok(None);
                        }
                        Ok(items.get(index as usize).cloned())
                    }
                    (ConstValue::Object(entries), ConstValue::Text(key)) => {
                        Ok(entries.get(&key).cloned())
                    }
                    _ => Ok(None),
                }
            }
            TaiExecExpr::Grouping(inner) => self.eval_const(inner),
        }
    }

    fn eval_const_binary(
        op: crate::tai_exec::TaiExecBinaryOp,
        left: ConstValue,
        right: ConstValue,
    ) -> Result<Option<ConstValue>, String> {
        use crate::tai_exec::TaiExecBinaryOp as Op;
        let result = match op {
            Op::Or => match (left, right) {
                (ConstValue::Boolean(left), ConstValue::Boolean(right)) => {
                    Some(ConstValue::Boolean(left || right))
                }
                _ => None,
            },
            Op::And => match (left, right) {
                (ConstValue::Boolean(left), ConstValue::Boolean(right)) => {
                    Some(ConstValue::Boolean(left && right))
                }
                _ => None,
            },
            Op::Equal => Some(ConstValue::Boolean(left == right)),
            Op::NotEqual => Some(ConstValue::Boolean(left != right)),
            Op::Greater => match (left, right) {
                (ConstValue::Integer(left), ConstValue::Integer(right)) => {
                    Some(ConstValue::Boolean(left > right))
                }
                _ => None,
            },
            Op::GreaterEqual => match (left, right) {
                (ConstValue::Integer(left), ConstValue::Integer(right)) => {
                    Some(ConstValue::Boolean(left >= right))
                }
                _ => None,
            },
            Op::Less => match (left, right) {
                (ConstValue::Integer(left), ConstValue::Integer(right)) => {
                    Some(ConstValue::Boolean(left < right))
                }
                _ => None,
            },
            Op::LessEqual => match (left, right) {
                (ConstValue::Integer(left), ConstValue::Integer(right)) => {
                    Some(ConstValue::Boolean(left <= right))
                }
                _ => None,
            },
            Op::Add => match (left, right) {
                (ConstValue::Integer(left), ConstValue::Integer(right)) => {
                    Some(ConstValue::Integer(left + right))
                }
                (ConstValue::Text(left), ConstValue::Text(right)) => {
                    Some(ConstValue::Text(format!("{}{}", left, right)))
                }
                _ => None,
            },
            Op::Subtract => match (left, right) {
                (ConstValue::Integer(left), ConstValue::Integer(right)) => {
                    Some(ConstValue::Integer(left - right))
                }
                _ => None,
            },
            Op::Multiply => match (left, right) {
                (ConstValue::Integer(left), ConstValue::Integer(right)) => {
                    Some(ConstValue::Integer(left * right))
                }
                _ => None,
            },
            Op::Divide => match (left, right) {
                (ConstValue::Integer(_), ConstValue::Integer(0)) => {
                    return Err("常量表达式中出现除以零".to_string())
                }
                (ConstValue::Integer(left), ConstValue::Integer(right)) => {
                    Some(ConstValue::Integer(left / right))
                }
                _ => None,
            },
            Op::Modulo => match (left, right) {
                (ConstValue::Integer(_), ConstValue::Integer(0)) => {
                    return Err("常量表达式中出现模零".to_string())
                }
                (ConstValue::Integer(left), ConstValue::Integer(right)) => {
                    Some(ConstValue::Integer(left % right))
                }
                _ => None,
            },
        };
        Ok(result)
    }

    fn const_to_scalar_hir(value: &ConstValue) -> Option<HirExpr> {
        match value {
            ConstValue::Integer(value) => Some(HirExpr {
                ty: TaiType::Integer,
                kind: HirExprKind::Number(*value),
            }),
            ConstValue::Boolean(value) => Some(HirExpr {
                ty: TaiType::Boolean,
                kind: HirExprKind::Bool(*value),
            }),
            ConstValue::Text(value) => Some(HirExpr {
                ty: TaiType::Text,
                kind: HirExprKind::String(value.clone()),
            }),
            ConstValue::Null => Some(HirExpr {
                ty: TaiType::Void,
                kind: HirExprKind::Null,
            }),
            ConstValue::Array(_) => None,
            ConstValue::Object(_) => None,
        }
    }

    fn apply_const_target_assignment(
        &mut self,
        target: &TaiExecExpr,
        value: ConstValue,
    ) -> Result<bool, String> {
        let (root_name, path) = self
            .decompose_const_target(target)?
            .ok_or_else(|| "当前集合赋值需要可静态求值的成员或下标路径".to_string())?;
        let Some(root_value) = self.constant_bindings.get(&root_name).cloned() else {
            return Ok(false);
        };
        let Some(updated_root) = Self::assign_const_path(&root_value, &path, value)? else {
            return Ok(false);
        };
        self.constant_bindings.insert(root_name, updated_root);
        Ok(true)
    }

    fn decompose_const_target(
        &self,
        target: &TaiExecExpr,
    ) -> Result<Option<(String, Vec<ConstPathSegment>)>, String> {
        match target {
            TaiExecExpr::Identifier(name) => Ok(Some((name.clone(), Vec::new()))),
            TaiExecExpr::Member { object, property } => {
                let Some((root, mut path)) = self.decompose_const_target(object)? else {
                    return Ok(None);
                };
                path.push(ConstPathSegment::Member(property.clone()));
                Ok(Some((root, path)))
            }
            TaiExecExpr::Index { object, index } => {
                let Some((root, mut path)) = self.decompose_const_target(object)? else {
                    return Ok(None);
                };
                let Some(index_value) = self.eval_const(index)? else {
                    return Ok(None);
                };
                path.push(ConstPathSegment::Index(index_value));
                Ok(Some((root, path)))
            }
            _ => Ok(None),
        }
    }

    fn assign_const_path(
        current: &ConstValue,
        path: &[ConstPathSegment],
        incoming: ConstValue,
    ) -> Result<Option<ConstValue>, String> {
        if path.is_empty() {
            return Ok(Some(incoming));
        }

        match (&path[0], current) {
            (ConstPathSegment::Member(property), ConstValue::Object(entries)) => {
                let mut updated = entries.clone();
                if path.len() == 1 {
                    updated.insert(property.clone(), incoming);
                    return Ok(Some(ConstValue::Object(updated)));
                }
                let Some(child) = entries.get(property) else {
                    return Ok(None);
                };
                let Some(next_child) = Self::assign_const_path(child, &path[1..], incoming)? else {
                    return Ok(None);
                };
                updated.insert(property.clone(), next_child);
                Ok(Some(ConstValue::Object(updated)))
            }
            (ConstPathSegment::Index(ConstValue::Integer(index)), ConstValue::Array(items)) => {
                if *index < 0 {
                    return Ok(None);
                }
                let index = *index as usize;
                if index >= items.len() {
                    return Ok(None);
                }
                let mut updated = items.clone();
                if path.len() == 1 {
                    updated[index] = incoming;
                    return Ok(Some(ConstValue::Array(updated)));
                }
                let Some(next_child) = Self::assign_const_path(&items[index], &path[1..], incoming)? else {
                    return Ok(None);
                };
                updated[index] = next_child;
                Ok(Some(ConstValue::Array(updated)))
            }
            (ConstPathSegment::Index(ConstValue::Text(key)), ConstValue::Object(entries)) => {
                let mut updated = entries.clone();
                if path.len() == 1 {
                    updated.insert(key.clone(), incoming);
                    return Ok(Some(ConstValue::Object(updated)));
                }
                let Some(child) = entries.get(key) else {
                    return Ok(None);
                };
                let Some(next_child) = Self::assign_const_path(child, &path[1..], incoming)? else {
                    return Ok(None);
                };
                updated.insert(key.clone(), next_child);
                Ok(Some(ConstValue::Object(updated)))
            }
            (ConstPathSegment::Index(_), _) | (ConstPathSegment::Member(_), _) => Ok(None),
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

impl ConstValue {
    fn scalar_type(&self) -> Option<TaiType> {
        match self {
            ConstValue::Integer(_) => Some(TaiType::Integer),
            ConstValue::Boolean(_) => Some(TaiType::Boolean),
            ConstValue::Text(_) => Some(TaiType::Text),
            ConstValue::Null => Some(TaiType::Void),
            ConstValue::Array(_) | ConstValue::Object(_) => None,
        }
    }

    fn is_collection(&self) -> bool {
        matches!(self, ConstValue::Array(_) | ConstValue::Object(_))
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
.子程序 加一(输入: 整数型) -> 整数型, , ,
.返回 输入 + 1

.子程序 主程序() -> 整数型, , ,
计数: 整数型 = 0
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
        assert!(matches!(hir.functions[1].body[0], HirStmt::Let { .. }));
        assert!(matches!(hir.functions[1].body[1], HirStmt::Assign { .. }));
        assert!(matches!(hir.functions[1].body[2], HirStmt::If { .. }));
    }

    #[test]
    fn lowers_loop_control_to_hir() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序(计数: 整数型) -> 整数型, , ,
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
.子程序 主程序() -> 整数型, , ,
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
.子程序 主程序() -> 整数型, , ,
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
.子程序 主程序() -> 整数型, , ,
计数: 整数型 = 1
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
    fn infers_text_type_for_untyped_local_let() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序() -> 整数型, , ,
名称: 文本型 = "结衣"
.如果 名称 等于 "结衣"
    .返回 1
.否则
    .返回 0
.如果结束
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should lower");
        let HirStmt::Let { ty, .. } = &hir.functions[0].body[0] else {
            panic!("expected let statement");
        };
        assert_eq!(*ty, TaiType::Text);
    }

    #[test]
    fn infers_boolean_type_for_untyped_local_let() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序() -> 整数型, , ,
已通过: 逻辑型 = 真
.如果 已通过
    .返回 1
.否则
    .返回 0
.如果结束
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should lower");
        let HirStmt::Let { ty, .. } = &hir.functions[0].body[0] else {
            panic!("expected let statement");
        };
        assert_eq!(*ty, TaiType::Boolean);
    }

    #[test]
    fn lowers_function_call_expr() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 加一(输入: 整数型) -> 整数型, , ,
.返回 输入 + 1

.子程序 主程序() -> 整数型, , ,
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
.子程序 主程序() -> 整数型, , ,
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
        assert!(matches!(condition.kind, HirExprKind::Bool(true)));
    }

    #[test]
    fn folds_constant_object_member_and_array_index_access() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序() -> 整数型, , ,
数据 = {"名称": "结衣", "分数": [7, 9, 11]}
.如果 数据.名称 等于 "结衣"
    .返回 数据.分数[1]
.否则
    .返回 0
.如果结束
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should lower");
        assert!(matches!(hir.functions[0].body[0], HirStmt::Noop));
        let HirStmt::If { condition, then_branch, .. } = &hir.functions[0].body[1] else {
            panic!("expected if statement");
        };
        assert!(matches!(condition.kind, HirExprKind::Bool(true)));
        let HirStmt::Return(Some(expr)) = &then_branch[0] else {
            panic!("expected folded return");
        };
        assert!(matches!(expr.kind, HirExprKind::Number(9)));
    }

    #[test]
    fn folds_constant_collection_assignment_and_followup_access() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序() -> 整数型, , ,
数据 = {"名称": "结衣", "分数": [3, 5, 8]}
数据.名称 = "真结衣"
数据["分数"][1] = 13
.如果 数据.名称 等于 "真结衣"
    .返回 数据.分数[1]
.否则
    .返回 0
.如果结束
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should lower");
        assert!(matches!(hir.functions[0].body[0], HirStmt::Noop));
        assert!(matches!(hir.functions[0].body[1], HirStmt::Noop));
        assert!(matches!(hir.functions[0].body[2], HirStmt::Noop));
        let HirStmt::If { condition, then_branch, .. } = &hir.functions[0].body[3] else {
            panic!("expected if statement");
        };
        assert!(matches!(condition.kind, HirExprKind::Bool(true)));
        let HirStmt::Return(Some(expr)) = &then_branch[0] else {
            panic!("expected folded return");
        };
        assert!(matches!(expr.kind, HirExprKind::Number(13)));
    }

    #[test]
    fn lowers_runtime_array_index_access() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序(索引: 整数型) -> 整数型, , ,
数据: 整数型[] = [3, 5, 8]
.返回 数据[索引]
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should lower");
        let HirStmt::Let { ty, value: Some(value), .. } = &hir.functions[0].body[0] else {
            panic!("expected runtime array let");
        };
        assert_eq!(ty, &TaiType::Array(Box::new(TaiType::Integer)));
        assert!(matches!(value.kind, HirExprKind::ArrayLiteral { .. }));
        let HirStmt::Return(Some(expr)) = &hir.functions[0].body[1] else {
            panic!("expected runtime array return");
        };
        assert!(matches!(expr.kind, HirExprKind::ArrayIndex { .. }));
    }
}
