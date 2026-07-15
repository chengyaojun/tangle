use insta::assert_snapshot;
use tangle_cli::codegen::js_emitter::emit_js;
use tangle_cli::codegen::py_emitter::emit_python;
use tangle_cli::codegen::go_emitter::emit_go;
use tangle_cli::ir::graph::*;

fn make_decision_graph_with_priority() -> RuleGraph {
    RuleGraph {
        nodes: vec![
            IRNode { id: "entry".into(), kind: IRNodeKind::Decision, label: "check".into(),
                source_span: None, source_text: None,
                group: Some("Approval".into()), style: None },
            IRNode { id: "auto".into(), kind: IRNodeKind::Action, label: "auto_approve".into(),
                source_span: None, source_text: Some("status = \"auto\"".into()),
                group: Some("Approval".into()), style: Some("highlight".into()) },
            IRNode { id: "manual".into(), kind: IRNodeKind::Action, label: "manual_review".into(),
                source_span: None, source_text: Some("status = \"manual\"".into()),
                group: None, style: None },
            IRNode { id: "reject".into(), kind: IRNodeKind::Action, label: "reject".into(),
                source_span: None, source_text: Some("status = \"rejected\"".into()),
                group: None, style: None },
            IRNode { id: "end".into(), kind: IRNodeKind::Terminal, label: "done".into(),
                source_span: None, source_text: None, group: None, style: None },
        ],
        edges: vec![
            IREdge { from: "entry".into(), to: "auto".into(), kind: IREdgeKind::Condition,
                guard: Some("amount < 100".into()), source_span: None,
                priority: Some(0), style: None },
            IREdge { from: "entry".into(), to: "manual".into(), kind: IREdgeKind::Condition,
                guard: Some("amount < 1000".into()), source_span: None,
                priority: Some(1), style: Some("stroke:#f00".into()) },
            IREdge { from: "entry".into(), to: "reject".into(), kind: IREdgeKind::Crossed,
                guard: None, source_span: None, priority: None, style: None },
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
fn snapshot_js_decision_branches_with_priority() {
    let graph = make_decision_graph_with_priority();
    let js = emit_js(&graph, "ApprovalFlow");
    assert_snapshot!(js);
}

#[test]
fn snapshot_py_metadata_comments() {
    let graph = make_decision_graph_with_priority();
    let py = emit_python(&graph, "ApprovalFlow");
    assert_snapshot!(py);
}

#[test]
fn snapshot_go_metadata_comments() {
    let graph = make_decision_graph_with_priority();
    let go = emit_go(&graph, "ApprovalFlow");
    assert_snapshot!(go);
}
