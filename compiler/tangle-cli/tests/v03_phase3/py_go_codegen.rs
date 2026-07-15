use tangle_cli::codegen::py_emitter::emit_python;
use tangle_cli::ir::graph::*;

fn make_graph_with_metadata() -> RuleGraph {
    RuleGraph {
        nodes: vec![
            IRNode {
                id: "n1".into(), kind: IRNodeKind::Action, label: "do_work".into(),
                source_span: None, source_text: None,
                group: Some("Approval".into()), style: Some("highlight".into()),
            },
            IRNode {
                id: "n2".into(), kind: IRNodeKind::Terminal, label: "done".into(),
                source_span: None, source_text: None, group: None, style: None,
            },
        ],
        edges: vec![
            IREdge { from: "n1".into(), to: "n2".into(), kind: IREdgeKind::Dashed,
                guard: None, source_span: None, priority: None, style: Some("stroke:#f00".into()) },
        ],
        error_edges: vec![], entry_node_id: "n1".into(),
        imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
    }
}

#[test]
fn py_emits_group_style_comments() {
    let graph = make_graph_with_metadata();
    let py = emit_python(&graph, "TestModule");
    assert!(py.contains("# group: Approval"), "Python should emit group comment, got:\n{}", py);
    assert!(py.contains("# style: highlight"), "Python should emit style comment, got:\n{}", py);
}

#[test]
fn py_emits_edge_type_comments() {
    let graph = make_graph_with_metadata();
    let py = emit_python(&graph, "TestModule");
    assert!(py.contains("# edge: dashed"), "Python should emit edge type comment, got:\n{}", py);
    assert!(py.contains("# edge-style: stroke:#f00"), "Python should emit edge style comment, got:\n{}", py);
}
