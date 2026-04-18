use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerBackend {
    SelfNative,
    Llvm,
}

impl CompilerBackend {
    pub fn as_cli_str(self) -> &'static str {
        match self {
            Self::SelfNative => "self-native",
            Self::Llvm => "llvm",
        }
    }
}

impl Default for CompilerBackend {
    fn default() -> Self {
        Self::SelfNative
    }
}

impl fmt::Display for CompilerBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_cli_str())
    }
}

impl FromStr for CompilerBackend {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "self-native" | "self" | "native" => Ok(Self::SelfNative),
            "llvm" => Ok(Self::Llvm),
            other => Err(format!(
                "不支持的后端 '{}', 可选值: self-native, llvm",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationLevel {
    O0,
    O1,
    O2,
}

impl OptimizationLevel {
    pub fn as_cli_str(self) -> &'static str {
        match self {
            Self::O0 => "0",
            Self::O1 => "1",
            Self::O2 => "2",
        }
    }

    pub fn enables_mir_optimizations(self) -> bool {
        !matches!(self, Self::O0)
    }
}

impl Default for OptimizationLevel {
    fn default() -> Self {
        Self::O1
    }
}

impl fmt::Display for OptimizationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_cli_str())
    }
}

impl FromStr for OptimizationLevel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "0" | "O0" | "o0" => Ok(Self::O0),
            "1" | "O1" | "o1" => Ok(Self::O1),
            "2" | "O2" | "o2" => Ok(Self::O2),
            other => Err(format!(
                "不支持的优化等级 '{}', 可选值: 0, 1, 2",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompileOptions {
    pub backend: CompilerBackend,
    pub opt_level: OptimizationLevel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_backend_aliases() {
        assert_eq!(
            "self-native".parse::<CompilerBackend>().unwrap(),
            CompilerBackend::SelfNative
        );
        assert_eq!(
            "llvm".parse::<CompilerBackend>().unwrap(),
            CompilerBackend::Llvm
        );
    }

    #[test]
    fn parses_optimization_levels() {
        assert_eq!("0".parse::<OptimizationLevel>().unwrap(), OptimizationLevel::O0);
        assert_eq!("1".parse::<OptimizationLevel>().unwrap(), OptimizationLevel::O1);
        assert_eq!("2".parse::<OptimizationLevel>().unwrap(), OptimizationLevel::O2);
    }
}
