use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Primitive(PrimitiveType),
    Struct(StructType),
    Sum(SumType),
    GenericInstance(GenericTypeInstance),
    Function(FunctionType),
    Interface(InterfaceType),
    Var(TypeVariable),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveType {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructType {
    pub name: String,
    pub fields: HashMap<String, Type>,
    pub methods: HashMap<String, CallableSignature>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SumType {
    pub variants: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GenericTypeInstance {
    pub base: String,
    pub args: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub returns: Box<Type>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceType {
    pub name: String,
    pub methods: HashMap<String, CallableSignature>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeVariable {
    pub id: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallableSignature {
    pub params: Vec<(String, Type)>,
    pub returns: Type,
}

pub fn types_equal(a: &Type, b: &Type) -> bool {
    match (a, b) {
        (Type::Primitive(a), Type::Primitive(b)) => a.name == b.name,
        (Type::Struct(a), Type::Struct(b)) => a.name == b.name,
        (Type::Interface(a), Type::Interface(b)) => a.name == b.name,
        _ => false,
    }
}

pub fn is_subtype(sub: &Type, sup: &Type) -> bool {
    match (sub, sup) {
        (Type::Struct(s), Type::Interface(i)) => i
            .methods
            .iter()
            .all(|(name, sig)| s.methods.get(name).map_or(false, |ms| callable_sigs_match(ms, sig))),
        _ => types_equal(sub, sup),
    }
}

fn callable_sigs_match(a: &CallableSignature, b: &CallableSignature) -> bool {
    a.params.len() == b.params.len()
        && a
            .params
            .iter()
            .zip(&b.params)
            .all(|((_, at), (_, bt))| types_equal(at, bt))
        && types_equal(&a.returns, &b.returns)
}
