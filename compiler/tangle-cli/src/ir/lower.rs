use crate::ast::Stmt;
use crate::ir::graph::*;

pub fn lower_statements(stmts: &[Stmt], _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph {
    let entry_id = id_gen.next();
    let mut graph = create_graph(entry_id.clone());

    graph.nodes.push(IRNode {
        id: entry_id.clone(), kind: IRNodeKind::Compute,
        label: "entry".into(), source_span: None,
    });

    let mut prev_id = entry_id;

    for stmt in stmts {
        let (node_kind, label) = match stmt {
            Stmt::Return(_) => (IRNodeKind::Action, "return".to_string()),
            Stmt::Let(s) => (IRNodeKind::Compute, format!("let {}", s.name)),
            Stmt::Const(s) => (IRNodeKind::Compute, format!("const {}", s.name)),
            Stmt::Expression(_) => (IRNodeKind::Action, "expr".to_string()),
        };

        let node_id = id_gen.next();
        graph.nodes.push(IRNode {
            id: node_id.clone(), kind: node_kind, label, source_span: None,
        });

        graph.edges.push(IREdge {
            from: prev_id, to: node_id.clone(), kind: IREdgeKind::Control,
            guard: None, source_span: None,
        });

        prev_id = node_id;
    }

    // Terminal node
    let terminal_id = id_gen.next();
    graph.nodes.push(IRNode {
        id: terminal_id.clone(), kind: IRNodeKind::Terminal,
        label: "exit".into(), source_span: None,
    });
    graph.edges.push(IREdge {
        from: prev_id, to: terminal_id, kind: IREdgeKind::Control,
        guard: None, source_span: None,
    });

    graph
}
