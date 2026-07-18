use std::collections::HashMap;

use crate::checker::types::*;

/// 类型变量替换表：TypeVarId → 实际类型
pub type Substitution = HashMap<usize, Type>;

/// 统一 expected 类型与 actual 类型，更新 subst。
/// 成功：类型匹配（或类型变量被绑定）；失败：返回冲突描述。
pub fn unify(expected: &Type, actual: &Type, subst: &mut Substitution) -> Result<(), String> {
    match (expected, actual) {
        // Any 总是成功（双向）
        (Type::Any, _) | (_, Type::Any) => Ok(()),

        // 类型变量（expected 侧）：绑定或递归检查
        (Type::Var(v), actual) => {
            if let Some(existing) = subst.get(&v.id).cloned() {
                unify(&existing, actual, subst)
            } else {
                subst.insert(v.id, actual.clone());
                Ok(())
            }
        }
        // 类型变量（actual 侧）：对称处理
        (expected, Type::Var(v)) => {
            if let Some(existing) = subst.get(&v.id).cloned() {
                unify(expected, &existing, subst)
            } else {
                subst.insert(v.id, expected.clone());
                Ok(())
            }
        }

        // 泛型实例：base 必须相同，递归统一参数
        (Type::GenericInstance(a), Type::GenericInstance(b)) => {
            if a.base != b.base {
                return Err(format!("Expected {}, got {}", a.base, b.base));
            }
            if a.args.len() != b.args.len() {
                return Err("Generic arity mismatch".into());
            }
            for (e, a) in a.args.iter().zip(&b.args) {
                unify(e, a, subst)?;
            }
            Ok(())
        }

        // 基本类型：名称匹配
        (Type::Primitive(a), Type::Primitive(b)) => {
            if a.name == b.name { Ok(()) } else { Err(format!("Expected {}, got {}", a.name, b.name)) }
        }

        // 结构体：名称匹配
        (Type::Struct(a), Type::Struct(b)) => {
            if a.name == b.name { Ok(()) } else { Err(format!("Expected {}, got {}", a.name, b.name)) }
        }

        // 函数类型：参数和返回类型递归统一
        (Type::Function(a), Type::Function(b)) => {
            if a.params.len() != b.params.len() {
                return Err("Function arity mismatch".into());
            }
            for (e, a) in a.params.iter().zip(&b.params) {
                unify(e, a, subst)?;
            }
            unify(&a.returns, &b.returns, subst)
        }

        _ => Err(format!("Type mismatch: {:?} vs {:?}", expected, actual)),
    }
}

/// 用 subst 替换类型中的 TypeVariable（递归）
pub fn substitute(ty: &Type, subst: &Substitution) -> Type {
    match ty {
        Type::Var(v) => subst.get(&v.id).cloned().unwrap_or_else(|| ty.clone()),
        Type::GenericInstance(g) => Type::GenericInstance(GenericTypeInstance {
            base: g.base.clone(),
            args: g.args.iter().map(|a| substitute(a, subst)).collect(),
        }),
        Type::Function(f) => Type::Function(FunctionType {
            params: f.params.iter().map(|p| substitute(p, subst)).collect(),
            returns: Box::new(substitute(&f.returns, subst)),
            is_variadic: f.is_variadic,
        }),
        Type::Struct(s) => {
            let mut fields = HashMap::new();
            for (k, v) in &s.fields {
                fields.insert(k.clone(), substitute(v, subst));
            }
            Type::Struct(StructType {
                name: s.name.clone(),
                fields,
                methods: s.methods.clone(),
            })
        }
        Type::Sum(s) => Type::Sum(SumType {
            variants: s.variants.iter().map(|v| substitute(v, subst)).collect(),
        }),
        _ => ty.clone(),
    }
}

/// 统一类型列表：以第一个为锚点，逐个 unify。
/// 成功返回统一后的类型（含 type_var 替换）；失败返回 None。
/// 用于 return 路径类型统一、Match arm body 类型统一。
pub fn unify_all(types: &[Type]) -> Option<Type> {
    if types.is_empty() {
        return None;
    }
    let mut subst: Substitution = HashMap::new();
    let anchor = &types[0];
    for other in &types[1..] {
        if unify(anchor, other, &mut subst).is_err() {
            return None;
        }
    }
    Some(substitute(anchor, &subst))
}

/// 统一两个类型（用于 If then/else 分支统一）。
/// 成功返回统一后的类型；失败返回 None。
pub fn unify_pair(a: &Type, b: &Type) -> Option<Type> {
    let mut subst: Substitution = HashMap::new();
    match unify(a, b, &mut subst) {
        Ok(()) => Some(substitute(a, &subst)),
        Err(_) => None,
    }
}

#[cfg(test)]
mod unify_all_tests {
    use super::*;

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType { name: name.to_string() })
    }

    fn list_int() -> Type {
        Type::GenericInstance(GenericTypeInstance {
            base: "List".to_string(),
            args: vec![prim("Int")],
        })
    }

    #[test]
    fn unify_all_empty_returns_none() {
        assert!(unify_all(&[]).is_none());
    }

    #[test]
    fn unify_all_single_returns_it() {
        let types = vec![prim("Int")];
        let result = unify_all(&types).unwrap();
        assert_eq!(result, prim("Int"));
    }

    #[test]
    fn unify_all_same_types_succeeds() {
        let types = vec![prim("Int"), prim("Int"), prim("Int")];
        let result = unify_all(&types).unwrap();
        assert_eq!(result, prim("Int"));
    }

    #[test]
    fn unify_all_conflict_returns_none() {
        let types = vec![prim("Int"), prim("String")];
        assert!(unify_all(&types).is_none());
    }

    #[test]
    fn unify_all_with_any_succeeds() {
        let types = vec![prim("Int"), Type::Any];
        let result = unify_all(&types).unwrap();
        assert_eq!(result, prim("Int"));
    }

    #[test]
    fn unify_all_generic_instances() {
        let types = vec![list_int(), list_int()];
        let result = unify_all(&types).unwrap();
        assert_eq!(result, list_int());
    }

    #[test]
    fn unify_pair_same_succeeds() {
        let result = unify_pair(&prim("Int"), &prim("Int")).unwrap();
        assert_eq!(result, prim("Int"));
    }

    #[test]
    fn unify_pair_conflict_returns_none() {
        assert!(unify_pair(&prim("Int"), &prim("String")).is_none());
    }

    #[test]
    fn unify_pair_with_any_succeeds() {
        let result = unify_pair(&prim("Int"), &Type::Any).unwrap();
        assert_eq!(result, prim("Int"));
    }

    #[test]
    fn unify_all_binds_type_var() {
        let var = Type::Var(TypeVariable { id: 0 });
        let types = vec![var, prim("Int")];
        let result = unify_all(&types).unwrap();
        assert_eq!(result, prim("Int"));
    }
}
