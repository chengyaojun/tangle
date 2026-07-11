use crate::checker::errors::ErrorRegistry;
use crate::checker::types::{FunctionType, Type};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ReceiverContext {
    pub struct_name: String,
    pub fields: HashMap<String, Type>,
}

#[derive(Debug, Clone)]
pub struct TypeEnv {
    pub variables: HashMap<String, Type>,
    pub structs: HashMap<String, Type>,
    pub interfaces: HashMap<String, Type>,
    pub functions: HashMap<String, FunctionType>,
    pub receiver: Option<ReceiverContext>,
    pub error_registry: Option<ErrorRegistry>,
}

impl TypeEnv {
    pub fn new() -> Self {
        TypeEnv {
            variables: HashMap::new(),
            structs: HashMap::new(),
            interfaces: HashMap::new(),
            functions: HashMap::new(),
            receiver: None,
            error_registry: None,
        }
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}
