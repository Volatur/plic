use crate::napi::thread::NativeFunction;
use crate::syntax::parser;

pub struct ModuleDef {
    pub path: String,
    pub uses: Vec<String>,
    pub functions: Vec<FunctionDef>,
    pub classes: Vec<ClassDef>,
    pub objects: Vec<ClassDef>
}

pub struct ClassDef {
    pub name: String,
    pub extends: Vec<String>,
    pub methods: Vec<FunctionDef>,
    pub allocation: usize,
}

pub struct FunctionDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: FunctionBodyDef
}

pub enum FunctionBodyDef {
    AST(parser::Block),
    Native(NativeFunction)
}