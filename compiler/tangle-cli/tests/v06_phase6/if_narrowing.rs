//! Phase 6d: `if x is Some(y) { ... }` source-level narrowing integration test.
//!
//! Verifies that IsExpr narrowing produces zero diagnostics and that the
//! `process` function's return type is inferred as `Int` (the narrowed
//! inner value of `Option<Int>`).

use std::path::PathBuf;

use tangle_cli::audit_support::run_collecting_ir;
use tangle_cli::checker::types::{PrimitiveType, Type};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join("v06_phase6")
        .join(name)
}

const FIXTURE: &str = "if_narrowing.tangle.md";

#[test]
fn if_narrowing_produces_zero_diagnostics() {
    let path = fixture_path(FIXTURE);
    let (_graph, diags) = run_collecting_ir(&path);
    assert!(
        diags.is_empty(),
        "expected zero diagnostics, got: {:?}",
        diags
    );
}

#[test]
fn if_narrowing_process_return_type_is_int() {
    let path = fixture_path(FIXTURE);
    let (graph, _diags) = run_collecting_ir(&path);

    let process = graph
        .functions
        .iter()
        .find(|f| f.name == "process")
        .expect("process function should exist");

    let return_type = process
        .return_type
        .as_ref()
        .expect("process should have returnType");

    assert_eq!(
        return_type,
        &Type::Primitive(PrimitiveType {
            name: "Int".to_string()
        })
    );
}
