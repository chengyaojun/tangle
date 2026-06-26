use crate::checker::types::{PrimitiveType, Type};
use std::collections::HashMap;
use std::sync::LazyLock;

pub static BUILTIN_TYPES: LazyLock<HashMap<String, Type>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert(
        "String".into(),
        Type::Primitive(PrimitiveType {
            name: "String".into(),
        }),
    );
    m.insert(
        "Int".into(),
        Type::Primitive(PrimitiveType {
            name: "Int".into(),
        }),
    );
    m.insert(
        "Bool".into(),
        Type::Primitive(PrimitiveType {
            name: "Bool".into(),
        }),
    );
    m
});

pub fn is_builtin_type(name: &str) -> bool {
    BUILTIN_TYPES.contains_key(name)
}
