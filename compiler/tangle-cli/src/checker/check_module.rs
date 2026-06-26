use crate::ast::{ParsedCodeBlock, Stmt};
use crate::model::{TangleDiagnostic, TangleModule};
use crate::checker::types::*;
use crate::checker::env::{ReceiverContext, TypeEnv};
use crate::checker::errors::ErrorRegistry;
use crate::checker::resolve::{find_receiver_heading, resolve_types};
use crate::checker::check::check_expression;
use crate::parser::lexer::tokenize;
use crate::parser::parser::parse_code_body;

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
                            .and_then(|tn| match tn.as_str() {
                                "String" => Some(Type::Primitive(PrimitiveType { name: "String".into() })),
                                "Int" => Some(Type::Primitive(PrimitiveType { name: "Int".into() })),
                                "Bool" => Some(Type::Primitive(PrimitiveType { name: "Bool".into() })),
                                _ => Some(Type::Primitive(PrimitiveType { name: tn.clone() })),
                            })
                            .unwrap_or(Type::Primitive(PrimitiveType { name: "String".into() }));
                        (p.name.clone(), ty)
                    }).collect();
                ReceiverContext { struct_name, fields }
            });

        let mut block_env = type_env.clone();
        block_env.receiver = receiver;

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
