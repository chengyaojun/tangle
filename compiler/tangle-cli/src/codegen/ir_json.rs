use crate::ir::graph::RuleGraph;

/// Serialize a RuleGraph to JSON string per the shared IR Schema
pub fn emit_ir_json(graph: &RuleGraph) -> String {
    serde_json::to_string_pretty(graph).unwrap_or_else(|e| {
        format!("{{\"error\": \"Failed to serialize IR: {}\"}}", e)
    })
}
