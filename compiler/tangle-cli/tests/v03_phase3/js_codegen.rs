use tangle_cli::codegen::js_emitter::emit_js;
use tangle_cli::ir::graph::*;

fn make_graph_with_group_style() -> RuleGraph {
    RuleGraph {
        nodes: vec![
            IRNode {
                id: "n1".into(),
                kind: IRNodeKind::Action,
                label: "do_work".into(),
                source_span: None,
                source_text: Some("let x = 1".into()),
                group: Some("Approval".into()),
                style: Some("highlight".into()),
            },
            IRNode {
                id: "n2".into(),
                kind: IRNodeKind::Terminal,
                label: "done".into(),
                source_span: None,
                source_text: None,
                group: None,
                style: None,
            },
        ],
        edges: vec![IREdge {
            from: "n1".into(),
            to: "n2".into(),
            kind: IREdgeKind::Control,
            guard: None,
            source_span: None,
            priority: None,
            style: None,
        }],
        error_edges: vec![],
        entry_node_id: "n1".into(),
        imported_stdlib: vec![],
        stdlib_imports: vec![],
        functions: vec![],
    }
}

#[test]
fn js_emits_group_comment_for_node() {
    let graph = make_graph_with_group_style();
    let js = emit_js(&graph, "TestModule");
    assert!(js.contains("// group: Approval"), "JS output should contain group comment, got:\n{}", js);
}

#[test]
fn js_emits_style_comment_for_node() {
    let graph = make_graph_with_group_style();
    let js = emit_js(&graph, "TestModule");
    assert!(js.contains("// style: highlight"), "JS output should contain style comment, got:\n{}", js);
}
