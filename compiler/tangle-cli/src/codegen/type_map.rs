use crate::checker::types::*;

/// 将 Tangle Type 映射为 Python 类型注解字符串。
/// None 表示无注解（emitter 省略 `: ...`）。
pub fn tangle_type_to_py(ty: &Type) -> Option<String> {
    match ty {
        Type::Any => None,
        Type::Primitive(p) => Some(match p.name.as_str() {
            "Int" => "int".into(),
            "String" => "str".into(),
            "Bool" => "bool".into(),
            "Float" => "float".into(),
            other => other.into(),
        }),
        Type::Struct(s) => Some(s.name.clone()),
        Type::Interface(i) => Some(i.name.clone()),
        Type::GenericInstance(g) => Some(match g.base.as_str() {
            "List" => format!("List[{}]", inner_py(&g.args[0])),
            "Map" => format!("Dict[{}, {}]", inner_py(&g.args[0]), inner_py(&g.args[1])),
            "Option" => format!("Optional[{}]", inner_py(&g.args[0])),
            "Set" => format!("Set[{}]", inner_py(&g.args[0])),
            other => format!("{}[{}]", other, g.args.iter().map(inner_py).collect::<Vec<_>>().join(", ")),
        }),
        Type::Var(_) => None,
        Type::Function(_) => Some("Callable".into()),
        Type::Sum(s) => {
            let parts: Vec<String> = s.variants.iter().filter_map(tangle_type_to_py).collect();
            if parts.is_empty() { None } else { Some(format!("Union[{}]", parts.join(", "))) }
        }
    }
}

fn inner_py(ty: &Type) -> String {
    tangle_type_to_py(ty).unwrap_or_else(|| "Any".into())
}

/// 将 Tangle Type 映射为 Go 类型字符串。
/// Go 必须有返回类型，无注解时返回 "any"。
pub fn tangle_type_to_go(ty: &Type) -> String {
    match ty {
        Type::Any => "any".into(),
        Type::Primitive(p) => match p.name.as_str() {
            "Int" => "int".into(),
            "String" => "string".into(),
            "Bool" => "bool".into(),
            "Float" => "float64".into(),
            other => other.into(),
        },
        Type::Struct(s) => s.name.clone(),
        Type::Interface(i) => i.name.clone(),
        Type::GenericInstance(g) => match g.base.as_str() {
            "List" => format!("[]{}", inner_go(&g.args[0])),
            "Map" => format!("map[{}]{}", inner_go(&g.args[0]), inner_go(&g.args[1])),
            "Option" => format!("*{}", inner_go(&g.args[0])),
            "Set" => format!("map[{}]struct{{}}", inner_go(&g.args[0])),
            other => format!("any /* {} */", other),
        },
        Type::Var(_) => "any".into(),
        Type::Function(_) => "func()".into(),
        Type::Sum(s) => {
            s.variants.first().map(inner_go).unwrap_or_else(|| "any".into())
        }
    }
}

fn inner_go(ty: &Type) -> String {
    tangle_type_to_go(ty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn prim(name: &str) -> Type {
        Type::Primitive(PrimitiveType { name: name.into() })
    }

    fn generic(base: &str, args: Vec<Type>) -> Type {
        Type::GenericInstance(GenericTypeInstance { base: base.into(), args })
    }

    #[test]
    fn test_py_primitives() {
        assert_eq!(tangle_type_to_py(&prim("Int")), Some("int".into()));
        assert_eq!(tangle_type_to_py(&prim("String")), Some("str".into()));
        assert_eq!(tangle_type_to_py(&prim("Bool")), Some("bool".into()));
        assert_eq!(tangle_type_to_py(&prim("Float")), Some("float".into()));
    }

    #[test]
    fn test_py_any_returns_none() {
        assert_eq!(tangle_type_to_py(&Type::Any), None);
    }

    #[test]
    fn test_py_generic_list() {
        let ty = generic("List", vec![prim("Int")]);
        assert_eq!(tangle_type_to_py(&ty), Some("List[int]".into()));
    }

    #[test]
    fn test_py_generic_map() {
        let ty = generic("Map", vec![prim("String"), prim("Int")]);
        assert_eq!(tangle_type_to_py(&ty), Some("Dict[str, int]".into()));
    }

    #[test]
    fn test_py_option() {
        let ty = generic("Option", vec![prim("String")]);
        assert_eq!(tangle_type_to_py(&ty), Some("Optional[str]".into()));
    }

    #[test]
    fn test_py_struct() {
        let ty = Type::Struct(StructType { name: "Order".into(), fields: HashMap::new(), methods: HashMap::new() });
        assert_eq!(tangle_type_to_py(&ty), Some("Order".into()));
    }

    #[test]
    fn test_go_primitives() {
        assert_eq!(tangle_type_to_go(&prim("Int")), "int");
        assert_eq!(tangle_type_to_go(&prim("String")), "string");
        assert_eq!(tangle_type_to_go(&prim("Bool")), "bool");
        assert_eq!(tangle_type_to_go(&prim("Float")), "float64");
    }

    #[test]
    fn test_go_any_returns_any() {
        assert_eq!(tangle_type_to_go(&Type::Any), "any");
    }

    #[test]
    fn test_go_generic_list() {
        let ty = generic("List", vec![prim("Int")]);
        assert_eq!(tangle_type_to_go(&ty), "[]int");
    }

    #[test]
    fn test_go_generic_map() {
        let ty = generic("Map", vec![prim("String"), prim("Int")]);
        assert_eq!(tangle_type_to_go(&ty), "map[string]int");
    }

    #[test]
    fn test_go_option() {
        let ty = generic("Option", vec![prim("String")]);
        assert_eq!(tangle_type_to_go(&ty), "*string");
    }

    #[test]
    fn test_go_set() {
        let ty = generic("Set", vec![prim("Int")]);
        assert_eq!(tangle_type_to_go(&ty), "map[int]struct{}");
    }
}
