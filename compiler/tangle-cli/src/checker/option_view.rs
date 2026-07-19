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
}
