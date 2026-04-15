//! LLVM 代码生成器
//! 
//! 使用 inkwell (LLVM Rust 绑定) 生成机器码

use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::builder::Builder;
use inkwell::types::BasicType;
use inkwell::values::FunctionValue;
use std::path::Path;

/// LLVM 代码生成器
pub struct CodeGenerator<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
}

impl<'ctx> CodeGenerator<'ctx> {
    /// 创建新的代码生成器
    pub fn new(context: &'ctx Context, name: &str) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();
        
        CodeGenerator {
            context,
            module,
            builder,
        }
    }

    /// 生成 LLVM IR（从 .tai）
    pub fn generate_ir(&mut self, tai: &str) -> Result<(), String> {
        // TODO: 解析 .tai 并生成 LLVM IR
        // 临时实现：生成简单的 main 函数
        
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[], false);
        
        let main_function = self.module.add_function("main", fn_type, None);
        let basic_block = self.context.append_basic_block(main_function, "entry");
        
        self.builder.position_at_end(basic_block);
        
        // 返回 0
        let zero = i32_type.const_int(0, false);
        self.builder.build_return(Some(&zero));
        
        Ok(())
    }

    /// 保存 IR 到文件
    pub fn save_ir_to_file(&self, path: &str) -> Result<(), String> {
        self.module.print_to_file(Path::new(path))
            .map_err(|e| format!("保存 IR 文件失败：{}", e))
    }
}

/// 编译 .tai 到可执行文件
pub fn compile_tai_to_executable(tai: &str, output: &str) -> Result<(), String> {
    let context = Context::create();
    let mut codegen = CodeGenerator::new(&context, "tailang_program");
    
    // 生成 LLVM IR
    codegen.generate_ir(tai)?;
    
    // 保存为 .ll 文件
    let ll_path = format!("{}.ll", output);
    codegen.save_ir_to_file(&ll_path)?;
    
    // 使用 clang 编译为可执行文件
    let status = std::process::Command::new("clang")
        .args(&[&ll_path, "-o", output, "-Wno-override-module"])
        .status()
        .map_err(|e| format!("调用 clang 失败：{}", e))?;
    
    if !status.success() {
        return Err("clang 编译失败".to_string());
    }
    
    // 清理临时文件
    let _ = std::fs::remove_file(&ll_path);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_codegen() {
        let context = Context::create();
        let codegen = CodeGenerator::new(&context, "test");
        assert!(codegen.get_module().get_name().to_str() == Ok("test"));
    }
}
