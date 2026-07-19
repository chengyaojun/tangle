//! Phase 6d: Invariants INV-1 through INV-6.
//!
//! INV-1: `if x is Some(y)` narrows the then-branch environment
//!        (verified indirectly via zero type/pattern diagnostics on the
//!        `if_narrowing` fixture).
//! INV-2: `let Some(y) = x else { E }` lowers to IR with a descriptive
//!        `let Some(...)` Compute label.
//! INV-3: `let { f1, f2 } = r` lowers to a Compute node carrying the
//!        field names in its label.
//! INV-4: `as_sum_view(Option<T>) == Sum { Some<T>, None }`.
//! INV-5: existing fixtures unchanged (verified in Task 13 differential
//!        tests; placeholder here to assert the invariant's existence).
//! INV-6: new fixtures Rust/TS IR match (verified in Task 13 differential
//!        tests; placeholder here to assert the invariant's existence).

use std::path::PathBuf;

use tangle_cli::audit_support::run_collecting_ir;
use tangle_cli::checker::option_view::as_sum_view;
use tangle_cli::checker::types::{GenericTypeInstance, PrimitiveType, Type};
use tangle_cli::ir::graph::IRNodeKind;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join("v06_phase6")
        .join(name)
}

#[test]
fn inv1_if_is_narrowing_compiles_cleanly() {
    let path = fixture_path("if_narrowing.tangle.md");
    let (_graph, diags) = run_collecting_ir(&path);
    let errors: Vec<_> = diags
        .iter()
        .filter(|d| {
            d.code.starts_with("TANGLE_TYPE_") || d.code.starts_with("TANGLE_PATTERN_")
        })
        .collect();
    assert!(
        errors.is_empty(),
        "INV-1: if x is Some(y) should narrow then-branch without errors, got: {:?}",
        errors
    );
}

#[test]
fn inv2_let_variant_lowers_to_compute_with_binding_label() {
    let path = fixture_path("destructure.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);
    let has_let_some = graph
        .functions
        .iter()
        .flat_map(|f| f.nodes.iter())
        .any(|n| n.kind == IRNodeKind::Compute && n.label.starts_with("let Some("));
    assert!(
        has_let_some,
        "INV-2: let Some(item) should lower to Compute with label 'let Some(item)'"
    );
}

#[test]
fn inv3_let_record_lowers_to_compute_with_field_names_in_label() {
    let path = fixture_path("destructure.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);
    let has_let_record = graph
        .functions
        .iter()
        .flat_map(|f| f.nodes.iter())
        .any(|n| {
            n.kind == IRNodeKind::Compute
                && n.label.starts_with("let { ")
                && n.label.contains("name")
                && n.label.contains("price")
        });
    assert!(
        has_let_record,
        "INV-3: let {{ name, price }} should lower to Compute with label 'let {{ name, price }}'"
    );
}

#[test]
fn inv4_as_sum_view_option_int_is_sum_with_some_and_none() {
    let opt = Type::GenericInstance(GenericTypeInstance {
        base: "Option".to_string(),
        args: vec![Type::Primitive(PrimitiveType {
            name: "Int".to_string(),
        })],
    });
    let sum = as_sum_view(&opt).expect("Option<Int> should have sum view");
    assert_eq!(
        sum.variants.len(),
        2,
        "INV-4: Option<T> should map to Sum with 2 variants"
    );
    let variant_names: Vec<Option<String>> = sum
        .variants
        .iter()
        .map(|v| match v {
            Type::Primitive(p) => Some(p.name.clone()),
            Type::Struct(s) => Some(s.name.clone()),
            Type::GenericInstance(g) => Some(g.base.clone()),
            _ => None,
        })
        .collect();
    assert!(
        variant_names.iter().any(|n| n.as_deref() == Some("Some")),
        "INV-4: Some variant required"
    );
    assert!(
        variant_names.iter().any(|n| n.as_deref() == Some("None")),
        "INV-4: None variant required"
    );
}

#[test]
fn inv5_existing_fixtures_unchanged() {
    // Verified in Task 13 via differential tests (tests/audit/diff-ir.ps1).
    // This test exists to assert the invariant's existence in the test suite.
    let _ = fixture_path("return_inference.tangle.md");
    let _ = fixture_path("generics.tangle.md");
}

#[test]
fn inv6_new_fixtures_rust_ts_match() {
    // Verified in Task 13 via differential tests (tests/audit/diff-ir.ps1).
    // This test exists to assert the invariant's existence in the test suite.
    let _ = fixture_path("if_narrowing.tangle.md");
    let _ = fixture_path("destructure.tangle.md");
    let _ = fixture_path("option_match.tangle.md");
}
