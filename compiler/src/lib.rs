//! Tailang Compiler
//! 
//! Compiles formal `.tai` programs to target artifacts.

pub mod lexer;
pub mod parser;
pub mod translator;
pub mod emitter;
pub mod compile_config;
pub mod codegen;
pub mod hir;
pub mod llvm_backend;
pub mod native_ir;
pub mod runtime;
pub mod types;
pub mod tai;
pub mod tai_ast;
pub mod tai_exec;
pub mod tai_lexer;
pub mod tai_parser;
pub mod precompiler;

pub use lexer::Lexer;
pub use parser::Parser;
pub use translator::Translator;
pub use emitter::{Emitter, TargetLanguage};
pub use compile_config::{
    CompileOptions,
    CompilerBackend,
    OptimizationLevel,
};
pub use codegen::{
    CodeGenerator,
    NativeExecutable,
    compile_tai_snapshot_to_executable,
    compile_tai_source_to_executable,
    compile_tai_snapshot_to_executable_with_options,
    compile_tai_source_to_executable_with_options,
};
pub use hir::{
    HirBinaryOp,
    HirExpr,
    HirProgram,
    HirStmt,
    HirUnaryOp,
    lower_tai_to_hir,
};
pub use llvm_backend::{
    LlvmEnvironment,
    compile_program_with_llvm,
};
pub use native_ir::{
    MirBinaryOp,
    MirBlock,
    MirInstruction,
    MirLocal,
    MirProgram,
    MirString,
    MirUnaryOp,
    lower_hir_to_mir,
};
pub use runtime::RuntimeAbi;
pub use types::TaiType;
pub use tai::{TaiCodeBlock, TaiFile, TaiFunction, TaiModule, TaiSource, TaiTranslator, TaiUnresolvedItem};
pub use tai_ast::{TaiCodeDecl, TaiFunctionDecl, TaiMetaField, TaiModuleDecl, TaiProgram, TaiUnresolvedDecl};
pub use tai_exec::{
    parse_native_tai_exec, TaiExecError, TaiExecExpr, TaiExecStmt,
};
/// Legacy transitional lexer kept only for compatibility with the old
/// block-based textual `.tai` experiment. v0.3 should use `TaiParser`
/// and `tai_exec` as the primary syntax entry points.
pub use tai_lexer::{TaiLexError, TaiLexer, TaiToken, TaiTokenKind};
pub use tai_parser::{TaiParseError, TaiParser};
pub use precompiler::{Precompiler, PrecompilerConfig, precompile_meng_file};

/// Temporary compatibility path:
/// compile a `.meng` source string into the legacy `.tai` JSON snapshot format.
/// New compiler work should prefer textual `.tai v0.3` as the primary source form.
pub fn compile_meng_to_tai_snapshot(input: &str) -> Result<String, String> {
    let tokens = Lexer::new(input)
        .lex()
        .map_err(|err| err.message)?;

    let parser = Parser::new(tokens);
    let ast = parser.parse().map_err(|err| err.message)?;

    let ir = Translator::new().translate(ast).map_err(|err| err.message)?;
    let tai = TaiTranslator::new().translate(&ir, "tailang_program");
    tai.to_pretty_json()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_meng_to_tai_snapshot_hello() {
        let input = r#"
            打印 "Hello, Tailang!"
        "#;
        
        let result = compile_meng_to_tai_snapshot(input);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("\"version\": \"0.1.0\""));
    }
}
