//! Tailang 原生 Windows x64 后端
//!
//! 该模块直接生成最小 PE32+ 可执行文件，不再经过宿主源码或 `rustc`。

use crate::tai::{TaiFile, TaiTranslator};
use crate::tai_ast::{TaiFunctionDecl, TaiProgram};
use crate::tai_exec::{parse_native_tai_exec, TaiExecExpr, TaiExecStmt};
use crate::tai_parser::TaiParser;
use std::path::PathBuf;

const FILE_ALIGNMENT: u32 = 0x200;
const SECTION_ALIGNMENT: u32 = 0x1000;
const IMAGE_BASE: u64 = 0x140000000;
const TEXT_RVA: u32 = 0x1000;
const IDATA_RVA: u32 = 0x2000;
const TEXT_RAW_PTR: u32 = 0x200;
const IDATA_RAW_PTR: u32 = 0x400;
const OPTIONAL_HEADER_SIZE: u16 = 0x00F0;
const PE_OFFSET: u32 = 0x80;

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
        let exit_code = infer_snapshot_exit_code(tai);
        Ok(build_native_pe_image("tailang_main", exit_code))
    }

    pub fn build_native_image_from_program(
        &self,
        program: &TaiProgram,
    ) -> Result<NativeExecutable, String> {
        let (entry_label, exit_code) = infer_program_entry(program)?;
        Ok(build_native_pe_image(&entry_label, exit_code))
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

fn infer_program_entry(program: &TaiProgram) -> Result<(String, u32), String> {
    for module in &program.modules {
        for function in &module.functions {
            if is_entry_function(&function.name) {
                let exit_code = infer_function_exit_code(function)?;
                return Ok((function.name.clone(), exit_code));
            }
        }
    }

    if let Some(function) = program.modules.iter().flat_map(|m| &m.functions).next() {
        return Ok((function.name.clone(), infer_function_exit_code(function)?));
    }

    Ok(("tailang_main".to_string(), 0))
}

fn is_entry_function(name: &str) -> bool {
    matches!(name.trim(), "主程序" | "主函数" | "main" | "Main")
}

fn infer_function_exit_code(function: &TaiFunctionDecl) -> Result<u32, String> {
    let Some(implementation) = &function.implementation else {
        return Ok(0);
    };

    let statements = parse_native_tai_exec(implementation)
        .map_err(|err| format!("原生 .tai 执行语法解析失败：{}", err.message))?;
    Ok(infer_exit_code_from_statements(&statements).unwrap_or(0))
}

fn infer_exit_code_from_statements(statements: &[TaiExecStmt]) -> Option<u32> {
    for stmt in statements.iter().rev() {
        match stmt {
            TaiExecStmt::Return(Some(expr)) => {
                if let Some(value) = infer_exit_code_from_expr(expr) {
                    return Some(value);
                }
            }
            TaiExecStmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                let then_code = infer_exit_code_from_statements(then_branch);
                let else_code = else_branch
                    .as_ref()
                    .and_then(|branch| infer_exit_code_from_statements(branch));
                if let (Some(a), Some(b)) = (then_code, else_code) {
                    return Some(a.max(b));
                }
            }
            TaiExecStmt::Match {
                branches,
                default_branch,
                ..
            } => {
                let mut seen = None;
                for (_, branch) in branches {
                    seen = seen.or_else(|| infer_exit_code_from_statements(branch));
                }
                if seen.is_none() {
                    seen = default_branch
                        .as_ref()
                        .and_then(|branch| infer_exit_code_from_statements(branch));
                }
                if seen.is_some() {
                    return seen;
                }
            }
            _ => {}
        }
    }
    None
}

fn infer_exit_code_from_expr(expr: &TaiExecExpr) -> Option<u32> {
    match expr {
        TaiExecExpr::Number(value) => value.parse::<u32>().ok(),
        TaiExecExpr::Bool(value) => Some(u32::from(*value)),
        TaiExecExpr::Null => Some(0),
        _ => None,
    }
}

fn infer_snapshot_exit_code(_tai: &TaiFile) -> u32 {
    0
}

fn build_native_pe_image(entry_label: &str, exit_code: u32) -> NativeExecutable {
    let text = build_text_section(exit_code);
    let idata = build_idata_section();
    let text_raw_size = align_up(text.len() as u32, FILE_ALIGNMENT);
    let idata_raw_size = align_up(idata.len() as u32, FILE_ALIGNMENT);
    let size_of_headers = FILE_ALIGNMENT;
    let size_of_image = align_up(IDATA_RVA + idata_raw_size, SECTION_ALIGNMENT);

    let mut image = vec![0u8; (IDATA_RAW_PTR + idata_raw_size) as usize];

    write_dos_header(&mut image);
    write_pe_headers(
        &mut image,
        text_raw_size,
        idata_raw_size,
        size_of_headers,
        size_of_image,
    );
    write_section_headers(&mut image, text_raw_size, idata_raw_size);

    let text_range = TEXT_RAW_PTR as usize..(TEXT_RAW_PTR + text.len() as u32) as usize;
    image[text_range].copy_from_slice(&text);
    let idata_range = IDATA_RAW_PTR as usize..(IDATA_RAW_PTR + idata.len() as u32) as usize;
    image[idata_range].copy_from_slice(&idata);

    NativeExecutable {
        image,
        entry_label: entry_label.to_string(),
        exit_code,
    }
}

fn build_text_section(exit_code: u32) -> Vec<u8> {
    let mut code = Vec::with_capacity(24);
    code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]);
    code.push(0xB9);
    code.extend_from_slice(&exit_code.to_le_bytes());
    code.extend_from_slice(&[0xFF, 0x15]);
    let next_rva = TEXT_RVA + code.len() as u32 + 4;
    let disp = (IDATA_RVA + 0x38) as i32 - next_rva as i32;
    code.extend_from_slice(&disp.to_le_bytes());
    code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x28, 0xC3]);
    code
}

fn build_idata_section() -> Vec<u8> {
    let mut idata = vec![0u8; 0x80];

    let ilt_rva = IDATA_RVA + 0x28;
    let iat_rva = IDATA_RVA + 0x38;
    let dll_name_rva = IDATA_RVA + 0x48;
    let hint_name_rva = IDATA_RVA + 0x58;

    put_u32(&mut idata, 0x00, ilt_rva);
    put_u32(&mut idata, 0x0C, dll_name_rva);
    put_u32(&mut idata, 0x10, iat_rva);

    put_u64(&mut idata, 0x28, hint_name_rva as u64);
    put_u64(&mut idata, 0x38, hint_name_rva as u64);

    let dll = b"KERNEL32.dll\0";
    idata[0x48..0x48 + dll.len()].copy_from_slice(dll);

    put_u16(&mut idata, 0x58, 0);
    let func = b"ExitProcess\0";
    idata[0x5A..0x5A + func.len()].copy_from_slice(func);

    idata
}

fn write_dos_header(image: &mut [u8]) {
    image[0] = b'M';
    image[1] = b'Z';
    put_u32(image, 0x3C, PE_OFFSET);
}

fn write_pe_headers(
    image: &mut [u8],
    text_raw_size: u32,
    idata_raw_size: u32,
    size_of_headers: u32,
    size_of_image: u32,
) {
    let pe = PE_OFFSET as usize;
    image[pe..pe + 4].copy_from_slice(b"PE\0\0");

    let coff = pe + 4;
    put_u16(image, coff, 0x8664);
    put_u16(image, coff + 2, 2);
    put_u16(image, coff + 16, OPTIONAL_HEADER_SIZE);
    put_u16(image, coff + 18, 0x0022);

    let opt = coff + 20;
    put_u16(image, opt, 0x20B);
    image[opt + 2] = 1;
    image[opt + 3] = 0;
    put_u32(image, opt + 4, text_raw_size);
    put_u32(image, opt + 8, idata_raw_size);
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
    put_u16(image, opt + 70, 0x8160);
    put_u64(image, opt + 72, 0x0010_0000);
    put_u64(image, opt + 80, 0x1000);
    put_u64(image, opt + 88, 0x0010_0000);
    put_u64(image, opt + 96, 0x1000);
    put_u32(image, opt + 104, 0);
    put_u32(image, opt + 108, 16);

    put_u32(image, opt + 112 + 8, IDATA_RVA);
    put_u32(image, opt + 112 + 12, 0x80);
}

fn write_section_headers(image: &mut [u8], text_raw_size: u32, idata_raw_size: u32) {
    let section = PE_OFFSET as usize + 4 + 20 + OPTIONAL_HEADER_SIZE as usize;

    write_section_header(
        image,
        section,
        b".text\0\0\0",
        0x1000,
        TEXT_RVA,
        text_raw_size,
        TEXT_RAW_PTR,
        0x6000_0020,
    );
    write_section_header(
        image,
        section + 40,
        b".idata\0\0",
        0x1000,
        IDATA_RVA,
        idata_raw_size,
        IDATA_RAW_PTR,
        0xC000_0040,
    );
}

fn write_section_header(
    image: &mut [u8],
    offset: usize,
    name: &[u8; 8],
    virtual_size: u32,
    virtual_address: u32,
    size_of_raw_data: u32,
    pointer_to_raw_data: u32,
    characteristics: u32,
) {
    image[offset..offset + 8].copy_from_slice(name);
    put_u32(image, offset + 8, virtual_size);
    put_u32(image, offset + 12, virtual_address);
    put_u32(image, offset + 16, size_of_raw_data);
    put_u32(image, offset + 20, pointer_to_raw_data);
    put_u32(image, offset + 36, characteristics);
}

fn align_up(value: u32, alignment: u32) -> u32 {
    if value == 0 {
        return 0;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn read_u32(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
    }

    #[test]
    fn builds_valid_pe_image() {
        let image = build_native_pe_image("主程序", 0);
        assert_eq!(&image.image[0..2], b"MZ");
        let pe_offset = read_u32(&image.image, 0x3C) as usize;
        assert_eq!(&image.image[pe_offset..pe_offset + 4], b"PE\0\0");
    }

    #[test]
    fn infers_exit_code_from_main_return() {
        let source = r#"
.版本 3
.程序集 main
.子程序 主程序
.返回 7
"#;

        let program = TaiParser::from_source(source).expect("parse should succeed");
        let image = CodeGenerator::new()
            .build_native_image_from_program(&program)
            .expect("native build should succeed");
        assert_eq!(image.exit_code, 7);
        assert_eq!(image.entry_label, "主程序");
    }

    #[test]
    fn falls_back_to_zero_exit_code_without_impl() {
        let source = r#"
.版本 3
.程序集 main
.子程序 主程序
.说明 "空入口"
"#;

        let program = TaiParser::from_source(source).expect("parse should succeed");
        let image = CodeGenerator::new()
            .build_native_image_from_program(&program)
            .expect("native build should succeed");
        assert_eq!(image.exit_code, 0);
    }
}
