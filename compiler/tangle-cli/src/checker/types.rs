use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Type {
    Primitive(PrimitiveType),
    Struct(StructType),
    Sum(SumType),
    GenericInstance(GenericTypeInstance),
    Function(FunctionType),
    Interface(InterfaceType),
    Var(TypeVariable),
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveType {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructType {
    pub name: String,
    pub fields: HashMap<String, Type>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub methods: HashMap<String, CallableSignature>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SumType {
    pub variants: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericTypeInstance {
    pub base: String,
    pub args: Vec<Type>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub returns: Box<Type>,
    #[serde(skip_serializing)]
    pub is_variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InterfaceType {
    pub name: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub methods: HashMap<String, CallableSignature>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeVariable {
    pub id: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallableSignature {
    pub params: Vec<(String, Type)>,
    pub returns: Type,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_variadic: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

/// 构造类型变量
pub fn type_var(id: usize) -> Type {
    Type::Var(TypeVariable { id })
}

/// 构造泛型实例
pub fn generic(base: &str, args: Vec<Type>) -> Type {
    Type::GenericInstance(GenericTypeInstance {
        base: base.to_string(),
        args,
    })
}

pub fn types_equal(a: &Type, b: &Type) -> bool {
    if matches!(a, Type::Any) || matches!(b, Type::Any) {
        return true;
    }
    match (a, b) {
        (Type::Primitive(a), Type::Primitive(b)) => a.name == b.name,
        (Type::Struct(a), Type::Struct(b)) => a.name == b.name,
        (Type::Interface(a), Type::Interface(b)) => a.name == b.name,
        _ => false,
    }
}

pub fn is_subtype(sub: &Type, sup: &Type) -> bool {
    if matches!(sup, Type::Any) {
        return true;
    }
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
            is_variadic: false,
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

    // --- 7. Type::Any matches everything ---

    #[test]
    fn types_equal_any_matches_all() {
        let any = Type::Any;
        let str_t = prim("String");
        let int_t = prim("Int");
        let struct_t = struct_type("Foo");

        assert!(types_equal(&any, &str_t));
        assert!(types_equal(&str_t, &any));
        assert!(types_equal(&any, &int_t));
        assert!(types_equal(&any, &struct_t));
        assert!(types_equal(&any, &any));
    }

    // --- 8. is_subtype with Any as top type ---

    #[test]
    fn is_subtype_any_is_top() {
        let any = Type::Any;
        let str_t = prim("String");
        let struct_t = struct_type("Foo");

        assert!(is_subtype(&str_t, &any));
        assert!(is_subtype(&struct_t, &any));
        assert!(is_subtype(&any, &any));
    }

    // --- 9. FunctionType has is_variadic field ---

    #[test]
    fn function_type_variadic() {
        let fixed = FunctionType {
            params: vec![prim("Int")],
            returns: Box::new(prim("Bool")),
            is_variadic: false,
        };
        let variadic = FunctionType {
            params: vec![prim("String")],
            returns: Box::new(prim("Void")),
            is_variadic: true,
        };
        assert!(!fixed.is_variadic);
        assert!(variadic.is_variadic);
    }

    // --- 10. type_var and generic constructors ---

    #[test]
    fn type_var_constructor() {
        let v = type_var(0);
        assert!(matches!(v, Type::Var(TypeVariable { id: 0 })));
    }

    #[test]
    fn generic_constructor() {
        let list_int = generic("List", vec![prim("Int")]);
        match list_int {
            Type::GenericInstance(g) => {
                assert_eq!(g.base, "List");
                assert_eq!(g.args.len(), 1);
            }
            _ => panic!("Expected GenericInstance"),
        }
    }
}

#[cfg(test)]
mod serde_tests {
    use super::*;

    #[test]
    fn test_primitive_serializes_with_kind_tag() {
        let ty = Type::Primitive(PrimitiveType { name: "Int".into() });
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "primitive");
        assert_eq!(json["name"], "Int");
    }

    #[test]
    fn test_generic_instance_serializes() {
        let ty = Type::GenericInstance(GenericTypeInstance {
            base: "List".into(),
            args: vec![Type::Primitive(PrimitiveType { name: "Int".into() })],
        });
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "genericInstance");
        assert_eq!(json["base"], "List");
        assert_eq!(json["args"][0]["kind"], "primitive");
        assert_eq!(json["args"][0]["name"], "Int");
    }

    #[test]
    fn test_struct_serializes() {
        let ty = Type::Struct(StructType {
            name: "Order".into(),
            fields: HashMap::new(),
            methods: HashMap::new(),
        });
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "struct");
        assert_eq!(json["name"], "Order");
    }

    #[test]
    fn test_sum_serializes() {
        let ty = Type::Sum(SumType {
            variants: vec![Type::Primitive(PrimitiveType { name: "Int".into() })],
        });
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "sum");
        assert_eq!(json["variants"][0]["kind"], "primitive");
    }

    #[test]
    fn test_any_serializes() {
        let ty = Type::Any;
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "any");
    }

    #[test]
    fn test_roundtrip_deserialize() {
        let ty = Type::GenericInstance(GenericTypeInstance {
            base: "Map".into(),
            args: vec![
                Type::Primitive(PrimitiveType { name: "String".into() }),
                Type::Primitive(PrimitiveType { name: "Int".into() }),
            ],
        });
        let json = serde_json::to_string(&ty).unwrap();
        let back: Type = serde_json::from_str(&json).unwrap();
        assert_eq!(ty, back);
    }

    #[test]
    fn test_struct_omits_empty_methods() {
        let ty = Type::Struct(StructType {
            name: "Order".into(),
            fields: HashMap::new(),
            methods: HashMap::new(),
        });
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "struct");
        assert_eq!(json["name"], "Order");
        assert!(
            json.get("methods").is_none(),
            "empty methods should be omitted"
        );
    }

    #[test]
    fn test_function_omits_is_variadic() {
        let ty = Type::Function(FunctionType {
            params: vec![],
            returns: Box::new(Type::Any),
            is_variadic: true,
        });
        let json = serde_json::to_value(&ty).unwrap();
        assert_eq!(json["kind"], "function");
        assert!(
            json.get("isVariadic").is_none(),
            "isVariadic should be skipped"
        );
        assert!(
            json.get("is_variadic").is_none(),
            "is_variadic should be skipped"
        );
    }
}
