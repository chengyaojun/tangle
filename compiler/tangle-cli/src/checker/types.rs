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
            .all(|(name, sig)| s.methods.get(name).is_some_and(|ms| callable_sigs_match(ms, sig))),
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- helpers ---

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType {
            name: name.to_string(),
        })
    }

    fn struct_type(name: &str) -> Type {
        Type::Struct(StructType {
            name: name.to_string(),
            fields: HashMap::new(),
            methods: HashMap::new(),
        })
    }

    fn interface_type(name: &str) -> Type {
        Type::Interface(InterfaceType {
            name: name.to_string(),
            methods: HashMap::new(),
        })
    }

    fn sig(params: Vec<(&str, Type)>, returns: Type) -> CallableSignature {
        CallableSignature {
            params: params
                .into_iter()
                .map(|(n, t)| (n.to_string(), t))
                .collect(),
            returns,
        }
    }

    // --- 1. types_equal for primitive types ---

    #[test]
    fn types_equal_primitives() {
        let string_a = prim("String");
        let string_b = prim("String");
        let int = prim("Int");

        assert!(types_equal(&string_a, &string_b));
        assert!(!types_equal(&string_a, &int));
    }

    // --- 2. types_equal for struct types ---

    #[test]
    fn types_equal_structs() {
        let user_a = struct_type("User");
        let user_b = struct_type("User");
        let order = struct_type("Order");

        assert!(types_equal(&user_a, &user_b));
        assert!(!types_equal(&user_a, &order));
    }

    // --- 3. types_equal for interface types ---

    #[test]
    fn types_equal_interfaces() {
        let comparable_a = interface_type("Comparable");
        let comparable_b = interface_type("Comparable");
        let serializable = interface_type("Serializable");

        assert!(types_equal(&comparable_a, &comparable_b));
        assert!(!types_equal(&comparable_a, &serializable));
    }

    // --- 4. is_subtype — struct satisfying an interface ---

    #[test]
    fn is_subtype_struct_satisfies_interface() {
        let mut struct_methods = HashMap::new();
        struct_methods.insert(
            "compare".to_string(),
            sig(
                vec![("other", prim("Int"))],
                prim("Bool"),
            ),
        );

        let mut iface_methods = HashMap::new();
        iface_methods.insert(
            "compare".to_string(),
            sig(
                vec![("other", prim("Int"))],
                prim("Bool"),
            ),
        );

        let sub = Type::Struct(StructType {
            name: "MyStruct".to_string(),
            fields: HashMap::new(),
            methods: struct_methods,
        });

        let sup = Type::Interface(InterfaceType {
            name: "Comparable".to_string(),
            methods: iface_methods,
        });

        assert!(is_subtype(&sub, &sup));
    }

    // --- 5. is_subtype — struct NOT satisfying an interface (missing method) ---

    #[test]
    fn is_subtype_struct_missing_method() {
        let mut iface_methods = HashMap::new();
        iface_methods.insert(
            "toString".to_string(),
            sig(vec![], prim("String")),
        );

        let sub = Type::Struct(StructType {
            name: "Bare".to_string(),
            fields: HashMap::new(),
            methods: HashMap::new(), // no methods
        });

        let sup = Type::Interface(InterfaceType {
            name: "Display".to_string(),
            methods: iface_methods,
        });

        assert!(!is_subtype(&sub, &sup));
    }

    // --- 6. CallableSignature matching ---

    #[test]
    fn callable_signature_match() {
        let a = sig(
            vec![("x", prim("Int")), ("y", prim("Int"))],
            prim("Bool"),
        );
        let b = sig(
            vec![("a", prim("Int")), ("b", prim("Int"))],
            prim("Bool"),
        );
        // different param names, same types — should match
        assert!(callable_sigs_match(&a, &b));

        let c = sig(vec![("x", prim("String"))], prim("Bool"));
        // different param type — should NOT match
        assert!(!callable_sigs_match(&a, &c));

        // different arity
        let d = sig(vec![], prim("Bool"));
        assert!(!callable_sigs_match(&a, &d));

        // same arity, different return type
        let e = sig(
            vec![("x", prim("Int")), ("y", prim("Int"))],
            prim("String"),
        );
        assert!(!callable_sigs_match(&a, &e));
    }
}
