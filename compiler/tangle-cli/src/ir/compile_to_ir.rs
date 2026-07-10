use crate::ast::{ParsedCodeBlock, Stmt};
use crate::checker::check_module::CheckedModule;
use crate::ir::graph::*;
use crate::ir::lower::lower_statements;
use crate::ir::lower::stmt_source;
use crate::ir::lower_rule_flow::lower_rule_flow;
use crate::ir::lower_rule_table::lower_rule_table;
use crate::ir::lower_rule_tree::lower_rule_tree;
use crate::ir::lower_rule_toggle::lower_rule_toggle;
use crate::ir::validate::validate_ir;
use crate::model::{HeadingRole, RuleKind, TangleDiagnostic, TangleHeading};

pub fn compile_to_ir(checked: &CheckedModule) -> (RuleGraph, Vec<TangleDiagnostic>) {
    let mut diagnostics = vec![];
    let mut id_gen = FreshNodeId::new();
    let mut merged_graph: Option<RuleGraph> = None;

    // Lower @tangle code blocks as statements
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

    // Lower rule blocks from headings
    let mut rule_graphs: Vec<RuleGraph> = vec![];
    collect_rule_graphs(&checked.headings, &checked.file, &mut id_gen, &mut rule_graphs);
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

    let mut graph = merged_graph.unwrap_or_else(|| {
        let entry_id = id_gen.next();
        let mut g = create_graph(entry_id.clone());
        g.nodes.push(IRNode {
            id: entry_id.clone(), kind: IRNodeKind::Terminal,
            label: "empty".into(), source_span: None, source_text: None,
        });
        g
    });

    // Build heading-defined functions (one per Callable heading with code blocks).
    // Multi-function emission is only activated when a `main` entry point exists;
    // otherwise the single merged function (module-named) is the fallback, which
    // correctly handles modules with only loose blocks or non-main callables.
    let mut functions: Vec<IRFunction> = vec![];
    collect_functions(&checked.headings, None, &checked.parsed_blocks, &mut id_gen, &mut functions);
    if functions.iter().any(|f| f.name == "main") {
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
) {
    for h in headings {
        if let Some(ref rule) = h.rule {
            let sub_graph = match rule.kind {
                RuleKind::Flow => lower_rule_flow(&rule.source, file, id_gen),
                RuleKind::Table => lower_rule_table(&rule.source, file, id_gen),
                RuleKind::Tree => lower_rule_tree(&rule.source, file, id_gen),
                RuleKind::Toggle => lower_rule_toggle(&rule.source, file, id_gen),
            };
            out.push(sub_graph);
        }
        collect_rule_graphs(&h.children, file, id_gen, out);
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
                let params: Vec<String> = h.params.iter().map(|p| p.name.clone()).collect();
                let blocks: Vec<&ParsedCodeBlock> = parsed_blocks.iter()
                    .filter(|b| b.heading_id == h.id)
                    .collect();
                let (nodes, edges, entry_id, error_edges) = lower_function_body(&blocks, id_gen);
                out.push(IRFunction {
                    name: name.clone(),
                    receiver,
                    params,
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
    let entry_id = id_gen.next();
    let mut nodes: Vec<IRNode> = vec![IRNode {
        id: entry_id.clone(), kind: IRNodeKind::Compute,
        label: "entry".into(), source_span: None, source_text: None,
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
            let node_id = id_gen.next();
            nodes.push(IRNode {
                id: node_id.clone(), kind: node_kind, label,
                source_span: None, source_text: Some(src),
            });
            edges.push(IREdge {
                from: prev_id, to: node_id.clone(), kind: IREdgeKind::Control,
                guard: None, source_span: None,
            });
            prev_id = node_id;
        }
    }

    let terminal_id = id_gen.next();
    nodes.push(IRNode {
        id: terminal_id.clone(), kind: IRNodeKind::Terminal,
        label: "exit".into(), source_span: None, source_text: None,
    });
    edges.push(IREdge {
        from: prev_id, to: terminal_id, kind: IREdgeKind::Control,
        guard: None, source_span: None,
    });

    (nodes, edges, entry_id, vec![])
}
