use tangle_cli::codegen::py_emitter::emit_python;
use tangle_cli::codegen::go_emitter::emit_go;
use tangle_cli::ir::graph::*;

fn make_decision_graph() -> RuleGraph {
    RuleGraph {
        nodes: vec![
            IRNode { id: "entry".into(), kind: IRNodeKind::Decision, label: "check".into(),
                source_span: None, source_text: None, group: None, style: None },
            IRNode { id: "auto".into(), kind: IRNodeKind::Action, label: "auto_approve".into(),
                source_span: None, source_text: None, group: None, style: None },
            IRNode { id: "manual".into(), kind: IRNodeKind::Action, label: "manual_review".into(),
                source_span: None, source_text: None, group: None, style: None },
            IRNode { id: "end".into(), kind: IRNodeKind::Terminal, label: "done".into(),
                source_span: None, source_text: None, group: None, style: None },
        ],
        edges: vec![
            IREdge { from: "entry".into(), to: "auto".into(), kind: IREdgeKind::Condition,
                guard: Some("amount < 100".into()), source_span: None,
                priority: Some(0), style: None },
            IREdge { from: "entry".into(), to: "manual".into(), kind: IREdgeKind::Condition,
                guard: Some("amount < 1000".into()), source_span: None,
                priority: Some(1), style: None },
            IREdge { from: "auto".into(), to: "end".into(), kind: IREdgeKind::Dashed,
                guard: None, source_span: None, priority: None, style: None },
            IREdge { from: "manual".into(), to: "end".into(), kind: IREdgeKind::Thick,
                guard: None, source_span: None, priority: None, style: None },
        ],
        error_edges: vec![], entry_node_id: "entry".into(),
        imported_stdlib: vec![], stdlib_imports: vec![], functions: vec![],
    }
}

#[test]
fn py_decision_if_elif_emission() {
    let graph = make_decision_graph();
    let py = emit_python(&graph, "ApprovalFlow");
    assert!(py.contains("if (amount < 100):"), "Py should emit if with guard, got:\n{}", py);
    assert!(py.contains("elif (amount < 1000):"), "Py should emit elif, got:\n{}", py);
}

#[test]
fn py_priority_ordering() {
    let graph = make_decision_graph();
    let py = emit_python(&graph, "ApprovalFlow");
    let if_pos = py.find("if (amount < 100)").unwrap();
    let elif_pos = py.find("elif (amount < 1000)").unwrap();
    assert!(if_pos < elif_pos, "priority 0 guard should come before priority 1");
}

#[test]
fn py_branch_body_recursion() {
    let graph = make_decision_graph();
    let py = emit_python(&graph, "ApprovalFlow");
    assert!(py.contains("# action: auto_approve"), "Py branch body should recurse, got:\n{}", py);
    assert!(py.contains("# action: manual_review"), "Py branch body should recurse");
}

#[test]
fn go_decision_if_else_emission() {
    let graph = make_decision_graph();
    let go = emit_go(&graph, "ApprovalFlow");
    assert!(go.contains("if amount < 100 {"), "Go should emit if with guard, got:\n{}", go);
    assert!(go.contains("} else if amount < 1000 {"), "Go should emit }} else if, got:\n{}", go);
}

#[test]
fn go_priority_ordering() {
    let graph = make_decision_graph();
    let go = emit_go(&graph, "ApprovalFlow");
    let if_pos = go.find("if amount < 100 {").unwrap();
    let else_if_pos = go.find("} else if amount < 1000 {").unwrap();
    assert!(if_pos < else_if_pos, "priority 0 guard should come before priority 1");
}

#[test]
fn go_branch_body_recursion() {
    let graph = make_decision_graph();
    let go = emit_go(&graph, "ApprovalFlow");
    assert!(go.contains("// action: auto_approve"), "Go branch body should recurse, got:\n{}", go);
    assert!(go.contains("// action: manual_review"), "Go branch body should recurse");
}

#[test]
fn py_go_crossed_edge_skipped() {
    let mut graph = make_decision_graph();
    graph.nodes.push(IRNode {
        id: "reject".into(), kind: IRNodeKind::Action, label: "reject".into(),
        source_span: None, source_text: None, group: None, style: None,
    });
    graph.edges.push(IREdge {
        from: "entry".into(), to: "reject".into(), kind: IREdgeKind::Crossed,
        guard: None, source_span: None, priority: None, style: None,
    });
    let py = emit_python(&graph, "TestFlow");
    let go = emit_go(&graph, "TestFlow");
    assert!(py.contains("# skipped: crossed edge to reject"), "Py should emit crossed skip comment, got:\n{}", py);
    assert!(go.contains("// skipped: crossed edge to reject"), "Go should emit crossed skip comment, got:\n{}", go);
    assert!(!py.contains("# action: reject"), "Py should not emit crossed target as action");
    assert!(!go.contains("// action: reject"), "Go should not emit crossed target as action");
}
