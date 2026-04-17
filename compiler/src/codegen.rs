//! Tailang 原生 Windows x64 后端
//!
//! 当前版本直接消费 MIR，提供可运行的整数/字符串/条件/循环最小执行子集。

use crate::hir::lower_tai_to_hir;
use crate::native_ir::{
    lower_hir_to_mir, MirBinaryOp, MirBlock, MirInstruction, MirProgram, MirUnaryOp,
};
use crate::runtime::RuntimeAbi;
use crate::tai::{TaiFile, TaiTranslator};
use crate::tai_ast::TaiProgram;
use crate::tai_parser::TaiParser;
use std::collections::BTreeMap;
use std::path::PathBuf;

const FILE_ALIGNMENT: u32 = 0x200;
const SECTION_ALIGNMENT: u32 = 0x1000;
const IMAGE_BASE: u64 = 0x140000000;
const TEXT_RVA: u32 = 0x1000;
const RDATA_RVA: u32 = 0x2000;
const IDATA_RVA: u32 = 0x3000;
const TEXT_RAW_PTR: u32 = 0x200;
const RDATA_RAW_PTR: u32 = 0x800;
const IDATA_RAW_PTR: u32 = 0xC00;
const OPTIONAL_HEADER_SIZE: u16 = 0x00F0;
const PE_OFFSET: u32 = 0x80;
const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;
const LOCAL_SLOT_SIZE: u32 = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExecutable {
    pub image: Vec<u8>,
    pub entry_label: String,
    pub exit_code: u32,
}

pub struct CodeGenerator;

impl CodeGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn build_legacy_snapshot_image(&self, tai: &TaiFile) -> Result<NativeExecutable, String> {
        let _ = tai;
        let mir = MirProgram {
            entry_label: "tailang_main".to_string(),
            locals: vec![],
            blocks: vec![MirBlock {
                label: "tailang_main".to_string(),
                instructions: vec![],
            }],
            strings: vec![],
            exit_code: Some(0),
        };
        Ok(build_native_pe_image(&mir))
    }

    pub fn build_native_image_from_program(
        &self,
        program: &TaiProgram,
    ) -> Result<NativeExecutable, String> {
        let hir = lower_tai_to_hir(program)?;
        let mir = lower_hir_to_mir(&hir)?;
        Ok(build_native_pe_image(&mir))
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn compile_tai_snapshot_to_executable(tai_json: &str, output: &str) -> Result<(), String> {
    let translator = TaiTranslator::new();
    let tai = translator.deserialize(tai_json)?;
    let generated = CodeGenerator::new().build_legacy_snapshot_image(&tai)?;
    write_native_image(&generated, output)
}

pub fn compile_tai_source_to_executable(tai_source: &str, output: &str) -> Result<(), String> {
    let program = TaiParser::from_source(tai_source)
        .map_err(|err| format!("parse .tai source failed at {}: {}", err.offset, err.message))?;
    let generated = CodeGenerator::new().build_native_image_from_program(&program)?;
    write_native_image(&generated, output)
}

fn write_native_image(generated: &NativeExecutable, output: &str) -> Result<(), String> {
    let output_path = PathBuf::from(output);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建输出目录失败：{}", e))?;
    }
    std::fs::write(&output_path, &generated.image)
        .map_err(|e| format!("写入原生可执行文件失败：{}", e))
}

fn build_native_pe_image(program: &MirProgram) -> NativeExecutable {
    let _runtime = RuntimeAbi::windows_x64();
    let frame = FrameLayout::for_program(program);
    let rdata = RdataLayout::for_program(program);
    let idata = ImportLayout::standard_kernel32();
    let text = build_text_section(program, &frame, &rdata, &idata);
    let rdata_bytes = rdata.build_bytes();
    let idata_bytes = idata.build_bytes();

    let text_raw_size = align_up(text.len() as u32, FILE_ALIGNMENT);
    let rdata_raw_size = align_up(rdata_bytes.len() as u32, FILE_ALIGNMENT);
    let idata_raw_size = align_up(idata_bytes.len() as u32, FILE_ALIGNMENT);
    let size_of_headers = FILE_ALIGNMENT;
    let size_of_image = align_up(IDATA_RVA + idata_raw_size, SECTION_ALIGNMENT);

    let total_size = IDATA_RAW_PTR + idata_raw_size;
    let mut image = vec![0u8; total_size as usize];

    write_dos_header(&mut image);
    write_pe_headers(
        &mut image,
        text_raw_size,
        rdata_raw_size,
        idata_raw_size,
        size_of_headers,
        size_of_image,
    );
    write_section_headers(&mut image, text_raw_size, rdata_raw_size, idata_raw_size);

    image[TEXT_RAW_PTR as usize..(TEXT_RAW_PTR + text.len() as u32) as usize].copy_from_slice(&text);
    image[RDATA_RAW_PTR as usize..(RDATA_RAW_PTR + rdata_bytes.len() as u32) as usize]
        .copy_from_slice(&rdata_bytes);
    image[IDATA_RAW_PTR as usize..(IDATA_RAW_PTR + idata_bytes.len() as u32) as usize]
        .copy_from_slice(&idata_bytes);

    NativeExecutable {
        image,
        entry_label: program.entry_label.clone(),
        exit_code: program.exit_code.unwrap_or(0) as u32,
    }
}

fn build_text_section(
    program: &MirProgram,
    frame: &FrameLayout,
    rdata: &RdataLayout,
    idata: &ImportLayout,
) -> Vec<u8> {
    let mut builder = TextBuilder::new(frame, rdata, idata);
    builder.emit_prologue();
    for block in &program.blocks {
        builder.mark_label(&block.label);
        for inst in &block.instructions {
            builder.emit_instruction(inst);
        }
    }
    builder.patch_branches();
    builder.code
}

struct FrameLayout {
    local_offsets: BTreeMap<usize, u32>,
    frame_size: u32,
    bytes_written_disp: u32,
    print_buffer_disp: u32,
}

impl FrameLayout {
    fn for_program(program: &MirProgram) -> Self {
        let max_slot = program.locals.iter().map(|local| local.slot).max().unwrap_or(0);
        let used_slots = if program.locals.is_empty() { 1 } else { max_slot as u32 + 1 };
        let mut local_offsets = BTreeMap::new();
        for slot in 0..used_slots {
            local_offsets.insert(slot as usize, 0x30 + (slot * LOCAL_SLOT_SIZE));
        }
        let bytes_written_disp = 0x28;
        let print_buffer_disp = 0x40 + (used_slots * LOCAL_SLOT_SIZE);
        let frame_size = align_up(print_buffer_disp as u32 + 32, 0x10);
        Self {
            local_offsets,
            frame_size,
            bytes_written_disp,
            print_buffer_disp,
        }
    }

    fn slot_disp(&self, slot: usize) -> u32 {
        self.local_offsets[&slot]
    }
}

struct TextBuilder<'a> {
    code: Vec<u8>,
    frame: &'a FrameLayout,
    rdata: &'a RdataLayout,
    idata: &'a ImportLayout,
    labels: BTreeMap<String, u32>,
    patches: Vec<(usize, String)>,
    internal_label_id: usize,
}

impl<'a> TextBuilder<'a> {
    fn new(frame: &'a FrameLayout, rdata: &'a RdataLayout, idata: &'a ImportLayout) -> Self {
        Self {
            code: Vec::new(),
            frame,
            rdata,
            idata,
            labels: BTreeMap::new(),
            patches: Vec::new(),
            internal_label_id: 0,
        }
    }

    fn current_rva(&self) -> u32 {
        TEXT_RVA + self.code.len() as u32
    }

    fn mark_label(&mut self, label: &str) {
        self.labels.insert(label.to_string(), self.current_rva());
    }

    fn emit_prologue(&mut self) {
        self.code.extend_from_slice(&[0x48, 0x81, 0xEC]);
        self.code.extend_from_slice(&self.frame.frame_size.to_le_bytes());
    }

    fn emit_instruction(&mut self, inst: &MirInstruction) {
        match inst {
            MirInstruction::ConstInt { target, value } => {
                emit_mov_rax_imm64(&mut self.code, *value as u64);
                emit_store_rax_local(&mut self.code, self.frame.slot_disp(*target));
            }
            MirInstruction::ConstBool { target, value } => {
                emit_mov_rax_imm64(&mut self.code, u64::from(*value));
                emit_store_rax_local(&mut self.code, self.frame.slot_disp(*target));
            }
            MirInstruction::ConstNull { target } => {
                emit_mov_rax_imm64(&mut self.code, 0);
                emit_store_rax_local(&mut self.code, self.frame.slot_disp(*target));
            }
            MirInstruction::ConstString { target, string_id } => {
                emit_mov_rax_imm64(&mut self.code, -((*string_id as i64) + 1) as u64);
                emit_store_rax_local(&mut self.code, self.frame.slot_disp(*target));
            }
            MirInstruction::Copy { target, source } => {
                emit_load_rax_local(&mut self.code, self.frame.slot_disp(*source));
                emit_store_rax_local(&mut self.code, self.frame.slot_disp(*target));
            }
            MirInstruction::Unary { target, op, operand } => {
                emit_load_rax_local(&mut self.code, self.frame.slot_disp(*operand));
                match op {
                    MirUnaryOp::Not => {
                        self.code.extend_from_slice(&[0x48, 0x85, 0xC0]);
                        self.code.extend_from_slice(&[0x0F, 0x94, 0xC0]);
                        self.code.extend_from_slice(&[0x48, 0x0F, 0xB6, 0xC0]);
                    }
                    MirUnaryOp::Negate => self.code.extend_from_slice(&[0x48, 0xF7, 0xD8]),
                    MirUnaryOp::Positive => {}
                }
                emit_store_rax_local(&mut self.code, self.frame.slot_disp(*target));
            }
            MirInstruction::Binary { target, left, op, right } => {
                emit_load_rax_local(&mut self.code, self.frame.slot_disp(*left));
                emit_load_rcx_local(&mut self.code, self.frame.slot_disp(*right));
                match op {
                    MirBinaryOp::Add => self.code.extend_from_slice(&[0x48, 0x01, 0xC8]),
                    MirBinaryOp::Subtract => self.code.extend_from_slice(&[0x48, 0x29, 0xC8]),
                    MirBinaryOp::Multiply => self.code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]),
                    MirBinaryOp::Divide => {
                        self.code.extend_from_slice(&[0x48, 0x99]);
                        self.code.extend_from_slice(&[0x48, 0xF7, 0xF9]);
                    }
                    MirBinaryOp::Modulo => {
                        self.code.extend_from_slice(&[0x48, 0x99]);
                        self.code.extend_from_slice(&[0x48, 0xF7, 0xF9]);
                        self.code.extend_from_slice(&[0x48, 0x89, 0xD0]);
                    }
                    MirBinaryOp::Equal
                    | MirBinaryOp::NotEqual
                    | MirBinaryOp::Greater
                    | MirBinaryOp::GreaterEqual
                    | MirBinaryOp::Less
                    | MirBinaryOp::LessEqual => {
                        self.code.extend_from_slice(&[0x48, 0x39, 0xC8]);
                        emit_setcc(&mut self.code, *op);
                        self.code.extend_from_slice(&[0x48, 0x0F, 0xB6, 0xC0]);
                    }
                    MirBinaryOp::And => {
                        self.code.extend_from_slice(&[0x48, 0x85, 0xC0]);
                        self.code.extend_from_slice(&[0x0F, 0x95, 0xC2]);
                        self.code.extend_from_slice(&[0x48, 0x85, 0xC9]);
                        self.code.extend_from_slice(&[0x0F, 0x95, 0xC1]);
                        self.code.extend_from_slice(&[0x20, 0xCA]);
                        self.code.extend_from_slice(&[0x48, 0x0F, 0xB6, 0xC2]);
                    }
                    MirBinaryOp::Or => {
                        self.code.extend_from_slice(&[0x48, 0x09, 0xC8]);
                        self.code.extend_from_slice(&[0x48, 0x85, 0xC0]);
                        self.code.extend_from_slice(&[0x0F, 0x95, 0xC0]);
                        self.code.extend_from_slice(&[0x48, 0x0F, 0xB6, 0xC0]);
                    }
                }
                emit_store_rax_local(&mut self.code, self.frame.slot_disp(*target));
            }
            MirInstruction::Print { value } => self.emit_print_value(*value),
            MirInstruction::Jump { target } => self.emit_jump(target),
            MirInstruction::JumpIfFalse { condition, target } => {
                emit_load_rax_local(&mut self.code, self.frame.slot_disp(*condition));
                self.code.extend_from_slice(&[0x48, 0x85, 0xC0, 0x0F, 0x84]);
                let patch = self.code.len();
                self.code.extend_from_slice(&0i32.to_le_bytes());
                self.patches.push((patch, target.clone()));
            }
            MirInstruction::Return { value } => {
                emit_load_rcx_local(&mut self.code, self.frame.slot_disp(*value));
                emit_call_iat(&mut self.code, self.idata.iat_rva("ExitProcess"));
            }
        }
    }

    fn emit_print_value(&mut self, slot: usize) {
        let string_label = self.new_internal_label("print_string");
        let done_label = self.new_internal_label("print_done");
        emit_load_rax_local(&mut self.code, self.frame.slot_disp(slot));
        self.code.extend_from_slice(&[0x48, 0x85, 0xC0]);
        self.emit_conditional_jump(0x88, &string_label);
        self.emit_print_integer(slot);
        self.emit_jump(&done_label);
        self.mark_label(&string_label);
        self.emit_print_string_ref(slot);
        self.mark_label(&done_label);
    }

    fn emit_print_string_ref(&mut self, slot: usize) {
        emit_load_rax_local(&mut self.code, self.frame.slot_disp(slot));
        self.code.extend_from_slice(&[0x48, 0xF7, 0xD8, 0x48, 0x83, 0xE8, 0x01]);
        for (id, rva) in &self.rdata.string_rvas {
            let next = self.new_internal_label("print_string_next");
            self.code.extend_from_slice(&[0x48, 0x3D]);
            self.code.extend_from_slice(&(*id as u32).to_le_bytes());
            self.emit_conditional_jump(0x85, &next);
            self.emit_write_file_call(*rva, self.rdata.string_lengths[id]);
            self.mark_label(&next);
        }
    }

    fn emit_print_integer(&mut self, slot: usize) {
        let disp = self.frame.print_buffer_disp;
        let non_negative = self.new_internal_label("int_non_negative");
        let non_zero = self.new_internal_label("int_non_zero");
        let digit_loop = self.new_internal_label("int_digit_loop");
        let maybe_minus = self.new_internal_label("int_maybe_minus");
        let write_out = self.new_internal_label("int_write");

        emit_mov_byte_rsp_disp32_imm8(&mut self.code, disp + 30, b'\r');
        emit_mov_byte_rsp_disp32_imm8(&mut self.code, disp + 31, b'\n');
        emit_lea_r10_rsp_disp32(&mut self.code, disp + 30);
        emit_mov_r8d_imm32(&mut self.code, 2);
        emit_xor_r9d_r9d(&mut self.code);
        emit_load_rax_local(&mut self.code, self.frame.slot_disp(slot));
        self.code.extend_from_slice(&[0x48, 0x85, 0xC0]);
        self.emit_conditional_jump(0x89, &non_negative);
        self.code.extend_from_slice(&[0x48, 0xF7, 0xD8]);
        emit_mov_r9d_imm32(&mut self.code, 1);
        self.mark_label(&non_negative);
        self.code.extend_from_slice(&[0x48, 0x85, 0xC0]);
        self.emit_conditional_jump(0x85, &non_zero);
        emit_dec_r10(&mut self.code);
        emit_mov_byte_r10_imm8(&mut self.code, b'0');
        emit_inc_r8d(&mut self.code);
        self.emit_jump(&maybe_minus);

        self.mark_label(&non_zero);
        self.mark_label(&digit_loop);
        emit_xor_edx_edx(&mut self.code);
        emit_mov_r11_imm32(&mut self.code, 10);
        emit_div_r11(&mut self.code);
        self.code.extend_from_slice(&[0x80, 0xC2, b'0']);
        emit_dec_r10(&mut self.code);
        emit_mov_byte_r10_dl(&mut self.code);
        emit_inc_r8d(&mut self.code);
        self.code.extend_from_slice(&[0x48, 0x85, 0xC0]);
        self.emit_conditional_jump(0x85, &digit_loop);

        self.mark_label(&maybe_minus);
        emit_test_r9d_r9d(&mut self.code);
        self.emit_conditional_jump(0x84, &write_out);
        emit_dec_r10(&mut self.code);
        emit_mov_byte_r10_imm8(&mut self.code, b'-');
        emit_inc_r8d(&mut self.code);

        self.mark_label(&write_out);
        self.emit_write_file_from_r10();
    }

    fn emit_write_file_from_r10(&mut self) {
        emit_mov_r12_r10(&mut self.code);
        emit_mov_r13d_r8d(&mut self.code);
        emit_mov_ecx_imm32(&mut self.code, STD_OUTPUT_HANDLE);
        emit_call_iat(&mut self.code, self.idata.iat_rva("GetStdHandle"));
        self.code.extend_from_slice(&[0x48, 0x89, 0xC1]);
        emit_mov_rdx_r12(&mut self.code);
        emit_mov_r8d_r13d(&mut self.code);
        emit_lea_r9_rsp_disp32(&mut self.code, self.frame.bytes_written_disp);
        emit_mov_qword_rsp_disp32_imm32(&mut self.code, 0x20, 0);
        emit_mov_dword_rsp_disp32_r8d(&mut self.code, self.frame.bytes_written_disp);
        emit_call_iat(&mut self.code, self.idata.iat_rva("WriteFile"));
    }

    fn emit_write_file_call(&mut self, rva: u32, len: u32) {
        emit_mov_ecx_imm32(&mut self.code, STD_OUTPUT_HANDLE);
        emit_call_iat(&mut self.code, self.idata.iat_rva("GetStdHandle"));
        self.code.extend_from_slice(&[0x48, 0x89, 0xC1]);
        self.code.extend_from_slice(&[0x48, 0xBA]);
        self.code.extend_from_slice(&(IMAGE_BASE + rva as u64).to_le_bytes());
        emit_mov_r8d_imm32(&mut self.code, len);
        emit_lea_r9_rsp_disp32(&mut self.code, self.frame.bytes_written_disp);
        emit_mov_qword_rsp_disp32_imm32(&mut self.code, 0x20, 0);
        emit_mov_qword_rsp_disp32_imm32(&mut self.code, self.frame.bytes_written_disp, 0);
        emit_call_iat(&mut self.code, self.idata.iat_rva("WriteFile"));
    }

    fn emit_jump(&mut self, target: &str) {
        self.code.push(0xE9);
        let patch = self.code.len();
        self.code.extend_from_slice(&0i32.to_le_bytes());
        self.patches.push((patch, target.to_string()));
    }

    fn emit_conditional_jump(&mut self, opcode: u8, target: &str) {
        self.code.extend_from_slice(&[0x0F, opcode]);
        let patch = self.code.len();
        self.code.extend_from_slice(&0i32.to_le_bytes());
        self.patches.push((patch, target.to_string()));
    }

    fn new_internal_label(&mut self, prefix: &str) -> String {
        let label = format!("__{}_{}", prefix, self.internal_label_id);
        self.internal_label_id += 1;
        label
    }

    fn patch_branches(&mut self) {
        for (patch_pos, label) in &self.patches {
            if let Some(target) = self.labels.get(label) {
                let next_rva = TEXT_RVA + (*patch_pos as u32) + 4;
                let disp = *target as i32 - next_rva as i32;
                self.code[*patch_pos..*patch_pos + 4].copy_from_slice(&disp.to_le_bytes());
            }
        }
    }
}

fn emit_setcc(code: &mut Vec<u8>, op: MirBinaryOp) {
    let opcode = match op {
        MirBinaryOp::Equal => 0x94,
        MirBinaryOp::NotEqual => 0x95,
        MirBinaryOp::Greater => 0x9F,
        MirBinaryOp::GreaterEqual => 0x9D,
        MirBinaryOp::Less => 0x9C,
        MirBinaryOp::LessEqual => 0x9E,
        _ => unreachable!("invalid setcc op"),
    };
    code.extend_from_slice(&[0x0F, opcode, 0xC0, 0x48, 0x0F, 0xB6, 0xC0]);
}

fn emit_call_iat(code: &mut Vec<u8>, target_rva: u32) {
    code.extend_from_slice(&[0xFF, 0x15]);
    let next_rva = TEXT_RVA + code.len() as u32 + 4;
    let disp = target_rva as i32 - next_rva as i32;
    code.extend_from_slice(&disp.to_le_bytes());
}

fn emit_mov_rax_imm64(code: &mut Vec<u8>, value: u64) {
    code.extend_from_slice(&[0x48, 0xB8]);
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_mov_ecx_imm32(code: &mut Vec<u8>, value: u32) {
    code.push(0xB9);
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_load_rax_local(code: &mut Vec<u8>, disp: u32) {
    code.extend_from_slice(&[0x48, 0x8B, 0x84, 0x24]);
    code.extend_from_slice(&disp.to_le_bytes());
}

fn emit_load_rcx_local(code: &mut Vec<u8>, disp: u32) {
    code.extend_from_slice(&[0x48, 0x8B, 0x8C, 0x24]);
    code.extend_from_slice(&disp.to_le_bytes());
}

fn emit_store_rax_local(code: &mut Vec<u8>, disp: u32) {
    code.extend_from_slice(&[0x48, 0x89, 0x84, 0x24]);
    code.extend_from_slice(&disp.to_le_bytes());
}

fn emit_mov_qword_rsp_disp32_imm32(code: &mut Vec<u8>, disp: u32, value: u32) {
    code.extend_from_slice(&[0x48, 0xC7, 0x84, 0x24]);
    code.extend_from_slice(&disp.to_le_bytes());
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_mov_dword_rsp_disp32_r8d(code: &mut Vec<u8>, disp: u32) {
    code.extend_from_slice(&[0x44, 0x89, 0x84, 0x24]);
    code.extend_from_slice(&disp.to_le_bytes());
}

fn emit_mov_byte_rsp_disp32_imm8(code: &mut Vec<u8>, disp: u32, value: u8) {
    code.extend_from_slice(&[0xC6, 0x84, 0x24]);
    code.extend_from_slice(&disp.to_le_bytes());
    code.push(value);
}

fn emit_lea_r10_rsp_disp32(code: &mut Vec<u8>, disp: u32) {
    code.extend_from_slice(&[0x4C, 0x8D, 0x94, 0x24]);
    code.extend_from_slice(&disp.to_le_bytes());
}

fn emit_lea_r9_rsp_disp32(code: &mut Vec<u8>, disp: u32) {
    code.extend_from_slice(&[0x4C, 0x8D, 0x8C, 0x24]);
    code.extend_from_slice(&disp.to_le_bytes());
}

fn emit_mov_r8d_imm32(code: &mut Vec<u8>, value: u32) {
    code.extend_from_slice(&[0x41, 0xB8]);
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_mov_r9d_imm32(code: &mut Vec<u8>, value: u32) {
    code.extend_from_slice(&[0x41, 0xB9]);
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_mov_r11_imm32(code: &mut Vec<u8>, value: u32) {
    code.extend_from_slice(&[0x49, 0xC7, 0xC3]);
    code.extend_from_slice(&value.to_le_bytes());
}

fn emit_xor_r9d_r9d(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x45, 0x31, 0xC9]);
}

fn emit_xor_edx_edx(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x31, 0xD2]);
}

fn emit_test_r9d_r9d(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x45, 0x85, 0xC9]);
}

fn emit_dec_r10(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x49, 0xFF, 0xCA]);
}

fn emit_inc_r8d(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x41, 0xFF, 0xC0]);
}

fn emit_mov_byte_r10_imm8(code: &mut Vec<u8>, value: u8) {
    code.extend_from_slice(&[0x41, 0xC6, 0x02, value]);
}

fn emit_mov_byte_r10_dl(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x41, 0x88, 0x12]);
}

fn emit_mov_r12_r10(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x4D, 0x89, 0xD4]);
}

fn emit_mov_rdx_r12(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x4C, 0x89, 0xE2]);
}

fn emit_mov_r13d_r8d(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x45, 0x89, 0xC5]);
}

fn emit_mov_r8d_r13d(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x45, 0x89, 0xE8]);
}

fn emit_div_r11(code: &mut Vec<u8>) {
    code.extend_from_slice(&[0x49, 0xF7, 0xF3]);
}

fn write_dos_header(image: &mut [u8]) {
    image[0] = b'M';
    image[1] = b'Z';
    put_u32(image, 0x3C, PE_OFFSET);
}

fn write_pe_headers(
    image: &mut [u8],
    text_raw_size: u32,
    rdata_raw_size: u32,
    idata_raw_size: u32,
    size_of_headers: u32,
    size_of_image: u32,
) {
    let pe = PE_OFFSET as usize;
    image[pe..pe + 4].copy_from_slice(b"PE\0\0");
    let coff = pe + 4;
    put_u16(image, coff, 0x8664);
    put_u16(image, coff + 2, 3);
    put_u16(image, coff + 16, OPTIONAL_HEADER_SIZE);
    put_u16(image, coff + 18, 0x0022);

    let opt = coff + 20;
    put_u16(image, opt, 0x20B);
    image[opt + 2] = 1;
    image[opt + 3] = 0;
    put_u32(image, opt + 4, text_raw_size);
    put_u32(image, opt + 8, rdata_raw_size + idata_raw_size);
    put_u32(image, opt + 16, TEXT_RVA);
    put_u32(image, opt + 20, TEXT_RVA);
    put_u64(image, opt + 24, IMAGE_BASE);
    put_u32(image, opt + 32, SECTION_ALIGNMENT);
    put_u32(image, opt + 36, FILE_ALIGNMENT);
    put_u16(image, opt + 40, 6);
    put_u16(image, opt + 48, 6);
    put_u32(image, opt + 56, size_of_image);
    put_u32(image, opt + 60, size_of_headers);
    put_u16(image, opt + 68, 3);
    put_u16(image, opt + 70, 0x0100);
    put_u64(image, opt + 72, 0x0010_0000);
    put_u64(image, opt + 80, 0x1000);
    put_u64(image, opt + 88, 0x0010_0000);
    put_u64(image, opt + 96, 0x1000);
    put_u32(image, opt + 104, 0);
    put_u32(image, opt + 108, 16);
    put_u32(image, opt + 112 + 8, IDATA_RVA);
    put_u32(image, opt + 112 + 12, idata_raw_size);
}

fn write_section_headers(
    image: &mut [u8],
    text_raw_size: u32,
    rdata_raw_size: u32,
    idata_raw_size: u32,
) {
    let section = PE_OFFSET as usize + 4 + 20 + OPTIONAL_HEADER_SIZE as usize;
    write_section_header(image, section, b".text\0\0\0", align_up(text_raw_size, SECTION_ALIGNMENT), TEXT_RVA, text_raw_size, TEXT_RAW_PTR, 0x6000_0020);
    write_section_header(image, section + 40, b".rdata\0\0", align_up(rdata_raw_size, SECTION_ALIGNMENT), RDATA_RVA, rdata_raw_size, RDATA_RAW_PTR, 0x4000_0040);
    write_section_header(image, section + 80, b".idata\0\0", align_up(idata_raw_size, SECTION_ALIGNMENT), IDATA_RVA, idata_raw_size, IDATA_RAW_PTR, 0xC000_0040);
}

fn write_section_header(image: &mut [u8], offset: usize, name: &[u8; 8], virtual_size: u32, virtual_address: u32, size_of_raw_data: u32, pointer_to_raw_data: u32, characteristics: u32) {
    image[offset..offset + 8].copy_from_slice(name);
    put_u32(image, offset + 8, virtual_size);
    put_u32(image, offset + 12, virtual_address);
    put_u32(image, offset + 16, size_of_raw_data);
    put_u32(image, offset + 20, pointer_to_raw_data);
    put_u32(image, offset + 36, characteristics);
}

fn align_up(value: u32, alignment: u32) -> u32 {
    if value == 0 { return 0; }
    ((value + alignment - 1) / alignment) * alignment
}

fn put_u16(buf: &mut [u8], offset: usize, value: u16) {
    buf[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn put_u32(buf: &mut [u8], offset: usize, value: u32) {
    buf[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn put_u64(buf: &mut [u8], offset: usize, value: u64) {
    buf[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

struct RdataLayout {
    string_rvas: BTreeMap<usize, u32>,
    string_lengths: BTreeMap<usize, u32>,
    bytes: Vec<u8>,
}

impl RdataLayout {
    fn for_program(program: &MirProgram) -> Self {
        let mut string_rvas = BTreeMap::new();
        let mut string_lengths = BTreeMap::new();
        let mut bytes = Vec::new();
        if program.strings.is_empty() {
            let fallback = b"Tailang runtime placeholder\r\n";
            string_rvas.insert(0, RDATA_RVA);
            string_lengths.insert(0, fallback.len() as u32);
            bytes.extend_from_slice(fallback);
        } else {
            for item in &program.strings {
                let rendered = format!("{}\r\n", item.value);
                let offset = bytes.len() as u32;
                bytes.extend_from_slice(rendered.as_bytes());
                string_rvas.insert(item.id, RDATA_RVA + offset);
                string_lengths.insert(item.id, rendered.len() as u32);
            }
        }
        Self { string_rvas, string_lengths, bytes }
    }

    fn build_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
}

struct ImportLayout {
    bytes: Vec<u8>,
    symbol_iat: BTreeMap<String, u32>,
}

impl ImportLayout {
    fn standard_kernel32() -> Self {
        let symbols = vec!["GetStdHandle".to_string(), "WriteFile".to_string(), "ExitProcess".to_string()];
        let descriptor_size = 20u32;
        let descriptor_count = 2u32;
        let descriptors_bytes = descriptor_size * descriptor_count;
        let ilt_offset = descriptors_bytes;
        let ilt_bytes = ((symbols.len() + 1) * 8) as u32;
        let iat_offset = ilt_offset + ilt_bytes;
        let iat_bytes = ilt_bytes;
        let names_offset = iat_offset + iat_bytes;

        let mut hint_name_offsets = Vec::new();
        let mut name_cursor = names_offset;
        for symbol in &symbols {
            hint_name_offsets.push(name_cursor);
            name_cursor += 2 + symbol.len() as u32 + 1;
        }
        let dll_name_offset = name_cursor;
        let dll_name = b"KERNEL32.dll\0";
        let total_size = dll_name_offset + dll_name.len() as u32;
        let mut bytes = vec![0u8; total_size as usize];

        put_u32(&mut bytes, 0x00, IDATA_RVA + ilt_offset);
        put_u32(&mut bytes, 0x0C, IDATA_RVA + dll_name_offset);
        put_u32(&mut bytes, 0x10, IDATA_RVA + iat_offset);

        let mut symbol_iat = BTreeMap::new();
        for (index, symbol) in symbols.iter().enumerate() {
            let hint_name_rva = IDATA_RVA + hint_name_offsets[index];
            put_u64(&mut bytes, (ilt_offset + (index as u32) * 8) as usize, hint_name_rva as u64);
            let iat_entry_offset = iat_offset + (index as u32) * 8;
            put_u64(&mut bytes, iat_entry_offset as usize, hint_name_rva as u64);
            symbol_iat.insert(symbol.clone(), IDATA_RVA + iat_entry_offset);
            put_u16(&mut bytes, hint_name_offsets[index] as usize, 0);
            let start = hint_name_offsets[index] as usize + 2;
            let name_bytes = symbol.as_bytes();
            bytes[start..start + name_bytes.len()].copy_from_slice(name_bytes);
            bytes[start + name_bytes.len()] = 0;
        }

        let dll_start = dll_name_offset as usize;
        bytes[dll_start..dll_start + dll_name.len()].copy_from_slice(dll_name);

        Self { bytes, symbol_iat }
    }

    fn build_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    fn iat_rva(&self, symbol: &str) -> u32 {
        self.symbol_iat[symbol]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    #[test]
    fn builds_valid_pe_image() {
        let mir = MirProgram {
            entry_label: "主程序".to_string(),
            locals: vec![],
            blocks: vec![MirBlock {
                label: "主程序".to_string(),
                instructions: vec![],
            }],
            strings: vec![],
            exit_code: Some(0),
        };
        let image = build_native_pe_image(&mir);
        assert_eq!(&image.image[0..2], b"MZ");
        let pe_offset = read_u32(&image.image, 0x3C) as usize;
        assert_eq!(&image.image[pe_offset..pe_offset + 4], b"PE\0\0");
    }

    #[test]
    fn builds_program_from_tai_source() {
        let source = r#"
.版本 3
.程序集 main
.子程序 主程序
.显示 "Hello World"
.返回 0
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let image = CodeGenerator::new()
            .build_native_image_from_program(&program)
            .expect("native build should succeed");
        assert_eq!(image.entry_label, "主程序");
    }

    #[test]
    fn uses_disp32_stack_slots_for_large_frames() {
        let source = r#"
.版本 3
.目标平台 windows

.程序集 测试
.子程序 主程序
.令 总和 = 0
.令 计数 = 0
.循环判断首 计数 小于 30
    总和 = 总和 + 1
    计数 = 计数 + 1
.循环判断尾
.显示 总和
.返回 0
"#;
        let program = TaiParser::from_source(source).expect("parse should succeed");
        let image = CodeGenerator::new()
            .build_native_image_from_program(&program)
            .expect("native build should succeed");
        assert!(
            image.image.windows(3).any(|window| window == [0x8B, 0x84, 0x24]),
            "generated code should use disp32 stack loads for large frames"
        );
        assert!(
            image.image.windows(3).any(|window| window == [0x89, 0x84, 0x24]),
            "generated code should use disp32 stack stores for large frames"
        );
    }
}
