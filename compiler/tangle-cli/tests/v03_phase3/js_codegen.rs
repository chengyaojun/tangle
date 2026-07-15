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

fn make_graph_with_decision_branches() -> RuleGraph {
    RuleGraph {
        nodes: vec![
            IRNode {
                id: "entry".into(), kind: IRNodeKind::Decision, label: "check".into(),
                source_span: None, source_text: None, group: None, style: None,
            },
            IRNode {
                id: "approve".into(), kind: IRNodeKind::Action, label: "approve".into(),
                source_span: None, source_text: Some("status = \"approved\"".into()),
                group: None, style: None,
            },
            IRNode {
                id: "reject".into(), kind: IRNodeKind::Action, label: "reject".into(),
                source_span: None, source_text: Some("status = \"rejected\"".into()),
                group: None, style: None,
            },
            IRNode {
                id: "end".into(), kind: IRNodeKind::Terminal, label: "done".into(),
                source_span: None, source_text: None, group: None, style: None,
            },
        ],
        edges: vec![
            IREdge { from: "entry".into(), to: "approve".into(), kind: IREdgeKind::Condition,
                guard: Some("amount < 1000".into()), source_span: None, priority: Some(0), style: None },
            IREdge { from: "entry".into(), to: "reject".into(), kind: IREdgeKind::Condition,
                guard: Some("amount >= 1000".into()), source_span: None, priority: Some(1), style: None },
            IREdge { from: "approve".into(), to: "end".into(), kind: IREdgeKind::Control,
                guard: None, source_span: None, priority: None, style: None },
            IREdge { from: "reject".into(), to: "end".into(), kind: IREdgeKind::Control,
                guard: None, source_span: None, priority: None, style: None },
        ],
        error_edges: vec![], entry_node_id: "entry".into(),
        imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
    }
}

#[test]
fn js_decision_emits_if_else_chain() {
    let graph = make_graph_with_decision_branches();
    let js = emit_js(&graph, "DecisionTest");
    assert!(js.contains("if (amount < 1000)"), "should emit if branch, got:\n{}", js);
    assert!(js.contains("else if (amount >= 1000)"), "should emit else-if branch, got:\n{}", js);
}

#[test]
fn js_decision_priority_orders_branches() {
    let graph = make_graph_with_decision_branches();
    let js = emit_js(&graph, "DecisionTest");
    let if_pos = js.find("if (amount < 1000)").unwrap();
    let elseif_pos = js.find("else if (amount >= 1000)").unwrap();
    assert!(if_pos < elseif_pos, "priority 0 branch should come before priority 1");
}

#[test]
fn js_crossed_edge_is_skipped() {
    let mut graph = make_graph_with_decision_branches();
    // 将 reject 边改为 Crossed
    graph.edges[1].kind = IREdgeKind::Crossed;
    graph.edges[1].guard = None;
    let js = emit_js(&graph, "CrossedTest");
    assert!(js.contains("// skipped: crossed edge"), "should emit skipped comment, got:\n{}", js);
}

#[test]
fn js_decision_shared_error_terminal_emitted_in_all_branches() {
    let graph = RuleGraph {
        nodes: vec![
            IRNode { id: "entry".into(), kind: IRNodeKind::Decision, label: "check".into(),
                source_span: None, source_text: None, group: None, style: None },
            IRNode { id: "ok".into(), kind: IRNodeKind::Action, label: "ok".into(),
                source_span: None, source_text: Some("status = \"ok\"".into()),
                group: None, style: None },
            IRNode { id: "bad".into(), kind: IRNodeKind::Action, label: "bad".into(),
                source_span: None, source_text: Some("status = \"bad\"".into()),
                group: None, style: None },
            IRNode { id: "err".into(), kind: IRNodeKind::ErrorTerminal, label: "SharedError".into(),
                source_span: None, source_text: None, group: None, style: None },
        ],
        edges: vec![
            IREdge { from: "entry".into(), to: "ok".into(), kind: IREdgeKind::Condition,
                guard: Some("x > 0".into()), source_span: None, priority: Some(0), style: None },
            IREdge { from: "entry".into(), to: "bad".into(), kind: IREdgeKind::Condition,
                guard: Some("x <= 0".into()), source_span: None, priority: Some(1), style: None },
            IREdge { from: "ok".into(), to: "err".into(), kind: IREdgeKind::Control,
                guard: None, source_span: None, priority: None, style: None },
            IREdge { from: "bad".into(), to: "err".into(), kind: IREdgeKind::Control,
                guard: None, source_span: None, priority: None, style: None },
        ],
        error_edges: vec![], entry_node_id: "entry".into(),
        imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
    };

    let js = emit_js(&graph, "SharedErrTest");
    // ErrorTerminal "SharedError" should appear in BOTH branches
    let count = js.matches("return Err('SharedError')").count();
    assert_eq!(count, 2, "SharedError ErrorTerminal should be emitted in both branches, got {} occurrences:\n{}", count, js);
}
