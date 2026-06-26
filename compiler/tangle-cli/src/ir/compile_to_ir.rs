use crate::checker::check_module::CheckedModule;
use crate::ir::graph::*;
use crate::ir::lower::lower_statements;
use crate::ir::lower_rule_flow::lower_rule_flow;
use crate::ir::lower_rule_table::lower_rule_table;
use crate::ir::lower_rule_tree::lower_rule_tree;
use crate::ir::lower_rule_toggle::lower_rule_toggle;
use crate::ir::validate::validate_ir;
use crate::model::{RuleKind, TangleDiagnostic, TangleHeading};

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
