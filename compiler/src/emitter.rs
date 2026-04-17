use crate::translator::{IRExpression, IRFunction, IRInstruction, IRProgram, IRVariable};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLanguage {
    Rust,
    Go,
    JavaScript,
}

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
    fn from(message: String) -> Self {
        Self { message }
    }
}

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

    pub fn emit(mut self, program: &IRProgram) -> Result<String, EmitError> {
        for function in &program.functions {
            self.emit_function(function)?;
            self.newline();
        }

        for variable in &program.variables {
            self.emit_variable(variable)?;
        }

        for instruction in &program.instructions {
            self.emit_instruction(instruction)?;
        }

        Ok(self.output)
    }

    fn emit_function(&mut self, function: &IRFunction) -> Result<(), EmitError> {
        match self.target {
            TargetLanguage::Rust => {
                let params = function
                    .params
                    .iter()
                    .map(|param| format!("{param}: i64"))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.write_line(&format!("fn {}({}) {{", function.name, params));
                self.indent_level += 1;
                if function.body.is_empty() {
                    self.write_line("// empty");
                } else {
                    for instruction in &function.body {
                        self.emit_instruction(instruction)?;
                    }
                }
                self.indent_level -= 1;
                self.write_line("}");
            }
            TargetLanguage::Go => {
                let params = function
                    .params
                    .iter()
                    .map(|param| format!("{param} int"))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.write_line(&format!("func {}({}) {{", function.name, params));
                self.indent_level += 1;
                if function.body.is_empty() {
                    self.write_line("// empty");
                } else {
                    for instruction in &function.body {
                        self.emit_instruction(instruction)?;
                    }
                }
                self.indent_level -= 1;
                self.write_line("}");
            }
            TargetLanguage::JavaScript => {
                self.write_line(&format!(
                    "function {}({}) {{",
                    function.name,
                    function.params.join(", ")
                ));
                self.indent_level += 1;
                if function.body.is_empty() {
                    self.write_line("// empty");
                } else {
                    for instruction in &function.body {
                        self.emit_instruction(instruction)?;
                    }
                }
                self.indent_level -= 1;
                self.write_line("}");
            }
        }

        Ok(())
    }

    fn emit_variable(&mut self, variable: &IRVariable) -> Result<(), EmitError> {
        let value = match &variable.value {
            Some(expr) => self.render_expr(expr)?,
            None => match self.target {
                TargetLanguage::Rust => "()".to_string(),
                TargetLanguage::Go => "nil".to_string(),
                TargetLanguage::JavaScript => "null".to_string(),
            },
        };

        match self.target {
            TargetLanguage::Rust => self.write_line(&format!("let {} = {};", variable.name, value)),
            TargetLanguage::Go => self.write_line(&format!("var {} = {}", variable.name, value)),
            TargetLanguage::JavaScript => self.write_line(&format!("let {} = {};", variable.name, value)),
        }

        Ok(())
    }

    fn emit_instruction(&mut self, instruction: &IRInstruction) -> Result<(), EmitError> {
        match instruction {
            IRInstruction::Declare(variable) => self.emit_variable(variable)?,
            IRInstruction::Assign { target, value } => {
                let rendered = self.render_expr(value)?;
                let suffix = if matches!(self.target, TargetLanguage::Go) { "" } else { ";" };
                self.write_line(&format!("{target} = {rendered}{suffix}"));
            }
            IRInstruction::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                match self.target {
                    TargetLanguage::Rust | TargetLanguage::Go => {
                        self.write_line(&format!("if {} {{", self.render_expr(condition)?));
                    }
                    TargetLanguage::JavaScript => {
                        self.write_line(&format!("if ({}) {{", self.render_expr(condition)?));
                    }
                }
                self.indent_level += 1;
                for instr in then_branch {
                    self.emit_instruction(instr)?;
                }
                self.indent_level -= 1;
                if let Some(else_branch) = else_branch {
                    self.write_line("} else {");
                    self.indent_level += 1;
                    for instr in else_branch {
                        self.emit_instruction(instr)?;
                    }
                    self.indent_level -= 1;
                }
                self.write_line("}");
            }
            IRInstruction::Loop { condition, body } => {
                match self.target {
                    TargetLanguage::Rust => self.write_line(&format!("while {} {{", self.render_expr(condition)?)),
                    TargetLanguage::Go => self.write_line(&format!("for {} {{", self.render_expr(condition)?)),
                    TargetLanguage::JavaScript => {
                        self.write_line(&format!("while ({}) {{", self.render_expr(condition)?))
                    }
                }
                self.indent_level += 1;
                for instr in body {
                    self.emit_instruction(instr)?;
                }
                self.indent_level -= 1;
                self.write_line("}");
            }
            IRInstruction::Return(value) => {
                let rendered = value
                    .as_ref()
                    .map(|expr| self.render_expr(expr))
                    .transpose()?
                    .unwrap_or_default();

                if rendered.is_empty() {
                    self.write_line("return;");
                } else {
                    self.write_line(&format!("return {};", rendered));
                }
            }
            IRInstruction::Expr(expr) => {
                let rendered = self.render_expr(expr)?;
                let suffix = if matches!(self.target, TargetLanguage::Go) { "" } else { ";" };
                self.write_line(&format!("{rendered}{suffix}"));
            }
            IRInstruction::Call { callee, arguments } => {
                let rendered_args = arguments
                    .iter()
                    .map(|arg| self.render_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                let suffix = if matches!(self.target, TargetLanguage::Go) { "" } else { ";" };
                self.write_line(&format!("{callee}({rendered_args}){suffix}"));
            }
            IRInstruction::CodeBlock(code) => {
                for line in code.lines() {
                    self.write_line(line);
                }
            }
        }

        Ok(())
    }

    fn render_expr(&self, expr: &IRExpression) -> Result<String, EmitError> {
        Ok(match expr {
            IRExpression::Identifier(name) => name.clone(),
            IRExpression::Number(value) => value.clone(),
            IRExpression::String(value) => format!("{:?}", value),
            IRExpression::Bool(value) => match self.target {
                TargetLanguage::Rust | TargetLanguage::Go | TargetLanguage::JavaScript => {
                    value.to_string()
                }
            },
            IRExpression::Null => match self.target {
                TargetLanguage::Rust => "()".to_string(),
                TargetLanguage::Go => "nil".to_string(),
                TargetLanguage::JavaScript => "null".to_string(),
            },
            IRExpression::Binary { left, op, right } => {
                format!("{} {} {}", self.render_expr(left)?, op, self.render_expr(right)?)
            }
            IRExpression::Unary { op, operand } => format!("{op}{}", self.render_expr(operand)?),
            IRExpression::Assign { target, value } => {
                format!("{} = {}", self.render_expr(target)?, self.render_expr(value)?)
            }
            IRExpression::Call { callee, arguments } => {
                let args = arguments
                    .iter()
                    .map(|arg| self.render_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                format!("{}({})", self.render_expr(callee)?, args)
            }
        })
    }

    fn write_line(&mut self, line: &str) {
        for _ in 0..self.indent_level {
            self.output.push_str("    ");
        }
        self.output.push_str(line);
        self.output.push('\n');
    }

    fn newline(&mut self) {
        self.output.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translator::{IRExpression, IRFunction, IRInstruction, IRProgram};

    #[test]
    fn emits_rust_function() {
        let program = IRProgram {
            functions: vec![IRFunction {
                name: "add".to_string(),
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

        let output = Emitter::new(TargetLanguage::Rust)
            .emit(&program)
            .expect("emit should succeed");

        assert!(output.contains("fn add(a: i64, b: i64) {"));
        assert!(output.contains("return a + b;"));
    }
}
