use crate::compile_config::{CompileOptions, OptimizationLevel};
use crate::hir::lower_tai_to_hir;
use crate::native_ir::{
    lower_hir_to_mir_with_options, MirBinaryOp, MirFunction, MirInstruction, MirProgram,
    MirUnaryOp,
};
use crate::tai_ast::TaiProgram;
use crate::types::TaiType;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlvmEnvironment {
    pub root: PathBuf,
    pub bin_dir: PathBuf,
    pub lib_dir: PathBuf,
    pub include_dir: PathBuf,
    pub clang_path: PathBuf,
    pub llvm_c_dll_path: PathBuf,
    pub llvm_c_lib_path: PathBuf,
}

impl LlvmEnvironment {
    pub fn detect() -> Result<Self, String> {
        let candidates = llvm_root_candidates();
        for root in candidates {
            if let Some(environment) = probe_llvm_root(&root) {
                return Ok(environment);
            }
        }

        Err("未检测到可用 LLVM 环境。请安装 LLVM，或将 LLVM 安装到 C:\\Program Files\\LLVM".to_string())
    }
}

fn llvm_root_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for key in ["TAILANG_LLVM_ROOT", "LLVM_SYS_180_PREFIX", "LLVM_HOME"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                candidates.push(PathBuf::from(trimmed));
            }
        }
    }
    candidates.push(PathBuf::from(r"C:\Program Files\LLVM"));
    candidates
}

fn probe_llvm_root(root: &Path) -> Option<LlvmEnvironment> {
    let bin_dir = root.join("bin");
    let lib_dir = root.join("lib");
    let include_dir = root.join("include");
    let clang_path = bin_dir.join("clang.exe");
    let llvm_c_dll_path = bin_dir.join("LLVM-C.dll");
    let llvm_c_lib_path = lib_dir.join("LLVM-C.lib");

    if clang_path.is_file() && llvm_c_dll_path.is_file() && llvm_c_lib_path.is_file() {
        return Some(LlvmEnvironment {
            root: root.to_path_buf(),
            bin_dir,
            lib_dir,
            include_dir,
            clang_path,
            llvm_c_dll_path,
            llvm_c_lib_path,
        });
    }

    None
}

pub fn compile_program_with_llvm(
    program: &TaiProgram,
    options: CompileOptions,
    output: &str,
) -> Result<(), String> {
    if output.trim().is_empty() {
        return Err("LLVM 后端需要显式输出路径".to_string());
    }

    let hir = lower_tai_to_hir(program)?;
    let mir = lower_hir_to_mir_with_options(&hir, options)?;
    let environment = LlvmEnvironment::detect()?;
    let ir = render_llvm_module(&mir)?;

    let output_path = PathBuf::from(output);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败：{}", e))?;
    }

    let temp_dir = make_temp_dir()?;
    let ir_path = temp_dir.join("program.ll");
    std::fs::write(&ir_path, ir).map_err(|e| format!("写入 LLVM IR 失败：{}", e))?;
    let compile_result = invoke_clang(&environment, &ir_path, &output_path, options.opt_level);
    let _ = std::fs::remove_dir_all(&temp_dir);
    compile_result
}

fn make_temp_dir() -> Result<PathBuf, String> {
    let mut dir = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("获取系统时间失败：{}", e))?
        .as_nanos();
    dir.push(format!("tailang-llvm-{}", stamp));
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建临时目录失败：{}", e))?;
    Ok(dir)
}

fn invoke_clang(
    environment: &LlvmEnvironment,
    ir_path: &Path,
    output_path: &Path,
    opt_level: OptimizationLevel,
) -> Result<(), String> {
    let opt_flag = match opt_level {
        OptimizationLevel::O0 => "-O0",
        OptimizationLevel::O1 => "-O1",
        OptimizationLevel::O2 => "-O2",
    };

    let result = Command::new(&environment.clang_path)
        .arg(opt_flag)
        .arg("-x")
        .arg("ir")
        .arg(ir_path)
        .arg("-luser32")
        .arg("-o")
        .arg(output_path)
        .output()
        .map_err(|e| format!("调用 clang 失败：{}", e))?;

    if result.status.success() {
        Ok(())
    } else {
        let stdout = String::from_utf8_lossy(&result.stdout);
        let stderr = String::from_utf8_lossy(&result.stderr);
        Err(format!(
            "LLVM 后端生成可执行文件失败。\nstdout:\n{}\nstderr:\n{}",
            stdout.trim(),
            stderr.trim()
        ))
    }
}

fn render_llvm_module(program: &MirProgram) -> Result<String, String> {
    let string_lengths = program
        .strings
        .iter()
        .map(|item| (item.id, item.value.as_bytes().len() + 3))
        .collect::<BTreeMap<_, _>>();

    let mut out = String::new();
    out.push_str("target triple = \"x86_64-pc-windows-msvc\"\n");
    out.push_str("@__tailang_crlf = private unnamed_addr constant [3 x i8] c\"\\0D\\0A\\00\"\n");
    out.push_str("@.fmt.int = private unnamed_addr constant [6 x i8] c\"%I64d\\00\"\n");
    for string in &program.strings {
        out.push_str(&format!(
            "@.str.{} = private unnamed_addr constant [{} x i8] c\"{}\"\n",
            string.id,
            string_lengths[&string.id],
            escape_llvm_bytes(&(string.value.clone() + "\r\n"))
        ));
    }
    out.push('\n');
    out.push_str("declare ptr @GetStdHandle(i32)\n");
    out.push_str("declare i32 @WriteFile(ptr, ptr, i32, ptr, ptr)\n");
    out.push_str("declare i32 @wsprintfA(ptr, ptr, ...)\n\n");

    for function in &program.functions {
        let mut renderer = FunctionRenderer::new(function, &string_lengths);
        out.push_str(&renderer.render()?);
        out.push('\n');
    }

    out.push_str("define i32 @main() {\n");
    out.push_str(&format!(
        "entry:\n  %entry_result = call {} @{}()\n",
        llvm_type(&entry_return_type(program)?),
        llvm_fn_name(&program.entry_label)
    ));
    match entry_return_type(program)? {
        TaiType::Integer => out.push_str("  %exit_code = trunc i64 %entry_result to i32\n"),
        TaiType::Boolean => {
            out.push_str("  %entry_result_i32 = zext i1 %entry_result to i32\n");
            out.push_str("  %exit_code = add i32 %entry_result_i32, 0\n");
        }
        TaiType::Text | TaiType::Void => out.push_str("  %exit_code = add i32 0, 0\n"),
    }
    out.push_str("  ret i32 %exit_code\n");
    out.push_str("}\n");
    Ok(out)
}

#[derive(Debug, Clone)]
struct LlvmBlock {
    label: String,
    body: Vec<MirInstruction>,
    terminator: LlvmTerminator,
}

#[derive(Debug, Clone)]
enum LlvmTerminator {
    Jump(String),
    BranchFalse { condition: usize, false_label: String, true_label: String },
    Return(usize),
}

struct FunctionRenderer<'a> {
    function: &'a MirFunction,
    string_lengths: &'a BTreeMap<usize, usize>,
    slot_types: BTreeMap<usize, TaiType>,
    blocks: Vec<LlvmBlock>,
    temp_index: usize,
}

impl<'a> FunctionRenderer<'a> {
    fn new(function: &'a MirFunction, string_lengths: &'a BTreeMap<usize, usize>) -> Self {
        let mut slot_types = BTreeMap::new();
        for param in &function.params {
            slot_types.insert(param.slot, param.ty.clone());
        }
        for local in &function.locals {
            slot_types.insert(local.slot, local.ty.clone());
        }

        let blocks = normalize_blocks(function);
        Self {
            function,
            string_lengths,
            slot_types,
            blocks,
            temp_index: 0,
        }
    }

    fn render(&mut self) -> Result<String, String> {
        let params = self
            .function
            .params
            .iter()
            .map(|param| format!("{} %arg{}", llvm_type(&param.ty), param.slot))
            .collect::<Vec<_>>()
            .join(", ");

        let mut out = String::new();
        out.push_str(&format!(
            "define {} @{}({}) {{\n",
            llvm_type(&self.function.return_type),
            llvm_fn_name(&self.function.label),
            params
        ));
        out.push_str("entry:\n");

        for (slot, ty) in &self.slot_types {
            out.push_str(&format!(
                "  %slot{} = alloca {}, align {}\n",
                slot,
                llvm_storage_type(ty),
                llvm_storage_align(ty)
            ));
        }
        for param in &self.function.params {
            out.push_str(&format!(
                "  store {} %arg{}, ptr %slot{}, align {}\n",
                llvm_type(&param.ty),
                param.slot,
                param.slot,
                llvm_storage_align(&param.ty)
            ));
        }
        out.push_str(&format!("  br label %{}\n", llvm_block_name(&self.blocks[0].label)));

        let blocks_snapshot = self.blocks.clone();
        for block in &blocks_snapshot {
            out.push_str(&format!("{}:\n", llvm_block_name(&block.label)));
            for inst in &block.body {
                self.render_non_terminator(inst, &mut out)?;
            }
            self.render_terminator(&block.terminator, &mut out)?;
        }

        out.push_str("}\n");
        Ok(out)
    }

    fn render_non_terminator(&mut self, inst: &MirInstruction, out: &mut String) -> Result<(), String> {
        match inst {
            MirInstruction::ConstInt { target, value } => {
                out.push_str(&format!(
                    "  store i64 {}, ptr %slot{}, align {}\n",
                    value,
                    target,
                    llvm_storage_align(&TaiType::Integer)
                ));
            }
            MirInstruction::ConstBool { target, value } => {
                let bit = if *value { 1 } else { 0 };
                out.push_str(&format!(
                    "  store i1 {}, ptr %slot{}, align {}\n",
                    bit,
                    target,
                    llvm_storage_align(&TaiType::Boolean)
                ));
            }
            MirInstruction::ConstNull { target } => match self.slot_type(*target)? {
                TaiType::Integer | TaiType::Void => {
                    out.push_str(&format!(
                        "  store i64 0, ptr %slot{}, align {}\n",
                        target,
                        llvm_storage_align(self.slot_type(*target)?)
                    ));
                }
                TaiType::Boolean => {
                    out.push_str(&format!(
                        "  store i1 0, ptr %slot{}, align {}\n",
                        target,
                        llvm_storage_align(self.slot_type(*target)?)
                    ));
                }
                TaiType::Text => {
                    out.push_str(&format!(
                        "  store ptr null, ptr %slot{}, align {}\n",
                        target,
                        llvm_storage_align(self.slot_type(*target)?)
                    ));
                }
            },
            MirInstruction::ConstString { target, string_id } => {
                let len = self
                    .string_lengths
                    .get(string_id)
                    .copied()
                    .ok_or_else(|| format!("LLVM 后端未找到字符串常量 {}", string_id))?;
                let reg = self.next_reg();
                out.push_str(&format!(
                    "  {} = getelementptr inbounds [{} x i8], ptr @.str.{}, i64 0, i64 0\n",
                    reg, len, string_id
                ));
                out.push_str(&format!(
                    "  store ptr {}, ptr %slot{}, align {}\n",
                    reg,
                    target,
                    llvm_storage_align(&TaiType::Text)
                ));
            }
            MirInstruction::Copy { target, source } => {
                let source_ty = self.slot_type(*source)?.clone();
                let value = self.load_slot(*source, &source_ty, out);
                out.push_str(&format!(
                    "  store {} {}, ptr %slot{}, align {}\n",
                    llvm_storage_type(&source_ty),
                    value,
                    target,
                    llvm_storage_align(&source_ty)
                ));
            }
            MirInstruction::Unary { target, op, operand } => {
                self.render_unary(*target, *op, *operand, out)?;
            }
            MirInstruction::Binary { target, left, op, right } => {
                self.render_binary(*target, *left, *op, *right, out)?;
            }
            MirInstruction::Call {
                target,
                callee,
                arguments,
            } => {
                let mut args = Vec::new();
                for slot in arguments {
                    let ty = self.slot_type(*slot)?.clone();
                    let value = self.load_slot(*slot, &ty, out);
                    args.push(format!("{} {}", llvm_type(&ty), value));
                }
                let reg = self.next_reg();
                out.push_str(&format!(
                    "  {} = call {} @{}({})\n",
                    reg,
                    llvm_type(self.slot_type(*target)?),
                    llvm_fn_name(callee),
                    args.join(", ")
                ));
                let target_ty = self.slot_type(*target)?.clone();
                out.push_str(&format!(
                    "  store {} {}, ptr %slot{}, align {}\n",
                    llvm_storage_type(&target_ty),
                    reg,
                    target,
                    llvm_storage_align(&target_ty)
                ));
            }
            MirInstruction::Print { value } => {
                self.render_print(*value, out)?;
            }
            MirInstruction::Jump { .. }
            | MirInstruction::JumpIfFalse { .. }
            | MirInstruction::Return { .. } => {
                return Err("LLVM 基本块规范化失败：body 中不应包含终结指令".to_string());
            }
        }
        Ok(())
    }

    fn render_terminator(&mut self, term: &LlvmTerminator, out: &mut String) -> Result<(), String> {
        match term {
            LlvmTerminator::Jump(target) => {
                out.push_str(&format!("  br label %{}\n", llvm_block_name(target)));
            }
            LlvmTerminator::BranchFalse {
                condition,
                false_label,
                true_label,
            } => {
                let cond = self.load_slot(*condition, &TaiType::Boolean, out);
                out.push_str(&format!(
                    "  br i1 {}, label %{}, label %{}\n",
                    cond,
                    llvm_block_name(true_label),
                    llvm_block_name(false_label)
                ));
            }
            LlvmTerminator::Return(slot) => {
                let ty = self.slot_type(*slot)?.clone();
                match ty {
                    TaiType::Integer => {
                        let value = self.load_slot(*slot, &TaiType::Integer, out);
                        out.push_str(&format!("  ret i64 {}\n", value));
                    }
                    TaiType::Boolean => {
                        let flag = self.load_slot(*slot, &TaiType::Boolean, out);
                        out.push_str(&format!("  ret i1 {}\n", flag));
                    }
                    TaiType::Text => {
                        let value = self.load_slot(*slot, &TaiType::Text, out);
                        out.push_str(&format!("  ret ptr {}\n", value));
                    }
                    TaiType::Void => {
                        out.push_str("  ret i64 0\n");
                    }
                }
            }
        }
        Ok(())
    }

    fn render_unary(
        &mut self,
        target: usize,
        op: MirUnaryOp,
        operand: usize,
        out: &mut String,
    ) -> Result<(), String> {
        match op {
            MirUnaryOp::Not => {
                let value = self.load_slot(operand, &TaiType::Boolean, out);
                let reg = self.next_reg();
                out.push_str(&format!("  {} = xor i1 {}, true\n", reg, value));
                out.push_str(&format!(
                    "  store i1 {}, ptr %slot{}, align {}\n",
                    reg,
                    target,
                    llvm_storage_align(&TaiType::Boolean)
                ));
            }
            MirUnaryOp::Negate => {
                let value = self.load_slot(operand, &TaiType::Integer, out);
                let reg = self.next_reg();
                out.push_str(&format!("  {} = sub i64 0, {}\n", reg, value));
                out.push_str(&format!(
                    "  store i64 {}, ptr %slot{}, align {}\n",
                    reg,
                    target,
                    llvm_storage_align(&TaiType::Integer)
                ));
            }
            MirUnaryOp::Positive => {
                let value = self.load_slot(operand, &TaiType::Integer, out);
                out.push_str(&format!(
                    "  store i64 {}, ptr %slot{}, align {}\n",
                    value,
                    target,
                    llvm_storage_align(&TaiType::Integer)
                ));
            }
        }
        Ok(())
    }

    fn render_binary(
        &mut self,
        target: usize,
        left: usize,
        op: MirBinaryOp,
        right: usize,
        out: &mut String,
    ) -> Result<(), String> {
        match op {
            MirBinaryOp::Add
            | MirBinaryOp::Subtract
            | MirBinaryOp::Multiply
            | MirBinaryOp::Divide
            | MirBinaryOp::Modulo => {
                let lhs = self.load_slot(left, &TaiType::Integer, out);
                let rhs = self.load_slot(right, &TaiType::Integer, out);
                let reg = self.next_reg();
                let opcode = match op {
                    MirBinaryOp::Add => "add",
                    MirBinaryOp::Subtract => "sub",
                    MirBinaryOp::Multiply => "mul",
                    MirBinaryOp::Divide => "sdiv",
                    MirBinaryOp::Modulo => "srem",
                    _ => unreachable!(),
                };
                out.push_str(&format!("  {} = {} i64 {}, {}\n", reg, opcode, lhs, rhs));
                out.push_str(&format!(
                    "  store i64 {}, ptr %slot{}, align {}\n",
                    reg,
                    target,
                    llvm_storage_align(&TaiType::Integer)
                ));
            }
            MirBinaryOp::Equal
            | MirBinaryOp::NotEqual
            | MirBinaryOp::Greater
            | MirBinaryOp::GreaterEqual
            | MirBinaryOp::Less
            | MirBinaryOp::LessEqual => {
                let operand_ty = self.slot_type(left)?.clone();
                let lhs = self.load_slot(left, &operand_ty, out);
                let rhs = self.load_slot(right, &operand_ty, out);
                let reg = self.next_reg();
                let predicate = match (&operand_ty, op) {
                    (TaiType::Integer, MirBinaryOp::Equal) => "eq",
                    (TaiType::Integer, MirBinaryOp::NotEqual) => "ne",
                    (TaiType::Integer, MirBinaryOp::Greater) => "sgt",
                    (TaiType::Integer, MirBinaryOp::GreaterEqual) => "sge",
                    (TaiType::Integer, MirBinaryOp::Less) => "slt",
                    (TaiType::Integer, MirBinaryOp::LessEqual) => "sle",
                    (TaiType::Boolean, MirBinaryOp::Equal) => "eq",
                    (TaiType::Boolean, MirBinaryOp::NotEqual) => "ne",
                    (TaiType::Text, MirBinaryOp::Equal) => "eq",
                    (TaiType::Text, MirBinaryOp::NotEqual) => "ne",
                    _ => return Err("LLVM 后端暂不支持该比较操作数类型".to_string()),
                };
                out.push_str(&format!(
                    "  {} = icmp {} {} {}, {}\n",
                    reg,
                    predicate,
                    llvm_type(&operand_ty),
                    lhs,
                    rhs
                ));
                out.push_str(&format!(
                    "  store i1 {}, ptr %slot{}, align {}\n",
                    reg,
                    target,
                    llvm_storage_align(&TaiType::Boolean)
                ));
            }
            MirBinaryOp::And | MirBinaryOp::Or => {
                let lhs = self.load_slot(left, &TaiType::Boolean, out);
                let rhs = self.load_slot(right, &TaiType::Boolean, out);
                let reg = self.next_reg();
                let opcode = if matches!(op, MirBinaryOp::And) { "and" } else { "or" };
                out.push_str(&format!("  {} = {} i1 {}, {}\n", reg, opcode, lhs, rhs));
                out.push_str(&format!(
                    "  store i1 {}, ptr %slot{}, align {}\n",
                    reg,
                    target,
                    llvm_storage_align(&TaiType::Boolean)
                ));
            }
        }
        Ok(())
    }

    fn render_print(&mut self, value: usize, out: &mut String) -> Result<(), String> {
        match self.slot_type(value)?.clone() {
            TaiType::Text => {
                let ptr_value = self.load_slot(value, &TaiType::Text, out);
                let len_ptr = self.next_reg();
                out.push_str(&format!("  {} = alloca i32, align 4\n", len_ptr));
                out.push_str(&format!("  store i32 0, ptr {}, align 4\n", len_ptr));

                let loop_label = self.synthetic_label("strlen_loop");
                let body_label = self.synthetic_label("strlen_body");
                let done_label = self.synthetic_label("strlen_done");

                out.push_str(&format!("  br label %{}\n", llvm_block_name(&loop_label)));
                out.push_str(&format!("{}:\n", llvm_block_name(&loop_label)));
                let cur_len = self.next_reg();
                out.push_str(&format!("  {} = load i32, ptr {}, align 4\n", cur_len, len_ptr));
                let ch_ptr = self.next_reg();
                out.push_str(&format!(
                    "  {} = getelementptr inbounds i8, ptr {}, i32 {}\n",
                    ch_ptr, ptr_value, cur_len
                ));
                let ch = self.next_reg();
                out.push_str(&format!("  {} = load i8, ptr {}, align 1\n", ch, ch_ptr));
                let is_end = self.next_reg();
                out.push_str(&format!("  {} = icmp eq i8 {}, 0\n", is_end, ch));
                out.push_str(&format!(
                    "  br i1 {}, label %{}, label %{}\n",
                    is_end,
                    llvm_block_name(&done_label),
                    llvm_block_name(&body_label)
                ));

                out.push_str(&format!("{}:\n", llvm_block_name(&body_label)));
                let next_len = self.next_reg();
                out.push_str(&format!("  {} = add i32 {}, 1\n", next_len, cur_len));
                out.push_str(&format!("  store i32 {}, ptr {}, align 4\n", next_len, len_ptr));
                out.push_str(&format!("  br label %{}\n", llvm_block_name(&loop_label)));

                out.push_str(&format!("{}:\n", llvm_block_name(&done_label)));
                let final_len = self.next_reg();
                out.push_str(&format!("  {} = load i32, ptr {}, align 4\n", final_len, len_ptr));
                self.emit_write_file(&ptr_value, &final_len, out);
            }
            TaiType::Integer => {
                let int_value = self.load_slot(value, &TaiType::Integer, out);
                self.emit_formatted_integer_write(&int_value, out);
            }
            TaiType::Boolean => {
                let flag = self.load_slot(value, &TaiType::Boolean, out);
                let int_value = self.next_reg();
                out.push_str(&format!("  {} = zext i1 {} to i64\n", int_value, flag));
                self.emit_formatted_integer_write(&int_value, out);
            }
            TaiType::Void => {}
        }
        Ok(())
    }

    fn emit_formatted_integer_write(&mut self, int_value: &str, out: &mut String) {
        let buf = self.next_reg();
        out.push_str(&format!("  {} = alloca [64 x i8], align 16\n", buf));
        let buf_ptr = self.next_reg();
        out.push_str(&format!(
            "  {} = getelementptr inbounds [64 x i8], ptr {}, i64 0, i64 0\n",
            buf_ptr, buf
        ));
        let fmt_ptr = self.next_reg();
        out.push_str(&format!(
            "  {} = getelementptr inbounds [6 x i8], ptr @.fmt.int, i64 0, i64 0\n",
            fmt_ptr
        ));
        let len = self.next_reg();
        out.push_str(&format!(
            "  {} = call i32 (ptr, ptr, ...) @wsprintfA(ptr {}, ptr {}, i64 {})\n",
            len, buf_ptr, fmt_ptr, int_value
        ));
        self.emit_write_file(&buf_ptr, &len, out);
        let crlf_ptr = self.next_reg();
        out.push_str(&format!(
            "  {} = getelementptr inbounds [3 x i8], ptr @__tailang_crlf, i64 0, i64 0\n",
            crlf_ptr
        ));
        self.emit_write_file(&crlf_ptr, "2", out);
    }

    fn emit_write_file(&mut self, ptr_value: &str, len_value: &str, out: &mut String) {
        let handle = self.next_reg();
        out.push_str(&format!("  {} = call ptr @GetStdHandle(i32 -11)\n", handle));
        let written = self.next_reg();
        out.push_str(&format!("  {} = alloca i32, align 4\n", written));
        out.push_str(&format!("  store i32 0, ptr {}, align 4\n", written));
        let write_result = self.next_reg();
        out.push_str(&format!(
            "  {} = call i32 @WriteFile(ptr {}, ptr {}, i32 {}, ptr {}, ptr null)\n",
            write_result, handle, ptr_value, len_value, written
        ));
    }

    fn load_slot(&mut self, slot: usize, ty: &TaiType, out: &mut String) -> String {
        let reg = self.next_reg();
        out.push_str(&format!(
            "  {} = load {}, ptr %slot{}, align {}\n",
            reg,
            llvm_storage_type(ty),
            slot,
            llvm_storage_align(ty)
        ));
        reg
    }

    fn slot_type(&self, slot: usize) -> Result<&TaiType, String> {
        self.slot_types
            .get(&slot)
            .ok_or_else(|| format!("LLVM 后端未找到槽位 {} 的类型信息", slot))
    }

    fn next_reg(&mut self) -> String {
        let name = format!("%t{}", self.temp_index);
        self.temp_index += 1;
        name
    }

    fn synthetic_label(&mut self, prefix: &str) -> String {
        let label = format!("{}_{}_{}", sanitize_symbol(&self.function.label), prefix, self.temp_index);
        self.temp_index += 1;
        label
    }
}

fn normalize_blocks(function: &MirFunction) -> Vec<LlvmBlock> {
    let mut result = Vec::new();
    let mut synthetic_id = 0usize;

    for (block_index, block) in function.blocks.iter().enumerate() {
        let next_original = function.blocks.get(block_index + 1).map(|item| item.label.clone());
        let mut current_label = block.label.clone();
        let mut body = Vec::new();
        let mut iter = block.instructions.iter().peekable();

        while let Some(inst) = iter.next() {
            match inst {
                MirInstruction::Jump { target } => {
                    result.push(LlvmBlock {
                        label: current_label.clone(),
                        body,
                        terminator: LlvmTerminator::Jump(target.clone()),
                    });
                    body = Vec::new();
                    if iter.peek().is_some() {
                        current_label = format!("{}_dead_{}", block.label, synthetic_id);
                        synthetic_id += 1;
                    }
                }
                MirInstruction::JumpIfFalse { condition, target } => {
                    let true_label = if iter.peek().is_some() {
                        let label = format!("{}_cont_{}", block.label, synthetic_id);
                        synthetic_id += 1;
                        label
                    } else if let Some(next) = &next_original {
                        next.clone()
                    } else {
                        let label = format!("{}_fallthrough_{}", block.label, synthetic_id);
                        synthetic_id += 1;
                        label
                    };
                    result.push(LlvmBlock {
                        label: current_label.clone(),
                        body,
                        terminator: LlvmTerminator::BranchFalse {
                            condition: *condition,
                            false_label: target.clone(),
                            true_label: true_label.clone(),
                        },
                    });
                    body = Vec::new();
                    current_label = true_label;
                }
                MirInstruction::Return { value } => {
                    result.push(LlvmBlock {
                        label: current_label.clone(),
                        body,
                        terminator: LlvmTerminator::Return(*value),
                    });
                    body = Vec::new();
                    if iter.peek().is_some() {
                        current_label = format!("{}_dead_{}", block.label, synthetic_id);
                        synthetic_id += 1;
                    }
                }
                other => body.push(other.clone()),
            }
        }

        if !body.is_empty() {
            let terminator = if let Some(next) = next_original.clone() {
                LlvmTerminator::Jump(next)
            } else {
                let zero_slot = function
                    .locals
                    .iter()
                    .find(|item| item.name.starts_with("__temp") && matches!(item.ty, TaiType::Integer))
                    .map(|item| item.slot)
                    .unwrap_or(0);
                LlvmTerminator::Return(zero_slot)
            };
            result.push(LlvmBlock {
                label: current_label,
                body,
                terminator,
            });
        }
    }

    result
}

fn llvm_fn_name(name: &str) -> String {
    format!("tailang_{}", sanitize_symbol(name))
}

fn llvm_block_name(name: &str) -> String {
    format!("bb_{}", sanitize_symbol(name))
}

fn sanitize_symbol(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push_str(&format!("_{:X}", ch as u32));
        }
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

fn escape_llvm_bytes(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        match byte {
            b' '..=b'~' if *byte != b'\\' && *byte != b'"' => out.push(*byte as char),
            _ => out.push_str(&format!("\\{:02X}", byte)),
        }
    }
    out.push_str("\\00");
    out
}

fn llvm_type(ty: &TaiType) -> &'static str {
    match ty {
        TaiType::Integer => "i64",
        TaiType::Boolean => "i1",
        TaiType::Text => "ptr",
        TaiType::Void => "i64",
    }
}

fn entry_return_type(program: &MirProgram) -> Result<TaiType, String> {
    program
        .functions
        .iter()
        .find(|function| function.label == program.entry_label)
        .map(|function| function.return_type.clone())
        .ok_or_else(|| format!("LLVM 后端未找到入口子程序 '{}'", program.entry_label))
}

fn llvm_storage_type(ty: &TaiType) -> &'static str {
    match ty {
        TaiType::Void => "i64",
        _ => llvm_type(ty),
    }
}

fn llvm_storage_align(ty: &TaiType) -> u32 {
    match ty {
        TaiType::Boolean => 1,
        TaiType::Integer | TaiType::Text | TaiType::Void => 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_config::{CompileOptions, CompilerBackend};
    use crate::tai_parser::TaiParser;
    use std::process::Command;

    struct ExecutedBinary {
        stdout: String,
        exit_code: i32,
    }

    fn compile_and_run(source: &str, name: &str) -> ExecutedBinary {
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let out_dir = make_temp_dir().expect("temp dir");
        let exe_path = out_dir.join(format!("{}.exe", name));
        compile_program_with_llvm(
            &program,
            CompileOptions {
                backend: CompilerBackend::Llvm,
                opt_level: OptimizationLevel::O1,
            },
            exe_path.to_str().unwrap(),
        )
        .expect("llvm compile should succeed");

        let output = Command::new(&exe_path)
            .output()
            .expect("compiled llvm executable should run");
        ExecutedBinary {
            stdout: String::from_utf8_lossy(&output.stdout).replace("\r\n", "\n"),
            exit_code: output.status.code().unwrap_or(-1),
        }
    }

    #[test]
    fn detects_installed_llvm_environment() {
        let environment = LlvmEnvironment::detect().expect("llvm environment should be detectable");
        assert!(environment.clang_path.is_file());
        assert!(environment.llvm_c_dll_path.is_file());
        assert!(environment.llvm_c_lib_path.is_file());
    }

    #[test]
    fn compiles_llvm_backend_executable() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序() -> 整数型, , ,
.返回 3
"#;
        let result = compile_and_run(source, "return3");
        assert_eq!(result.exit_code, 3);
    }

    #[test]
    fn runs_internal_function_call_through_llvm_backend() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 加一(输入: 整数型) -> 整数型, , ,
.返回 输入 + 1

.子程序 主程序() -> 整数型, , ,
.返回 加一(2)
"#;
        let result = compile_and_run(source, "call_add_one");
        assert_eq!(result.exit_code, 3);
    }

    #[test]
    fn runs_boolean_return_function_call_through_llvm_backend() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 是否三(输入: 整数型) -> 逻辑型, , ,
.返回 输入 等于 3

.子程序 主程序() -> 整数型, , ,
.如果 是否三(3)
    .返回 1
.否则
    .返回 0
.如果结束
"#;
        let result = compile_and_run(source, "bool_return");
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn runs_void_return_function_call_through_llvm_backend() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 打招呼() -> 空, , ,
.显示 "hi"
.返回

.子程序 主程序() -> 整数型, , ,
打招呼()
.返回 0
"#;
        let result = compile_and_run(source, "void_return");
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "hi\n");
    }

    #[test]
    fn runs_text_return_function_call_through_llvm_backend() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 取文本() -> 文本型, , ,
.返回 "ok"

.子程序 主程序() -> 整数型, , ,
.显示 取文本()
.返回 0
"#;
        let result = compile_and_run(source, "text_return");
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "ok\n");
    }

    #[test]
    fn prints_hello_world_through_llvm_backend() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序() -> 整数型, , ,
.显示 "Hello World"
.返回 0
"#;
        let result = compile_and_run(source, "hello_world");
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "Hello World\n");
    }

    #[test]
    fn runs_numeric_loop_through_llvm_backend() {
        let source = r#"
.版本 3
.程序集 性能测试
.子程序 主程序() -> 整数型, , ,
总和: 整数型 = 0
计数: 整数型 = 0
.循环判断首 计数 小于 1000000
    总和 = 总和 + 1
    计数 = 计数 + 1
.循环判断尾
.显示 总和
.返回 0
"#;
        let result = compile_and_run(source, "numeric_loop");
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "1000000\n");
    }

    #[test]
    fn runs_match_control_flow_through_llvm_backend() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序() -> 整数型, , ,
状态: 整数型 = 2
.判断开始 状态
.判断 1
    .返回 10
.判断 2
    .返回 20
.默认
    .返回 30
.判断结束
"#;
        let result = compile_and_run(source, "match_flow");
        assert_eq!(result.exit_code, 20);
    }

    #[test]
    fn runs_text_equality_through_llvm_backend() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序() -> 整数型, , ,
.如果 "同一文本" 等于 "同一文本"
    .返回 1
.否则
    .返回 0
.如果结束
"#;
        let result = compile_and_run(source, "text_equal");
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn runs_text_inequality_through_llvm_backend() {
        let source = r#"
.版本 3
.程序集 演示
.子程序 主程序() -> 整数型, , ,
.如果 "甲" 不等于 "乙"
    .返回 1
.否则
    .返回 0
.如果结束
"#;
        let result = compile_and_run(source, "text_not_equal");
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn runs_inferred_text_local_through_llvm_backend() {
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
        let result = compile_and_run(source, "inferred_text_local");
        assert_eq!(result.exit_code, 1);
    }

    #[test]
    fn runs_inferred_boolean_local_through_llvm_backend() {
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
        let result = compile_and_run(source, "inferred_boolean_local");
        assert_eq!(result.exit_code, 1);
    }
}
