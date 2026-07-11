use crate::ast::{ParsedCodeBlock, Stmt};
use crate::model::{TangleDiagnostic, TangleImport, TangleModule};
use crate::checker::types::*;
use crate::checker::env::{ReceiverContext, TypeEnv};
use crate::checker::errors::ErrorRegistry;
use crate::checker::resolve::{find_receiver_heading, resolve_types};
use crate::checker::check::check_expression;
use crate::parser::lexer::tokenize;
use crate::parser::parser::parse_code_body;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CheckedModule {
    pub file: String,
    pub module_name: String,
    pub imports: Vec<crate::model::TangleImport>,
    pub headings: Vec<crate::model::TangleHeading>,
    pub symbols: Vec<crate::model::TangleSymbol>,
    pub diagnostics: Vec<TangleDiagnostic>,
    pub parsed_blocks: Vec<ParsedCodeBlock>,
    pub type_env: TypeEnv,
}

/// Parse all @tangle code blocks in the module
pub fn parse_code_blocks(module: &TangleModule) -> Vec<ParsedCodeBlock> {
    let mut blocks = vec![];
    collect_code_blocks(&module.headings, &module.file, &mut blocks);
    blocks
}

fn collect_code_blocks(headings: &[crate::model::TangleHeading], file: &str, out: &mut Vec<ParsedCodeBlock>) {
    for h in headings {
        for cb in &h.code_blocks {
            let (tokens, lexer_diags) = tokenize(&cb.value, file);
            let (body, mut parse_diags) = parse_code_body(&tokens);
            let mut all_diags = lexer_diags;
            all_diags.append(&mut parse_diags);
            out.push(ParsedCodeBlock {
                heading_id: h.id.clone(),
                source: cb.value.clone(),
                body,
                diagnostics: all_diags,
            });
        }
        collect_code_blocks(&h.children, file, out);
    }
}

fn is_stdlib_import(target: &str) -> bool {
    !target.contains('/') && !target.contains('\\') && !target.starts_with('.')
}

fn stdlib_ops(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "fmt" => Some(&["print", "println", "input", "debug", "error", "format"]),
        "IO"  => Some(&["readFile", "writeFile", "exists", "stat", "mkdir", "read_dir", "remove", "rename", "copy", "chmod", "size", "is_dir", "is_file"]),
        "List" => Some(&["length", "map", "filter", "push", "get"]),
        "Map"  => Some(&["get", "set", "has", "keys", "values", "delete"]),
        "Set"  => Some(&["add", "remove", "contains", "size", "union", "intersection", "difference", "to_list"]),
        "Option" => Some(&["Some", "None", "unwrap", "is_some", "is_none", "map", "or_else"]),
        "Math" => Some(&["abs", "min", "max", "floor", "ceil", "round", "sqrt", "pow"]),
        "String" => Some(&["length", "concat", "split", "replace", "to_upper", "to_lower", "trim", "contains"]),
        "Env"  => Some(&["get", "set", "remove", "args", "current_dir", "exit"]),
        "Path" => Some(&["join", "basename", "dirname", "extension", "is_absolute", "normalize", "relative", "split"]),
        _ => None,
    }
}

fn resolve_stdlib_imports(imports: &[TangleImport], env: &mut TypeEnv) {
    for imp in imports {
        if !is_stdlib_import(&imp.target) { continue; }
        if let Some(operations) = stdlib_ops(&imp.target) {
            // Split comma-separated aliases: [print, println](fmt)
            let aliases: Vec<&str> = imp.alias.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
            let fn_import = aliases.len() > 1
                || (aliases.len() == 1 && operations.contains(&aliases[0]));

            if fn_import {
                // Function import: [println](fmt) -> inject each as variable
                for alias in &aliases {
                    if operations.contains(alias) {
                        env.variables.insert(alias.to_string(), Type::Function(FunctionType {
                            params: vec![],
                            returns: Box::new(Type::Primitive(PrimitiveType { name: "String".into() })),
                        }));
                    }
                }
            } else {
                // Module import: [fmt](fmt) -> inject struct type
                let methods: HashMap<String, CallableSignature> = operations.iter().map(|op| {
                    (op.to_string(), CallableSignature {
                        params: vec![],
                        returns: Type::Primitive(PrimitiveType { name: "String".into() }),
                    })
                }).collect();
                env.structs.insert(imp.alias.clone(), Type::Struct(StructType {
                    name: imp.target.clone(),
                    fields: HashMap::new(),
                    methods,
                }));
            }
        }
    }
}

/// Main type-checking orchestrator
pub fn check_module(module: TangleModule) -> CheckedModule {
    let mut diagnostics = module.diagnostics.clone();
    let parsed_blocks = parse_code_blocks(&module);
    let (base_env, mut resolve_diags) = resolve_types(&module);
    diagnostics.append(&mut resolve_diags);

    let mut error_registry = ErrorRegistry::new();
    error_registry.collect_from_headings(&module.headings);

    let mut type_env = base_env;
    type_env.error_registry = Some(error_registry);

    // Resolve stdlib imports: [fmt](fmt) -> inject struct type
    resolve_stdlib_imports(&module.imports, &mut type_env);

    // Type-check each code block
    for block in &parsed_blocks {
        let mut block_diags = block.diagnostics.clone();
        diagnostics.append(&mut block_diags);

        // Find receiver context (implicit this binding)
        let receiver = find_receiver_heading(&block.heading_id, &module.headings)
            .map(|parent_heading| {
                let struct_name = parent_heading.symbol_name.clone()
                    .unwrap_or_else(|| parent_heading.title.clone());
                let fields: std::collections::HashMap<String, Type> = parent_heading.params.iter()
                    .map(|p| {
                        let ty = p.type_name.as_ref()
                            .map(|tn| match tn.as_str() {
                                "String" => Type::Primitive(PrimitiveType { name: "String".into() }),
                                "Int" => Type::Primitive(PrimitiveType { name: "Int".into() }),
                                "Bool" => Type::Primitive(PrimitiveType { name: "Bool".into() }),
                                _ => Type::Primitive(PrimitiveType { name: tn.clone() }),
                            })
                            .unwrap_or(Type::Primitive(PrimitiveType { name: "String".into() }));
                        (p.name.clone(), ty)
                    }).collect();
                ReceiverContext { struct_name, fields }
            });

        let mut block_env = type_env.clone();
        block_env.receiver = receiver;

        // Inject current heading's params into local scope so method bodies
        // can reference their declared parameters.
        // Find the heading that owns this code block by heading_id.
        if let Some(owner) = find_heading_by_id(&block.heading_id, &module.headings) {
            for p in &owner.params {
                let ty = p.type_name.as_ref()
                    .map(|tn| match tn.as_str() {
                        "String" => Type::Primitive(PrimitiveType { name: "String".into() }),
                        "Int" => Type::Primitive(PrimitiveType { name: "Int".into() }),
                        "Bool" => Type::Primitive(PrimitiveType { name: "Bool".into() }),
                        _ => Type::Primitive(PrimitiveType { name: tn.clone() }),
                    })
                    .unwrap_or(Type::Primitive(PrimitiveType { name: "String".into() }));
                block_env.variables.insert(p.name.clone(), ty);
            }
        }

        for stmt in &block.body.statements {
            match stmt {
                Stmt::Let(s) => {
                    let (val_ty, mut val_diags) = check_expression(&s.value, &block_env);
                    diagnostics.append(&mut val_diags);
                    block_env.variables.insert(s.name.clone(), val_ty);
                }
                Stmt::Const(s) => {
                    let (val_ty, mut val_diags) = check_expression(&s.value, &block_env);
                    diagnostics.append(&mut val_diags);
                    block_env.variables.insert(s.name.clone(), val_ty);
                }
                Stmt::Return(s) => {
                    if let Some(ref value) = s.value {
                        let (_, mut val_diags) = check_expression(value, &block_env);
                        diagnostics.append(&mut val_diags);
                    }
                }
                Stmt::Expression(s) => {
                    let (_, mut expr_diags) = check_expression(&s.expr, &block_env);
                    diagnostics.append(&mut expr_diags);
                }
            }
        }
    }

    CheckedModule {
        file: module.file,
        module_name: module.module_name,
        imports: module.imports,
        headings: module.headings,
        symbols: module.symbols,
        diagnostics,
        parsed_blocks,
        type_env,
    }
}

/// Find a heading by id, recursively searching the heading tree.
fn find_heading_by_id<'a>(target_id: &str, headings: &'a [crate::model::TangleHeading]) -> Option<&'a crate::model::TangleHeading> {
    for h in headings {
        if h.id == target_id {
            return Some(h);
        }
        if let Some(found) = find_heading_by_id(target_id, &h.children) {
            return Some(found);
        }
    }
    None
}
