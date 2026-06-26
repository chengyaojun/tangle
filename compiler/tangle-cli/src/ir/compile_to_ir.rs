use crate::checker::check_module::CheckedModule;
use crate::ir::graph::*;
use crate::ir::lower::lower_statements;
use crate::ir::validate::validate_ir;
use crate::model::TangleDiagnostic;

pub fn compile_to_ir(checked: &CheckedModule) -> (RuleGraph, Vec<TangleDiagnostic>) {
    let mut diagnostics = vec![];
    let mut id_gen = FreshNodeId::new();
    let mut merged_graph: Option<RuleGraph> = None;

    for block in &checked.parsed_blocks {
        let sub_graph = lower_statements(&block.body.statements, &checked.file, &mut id_gen);
        match &mut merged_graph {
            None => merged_graph = Some(sub_graph),
            Some(ref mut g) => {
                // Merge: append all nodes/edges from sub_graph
                g.nodes.extend(sub_graph.nodes);
                g.edges.extend(sub_graph.edges);
                g.error_edges.extend(sub_graph.error_edges);
            }
        }
    }

    let graph = merged_graph.unwrap_or_else(|| {
        let entry_id = id_gen.next();
        let mut g = create_graph(entry_id.clone());
        g.nodes.push(IRNode {
            id: entry_id.clone(), kind: IRNodeKind::Terminal,
            label: "empty".into(), source_span: None,
        });
        g
    });

    let validate_diags = validate_ir(&graph);
    diagnostics.extend(validate_diags);

    (graph, diagnostics)
}
