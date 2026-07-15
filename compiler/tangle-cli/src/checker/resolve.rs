use crate::checker::env::TypeEnv;
use crate::checker::types::*;
use crate::model::{HeadingRole, TangleDiagnostic, TangleHeading, TangleModule};
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

fn type_name_to_type(name: &str) -> Option<Type> {
    match name {
        "String" => Some(Type::Primitive(PrimitiveType {
            name: "String".into(),
        })),
        "Int" => Some(Type::Primitive(PrimitiveType {
            name: "Int".into(),
        })),
        "Bool" => Some(Type::Primitive(PrimitiveType {
            name: "Bool".into(),
        })),
        _ => Some(Type::Primitive(PrimitiveType {
            name: name.into(),
        })),
    }
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
