use crate::ast::{
    FunctionTypeExpr, GenericTypeExpr, NamedTypeExpr, PrimitiveTypeExpr, SumTypeExpr, TypeExpr,
};
use crate::checker::env::TypeEnv;
use crate::checker::types::*;
use crate::model::{HeadingRole, TangleDiagnostic, TangleHeading, TangleModule};
use crate::parser::type_parser::parse_type_expr;
use std::collections::HashMap;

pub fn resolve_types(module: &TangleModule) -> (TypeEnv, Vec<TangleDiagnostic>) {
    let diagnostics = vec![];
    let mut env = TypeEnv::new();
    collect_types_recursive(&module.headings, &mut env);

    // Pass 2: Collect top-level Callables (not children of any Type heading)
    for heading in flatten_headings(&module.headings) {
        if heading.role == HeadingRole::Callable
            && !is_child_of_type_heading(&heading.id, &module.headings)
        {
            if let Some(ref name) = heading.symbol_name {
                let params: Vec<Type> = heading
                    .params
                    .iter()
                    .map(|p| {
                        p.type_name
                            .as_ref()
                            .and_then(|tn| type_name_to_type(tn))
                            .unwrap_or(Type::Any)
                    })
                    .collect();
                env.functions.insert(
                    name.clone(),
                    FunctionType {
                        params,
                        returns: Box::new(Type::Any),
                        is_variadic: false,
                    },
                );
            }
        }
    }

    (env, diagnostics)
}

fn collect_types_recursive(headings: &[TangleHeading], env: &mut TypeEnv) {
    for heading in headings {
        if heading.role == HeadingRole::Type {
            let name = heading
                .symbol_name
                .clone()
                .unwrap_or_else(|| heading.title.clone());
            let is_interface = heading.title.contains("接口") || heading.title.contains("interface");

            if is_interface {
                let methods = collect_method_sigs(&heading.children);
                env.interfaces.insert(
                    name.clone(),
                    Type::Interface(InterfaceType { name, methods }),
                );
            } else {
                let fields: HashMap<String, Type> = heading
                    .params
                    .iter()
                    .map(|p| {
                        let ty = p
                            .type_name
                            .as_ref()
                            .and_then(|tn| type_name_to_type(tn))
                            .unwrap_or(Type::Primitive(PrimitiveType {
                                name: "String".into(),
                            }));
                        (p.name.clone(), ty)
                    })
                    .collect();

                let methods = collect_method_sigs(&heading.children);

                env.structs.insert(
                    name.clone(),
                    Type::Struct(StructType {
                        name,
                        fields,
                        methods,
                    }),
                );
            }
        }
        // Recurse into children regardless of this heading's role —
        // a Type heading may be nested under a Program or Section heading.
        collect_types_recursive(&heading.children, env);
    }
}

fn collect_method_sigs(children: &[TangleHeading]) -> HashMap<String, CallableSignature> {
    let mut methods = HashMap::new();
    for child in children {
        if child.role == HeadingRole::Callable {
            if let Some(ref name) = child.symbol_name {
                let params: Vec<(String, Type)> = child
                    .params
                    .iter()
                    .map(|p| {
                        let ty = p
                            .type_name
                            .as_ref()
                            .and_then(|tn| type_name_to_type(tn))
                            .unwrap_or(Type::Primitive(PrimitiveType {
                                name: "String".into(),
                            }));
                        (p.name.clone(), ty)
                    })
                    .collect();
                methods.insert(
                    name.clone(),
                    CallableSignature {
                        params,
                        returns: Type::Any,
                        is_variadic: false,
                    },
                );
            }
        }
    }
    methods
}

/// 将 TypeExpr（语法树）转换为 Type（语义类型）。
/// 镜像 TS 端 reference/src/checker/resolve.ts:typeExprToType。
pub fn type_expr_to_type(te: &TypeExpr) -> Type {
    match te {
        TypeExpr::Primitive(PrimitiveTypeExpr { name, .. }) => {
            Type::Primitive(PrimitiveType { name: name.clone() })
        }
        TypeExpr::Named(NamedTypeExpr { name, .. }) => {
            // 用户定义类型名 → Struct（字段/方法在 resolve_types 中填充）
            Type::Struct(StructType {
                name: name.clone(),
                fields: HashMap::new(),
                methods: HashMap::new(),
            })
        }
        TypeExpr::Sum(SumTypeExpr { variants, .. }) => Type::Sum(SumType {
            variants: variants.iter().map(type_expr_to_type).collect(),
        }),
        TypeExpr::Generic(GenericTypeExpr {
            base, type_args, ..
        }) => Type::GenericInstance(GenericTypeInstance {
            base: base.clone(),
            args: type_args.iter().map(type_expr_to_type).collect(),
        }),
        TypeExpr::Function(FunctionTypeExpr { params, returns, .. }) => {
            Type::Function(FunctionType {
                params: params.iter().map(type_expr_to_type).collect(),
                returns: Box::new(type_expr_to_type(returns)),
                is_variadic: false,
            })
        }
    }
}

/// 解析 Tangle 类型注解字符串（如 "List<Int>"、"Order"、"String"）为 Type。
/// 使用 type_parser 解析泛型语法，解析失败返回 None。
/// 调用方（resolve_types）用 .unwrap_or(Type::Any) 处理 None。
pub fn type_name_to_type(name: &str) -> Option<Type> {
    let (te, diags) = parse_type_expr(name, "");
    if !diags.is_empty() {
        return None;
    }
    te.map(|te| type_expr_to_type(&te))
}

/// Find parent type heading for a given heading id (implicit this binding)
pub fn find_receiver_heading<'a>(
    heading_id: &str,
    headings: &'a [TangleHeading],
) -> Option<&'a TangleHeading> {
    find_receiver_recursive(heading_id, headings)
}

fn find_receiver_recursive<'a>(
    target_id: &str,
    headings: &'a [TangleHeading],
) -> Option<&'a TangleHeading> {
    for h in headings {
        if h.role == HeadingRole::Type && h.children.iter().any(|c| c.id == target_id) {
            return Some(h);
        }
        if let Some(found) = find_receiver_recursive(target_id, &h.children) {
            return Some(found);
        }
    }
    None
}

/// Flatten the heading tree into a flat list (depth-first).
fn flatten_headings(headings: &[TangleHeading]) -> Vec<&TangleHeading> {
    let mut result = vec![];
    for h in headings {
        result.push(h);
        result.extend(flatten_headings(&h.children));
    }
    result
}

/// Check if a heading (by id) is a direct child of a Type heading.
fn is_child_of_type_heading(heading_id: &str, headings: &[TangleHeading]) -> bool {
    for h in headings {
        if h.role == HeadingRole::Type
            && h.children.iter().any(|c| c.id == heading_id)
        {
            return true;
        }
        if is_child_of_type_heading(heading_id, &h.children) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod type_name_tests {
    use super::*;

    #[test]
    fn test_basic_types() {
        assert!(matches!(type_name_to_type("Int").unwrap(), Type::Primitive(_)));
        assert!(matches!(type_name_to_type("String").unwrap(), Type::Primitive(_)));
        assert!(matches!(type_name_to_type("Bool").unwrap(), Type::Primitive(_)));
    }

    #[test]
    fn test_generic_list() {
        let ty = type_name_to_type("List<Int>").unwrap();
        match ty {
            Type::GenericInstance(g) => {
                assert_eq!(g.base, "List");
                assert_eq!(g.args.len(), 1);
                match &g.args[0] {
                    Type::Primitive(p) => assert_eq!(p.name, "Int"),
                    other => panic!("expected Primitive, got {:?}", other),
                }
            }
            other => panic!("expected GenericInstance, got {:?}", other),
        }
    }

    #[test]
    fn test_generic_map_two_args() {
        let ty = type_name_to_type("Map<String, Int>").unwrap();
        match ty {
            Type::GenericInstance(g) => {
                assert_eq!(g.base, "Map");
                assert_eq!(g.args.len(), 2);
            }
            other => panic!("expected GenericInstance, got {:?}", other),
        }
    }

    #[test]
    fn test_named_type_becomes_struct() {
        let ty = type_name_to_type("Order").unwrap();
        assert!(matches!(ty, Type::Struct(_)));
    }

    #[test]
    fn test_option_nested() {
        let ty = type_name_to_type("Option<String>").unwrap();
        match ty {
            Type::GenericInstance(g) => {
                assert_eq!(g.base, "Option");
                assert_eq!(g.args.len(), 1);
            }
            other => panic!("expected GenericInstance, got {:?}", other),
        }
    }
}
