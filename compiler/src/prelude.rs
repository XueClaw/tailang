use crate::hir::HirBuiltinFunction;
use crate::tai_ast::TaiProgram;
use crate::tai_parser::TaiParser;
use crate::types::TaiType;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreludeImplementation {
    Intrinsic(HirBuiltinFunction),
    Textual,
}

#[derive(Debug, Clone)]
pub struct PreludeLibrary {
    pub program: TaiProgram,
    reserved_function_names: BTreeSet<String>,
}

impl PreludeLibrary {
    pub fn load() -> Result<Self, String> {
        let source = include_str!("../prelude/core.tai");
        let program = TaiParser::from_source(source)
            .map_err(|err| format!("加载内置 prelude 失败：{}", err.message))?;
        let reserved_function_names = program
            .modules
            .iter()
            .flat_map(|module| module.functions.iter().map(|function| function.name.clone()))
            .collect::<BTreeSet<_>>();
        Ok(Self {
            program,
            reserved_function_names,
        })
    }

    pub fn is_reserved_function_name(&self, name: &str) -> bool {
        self.reserved_function_names.contains(name)
    }

    pub fn implementation_for(
        &self,
        name: &str,
        param_types: &[TaiType],
    ) -> PreludeImplementation {
        match (name, param_types) {
            ("显示", [TaiType::Text])
            | ("显示", [TaiType::Integer])
            | ("显示", [TaiType::Boolean])
            | ("print", [TaiType::Text])
            | ("print", [TaiType::Integer])
            | ("print", [TaiType::Boolean]) => {
                PreludeImplementation::Intrinsic(HirBuiltinFunction::Print)
            }
            ("文本长度", [TaiType::Text]) | ("text_len", [TaiType::Text]) => {
                PreludeImplementation::Intrinsic(HirBuiltinFunction::TextLen)
            }
            ("数组长度", [TaiType::Array(inner)]) | ("array_len", [TaiType::Array(inner)])
                if matches!(inner.as_ref(), TaiType::Integer | TaiType::Boolean | TaiType::Text) =>
            {
                PreludeImplementation::Intrinsic(HirBuiltinFunction::ArrayLen)
            }
            _ => PreludeImplementation::Textual,
        }
    }
}
