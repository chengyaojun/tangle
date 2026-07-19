//! Phase 6d: `match opt { Some(x) => x, None => 0 }` integration test.
//!
//! Verifies that match-arm narrowing on `Option<Int>` produces zero
//! diagnostics and that the `double` function's return type is inferred
//! as `Int`.

use std::path::PathBuf;

use tangle_cli::audit_support::run_collecting_ir;
use tangle_cli::checker::types::{PrimitiveType, Type};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join("v06_phase6")
        .join(name)
}

const FIXTURE: &str = "option_match.tangle.md";

#[test]
fn option_match_produces_zero_diagnostics() {
    let path = fixture_path(FIXTURE);
    let (_graph, diags) = run_collecting_ir(&path);
    assert!(
        diags.is_empty(),
        "expected zero diagnostics, got: {:?}",
        diags
    );
}

#[test]
fn option_match_double_return_type_is_int() {
    let path = fixture_path(FIXTURE);
    let (graph, _diags) = run_collecting_ir(&path);

    let double = graph
        .functions
        .iter()
        .find(|f| f.name == "double")
        .expect("double function should exist");

    let return_type = double
        .return_type
        .as_ref()
        .expect("double should have returnType");

    assert_eq!(
        return_type,
        &Type::Primitive(PrimitiveType {
            name: "Int".to_string()
        })
    );
}
