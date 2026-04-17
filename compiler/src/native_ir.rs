use crate::hir::{HirBinaryOp, HirExpr, HirProgram, HirStmt, HirUnaryOp};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirProgram {
    pub entry_label: String,
    pub locals: Vec<MirLocal>,
    pub blocks: Vec<MirBlock>,
    pub strings: Vec<MirString>,
    pub exit_code: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MirLocal {
    pub name: String,
    pub slot: usize,
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
    let mut builder = MirBuilder::new(program.entry_label.clone());
    builder.lower_stmts(&program.body)?;
    if !builder
        .current_block
        .instructions
        .iter()
        .any(|inst| matches!(inst, MirInstruction::Return { .. }))
    {
        let zero = builder.allocate_temp();
        builder.emit(MirInstruction::ConstInt { target: zero, value: 0 });
        builder.emit(MirInstruction::Return { value: zero });
    }
    Ok(builder.finish())
}

struct MirBuilder {
    entry_label: String,
    current_block: MirBlock,
    blocks: Vec<MirBlock>,
    locals: Vec<MirLocal>,
    local_map: BTreeMap<String, usize>,
    strings: Vec<MirString>,
    next_slot: usize,
    next_label_id: usize,
}

impl MirBuilder {
    fn new(entry_label: String) -> Self {
        Self {
            current_block: MirBlock {
                label: entry_label.clone(),
                instructions: Vec::new(),
            },
            entry_label,
            blocks: Vec::new(),
            locals: Vec::new(),
            local_map: BTreeMap::new(),
            strings: Vec::new(),
            next_slot: 0,
            next_label_id: 0,
        }
    }

    fn finish(mut self) -> MirProgram {
        self.flush_current_block();
        let exit_code = self
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
            .find_map(|inst| match inst {
                MirInstruction::ConstInt { value, .. } => Some(*value),
                _ => None,
            });

        MirProgram {
            entry_label: self.entry_label,
            locals: self.locals,
            blocks: self.blocks,
            strings: self.strings,
            exit_code,
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

    fn allocate_named(&mut self, name: &str) -> usize {
        if let Some(slot) = self.local_map.get(name) {
            return *slot;
        }
        let slot = self.next_slot;
        self.next_slot += 1;
        self.local_map.insert(name.to_string(), slot);
        self.locals.push(MirLocal {
            name: name.to_string(),
            slot,
        });
        slot
    }

    fn allocate_temp(&mut self) -> usize {
        let slot = self.next_slot;
        self.next_slot += 1;
        self.locals.push(MirLocal {
            name: format!("__temp{}", slot),
            slot,
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
        let label = format!("{}_{}", prefix, self.next_label_id);
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
            HirStmt::Let { name, value } => {
                let slot = self.allocate_named(name);
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
                let slot = self.allocate_named(name);
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
                    let zero = self.allocate_temp();
                    self.emit(MirInstruction::ConstInt { target: zero, value: 0 });
                    zero
                };
                self.emit(MirInstruction::Return { value: slot });
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
                self.lower_stmts(body)?;
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
                    let cond = self.allocate_temp();
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
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> Result<usize, String> {
        match expr {
            HirExpr::Identifier(name) => Ok(self.allocate_named(name)),
            HirExpr::Number(value) => {
                let slot = self.allocate_temp();
                self.emit(MirInstruction::ConstInt {
                    target: slot,
                    value: *value,
                });
                Ok(slot)
            }
            HirExpr::String(value) => {
                let slot = self.allocate_temp();
                let string_id = self.intern_string(value);
                self.emit(MirInstruction::ConstString {
                    target: slot,
                    string_id,
                });
                Ok(slot)
            }
            HirExpr::Bool(value) => {
                let slot = self.allocate_temp();
                self.emit(MirInstruction::ConstBool {
                    target: slot,
                    value: *value,
                });
                Ok(slot)
            }
            HirExpr::Null => {
                let slot = self.allocate_temp();
                self.emit(MirInstruction::ConstNull { target: slot });
                Ok(slot)
            }
            HirExpr::Unary { op, right } => {
                let operand = self.lower_expr(right)?;
                let target = self.allocate_temp();
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
            HirExpr::Binary { left, op, right } => {
                let left_slot = self.lower_expr(left)?;
                let right_slot = self.lower_expr(right)?;
                let target = self.allocate_temp();
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
    use crate::hir::lower_tai_to_hir;
    use crate::tai_parser::TaiParser;

    #[test]
    fn lowers_hir_to_mir() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序
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
        assert!(!mir.blocks.is_empty());
        assert!(mir.locals.iter().any(|local| local.name == "总和"));
        assert!(mir.blocks.iter().flat_map(|b| b.instructions.iter()).any(|inst| matches!(inst, MirInstruction::JumpIfFalse { .. })));
    }
}
