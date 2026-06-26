use crate::ir::graph::*;
use crate::model::{SourceSpan, TangleDiagnostic};
use std::collections::HashSet;

pub fn validate_ir(graph: &RuleGraph) -> Vec<TangleDiagnostic> {
    let mut diagnostics = vec![];
    let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();

    // Check entry node exists
    if !node_ids.contains(graph.entry_node_id.as_str()) {
        diagnostics.push(TangleDiagnostic {
            code: "TANGLE_IR_VALIDATION_ERROR".into(),
            message: format!("Entry node '{}' not found in graph", graph.entry_node_id),
            span: SourceSpan {
                file: String::new(), start_line: 0, start_column: 0, end_line: 0, end_column: 0,
            },
        });
    }

    // Check all edges reference existing nodes
    for edge in &graph.edges {
        if !node_ids.contains(edge.from.as_str()) {
            diagnostics.push(TangleDiagnostic {
                code: "TANGLE_IR_VALIDATION_ERROR".into(),
                message: format!("Edge from '{}' references non-existent node", edge.from),
                span: edge.source_span.clone().unwrap_or(SourceSpan {
                    file: String::new(), start_line: 0, start_column: 0, end_line: 0, end_column: 0,
                }),
            });
        }
        if !node_ids.contains(edge.to.as_str()) {
            diagnostics.push(TangleDiagnostic {
                code: "TANGLE_IR_VALIDATION_ERROR".into(),
                message: format!("Edge to '{}' references non-existent node", edge.to),
                span: edge.source_span.clone().unwrap_or(SourceSpan {
                    file: String::new(), start_line: 0, start_column: 0, end_line: 0, end_column: 0,
                }),
            });
        }
    }

    // Check error edges reference existing nodes
    for err_edge in &graph.error_edges {
        if !node_ids.contains(err_edge.from.as_str()) {
            diagnostics.push(TangleDiagnostic {
                code: "TANGLE_IR_VALIDATION_ERROR".into(),
                message: format!("Error edge from '{}' references non-existent node", err_edge.from),
                span: err_edge.source_span.clone().unwrap_or(SourceSpan {
                    file: String::new(), start_line: 0, start_column: 0, end_line: 0, end_column: 0,
                }),
            });
        }
    }

    diagnostics
}
