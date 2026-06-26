use crate::ir::graph::RuleGraph;
use crate::model::TangleDiagnostic;

pub fn check_ir_visibility(_graph: &RuleGraph, _exported_symbols: &[String]) -> Vec<TangleDiagnostic> {
    // Placeholder — cross-module reference validation
    vec![]
}
