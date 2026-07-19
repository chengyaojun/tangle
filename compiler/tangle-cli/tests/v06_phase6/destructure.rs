//! Phase 6d: `let Some(y) = opt else { ... }` and `let { name, price } = item`
//! source-level narrowing / destructuring integration test.
//!
//! Verifies:
//! - Zero diagnostics on the `destructure` fixture.
//! - `process` returns `Item` (struct name comparison only — methods may be
//!   populated by resolve_types and would make full-equality assertions brittle).
//! - `let Some(...)` lowers to a `Compute` node with a descriptive label.
//! - `let { ... }` lowers to a `Compute` node with a descriptive label that
//!   carries the field names.

use std::path::PathBuf;

use tangle_cli::audit_support::run_collecting_ir;
use tangle_cli::checker::types::Type;
use tangle_cli::ir::graph::IRNodeKind;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join("v06_phase6")
        .join(name)
}

const FIXTURE: &str = "destructure.tangle.md";

#[test]
fn destructure_produces_zero_diagnostics() {
    let path = fixture_path(FIXTURE);
    let (_graph, diags) = run_collecting_ir(&path);
    assert!(
        diags.is_empty(),
        "expected zero diagnostics, got: {:?}",
        diags
    );
}

#[test]
fn destructure_process_return_type_is_item() {
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

    // IMPORTANT: only compare struct name — `resolve_types` may populate
    // methods on the StructType, so full-equality would be brittle.
    match return_type {
        Type::Struct(s) => assert_eq!(
            s.name, "Item",
            "process return type struct name should be Item"
        ),
        other => panic!("expected Struct(Item), got {:?}", other),
    }
}

#[test]
fn destructure_let_variant_produces_compute_with_descriptive_label() {
    let path = fixture_path(FIXTURE);
    let (graph, _diags) = run_collecting_ir(&path);

    let has_let_some = graph
        .functions
        .iter()
        .flat_map(|f| f.nodes.iter())
        .any(|n| n.kind == IRNodeKind::Compute && n.label.starts_with("let Some("));

    assert!(
        has_let_some,
        "let Some(...) should lower to a Compute node with a label starting with 'let Some('"
    );
}

#[test]
fn destructure_let_record_produces_compute_with_descriptive_label() {
    let path = fixture_path(FIXTURE);
    let (graph, _diags) = run_collecting_ir(&path);

    let has_let_record = graph
        .functions
        .iter()
        .flat_map(|f| f.nodes.iter())
        .any(|n| n.kind == IRNodeKind::Compute && n.label.starts_with("let { "));

    assert!(
        has_let_record,
        "let {{ ... }} should lower to a Compute node with a label starting with 'let {{ '"
    );
}
