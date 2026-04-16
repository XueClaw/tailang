#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaiProgram {
    pub version: Option<String>,
    pub meta: Vec<TaiMetaField>,
    pub target: Option<String>,
    pub modules: Vec<TaiModuleDecl>,
    pub unresolved: Vec<TaiUnresolvedDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaiMetaField {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaiModuleDecl {
    pub name: String,
    pub globals: Vec<TaiVarDecl>,
    pub doc: Option<String>,
    pub functions: Vec<TaiFunctionDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaiFunctionDecl {
    pub name: String,
    pub return_type: Option<String>,
    pub params: Vec<String>,
    pub param_decls: Vec<TaiVarDecl>,
    pub locals: Vec<TaiVarDecl>,
    pub doc: Option<String>,
    pub validations: Vec<String>,
    pub implementation: Option<String>,
    pub code_blocks: Vec<TaiCodeDecl>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaiVarDecl {
    pub name: String,
    pub ty: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaiCodeDecl {
    pub language: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaiUnresolvedDecl {
    pub kind: String,
    pub description: String,
}
