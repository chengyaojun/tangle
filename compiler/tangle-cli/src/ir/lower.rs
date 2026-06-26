use crate::ast::Stmt;
use crate::ir::graph::*;

fn stmt_source(stmt: &Stmt, source: &str) -> String {
    let span = match stmt {
        Stmt::Return(s) => &s.span,
        Stmt::Let(s) => &s.span,
        Stmt::Const(s) => &s.span,
        Stmt::Expression(s) => &s.span,
    };
    let lines: Vec<&str> = source.lines().collect();
    let start = span.start_line.saturating_sub(1);
    let end = span.end_line.min(lines.len());
    if start < end {
        lines[start..end].join("\n").trim().trim_end_matches(';').to_string()
    } else {
        String::new()
    }
}

pub fn lower_statements(stmts: &[Stmt], source: &str, _file: &str, id_gen: &mut FreshNodeId) -> RuleGraph {
    let entry_id = id_gen.next();
    let mut graph = create_graph(entry_id.clone());

    graph.nodes.push(IRNode {
        id: entry_id.clone(), kind: IRNodeKind::Compute,
        label: "entry".into(), source_span: None, source_text: None,
    });

    let mut prev_id = entry_id;

    for stmt in stmts {
        let (node_kind, label) = match stmt {
            Stmt::Return(_) => (IRNodeKind::Action, "return".to_string()),
            Stmt::Let(s) => (IRNodeKind::Compute, format!("let {}", s.name)),
            Stmt::Const(s) => (IRNodeKind::Compute, format!("const {}", s.name)),
            Stmt::Expression(_) => (IRNodeKind::Action, "expr".to_string()),
        };
        let src = stmt_source(stmt, source);

        let node_id = id_gen.next();
        graph.nodes.push(IRNode {
            id: node_id.clone(), kind: node_kind, label,
            source_span: None, source_text: Some(src),
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
        label: "exit".into(), source_span: None, source_text: None,
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
            start_line: 1, start_column: 1, end_line: 1, end_column: 5,
        }
    }

    fn test_expr() -> Expr {
        Expr::Literal(LiteralExpr {
            literal_kind: LiteralKind::Number, value: "42".into(), span: test_span(),
        })
    }

    #[test]
    fn lower_single_let_statement() {
        let source = "let x = 42";
        let stmts = vec![Stmt::Let(LetStmt {
            name: "x".into(), type_annotation: None, value: test_expr(), span: test_span(),
        })];
        let mut id_gen = FreshNodeId::new();
        let graph = lower_statements(&stmts, source, "t.md", &mut id_gen);

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[1].source_text.as_deref(), Some("let x = 42"));
    }

    #[test]
    fn lower_return_with_value() {
        let source = "return 42";
        let stmts = vec![Stmt::Return(ReturnStmt {
            value: Some(test_expr()), span: test_span(),
        })];
        let mut id_gen = FreshNodeId::new();
        let graph = lower_statements(&stmts, source, "t.md", &mut id_gen);

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[1].source_text.as_deref(), Some("return 42"));
    }

    #[test]
    fn lower_expression_statement() {
        let source = "fmt.println(\"Hello\")";
        let stmts = vec![Stmt::Expression(ExpressionStmt {
            expr: test_expr(), span: test_span(),
        })];
        let mut id_gen = FreshNodeId::new();
        let graph = lower_statements(&stmts, source, "t.md", &mut id_gen);

        assert_eq!(graph.nodes.len(), 3);
        assert_eq!(graph.nodes[1].source_text.as_deref(), Some("fmt.println(\"Hello\")"));
    }

    #[test]
    fn lower_multiple_statements() {
        let source = "let a = 42\nfmt.println(\"Hello\")\nreturn a";
        let stmts = vec![
            Stmt::Let(LetStmt { name: "a".into(), type_annotation: None, value: test_expr(), span: SourceSpan { file: "t.md".into(), start_line: 1, start_column: 1, end_line: 1, end_column: 10 } }),
            Stmt::Expression(ExpressionStmt { expr: test_expr(), span: SourceSpan { file: "t.md".into(), start_line: 2, start_column: 1, end_line: 2, end_column: 22 } }),
            Stmt::Return(ReturnStmt { value: Some(test_expr()), span: SourceSpan { file: "t.md".into(), start_line: 3, start_column: 1, end_line: 3, end_column: 9 } }),
        ];
        let mut id_gen = FreshNodeId::new();
        let graph = lower_statements(&stmts, source, "t.md", &mut id_gen);

        assert_eq!(graph.nodes.len(), 5);
        assert_eq!(graph.nodes[1].source_text.as_deref(), Some("let a = 42"));
        assert_eq!(graph.nodes[2].source_text.as_deref(), Some("fmt.println(\"Hello\")"));
        assert_eq!(graph.nodes[3].source_text.as_deref(), Some("return a"));
    }
}
