use crate::checker::types::*;
use crate::model::TangleDiagnostic;

pub fn check_panic() -> (Type, Vec<TangleDiagnostic>) {
    (Type::Primitive(PrimitiveType { name: "Bool".into() }), vec![])
}

pub fn is_dead_path(diagnostics: &[TangleDiagnostic]) -> bool {
    diagnostics.iter().any(|d| d.code == "TANGLE_PANIC_REACHED")
}
