use crate::tai_ast::TaiVarDecl;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaiType {
    Integer,
    Boolean,
    Text,
    Array(Box<TaiType>),
    Void,
}

impl TaiType {
    pub fn from_decl_name(name: &str) -> Result<Self, String> {
        let name = name.trim();
        if let Some(inner) = name.strip_suffix("[]") {
            return Ok(Self::Array(Box::new(Self::from_decl_name(inner.trim())?)));
        }
        match name {
            "整数型" | "整数" | "int" => Ok(Self::Integer),
            "逻辑型" | "布尔型" | "真假型" | "布尔" | "bool" => Ok(Self::Boolean),
            "文本型" | "文本" | "text" => Ok(Self::Text),
            "整数型数组" | "整数数组" => Ok(Self::Array(Box::new(Self::Integer))),
            "逻辑型数组" | "布尔型数组" | "布尔数组" => Ok(Self::Array(Box::new(Self::Boolean))),
            "文本型数组" | "文本数组" => Ok(Self::Array(Box::new(Self::Text))),
            "空" | "空型" | "无返回" | "void" => Ok(Self::Void),
            other => Err(format!("当前类型系统暂不支持类型 '{}'", other)),
        }
    }

    pub fn parse_optional(name: Option<&str>) -> Result<Self, String> {
        match name {
            Some(value) => Self::from_decl_name(value),
            None => Ok(Self::Void),
        }
    }

    pub fn from_var_decl(decl: &TaiVarDecl) -> Result<Option<Self>, String> {
        decl.ty
            .as_deref()
            .map(Self::from_decl_name)
            .transpose()
    }
}

impl fmt::Display for TaiType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            TaiType::Integer => "整数型",
            TaiType::Boolean => "逻辑型",
            TaiType::Text => "文本型",
            TaiType::Array(inner) => return write!(f, "{}[]", inner),
            TaiType::Void => "空",
        };
        f.write_str(value)
    }
}
