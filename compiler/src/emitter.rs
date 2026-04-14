use crate::translator::{IRProgram, IRFunction, IRVariable, IRInstruction, IRExpression};

/// 目标语言
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLanguage {
    Python,
    Go,
    JavaScript,
}

/// 发射器错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitError {
    pub message: String,
}

impl std::fmt::Display for EmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for EmitError {}

impl From<String> for EmitError {
    fn from(msg: String) -> Self {
        Self { message: msg }
    }
}

/// 发射器：IR → 目标代码
pub struct Emitter {
    target: TargetLanguage,
    output: String,
    indent_level: usize,
}

impl Emitter {
    pub fn new(target: TargetLanguage) -> Self {
        Self {
            target,
            output: String::new(),
            indent_level: 0,
        }
    }

    pub fn emit(mut self, program: IRProgram) -> Result<String, EmitError> {
        // 生成函数
        for function in &program.functions {
            self.emit_function(function)?;
            self.writeln("");
        }

        // 生成变量
        for variable in &program.variables {
            self.emit_variable(variable)?;
        }

        // 生成指令
        for instruction in &program.instructions {
            self.emit_instruction(instruction)?;
        }

        Ok(self.output)
    }

    fn emit_function(&mut self, function: &IRFunction) -> Result<(), EmitError> {
        match self.target {
            TargetLanguage::Python => {
                self.write(&format!("def {}(", function.name));
                let params: Vec<String> = function.params.clone();
                self.write(&params.join(", "));
                self.writeln("):");
                
                self.indent_level += 1;
                for instruction in &function.body {
                    self.emit_instruction(instruction)?;
                }
                self.indent_level -= 1;
            }

            TargetLanguage::Go => {
                self.write(&format!("func {}(", function.name));
                if function.params.is_empty() {
                    self.write(")");
                } else {
                    let params: Vec<String> = function.params.iter()
                        .map(|p| format!("{} interface{{}}", p))
                        .collect();
                    self.write(&params.join(", "));
                    self.write(")");
                }
                self.writeln(" {");
                
                self.indent_level += 1;
                for instruction in &function.body {
                    self.emit_instruction(instruction)?;
                }
                self.indent_level -= 1;
                
                self.writeln("}");
            }

            TargetLanguage::JavaScript => {
                self.write(&format!("function {}(", function.name));
                self.write(&function.params.join(", "));
                self.writeln(") {");
                
                self.indent_level += 1;
                for instruction in &function.body {
                    self.emit_instruction(instruction)?;
                }
                self.indent_level -= 1;
                
                self.writeln("}");
            }
        }

        Ok(())
    }

    fn emit_variable(&mut self, variable: &IRVariable) -> Result<(), EmitError> {
        match self.target {
            TargetLanguage::Python => {
                self.write(&format!("{} = ", variable.name));
                if let Some(value) = &variable.value {
                    self.emit_expr(value)?;
                } else {
                    self.write("None");
                }
                self.writeln("");
            }

            TargetLanguage::Go => {
                self.write(&format!("var {} = ", variable.name));
                if let Some(value) = &variable.value {
                    self.emit_expr(value)?;
                } else {
                    self.write("nil");
                }
                self.writeln("");
            }

            TargetLanguage::JavaScript => {
                self.write(&format!("let {} = ", variable.name));
                if let Some(value) = &variable.value {
                    self.emit_expr(value)?;
                } else {
                    self.write("null");
                }
                self.writeln(";");
            }
        }

        Ok(())
    }

    fn emit_instruction(&mut self, instruction: &IRInstruction) -> Result<(), EmitError> {
        match instruction {
            IRInstruction::Declare(variable) => {
                self.emit_variable(variable)?;
            }

            IRInstruction::Assign { target, value } => {
                match self.target {
                    TargetLanguage::Python | TargetLanguage::Go => {
                        self.write(&format!("{} = ", target));
                    }
                    TargetLanguage::JavaScript => {
                        self.write(&format!("{} = ", target));
                    }
                }
                self.emit_expr(value)?;
                self.emit_statement_terminator();
            }

            IRInstruction::Conditional { condition, then_branch, else_branch } => {
                match self.target {
                    TargetLanguage::Python => {
                        self.write("if ");
                        self.emit_expr(condition)?;
                        self.writeln(":");
                        
                        self.indent_level += 1;
                        for instr in then_branch {
                            self.emit_instruction(instr)?;
                        }
                        self.indent_level -= 1;

                        if let Some(else_instrs) = else_branch {
                            self.writeln("else:");
                            self.indent_level += 1;
                            for instr in else_instrs {
                                self.emit_instruction(instr)?;
                            }
                            self.indent_level -= 1;
                        }
                    }

                    TargetLanguage::Go => {
                        self.write("if ");
                        self.emit_expr(condition)?;
                        self.writeln(" {");
                        
                        self.indent_level += 1;
                        for instr in then_branch {
                            self.emit_instruction(instr)?;
                        }
                        self.indent_level -= 1;
                        
                        if let Some(else_instrs) = else_branch {
                            self.writeln("} else {");
                            self.indent_level += 1;
                            for instr in else_instrs {
                                self.emit_instruction(instr)?;
                            }
                            self.indent_level -= 1;
                        }
                        self.writeln("}");
                    }

                    TargetLanguage::JavaScript => {
                        self.write("if (");
                        self.emit_expr(condition)?;
                        self.writeln(") {");
                        
                        self.indent_level += 1;
                        for instr in then_branch {
                            self.emit_instruction(instr)?;
                        }
                        self.indent_level -= 1;
                        
                        if let Some(else_instrs) = else_branch {
                            self.writeln("} else {");
                            self.indent_level += 1;
                            for instr in else_instrs {
                                self.emit_instruction(instr)?;
                            }
                            self.indent_level -= 1;
                        }
                        self.writeln("}");
                    }
                }
            }

            IRInstruction::Loop { condition, body } => {
                match self.target {
                    TargetLanguage::Python => {
                        self.write("while ");
                        self.emit_expr(condition)?;
                        self.writeln(":");
                        
                        self.indent_level += 1;
                        for instr in body {
                            self.emit_instruction(instr)?;
                        }
                        self.indent_level -= 1;
                    }

                    TargetLanguage::Go => {
                        self.write("for ");
                        self.emit_expr(condition)?;
                        self.writeln(" {");
                        
                        self.indent_level += 1;
                        for instr in body {
                            self.emit_instruction(instr)?;
                        }
                        self.indent_level -= 1;
                        self.writeln("}");
                    }

                    TargetLanguage::JavaScript => {
                        self.write("while (");
                        self.emit_expr(condition)?;
                        self.writeln(") {");
                        
                        self.indent_level += 1;
                        for instr in body {
                            self.emit_instruction(instr)?;
                        }
                        self.indent_level -= 1;
                        self.writeln("}");
                    }
                }
            }

            IRInstruction::Return(value) => {
                match self.target {
                    TargetLanguage::Python => {
                        self.write("return");
                        if let Some(expr) = value {
                            self.write(" ");
                            self.emit_expr(expr)?;
                        }
                        self.writeln("");
                    }

                    TargetLanguage::Go => {
                        self.write("return");
                        if let Some(expr) = value {
                            self.write(" ");
                            self.emit_expr(expr)?;
                        }
                        self.emit_statement_terminator();
                    }

                    TargetLanguage::JavaScript => {
                        self.write("return");
                        if let Some(expr) = value {
                            self.write(" ");
                            self.emit_expr(expr)?;
                        }
                        self.emit_statement_terminator();
                    }
                }
            }

            IRInstruction::Expr(expr) => {
                self.emit_expr(expr)?;
                self.emit_statement_terminator();
            }

            IRInstruction::Call { callee, arguments } => {
                self.write(&format!("{}(", callee));
                let args: Result<Vec<String>, EmitError> = arguments.iter()
                    .map(|arg| {
                        let mut emitter = Emitter::new(self.target);
                        emitter.emit_expr(arg)?;
                        Ok(emitter.output.trim().to_string())
                    })
                    .collect();
                self.write(&args?.join(", "));
                self.write(")");
                self.emit_statement_terminator();
            }

            IRInstruction::CodeBlock(code) => {
                // 直接输出代码块
                self.writeln(code);
            }
        }

        Ok(())
    }

    fn emit_expr(&mut self, expr: &IRExpression) -> Result<(), EmitError> {
        match expr {
            IRExpression::Identifier(name) => {
                self.write(name);
            }

            IRExpression::Number(value) => {
                self.write(value);
            }

            IRExpression::String(value) => {
                match self.target {
                    TargetLanguage::Python | TargetLanguage::JavaScript => {
                        self.write(&format!("\"{}\"", value));
                    }
                    TargetLanguage::Go => {
                        self.write(&format!("\"{}\"", value));
                    }
                }
            }

            IRExpression::Bool(value) => {
                match self.target {
                    TargetLanguage::Python => {
                        self.write(if *value { "True" } else { "False" });
                    }
                    TargetLanguage::Go => {
                        self.write(if *value { "true" } else { "false" });
                    }
                    TargetLanguage::JavaScript => {
                        self.write(if *value { "true" } else { "false" });
                    }
                }
            }

            IRExpression::Null => {
                match self.target {
                    TargetLanguage::Python => self.write("None"),
                    TargetLanguage::Go => self.write("nil"),
                    TargetLanguage::JavaScript => self.write("null"),
                }
            }

            IRExpression::Binary { left, op, right } => {
                self.emit_expr(left)?;
                self.write(&format!(" {} ", op));
                self.emit_expr(right)?;
            }

            IRExpression::Unary { op, operand } => {
                self.write(op);
                self.emit_expr(operand)?;
            }

            IRExpression::Assign { target, value } => {
                self.emit_expr(target)?;
                self.write(" = ");
                self.emit_expr(value)?;
            }

            IRExpression::Call { callee, arguments } => {
                self.emit_expr(callee)?;
                self.write("(");
                let args: Result<Vec<String>, EmitError> = arguments.iter()
                    .map(|arg| {
                        let mut emitter = Emitter::new(self.target);
                        emitter.emit_expr(arg)?;
                        Ok(emitter.output.trim().to_string())
                    })
                    .collect();
                self.write(&args?.join(", "));
                self.write(")");
            }
        }

        Ok(())
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn writeln(&mut self, s: &str) {
        for _ in 0..self.indent_level {
            match self.target {
                TargetLanguage::Python | TargetLanguage::Go => {
                    self.output.push_str("    ");
                }
                TargetLanguage::JavaScript => {
                    self.output.push_str("  ");
                }
            }
        }
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn emit_statement_terminator(&mut self) {
        match self.target {
            TargetLanguage::Python => self.writeln(""),
            TargetLanguage::Go => self.writeln(";"),
            TargetLanguage::JavaScript => self.writeln(";"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translator::{IRProgram, IRFunction, IRVariable, IRInstruction, IRExpression};

    #[test]
    fn test_emit_python_function() {
        let program = IRProgram {
            functions: vec![IRFunction {
                name: "加法".to_string(),
                params: vec!["a".to_string(), "b".to_string()],
                body: vec![IRInstruction::Return(Some(IRExpression::Binary {
                    left: Box::new(IRExpression::Identifier("a".to_string())),
                    op: "+".to_string(),
                    right: Box::new(IRExpression::Identifier("b".to_string())),
                }))],
            }],
            variables: vec![],
            instructions: vec![],
        };

        let emitter = Emitter::new(TargetLanguage::Python);
        let output = emitter.emit(program).expect("emit failed");

        assert!(output.contains("def 加法(a, b):"));
        assert!(output.contains("return a + b"));
    }

    #[test]
    fn test_emit_go_function() {
        let program = IRProgram {
            functions: vec![IRFunction {
                name: "加法".to_string(),
                params: vec!["a".to_string(), "b".to_string()],
                body: vec![IRInstruction::Return(Some(IRExpression::Binary {
                    left: Box::new(IRExpression::Identifier("a".to_string())),
                    op: "+".to_string(),
                    right: Box::new(IRExpression::Identifier("b".to_string())),
                }))],
            }],
            variables: vec![],
            instructions: vec![],
        };

        let emitter = Emitter::new(TargetLanguage::Go);
        let output = emitter.emit(program).expect("emit failed");

        assert!(output.contains("func 加法("));
        assert!(output.contains("return a + b"));
    }

    #[test]
    fn test_emit_javascript_function() {
        let program = IRProgram {
            functions: vec![IRFunction {
                name: "加法".to_string(),
                params: vec!["a".to_string(), "b".to_string()],
                body: vec![IRInstruction::Return(Some(IRExpression::Binary {
                    left: Box::new(IRExpression::Identifier("a".to_string())),
                    op: "+".to_string(),
                    right: Box::new(IRExpression::Identifier("b".to_string())),
                }))],
            }],
            variables: vec![],
            instructions: vec![],
        };

        let emitter = Emitter::new(TargetLanguage::JavaScript);
        let output = emitter.emit(program).expect("emit failed");

        assert!(output.contains("function 加法(a, b)"));
        assert!(output.contains("return a + b"));
    }

    #[test]
    fn test_emit_if_else() {
        let program = IRProgram {
            functions: vec![],
            variables: vec![],
            instructions: vec![IRInstruction::Conditional {
                condition: IRExpression::Bool(true),
                then_branch: vec![IRInstruction::Return(Some(IRExpression::Number("1".to_string())))],
                else_branch: Some(vec![IRInstruction::Return(Some(IRExpression::Number("2".to_string())))]),
            }],
        };

        let emitter = Emitter::new(TargetLanguage::Python);
        let output = emitter.emit(program).expect("emit failed");

        assert!(output.contains("if True:"));
        assert!(output.contains("return 1"));
        assert!(output.contains("else:"));
        assert!(output.contains("return 2"));
    }
}
