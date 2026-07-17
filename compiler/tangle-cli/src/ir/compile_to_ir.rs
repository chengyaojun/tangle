use crate::ast::{ParsedCodeBlock, Stmt};
use crate::checker::check_module::CheckedModule;
use crate::checker::resolve::type_name_to_type;
use crate::ir::graph::*;
use crate::ir::lower::lower_statements;
use crate::ir::lower::stmt_source;
use crate::ir::lower_rule_flow::lower_rule_flow;
use crate::ir::lower_rule_table::lower_rule_table_with_diagnostics;
use crate::ir::lower_rule_tree::lower_rule_tree;
use crate::ir::lower_rule_toggle::lower_rule_toggle;
use crate::ir::validate::validate_ir;
use crate::model::{HeadingRole, RuleKind, TangleDiagnostic, TangleHeading};

pub fn compile_to_ir(checked: &CheckedModule) -> (RuleGraph, Vec<TangleDiagnostic>) {
    let mut diagnostics = vec![];
    let mut id_gen = FreshNodeId::new();
    let mut merged_graph: Option<RuleGraph> = None;

    // Multi-function mode: a `main` Callable heading turns the module into a
    // collection of functions. @tangle blocks then live inside `functions[]`
    // only and must NOT also be merged into the top-level graph (dual-entry fix
    // A1-1). Without `main`, the fallback single-function mode merges blocks at
    // the top level.
    let has_main = has_main_callable(&checked.headings);

    // Lower @tangle code blocks as statements (fallback mode only).
    if !has_main {
        for block in &checked.parsed_blocks {
            let sub_graph = lower_statements(&block.body.statements, &block.source, &checked.file, &mut id_gen);
            match &mut merged_graph {
                None => merged_graph = Some(sub_graph),
                Some(ref mut g) => {
                    g.nodes.extend(sub_graph.nodes);
                    g.edges.extend(sub_graph.edges);
                    g.error_edges.extend(sub_graph.error_edges);
                }
            }
        }
    }

    // Lower rule blocks from headings
    let mut rule_graphs: Vec<RuleGraph> = vec![];
    collect_rule_graphs(&checked.headings, &checked.file, &mut id_gen, &mut rule_graphs, &mut diagnostics);
    for sub_graph in rule_graphs {
        match &mut merged_graph {
            None => merged_graph = Some(sub_graph),
            Some(ref mut g) => {
                g.nodes.extend(sub_graph.nodes);
                g.edges.extend(sub_graph.edges);
                g.error_edges.extend(sub_graph.error_edges);
            }
        }
    }

    // Build the top-level graph. In multi-function mode the top-level holds
    // only rule graphs (if any); do NOT synthesize an "empty" placeholder node
    // — `functions[]` carries all @tangle content and the shell must stay empty.
    // In fallback mode, preserve the existing "empty" terminal placeholder when
    // nothing was merged.
    let mut graph = if has_main {
        merged_graph.unwrap_or_else(|| create_graph(id_gen.fresh()))
    } else {
        merged_graph.unwrap_or_else(|| {
            let entry_id = id_gen.fresh();
            let mut g = create_graph(entry_id.clone());
            g.nodes.push(IRNode {
                id: entry_id.clone(), kind: IRNodeKind::Terminal,
                label: "empty".into(), source_span: None, source_text: None,
                group: None, style: None,
            });
            g
        })
    };

    // Build heading-defined functions (multi-function mode only). `has_main`
    // already mirrors the condition under which `collect_functions` would emit a
    // `main` entry, so the top-level / functions[] split stays consistent.
    if has_main {
        let mut functions: Vec<IRFunction> = vec![];
        collect_functions(&checked.headings, None, &checked.parsed_blocks, &mut id_gen, &mut functions);
        graph.functions = functions;
    }

    // Collect stdlib import names and alias mappings
    graph.stdlib_imports = checked.imports.iter()
        .filter(|imp| !imp.target.contains('/') && !imp.target.contains('\\') && !imp.target.starts_with('.'))
        .map(|imp| (imp.alias.clone(), imp.target.clone()))
        .collect();
    graph.imported_stdlib = graph.stdlib_imports.iter()
        .map(|(_, target)| target.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let validate_diags = validate_ir(&graph);
    diagnostics.extend(validate_diags);

    (graph, diagnostics)
}

/// Recursively collect rule subgraphs from headings
fn collect_rule_graphs(
    headings: &[TangleHeading],
    file: &str,
    id_gen: &mut FreshNodeId,
    out: &mut Vec<RuleGraph>,
    diagnostics: &mut Vec<TangleDiagnostic>,
) {
    for h in headings {
        if let Some(ref rule) = h.rule {
            let (sub_graph, rule_diags) = match rule.kind {
                RuleKind::Flow => {
                    (lower_rule_flow(&rule.source, file, id_gen), vec![])
                }
                RuleKind::Table => lower_rule_table_with_diagnostics(&rule.source, file, id_gen),
                RuleKind::Tree => lower_rule_tree(&rule.source, file, id_gen),
                RuleKind::Toggle => lower_rule_toggle(&rule.source, file, id_gen),
            };
            out.push(sub_graph);
            diagnostics.extend(rule_diags);
        }
        collect_rule_graphs(&h.children, file, id_gen, out, diagnostics);
    }
}

/// Walk the heading tree and build an `IRFunction` for each Callable heading
/// (depth 4) that has `@tangle` code blocks. `parent` determines the receiver:
/// a Callable under a Type heading (e.g. `#### create` under `### Order`) becomes
/// a method `Order.create`; `main` and free callables get `receiver = None`.
fn collect_functions(
    headings: &[TangleHeading],
    parent: Option<&TangleHeading>,
    parsed_blocks: &[ParsedCodeBlock],
    id_gen: &mut FreshNodeId,
    out: &mut Vec<IRFunction>,
) {
    for h in headings {
        if h.role == HeadingRole::Callable && !h.code_blocks.is_empty() {
            if let Some(ref name) = h.symbol_name {
                let receiver = if name != "main" {
                    parent.and_then(|p| {
                        if p.role == HeadingRole::Type { p.symbol_name.clone() } else { None }
                    })
                } else {
                    None
                };
                let params: Vec<IRParam> = h.params.iter().map(|p| IRParam {
                    name: p.name.clone(),
                    type_: p.type_name.as_ref().and_then(|tn| type_name_to_type(tn)),
                }).collect();
                let blocks: Vec<&ParsedCodeBlock> = parsed_blocks.iter()
                    .filter(|b| b.heading_id == h.id)
                    .collect();
                let (nodes, edges, entry_id, error_edges) = lower_function_body(&blocks, id_gen);
                out.push(IRFunction {
                    name: name.clone(),
                    receiver,
                    params,
                    return_type: None,
                    nodes,
                    edges,
                    entry_node_id: entry_id,
                    error_edges,
                });
            }
        }
        collect_functions(&h.children, Some(h), parsed_blocks, id_gen, out);
    }
}

/// Lower a function body from its parsed code blocks into IR nodes/edges.
/// Chains statements across multiple blocks sequentially (entry → stmts → terminal).
fn lower_function_body(
    blocks: &[&ParsedCodeBlock],
    id_gen: &mut FreshNodeId,
) -> (Vec<IRNode>, Vec<IREdge>, String, Vec<IRErrorEdge>) {
    let entry_id = id_gen.fresh();
    let mut nodes: Vec<IRNode> = vec![IRNode {
        id: entry_id.clone(), kind: IRNodeKind::Compute,
        label: "entry".into(), source_span: None, source_text: None,
        group: None, style: None,
    }];
    let mut edges: Vec<IREdge> = vec![];
    let mut prev_id = entry_id.clone();

    for block in blocks {
        for stmt in &block.body.statements {
            let (node_kind, label) = match stmt {
                Stmt::Return(_) => (IRNodeKind::Action, "return".to_string()),
                Stmt::Let(s) => (IRNodeKind::Compute, format!("let {}", s.name)),
                Stmt::Const(s) => (IRNodeKind::Compute, format!("const {}", s.name)),
                Stmt::Expression(_) => (IRNodeKind::Action, "expr".to_string()),
            };
            let src = stmt_source(stmt, &block.source);
            let node_id = id_gen.fresh();
            nodes.push(IRNode {
                id: node_id.clone(), kind: node_kind, label,
                source_span: None, source_text: Some(src),
                group: None, style: None,
            });
            edges.push(IREdge {
                from: prev_id, to: node_id.clone(), kind: IREdgeKind::Control,
                guard: None, source_span: None,
                priority: None, style: None,
            });
            prev_id = node_id;
        }
    }

    let terminal_id = id_gen.fresh();
    nodes.push(IRNode {
        id: terminal_id.clone(), kind: IRNodeKind::Terminal,
        label: "exit".into(), source_span: None, source_text: None,
        group: None, style: None,
    });
    edges.push(IREdge {
        from: prev_id, to: terminal_id, kind: IREdgeKind::Control,
        guard: None, source_span: None,
        priority: None, style: None,
    });

    (nodes, edges, entry_id, vec![])
}

/// Check whether the module has a `main` Callable heading that owns `@tangle`
/// code blocks. This enables multi-function mode: `@tangle` blocks live inside
/// `functions[]` only and the top-level graph stays clear of them.
///
/// The predicate mirrors `collect_functions` exactly (Callable role, non-empty
/// `code_blocks`, `symbol_name == "main"`), recursing into child headings, so the
/// decision is consistent with whatever `collect_functions` would emit.
fn has_main_callable(headings: &[TangleHeading]) -> bool {
    for h in headings {
        if h.role == HeadingRole::Callable
            && !h.code_blocks.is_empty()
            && h.symbol_name.as_deref() == Some("main")
        {
            return true;
        }
        if has_main_callable(&h.children) {
            return true;
        }
    }
    false
}
