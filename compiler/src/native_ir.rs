use crate::compile_config::{CompileOptions, OptimizationLevel};
use crate::hir::{HirBinaryOp, HirExpr, HirExprKind, HirProgram, HirStmt, HirUnaryOp};
use crate::types::TaiType;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirProgram {
    pub entry_label: String,
    pub functions: Vec<MirFunction>,
    pub strings: Vec<MirString>,
    pub exit_code: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirFunction {
    pub label: String,
    pub return_type: TaiType,
    pub locals: Vec<MirLocal>,
    pub params: Vec<MirParam>,
    pub blocks: Vec<MirBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirLocal {
    pub name: String,
    pub slot: usize,
    pub ty: TaiType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirParam {
    pub name: String,
    pub slot: usize,
    pub ty: TaiType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirBlock {
    pub label: String,
    pub instructions: Vec<MirInstruction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirString {
    pub id: usize,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MirInstruction {
    ConstInt {
        target: usize,
        value: i64,
    },
    ConstBool {
        target: usize,
        value: bool,
    },
    ConstNull {
        target: usize,
    },
    ConstString {
        target: usize,
        string_id: usize,
    },
    Copy {
        target: usize,
        source: usize,
    },
    Unary {
        target: usize,
        op: MirUnaryOp,
        operand: usize,
    },
    Binary {
        target: usize,
        left: usize,
        op: MirBinaryOp,
        right: usize,
    },
    Call {
        target: usize,
        callee: String,
        arguments: Vec<usize>,
    },
    Print {
        value: usize,
    },
    Jump {
        target: String,
    },
    JumpIfFalse {
        condition: usize,
        target: String,
    },
    Return {
        value: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirUnaryOp {
    Not,
    Negate,
    Positive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MirBinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    And,
    Or,
}

pub fn lower_hir_to_mir(program: &HirProgram) -> Result<MirProgram, String> {
    lower_hir_to_mir_with_options(program, CompileOptions::default())
}

pub fn lower_hir_to_mir_with_options(
    program: &HirProgram,
    options: CompileOptions,
) -> Result<MirProgram, String> {
    let mut strings = Vec::new();
    let mut functions = Vec::new();
    let mut exit_code = None;

    for function in &program.functions {
        let mut builder = MirBuilder::new(
            function.name.clone(),
            function.return_type.clone(),
            &mut strings,
        );
        builder.seed_params(&function.params);
        builder.seed_locals(&function.locals);
        builder.lower_stmts(&function.body)?;
        if !builder
            .current_block
            .instructions
            .iter()
            .any(|inst| matches!(inst, MirInstruction::Return { .. }))
        {
            let zero = builder.allocate_temp(TaiType::Integer);
            builder.emit(MirInstruction::ConstInt { target: zero, value: 0 });
            builder.emit(MirInstruction::Return { value: zero });
        }
        let mut mir_function = builder.finish();
        if options.opt_level.enables_mir_optimizations() {
            mir_function = optimize_function(mir_function, options.opt_level);
        }
        if function.name == program.entry_label {
            exit_code = mir_function
                .blocks
                .iter()
                .flat_map(|block| block.instructions.iter())
                .find_map(|inst| match inst {
                    MirInstruction::ConstInt { value, .. } => Some(*value),
                    _ => None,
                });
        }
        functions.push(mir_function);
    }

    Ok(MirProgram {
        entry_label: program.entry_label.clone(),
        functions,
        strings,
        exit_code,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum KnownValue {
    Integer(i64),
    Boolean(bool),
    Null,
}

fn optimize_function(mut function: MirFunction, opt_level: OptimizationLevel) -> MirFunction {
    for pass in optimization_passes(opt_level) {
        function = apply_optimization_pass(function, *pass);
    }
    function
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OptimizationPass {
    ConstantFoldAndBranchSimplify,
}

fn optimization_passes(opt_level: OptimizationLevel) -> &'static [OptimizationPass] {
    match opt_level {
        OptimizationLevel::O0 => &[],
        OptimizationLevel::O1 | OptimizationLevel::O2 => {
            &[OptimizationPass::ConstantFoldAndBranchSimplify]
        }
    }
}

fn apply_optimization_pass(
    mut function: MirFunction,
    pass: OptimizationPass,
) -> MirFunction {
    match pass {
        OptimizationPass::ConstantFoldAndBranchSimplify => {
            function = constant_fold_and_branch_simplify(function);
        }
    }
    function
}

fn constant_fold_and_branch_simplify(mut function: MirFunction) -> MirFunction {
    for block in &mut function.blocks {
        let mut known = BTreeMap::<usize, KnownValue>::new();
        let mut optimized = Vec::with_capacity(block.instructions.len());
        for inst in &block.instructions {
            match inst {
                MirInstruction::ConstInt { target, value } => {
                    known.insert(*target, KnownValue::Integer(*value));
                    optimized.push(inst.clone());
                }
                MirInstruction::ConstBool { target, value } => {
                    known.insert(*target, KnownValue::Boolean(*value));
                    optimized.push(inst.clone());
                }
                MirInstruction::ConstNull { target } => {
                    known.insert(*target, KnownValue::Null);
                    optimized.push(inst.clone());
                }
                MirInstruction::ConstString { target, .. } => {
                    known.remove(target);
                    optimized.push(inst.clone());
                }
                MirInstruction::Copy { target, source } => {
                    if let Some(value) = known.get(source).cloned() {
                        match value {
                            KnownValue::Integer(value) => {
                                known.insert(*target, KnownValue::Integer(value));
                                optimized.push(MirInstruction::ConstInt {
                                    target: *target,
                                    value,
                                });
                            }
                            KnownValue::Boolean(value) => {
                                known.insert(*target, KnownValue::Boolean(value));
                                optimized.push(MirInstruction::ConstBool {
                                    target: *target,
                                    value,
                                });
                            }
                            KnownValue::Null => {
                                known.insert(*target, KnownValue::Null);
                                optimized.push(MirInstruction::ConstNull { target: *target });
                            }
                        }
                    } else {
                        known.remove(target);
                        optimized.push(inst.clone());
                    }
                }
                MirInstruction::Unary {
                    target,
                    op,
                    operand,
                } => {
                    if let Some(value) = fold_unary(*op, known.get(operand)) {
                        match value {
                            KnownValue::Integer(value) => {
                                known.insert(*target, KnownValue::Integer(value));
                                optimized.push(MirInstruction::ConstInt {
                                    target: *target,
                                    value,
                                });
                            }
                            KnownValue::Boolean(value) => {
                                known.insert(*target, KnownValue::Boolean(value));
                                optimized.push(MirInstruction::ConstBool {
                                    target: *target,
                                    value,
                                });
                            }
                            KnownValue::Null => {
                                known.insert(*target, KnownValue::Null);
                                optimized.push(MirInstruction::ConstNull { target: *target });
                            }
                        }
                    } else {
                        known.remove(target);
                        optimized.push(inst.clone());
                    }
                }
                MirInstruction::Binary {
                    target,
                    left,
                    op,
                    right,
                } => {
                    if let Some(value) = fold_binary(*op, known.get(left), known.get(right)) {
                        match value {
                            KnownValue::Integer(value) => {
                                known.insert(*target, KnownValue::Integer(value));
                                optimized.push(MirInstruction::ConstInt {
                                    target: *target,
                                    value,
                                });
                            }
                            KnownValue::Boolean(value) => {
                                known.insert(*target, KnownValue::Boolean(value));
                                optimized.push(MirInstruction::ConstBool {
                                    target: *target,
                                    value,
                                });
                            }
                            KnownValue::Null => {
                                known.insert(*target, KnownValue::Null);
                                optimized.push(MirInstruction::ConstNull { target: *target });
                            }
                        }
                    } else {
                        known.remove(target);
                        optimized.push(inst.clone());
                    }
                }
                MirInstruction::JumpIfFalse { condition, target } => {
                    match known.get(condition) {
                        Some(KnownValue::Boolean(false)) => {
                            optimized.push(MirInstruction::Jump {
                                target: target.clone(),
                            });
                            break;
                        }
                        Some(KnownValue::Boolean(true)) => {}
                        _ => optimized.push(inst.clone()),
                    }
                }
                MirInstruction::Jump { .. } | MirInstruction::Return { .. } => {
                    optimized.push(inst.clone());
                    break;
                }
                MirInstruction::Call { target, .. } => {
                    known.remove(target);
                    optimized.push(inst.clone());
                }
                MirInstruction::Print { .. } => optimized.push(inst.clone()),
            }
        }
        block.instructions = optimized;
    }
    function
}

fn fold_unary(op: MirUnaryOp, operand: Option<&KnownValue>) -> Option<KnownValue> {
    match (op, operand?) {
        (MirUnaryOp::Not, KnownValue::Boolean(value)) => Some(KnownValue::Boolean(!value)),
        (MirUnaryOp::Negate, KnownValue::Integer(value)) => Some(KnownValue::Integer(-value)),
        (MirUnaryOp::Positive, KnownValue::Integer(value)) => Some(KnownValue::Integer(*value)),
        _ => None,
    }
}

fn fold_binary(
    op: MirBinaryOp,
    left: Option<&KnownValue>,
    right: Option<&KnownValue>,
) -> Option<KnownValue> {
    match (op, left?, right?) {
        (MirBinaryOp::Add, KnownValue::Integer(left), KnownValue::Integer(right)) => {
            Some(KnownValue::Integer(left + right))
        }
        (MirBinaryOp::Subtract, KnownValue::Integer(left), KnownValue::Integer(right)) => {
            Some(KnownValue::Integer(left - right))
        }
        (MirBinaryOp::Multiply, KnownValue::Integer(left), KnownValue::Integer(right)) => {
            Some(KnownValue::Integer(left * right))
        }
        (MirBinaryOp::Divide, KnownValue::Integer(left), KnownValue::Integer(right)) => {
            if *right == 0 {
                None
            } else {
                Some(KnownValue::Integer(left / right))
            }
        }
        (MirBinaryOp::Modulo, KnownValue::Integer(left), KnownValue::Integer(right)) => {
            if *right == 0 {
                None
            } else {
                Some(KnownValue::Integer(left % right))
            }
        }
        (MirBinaryOp::Equal, KnownValue::Integer(left), KnownValue::Integer(right)) => {
            Some(KnownValue::Boolean(left == right))
        }
        (MirBinaryOp::NotEqual, KnownValue::Integer(left), KnownValue::Integer(right)) => {
            Some(KnownValue::Boolean(left != right))
        }
        (MirBinaryOp::Greater, KnownValue::Integer(left), KnownValue::Integer(right)) => {
            Some(KnownValue::Boolean(left > right))
        }
        (MirBinaryOp::GreaterEqual, KnownValue::Integer(left), KnownValue::Integer(right)) => {
            Some(KnownValue::Boolean(left >= right))
        }
        (MirBinaryOp::Less, KnownValue::Integer(left), KnownValue::Integer(right)) => {
            Some(KnownValue::Boolean(left < right))
        }
        (MirBinaryOp::LessEqual, KnownValue::Integer(left), KnownValue::Integer(right)) => {
            Some(KnownValue::Boolean(left <= right))
        }
        (MirBinaryOp::Equal, KnownValue::Boolean(left), KnownValue::Boolean(right)) => {
            Some(KnownValue::Boolean(left == right))
        }
        (MirBinaryOp::NotEqual, KnownValue::Boolean(left), KnownValue::Boolean(right)) => {
            Some(KnownValue::Boolean(left != right))
        }
        (MirBinaryOp::And, KnownValue::Boolean(left), KnownValue::Boolean(right)) => {
            Some(KnownValue::Boolean(*left && *right))
        }
        (MirBinaryOp::Or, KnownValue::Boolean(left), KnownValue::Boolean(right)) => {
            Some(KnownValue::Boolean(*left || *right))
        }
        _ => None,
    }
}

struct MirBuilder<'a> {
    function_label: String,
    return_type: TaiType,
    current_block: MirBlock,
    blocks: Vec<MirBlock>,
    locals: Vec<MirLocal>,
    params: Vec<MirParam>,
    local_map: BTreeMap<String, usize>,
    strings: &'a mut Vec<MirString>,
    slot_types: BTreeMap<usize, TaiType>,
    next_slot: usize,
    next_label_id: usize,
    loop_stack: Vec<LoopLabels>,
}

struct LoopLabels {
    continue_label: String,
    break_label: String,
}

impl<'a> MirBuilder<'a> {
    fn new(function_label: String, return_type: TaiType, strings: &'a mut Vec<MirString>) -> Self {
        Self {
            current_block: MirBlock {
                label: function_label.clone(),
                instructions: Vec::new(),
            },
            function_label,
            return_type,
            blocks: Vec::new(),
            locals: Vec::new(),
            params: Vec::new(),
            local_map: BTreeMap::new(),
            strings,
            slot_types: BTreeMap::new(),
            next_slot: 0,
            next_label_id: 0,
            loop_stack: Vec::new(),
        }
    }

    fn finish(mut self) -> MirFunction {
        self.flush_current_block();
        MirFunction {
            label: self.function_label,
            return_type: self.return_type,
            locals: self.locals,
            params: self.params,
            blocks: self.blocks,
        }
    }

    fn seed_params(&mut self, params: &[crate::hir::HirBinding]) {
        for param in params {
            let slot = self.next_slot;
            self.next_slot += 1;
            self.local_map.insert(param.name.clone(), slot);
            self.slot_types.insert(slot, param.ty.clone());
            self.params.push(MirParam {
                name: param.name.clone(),
                slot,
                ty: param.ty.clone(),
            });
        }
    }

    fn seed_locals(&mut self, locals: &[crate::hir::HirBinding]) {
        for local in locals {
            if self.local_map.contains_key(&local.name) {
                continue;
            }
            let slot = self.next_slot;
            self.next_slot += 1;
            self.local_map.insert(local.name.clone(), slot);
            self.slot_types.insert(slot, local.ty.clone());
            self.locals.push(MirLocal {
                name: local.name.clone(),
                slot,
                ty: local.ty.clone(),
            });
        }
    }

    fn flush_current_block(&mut self) {
        if !self.current_block.instructions.is_empty() || self.blocks.is_empty() {
            self.blocks.push(MirBlock {
                label: self.current_block.label.clone(),
                instructions: std::mem::take(&mut self.current_block.instructions),
            });
        }
    }

    fn start_block(&mut self, label: String) {
        self.flush_current_block();
        self.current_block = MirBlock {
            label,
            instructions: Vec::new(),
        };
    }

    fn emit(&mut self, inst: MirInstruction) {
        self.current_block.instructions.push(inst);
    }

    fn allocate_named(&self, name: &str) -> Result<usize, String> {
        self.local_map
            .get(name)
            .copied()
            .ok_or_else(|| format!("MIR lowering failed: unknown local '{name}'"))
    }

    fn allocate_temp(&mut self, ty: TaiType) -> usize {
        let slot = self.next_slot;
        self.next_slot += 1;
        self.slot_types.insert(slot, ty.clone());
        self.locals.push(MirLocal {
            name: format!("__temp{}", slot),
            slot,
            ty,
        });
        slot
    }

    fn intern_string(&mut self, value: &str) -> usize {
        if let Some(existing) = self.strings.iter().find(|item| item.value == value) {
            return existing.id;
        }
        let id = self.strings.len();
        self.strings.push(MirString {
            id,
            value: value.to_string(),
        });
        id
    }

    fn new_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}_{}", self.function_label, prefix, self.next_label_id);
        self.next_label_id += 1;
        label
    }

    fn lower_stmts(&mut self, stmts: &[HirStmt]) -> Result<(), String> {
        for stmt in stmts {
            self.lower_stmt(stmt)?;
        }
        Ok(())
    }

    fn lower_stmt(&mut self, stmt: &HirStmt) -> Result<(), String> {
        match stmt {
            HirStmt::Let { name, value, .. } => {
                let slot = self.allocate_named(name)?;
                if let Some(value) = value {
                    let value_slot = self.lower_expr(value)?;
                    self.emit(MirInstruction::Copy {
                        target: slot,
                        source: value_slot,
                    });
                } else {
                    self.emit(MirInstruction::ConstNull { target: slot });
                }
                Ok(())
            }
            HirStmt::Assign { name, value } => {
                let slot = self.allocate_named(name)?;
                let value_slot = self.lower_expr(value)?;
                self.emit(MirInstruction::Copy {
                    target: slot,
                    source: value_slot,
                });
                Ok(())
            }
            HirStmt::Print(expr) => {
                let value = self.lower_expr(expr)?;
                self.emit(MirInstruction::Print { value });
                Ok(())
            }
            HirStmt::Return(expr) => {
                let slot = if let Some(expr) = expr {
                    self.lower_expr(expr)?
                } else {
                    match self.return_type {
                        TaiType::Void => {
                            let unit = self.allocate_temp(TaiType::Void);
                            self.emit(MirInstruction::ConstNull { target: unit });
                            unit
                        }
                        _ => {
                            let zero = self.allocate_temp(TaiType::Integer);
                            self.emit(MirInstruction::ConstInt { target: zero, value: 0 });
                            zero
                        }
                    }
                };
                self.emit(MirInstruction::Return { value: slot });
                Ok(())
            }
            HirStmt::Break => {
                let labels = self
                    .loop_stack
                    .last()
                    .ok_or_else(|| ".跳出循环 只能出现在循环内部".to_string())?;
                self.emit(MirInstruction::Jump {
                    target: labels.break_label.clone(),
                });
                Ok(())
            }
            HirStmt::Continue => {
                let labels = self
                    .loop_stack
                    .last()
                    .ok_or_else(|| ".到循环尾 只能出现在循环内部".to_string())?;
                self.emit(MirInstruction::Jump {
                    target: labels.continue_label.clone(),
                });
                Ok(())
            }
            HirStmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let else_label = self.new_label("if_else");
                let end_label = self.new_label("if_end");
                let cond_slot = self.lower_expr(condition)?;
                self.emit(MirInstruction::JumpIfFalse {
                    condition: cond_slot,
                    target: else_label.clone(),
                });
                self.lower_stmts(then_branch)?;
                self.emit(MirInstruction::Jump {
                    target: end_label.clone(),
                });
                self.start_block(else_label);
                self.lower_stmts(else_branch)?;
                self.emit(MirInstruction::Jump {
                    target: end_label.clone(),
                });
                self.start_block(end_label);
                Ok(())
            }
            HirStmt::While { condition, body } => {
                let head = self.new_label("while_head");
                let body_label = self.new_label("while_body");
                let end = self.new_label("while_end");

                self.emit(MirInstruction::Jump { target: head.clone() });
                self.start_block(head.clone());
                let cond_slot = self.lower_expr(condition)?;
                self.emit(MirInstruction::JumpIfFalse {
                    condition: cond_slot,
                    target: end.clone(),
                });
                self.emit(MirInstruction::Jump {
                    target: body_label.clone(),
                });
                self.start_block(body_label);
                self.loop_stack.push(LoopLabels {
                    continue_label: head.clone(),
                    break_label: end.clone(),
                });
                self.lower_stmts(body)?;
                self.loop_stack.pop();
                self.emit(MirInstruction::Jump { target: head });
                self.start_block(end);
                Ok(())
            }
            HirStmt::Match {
                subject,
                branches,
                default_branch,
            } => {
                let end = self.new_label("match_end");
                let default_label = self.new_label("match_default");
                let subject_slot = self.lower_expr(subject)?;

                for (index, (expr, body)) in branches.iter().enumerate() {
                    let branch_label = self.new_label("match_case");
                    let next_label = if index + 1 < branches.len() {
                        self.new_label("match_next")
                    } else {
                        default_label.clone()
                    };
                    let case_slot = self.lower_expr(expr)?;
                    let cond = self.allocate_temp(TaiType::Boolean);
                    self.emit(MirInstruction::Binary {
                        target: cond,
                        left: subject_slot,
                        op: MirBinaryOp::Equal,
                        right: case_slot,
                    });
                    self.emit(MirInstruction::JumpIfFalse {
                        condition: cond,
                        target: next_label.clone(),
                    });
                    self.emit(MirInstruction::Jump {
                        target: branch_label.clone(),
                    });
                    self.start_block(branch_label);
                    self.lower_stmts(body)?;
                    self.emit(MirInstruction::Jump {
                        target: end.clone(),
                    });
                    if index + 1 < branches.len() {
                        self.start_block(next_label);
                    }
                }

                self.emit(MirInstruction::Jump {
                    target: default_label.clone(),
                });
                self.start_block(default_label);
                self.lower_stmts(default_branch)?;
                self.emit(MirInstruction::Jump { target: end.clone() });
                self.start_block(end);
                Ok(())
            }
            HirStmt::Expr(expr) => {
                let _ = self.lower_expr(expr)?;
                Ok(())
            }
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> Result<usize, String> {
        match &expr.kind {
            HirExprKind::Identifier(name) => self.allocate_named(name),
            HirExprKind::Number(value) => {
                let slot = self.allocate_temp(TaiType::Integer);
                self.emit(MirInstruction::ConstInt {
                    target: slot,
                    value: *value,
                });
                Ok(slot)
            }
            HirExprKind::String(value) => {
                let slot = self.allocate_temp(TaiType::Text);
                let string_id = self.intern_string(value);
                self.emit(MirInstruction::ConstString {
                    target: slot,
                    string_id,
                });
                Ok(slot)
            }
            HirExprKind::Bool(value) => {
                let slot = self.allocate_temp(TaiType::Boolean);
                self.emit(MirInstruction::ConstBool {
                    target: slot,
                    value: *value,
                });
                Ok(slot)
            }
            HirExprKind::Null => {
                let slot = self.allocate_temp(TaiType::Void);
                self.emit(MirInstruction::ConstNull { target: slot });
                Ok(slot)
            }
            HirExprKind::Call { callee, arguments } => {
                let mut lowered_args = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    lowered_args.push(self.lower_expr(argument)?);
                }
                let target = self.allocate_temp(expr.ty.clone());
                self.emit(MirInstruction::Call {
                    target,
                    callee: callee.clone(),
                    arguments: lowered_args,
                });
                Ok(target)
            }
            HirExprKind::Unary { op, right } => {
                let operand = self.lower_expr(right)?;
                let target = self.allocate_temp(expr.ty.clone());
                self.emit(MirInstruction::Unary {
                    target,
                    op: match op {
                        HirUnaryOp::Not => MirUnaryOp::Not,
                        HirUnaryOp::Negate => MirUnaryOp::Negate,
                        HirUnaryOp::Positive => MirUnaryOp::Positive,
                    },
                    operand,
                });
                Ok(target)
            }
            HirExprKind::Binary { left, op, right } => {
                let left_slot = self.lower_expr(left)?;
                let right_slot = self.lower_expr(right)?;
                let target = self.allocate_temp(expr.ty.clone());
                self.emit(MirInstruction::Binary {
                    target,
                    left: left_slot,
                    op: match op {
                        HirBinaryOp::Or => MirBinaryOp::Or,
                        HirBinaryOp::And => MirBinaryOp::And,
                        HirBinaryOp::Equal => MirBinaryOp::Equal,
                        HirBinaryOp::NotEqual => MirBinaryOp::NotEqual,
                        HirBinaryOp::Greater => MirBinaryOp::Greater,
                        HirBinaryOp::GreaterEqual => MirBinaryOp::GreaterEqual,
                        HirBinaryOp::Less => MirBinaryOp::Less,
                        HirBinaryOp::LessEqual => MirBinaryOp::LessEqual,
                        HirBinaryOp::Add => MirBinaryOp::Add,
                        HirBinaryOp::Subtract => MirBinaryOp::Subtract,
                        HirBinaryOp::Multiply => MirBinaryOp::Multiply,
                        HirBinaryOp::Divide => MirBinaryOp::Divide,
                        HirBinaryOp::Modulo => MirBinaryOp::Modulo,
                    },
                    right: right_slot,
                });
                Ok(target)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_config::{CompileOptions, CompilerBackend, OptimizationLevel};
    use crate::hir::lower_tai_to_hir;
    use crate::tai_parser::TaiParser;

    #[test]
    fn lowers_hir_to_mir() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序, 整数型
.令 总和 = 0
.循环判断首 总和 小于 3
    总和 = 总和 + 1
.循环判断尾
.显示 总和
.返回 0
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should succeed");
        let mir = lower_hir_to_mir(&hir).expect("mir should succeed");
        assert_eq!(mir.functions.len(), 1);
        assert!(!mir.functions[0].blocks.is_empty());
        assert!(mir.functions[0]
            .locals
            .iter()
            .any(|local| local.name == "总和"));
        assert!(mir.functions[0]
            .blocks
            .iter()
            .flat_map(|b| b.instructions.iter())
            .any(|inst| matches!(inst, MirInstruction::Jump { .. } | MirInstruction::JumpIfFalse { .. })));
    }

    #[test]
    fn lowers_break_and_continue_to_jumps() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序, 整数型
.令 计数 = 0
.循环判断首 计数 小于 3
    计数 = 计数 + 1
    .到循环尾
    .跳出循环
.循环判断尾
.返回 0
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should succeed");
        let mir = lower_hir_to_mir(&hir).expect("mir should succeed");
        let jump_targets = mir.functions[0]
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .filter_map(|inst| match inst {
                MirInstruction::Jump { target } => Some(target.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(jump_targets.iter().any(|target| target.contains("while_head_")));
        assert!(jump_targets
            .iter()
            .any(|target| target.contains("while_head_") || target.contains("while_end_")));
    }

    #[test]
    fn lowers_function_call_to_mir() {
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
        let hir = lower_tai_to_hir(&program).expect("hir should succeed");
        let mir = lower_hir_to_mir(&hir).expect("mir should succeed");
        assert_eq!(mir.functions.len(), 2);
        assert!(mir.functions[1]
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .any(|inst| matches!(inst, MirInstruction::Call { callee, arguments, .. } if callee == "加一" && arguments.len() == 1)));
        assert_eq!(mir.functions[0].return_type, TaiType::Integer);
        assert_eq!(mir.functions[1].return_type, TaiType::Integer);
    }

    #[test]
    fn preserves_boolean_function_return_type_in_mir() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 是否三, 逻辑型
.参数 输入, 整数型
.返回 输入 等于 3

.子程序 主程序, 整数型
.如果 是否三(3)
    .返回 1
.否则
    .返回 0
.如果结束
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should succeed");
        let mir = lower_hir_to_mir(&hir).expect("mir should succeed");
        assert_eq!(mir.functions[0].return_type, TaiType::Boolean);
        assert_eq!(mir.functions[1].return_type, TaiType::Integer);
    }

    #[test]
    fn preserves_void_function_return_type_in_mir() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 打招呼, 空
.显示 "hi"
.返回

.子程序 主程序, 整数型
打招呼()
.返回 0
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should succeed");
        let mir = lower_hir_to_mir(&hir).expect("mir should succeed");
        assert_eq!(mir.functions[0].return_type, TaiType::Void);
        assert_eq!(mir.functions[1].return_type, TaiType::Integer);
    }

    #[test]
    fn folds_constant_arithmetic_in_mir() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序, 整数型
.返回 1 + 2
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should succeed");
        let mir = lower_hir_to_mir(&hir).expect("mir should succeed");
        let instructions = mir.functions[0]
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .collect::<Vec<_>>();
        assert!(instructions.iter().any(|inst| {
            matches!(inst, MirInstruction::ConstInt { value, .. } if *value == 3)
        }));
        assert!(!instructions.iter().any(|inst| matches!(inst, MirInstruction::Binary { .. })));
    }

    #[test]
    fn removes_jump_if_false_when_condition_is_constant_true() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序, 整数型
.如果 1 等于 1
    .返回 1
.否则
    .返回 2
.如果结束
.返回 0
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should succeed");
        let mir = lower_hir_to_mir(&hir).expect("mir should succeed");
        assert!(!mir.functions[0]
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .any(|inst| matches!(inst, MirInstruction::JumpIfFalse { .. })));
    }

    #[test]
    fn keeps_binary_when_opt_level_is_o0() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序, 整数型
.返回 1 + 2
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let hir = lower_tai_to_hir(&program).expect("hir should succeed");
        let mir = lower_hir_to_mir_with_options(
            &hir,
            CompileOptions {
                backend: CompilerBackend::SelfNative,
                opt_level: OptimizationLevel::O0,
            },
        )
        .expect("mir should succeed");
        assert!(mir.functions[0]
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .any(|inst| matches!(inst, MirInstruction::Binary { .. })));
    }
}
