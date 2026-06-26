use crate::ir::graph::*;

pub fn lower_rule_flow(_mermaid_source: &str, _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph {
    let entry_id = id_gen.next();
    let mut graph = create_graph(entry_id.clone());

    graph.nodes.push(IRNode {
        id: entry_id.clone(), kind: IRNodeKind::Decision,
        label: "rule.flow entry".into(), source_span: None,
    });

    let terminal_id = id_gen.next();
    graph.nodes.push(IRNode {
        id: terminal_id.clone(), kind: IRNodeKind::Terminal,
        label: "rule.flow exit".into(), source_span: None,
    });

    graph.edges.push(IREdge {
        from: entry_id, to: terminal_id, kind: IREdgeKind::Control,
        guard: None, source_span: None,
    });

    graph
}
