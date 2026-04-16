//! Tailang Compiler
//! 
//! Compiles formal `.tai` programs to target artifacts.

pub mod lexer;
pub mod parser;
pub mod translator;
pub mod emitter;
pub mod codegen;
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
pub use codegen::{
    CodeGenerator,
    RustCodegenOutput,
    compile_tai_snapshot_to_executable,
    compile_tai_source_to_executable,
    generate_rust_from_tai_snapshot,
    generate_rust_from_tai_source,
};
pub use tai::{TaiCodeBlock, TaiFile, TaiFunction, TaiModule, TaiSource, TaiTranslator, TaiUnresolvedItem};
pub use tai_ast::{TaiCodeDecl, TaiFunctionDecl, TaiMetaField, TaiModuleDecl, TaiProgram, TaiUnresolvedDecl};
pub use tai_exec::{
    parse_native_tai_exec, render_native_tai_exec_to_rust, render_native_tai_expr_to_rust,
    TaiExecError, TaiExecExpr, TaiExecStmt,
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

    let mut parser = Parser::new(tokens);
    let ast = parser.parse().map_err(|err| err.message)?;

    let ir = Translator::new().translate(ast).map_err(|err| err.message)?;
    let tai = TaiTranslator::new().translate(&ir, "tailang_program");
    tai.to_pretty_json()
}

/// Temporary compatibility path:
/// compile legacy `.tai` JSON snapshot into Rust source code.
pub fn compile_tai_snapshot_to_rust_source(tai_snapshot: &str) -> Result<String, String> {
    generate_rust_from_tai_snapshot(tai_snapshot).map(|output| output.rust_source)
}

/// Compile formal textual `.tai` source into Rust source code.
pub fn compile_tai_source_to_rust_source(tai_source: &str) -> Result<String, String> {
    generate_rust_from_tai_source(tai_source).map(|output| output.rust_source)
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
