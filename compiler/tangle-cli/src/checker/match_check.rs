use crate::ast::{MatchArm, MatchPattern};
use crate::checker::types::{SumType, Type};

/// 提取 variant 名（Primitive/Struct/GenericInstance）。
/// 其他类型（Sum/Function/Var/Any/Interface）不支持作为命名 variant。
pub fn variant_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Primitive(p) => Some(p.name.clone()),
        Type::Struct(s) => Some(s.name.clone()),
        Type::GenericInstance(g) => Some(g.base.clone()),
        _ => None,
    }
}

/// 提取 binding 类型。
/// GenericInstance 返回 args[0]（payload）；其他返回 variant 类型本身。
pub fn binding_type_of(variant_ty: &Type) -> Type {
    match variant_ty {
        Type::GenericInstance(g) => g.args.first().cloned().unwrap_or(Type::Any),
        other => other.clone(),
    }
}

/// 在 Sum 的 variants 中按名查找。
pub fn find_variant_by_name<'a>(sum: &'a SumType, name: &str) -> Option<&'a Type> {
    sum.variants
        .iter()
        .find(|v| variant_name(v).as_deref() == Some(name))
}

/// Check match exhaustiveness. Returns list of missing variant names.
/// 支持 Primitive/Struct/GenericInstance variant。
pub fn check_match_exhaustiveness(sum: &SumType, arms: &[MatchArm]) -> Vec<String> {
    let has_wildcard = arms
        .iter()
        .any(|a| matches!(a.pattern, MatchPattern::Wildcard));
    if has_wildcard {
        return vec![];
    }

    let mut missing = vec![];
    for variant in &sum.variants {
        if let Some(name) = variant_name(variant) {
            let covered = arms.iter().any(|a| match &a.pattern {
                MatchPattern::Variant { name: pn, .. } => pn == &name,
                _ => false,
            });
            if !covered {
                missing.push(name);
            }
        }
    }
    missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{MatchArm, MatchPattern, Expr, LiteralExpr, LiteralKind};
    use crate::checker::types::*;
    use crate::model::SourceSpan;
    use std::collections::HashMap;

    fn span() -> SourceSpan {
        SourceSpan { file: "".into(), start_line: 0, start_column: 0, end_line: 0, end_column: 0 }
    }

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType { name: name.to_string() })
    }

    fn generic(base: &str, args: Vec<Type>) -> Type {
        Type::GenericInstance(GenericTypeInstance { base: base.to_string(), args })
    }

    fn struct_type(name: &str) -> Type {
        Type::Struct(StructType { name: name.to_string(), fields: HashMap::new(), methods: HashMap::new() })
    }

    fn arm(name: &str, binding: Option<&str>) -> MatchArm {
        MatchArm {
            pattern: MatchPattern::Variant {
                name: name.to_string(),
                binding: binding.map(|s| s.to_string()),
            },
            body: Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Number, value: "0".to_string(), span: span() }),
            span: span(),
        }
    }

    fn wildcard_arm() -> MatchArm {
        MatchArm {
            pattern: MatchPattern::Wildcard,
            body: Expr::Literal(LiteralExpr { literal_kind: LiteralKind::Number, value: "0".to_string(), span: span() }),
            span: span(),
        }
    }

    // --- variant_name tests ---

    #[test]
    fn variant_name_primitive() {
        assert_eq!(variant_name(&prim("Int")), Some("Int".to_string()));
    }

    #[test]
    fn variant_name_struct() {
        assert_eq!(variant_name(&struct_type("Order")), Some("Order".to_string()));
    }

    #[test]
    fn variant_name_generic_instance() {
        assert_eq!(variant_name(&generic("Some", vec![prim("Int")])), Some("Some".to_string()));
    }

    #[test]
    fn variant_name_sum_returns_none() {
        let sum = Type::Sum(SumType { variants: vec![] });
        assert_eq!(variant_name(&sum), None);
    }

    #[test]
    fn variant_name_any_returns_none() {
        assert_eq!(variant_name(&Type::Any), None);
    }

    // --- binding_type_of tests ---

    #[test]
    fn binding_type_of_generic_instance_returns_payload() {
        let some_int = generic("Some", vec![prim("Int")]);
        assert_eq!(binding_type_of(&some_int), prim("Int"));
    }

    #[test]
    fn binding_type_of_generic_instance_no_args_returns_any() {
        let some_empty = generic("Some", vec![]);
        assert_eq!(binding_type_of(&some_empty), Type::Any);
    }

    #[test]
    fn binding_type_of_primitive_returns_itself() {
        assert_eq!(binding_type_of(&prim("Int")), prim("Int"));
    }

    #[test]
    fn binding_type_of_struct_returns_itself() {
        let s = struct_type("Order");
        assert_eq!(binding_type_of(&s), s);
    }

    // --- find_variant_by_name tests ---

    #[test]
    fn find_variant_by_name_found() {
        let sum = SumType { variants: vec![prim("Int"), prim("String")] };
        let found = find_variant_by_name(&sum, "String");
        assert!(found.is_some());
        assert_eq!(*found.unwrap(), prim("String"));
    }

    #[test]
    fn find_variant_by_name_not_found() {
        let sum = SumType { variants: vec![prim("Int")] };
        assert!(find_variant_by_name(&sum, "Bool").is_none());
    }

    #[test]
    fn find_variant_by_name_generic_instance() {
        let sum = SumType {
            variants: vec![generic("Some", vec![prim("Int")]), prim("None")],
        };
        let found = find_variant_by_name(&sum, "Some");
        assert!(found.is_some());
    }

    // --- check_match_exhaustiveness tests ---

    #[test]
    fn exhaustiveness_all_covered() {
        let sum = SumType { variants: vec![prim("Int"), prim("String")] };
        let arms = vec![arm("Int", None), arm("String", None)];
        assert!(check_match_exhaustiveness(&sum, &arms).is_empty());
    }

    #[test]
    fn exhaustiveness_missing_variant() {
        let sum = SumType { variants: vec![prim("Int"), prim("String")] };
        let arms = vec![arm("Int", None)];
        let missing = check_match_exhaustiveness(&sum, &arms);
        assert_eq!(missing, vec!["String".to_string()]);
    }

    #[test]
    fn exhaustiveness_wildcard_covers_all() {
        let sum = SumType { variants: vec![prim("Int"), prim("String")] };
        let arms = vec![arm("Int", None), wildcard_arm()];
        assert!(check_match_exhaustiveness(&sum, &arms).is_empty());
    }

    #[test]
    fn exhaustiveness_generic_instance_variant() {
        let sum = SumType {
            variants: vec![generic("Some", vec![prim("Int")]), prim("None")],
        };
        let arms = vec![arm("Some", Some("y")), arm("None", None)];
        assert!(check_match_exhaustiveness(&sum, &arms).is_empty());
    }

    #[test]
    fn exhaustiveness_struct_variant() {
        let sum = SumType {
            variants: vec![struct_type("Order"), struct_type("User")],
        };
        let arms = vec![arm("Order", None)];
        let missing = check_match_exhaustiveness(&sum, &arms);
        assert_eq!(missing, vec!["User".to_string()]);
    }
}
