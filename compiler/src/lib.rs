//! Tailang Compiler
//! 
//! Compiles .meng files to executable

pub mod lexer;
pub mod parser;
pub mod translator;
pub mod emitter;

pub use lexer::Lexer;
pub use parser::Parser;
pub use translator::Translator;
pub use emitter::Emitter;

/// Compile a .meng file to executable
pub fn compile(input: &str) -> Result<Vec<u8>, String> {
    let lexer = Lexer::new(input);
    let tokens = lexer.tokenize()?;
    
    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;
    
    let translator = Translator::new();
    let tai = translator.translate(ast)?;
    
    let emitter = Emitter::new();
    let executable = emitter.emit(tai)?;
    
    Ok(executable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_hello() {
        let input = r#"
            打印 "Hello, Tailang!"
        "#;
        
        let result = compile(input);
        assert!(result.is_ok());
    }
}
