use crate::checker::env::TypeEnv;
use crate::checker::types::*;
use std::collections::HashMap;

/// 把已知类型识别为 Sum 视图。
/// 当前仅识别 `Option<T>`；`Result<T,E>` 推迟到 Phase 6e。
pub fn as_sum_view(ty: &Type) -> Option<SumType> {
    match ty {
        Type::Sum(s) => Some(s.clone()),
        Type::GenericInstance(g) if g.base == "Option" => {
            let inner = g.args.first().cloned().unwrap_or(Type::Any);
            Some(SumType {
                variants: vec![
                    Type::GenericInstance(GenericTypeInstance {
                        base: "Some".into(),
                        args: vec![inner.clone()],
                    }),
                    Type::Struct(StructType {
                        name: "None".into(),
                        fields: HashMap::new(),
                        methods: HashMap::new(),
                    }),
                ],
            })
        }
        _ => None,
    }
}

/// 在 env.structs 中查找同名结构体，若有则返回带字段的完整定义。
///
/// 背景：`type_expr_to_type` 解析 `Option<Item>` 时会生成空字段的外壳
/// `Type::Struct(Item { fields: {} })`。真实带字段的定义位于 `env.structs`。
/// 此函数用于在 binding 类型注入前补全结构体定义，使后续 `let { ... } = x`
/// 解构能找到字段。
pub fn resolve_struct_in_env(ty: &Type, env: &TypeEnv) -> Type {
    match ty {
        Type::Struct(s) => {
            if let Some(Type::Struct(full)) = env.structs.get(&s.name) {
                Type::Struct(full.clone())
            } else {
                ty.clone()
            }
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType {
            name: name.to_string(),
        })
    }

    fn option_of(inner: Type) -> Type {
        Type::GenericInstance(GenericTypeInstance {
            base: "Option".to_string(),
            args: vec![inner],
        })
    }

    fn some_of(inner: Type) -> Type {
        Type::GenericInstance(GenericTypeInstance {
            base: "Some".to_string(),
            args: vec![inner],
        })
    }

    fn none() -> Type {
        Type::Struct(StructType {
            name: "None".to_string(),
            fields: HashMap::new(),
            methods: HashMap::new(),
        })
    }

    #[test]
    fn as_sum_view_option_int() {
        let opt = option_of(prim("Int"));
        let sum = as_sum_view(&opt).expect("should recognize Option<Int>");
        assert_eq!(sum.variants.len(), 2);
        assert_eq!(sum.variants[0], some_of(prim("Int")));
        assert_eq!(sum.variants[1], none());
    }

    #[test]
    fn as_sum_view_option_any_when_no_args() {
        let opt = Type::GenericInstance(GenericTypeInstance {
            base: "Option".to_string(),
            args: vec![],
        });
        let sum = as_sum_view(&opt).expect("should recognize Option without args");
        assert_eq!(sum.variants[0], some_of(Type::Any));
    }

    #[test]
    fn as_sum_view_passes_through_sum_type() {
        let sum_ty = Type::Sum(SumType {
            variants: vec![prim("Int"), prim("String")],
        });
        let view = as_sum_view(&sum_ty).expect("Sum should pass through");
        assert_eq!(view.variants.len(), 2);
    }

    #[test]
    fn as_sum_view_rejects_non_option_generic() {
        let list_int = Type::GenericInstance(GenericTypeInstance {
            base: "List".to_string(),
            args: vec![prim("Int")],
        });
        assert!(as_sum_view(&list_int).is_none());
    }

    #[test]
    fn as_sum_view_rejects_primitive() {
        assert!(as_sum_view(&prim("Int")).is_none());
    }

    #[test]
    fn as_sum_view_rejects_struct() {
        let s = Type::Struct(StructType {
            name: "MyType".to_string(),
            fields: HashMap::new(),
            methods: HashMap::new(),
        });
        assert!(as_sum_view(&s).is_none());
    }

    #[test]
    fn as_sum_view_rejects_any() {
        assert!(as_sum_view(&Type::Any).is_none());
    }

    fn empty_env() -> TypeEnv {
        TypeEnv {
            variables: HashMap::new(),
            structs: HashMap::new(),
            functions: HashMap::new(),
            interfaces: HashMap::new(),
            receiver: None,
            error_registry: None,
        }
    }

    fn env_with_struct(name: &str, fields: Vec<(&str, Type)>) -> TypeEnv {
        let mut env = empty_env();
        let mut field_map = HashMap::new();
        for (fn_, ty) in fields {
            field_map.insert(fn_.to_string(), ty);
        }
        env.structs.insert(
            name.to_string(),
            Type::Struct(StructType {
                name: name.to_string(),
                fields: field_map,
                methods: HashMap::new(),
            }),
        );
        env
    }

    fn empty_struct(name: &str) -> Type {
        Type::Struct(StructType {
            name: name.to_string(),
            fields: HashMap::new(),
            methods: HashMap::new(),
        })
    }

    #[test]
    fn resolve_struct_in_env_fills_fields_from_env() {
        // env.structs["Item"] = { name: String, price: Int }
        // 输入 ty = Struct(Item { fields: {} })（来自 type_expr_to_type 的外壳）
        // 期望：返回带字段的 Struct
        let env = env_with_struct("Item", vec![
            ("name", prim("String")),
            ("price", prim("Int")),
        ]);
        let ty = empty_struct("Item");
        let resolved = resolve_struct_in_env(&ty, &env);
        match resolved {
            Type::Struct(s) => {
                assert_eq!(s.fields.len(), 2);
                assert!(s.fields.contains_key("name"));
                assert!(s.fields.contains_key("price"));
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn resolve_struct_in_env_passes_through_when_not_in_env() {
        // env 无 Item，输入空 Struct(Item)，期望原样返回
        let env = empty_env();
        let ty = empty_struct("Item");
        let resolved = resolve_struct_in_env(&ty, &env);
        match resolved {
            Type::Struct(s) => {
                assert_eq!(s.name, "Item");
                assert!(s.fields.is_empty());
            }
            other => panic!("expected Struct, got {:?}", other),
        }
    }

    #[test]
    fn resolve_struct_in_env_passes_through_non_struct() {
        let env = empty_env();
        let ty = prim("Int");
        let resolved = resolve_struct_in_env(&ty, &env);
        assert_eq!(resolved, prim("Int"));
    }
}
