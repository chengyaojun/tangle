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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, ExpressionStmt, LetStmt, LiteralExpr, LiteralKind, ReturnStmt};
    use crate::model::SourceSpan;

    fn test_span() -> SourceSpan {
        SourceSpan {
            file: "t.md".into(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 5,
        }
    }

    fn test_expr() -> Expr {
        Expr::Literal(LiteralExpr {
            literal_kind: LiteralKind::Number,
            value: "42".into(),
            span: test_span(),
        })
    }

    #[test]
    fn lower_single_let_statement() {
        let stmts = vec![Stmt::Let(LetStmt {
            name: "x".into(),
            type_annotation: None,
            value: test_expr(),
            span: test_span(),
        })];
        let mut id_gen = FreshNodeId::new();
        let graph = lower_statements(&stmts, "t.md", &mut id_gen);

        // 3 nodes: entry compute + let compute + exit terminal
        assert_eq!(graph.nodes.len(), 3, "expected 3 nodes, got {:?}", graph.nodes);
        assert_eq!(graph.nodes[0].label, "entry");
        assert_eq!(graph.nodes[0].kind, IRNodeKind::Compute);
        assert_eq!(graph.nodes[1].label, "let x");
        assert_eq!(graph.nodes[1].kind, IRNodeKind::Compute);
        assert_eq!(graph.nodes[2].label, "exit");
        assert_eq!(graph.nodes[2].kind, IRNodeKind::Terminal);

        // 2 edges: entry -> let, let -> exit
        assert_eq!(graph.edges.len(), 2);
        assert_eq!(graph.edges[0].from, graph.nodes[0].id);
        assert_eq!(graph.edges[0].to, graph.nodes[1].id);
        assert_eq!(graph.edges[1].from, graph.nodes[1].id);
        assert_eq!(graph.edges[1].to, graph.nodes[2].id);

        // entry_node_id matches first node
        assert_eq!(graph.entry_node_id, graph.nodes[0].id);
    }

    #[test]
    fn lower_return_with_value() {
        let stmts = vec![Stmt::Return(ReturnStmt {
            value: Some(test_expr()),
            span: test_span(),
        })];
        let mut id_gen = FreshNodeId::new();
        let graph = lower_statements(&stmts, "t.md", &mut id_gen);

        // 3 nodes: entry + return + exit
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[0].label, "entry");
        assert_eq!(graph.nodes[1].label, "return");
        assert_eq!(graph.nodes[1].kind, IRNodeKind::Action);
        assert_eq!(graph.nodes[2].label, "exit");
        assert_eq!(graph.nodes[2].kind, IRNodeKind::Terminal);

        // edge from entry to return
        assert_eq!(graph.edges[0].from, graph.nodes[0].id);
        assert_eq!(graph.edges[0].to, graph.nodes[1].id);
        assert_eq!(graph.edges[0].kind, IREdgeKind::Control);
    }

    #[test]
    fn lower_expression_statement() {
        let stmts = vec![Stmt::Expression(ExpressionStmt {
            expr: test_expr(),
            span: test_span(),
        })];
        let mut id_gen = FreshNodeId::new();
        let graph = lower_statements(&stmts, "t.md", &mut id_gen);

        // 3 nodes: entry + expr + exit
        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[0].label, "entry");
        assert_eq!(graph.nodes[1].label, "expr");
        assert_eq!(graph.nodes[1].kind, IRNodeKind::Action);
        assert_eq!(graph.nodes[2].label, "exit");
        assert_eq!(graph.nodes[2].kind, IRNodeKind::Terminal);

        // edge from entry to expr
        assert_eq!(graph.edges[0].from, graph.nodes[0].id);
        assert_eq!(graph.edges[0].to, graph.nodes[1].id);
    }

    #[test]
    fn lower_multiple_statements() {
        let stmts = vec![
            Stmt::Let(LetStmt {
                name: "a".into(),
                type_annotation: None,
                value: test_expr(),
                span: test_span(),
            }),
            Stmt::Expression(ExpressionStmt {
                expr: test_expr(),
                span: test_span(),
            }),
            Stmt::Return(ReturnStmt {
                value: Some(test_expr()),
                span: test_span(),
            }),
        ];
        let mut id_gen = FreshNodeId::new();
        let graph = lower_statements(&stmts, "t.md", &mut id_gen);

        // 5 nodes: entry + let + expr + return + exit
        assert_eq!(graph.nodes.len(), 5);
        assert_eq!(graph.nodes[0].label, "entry");
        assert_eq!(graph.nodes[1].label, "let a");
        assert_eq!(graph.nodes[2].label, "expr");
        assert_eq!(graph.nodes[3].label, "return");
        assert_eq!(graph.nodes[4].label, "exit");

        // 4 edges chain correctly
        assert_eq!(graph.edges.len(), 4);
        assert_eq!(graph.edges[0].from, graph.nodes[0].id);
        assert_eq!(graph.edges[0].to, graph.nodes[1].id);
        assert_eq!(graph.edges[1].from, graph.nodes[1].id);
        assert_eq!(graph.edges[1].to, graph.nodes[2].id);
        assert_eq!(graph.edges[2].from, graph.nodes[2].id);
        assert_eq!(graph.edges[2].to, graph.nodes[3].id);
        assert_eq!(graph.edges[3].from, graph.nodes[3].id);
        assert_eq!(graph.edges[3].to, graph.nodes[4].id);
    }
}
