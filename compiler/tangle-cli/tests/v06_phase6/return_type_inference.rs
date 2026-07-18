//! Phase 6c: Return type inference + Match arm narrowing integration tests.
//!
//! Verifies that `IRFunction.return_type` is populated correctly from
//! return statements and Match arm narrowing.

use std::path::PathBuf;

use tangle_cli::audit_support::run_collecting_ir;
use tangle_cli::checker::types::Type;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join("v06_phase6")
        .join(name)
}

// --- Fixture 1: return_inference.tangle.md ---

#[test]
fn test_return_inference_process_returns_list_int() {
    let path = fixture_path("return_inference.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    let process = graph
        .functions
        .iter()
        .find(|f| f.name == "process")
        .expect("fixture should define process");

    let ty = process
        .return_type
        .as_ref()
        .expect("process should have return_type");

    match ty {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "List", "process return type base should be List");
            assert_eq!(g.args.len(), 1, "List should have 1 type arg");
            match &g.args[0] {
                Type::Primitive(p) => assert_eq!(p.name, "Int", "List arg should be Int"),
                other => panic!("expected Int, got {:?}", other),
            }
        }
        other => panic!("expected GenericInstance List<Int>, got {:?}", other),
    }
}

#[test]
fn test_return_inference_main_returns_int() {
    let path = fixture_path("return_inference.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    let main = graph
        .functions
        .iter()
        .find(|f| f.name == "main")
        .expect("fixture should define main");

    let ty = main
        .return_type
        .as_ref()
        .expect("main should have return_type");

    match ty {
        Type::Primitive(p) => assert_eq!(p.name, "Int", "main return type should be Int"),
        other => panic!("expected Int, got {:?}", other),
    }
}

// --- Fixture 2: match_narrowing.tangle.md ---

#[test]
fn test_match_narrowing_process_returns_int() {
    let path = fixture_path("match_narrowing.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    let process = graph
        .functions
        .iter()
        .find(|f| f.name == "process")
        .expect("fixture should define process");

    let ty = process
        .return_type
        .as_ref()
        .expect("process should have return_type");

    match ty {
        Type::Primitive(p) => assert_eq!(p.name, "Int", "process return type should be Int"),
        other => panic!("expected Int (from narrowed match arms), got {:?}", other),
    }
}

// --- Fixture 3: return_conflict.tangle.md ---

#[test]
fn test_return_conflict_process_returns_any() {
    let path = fixture_path("return_conflict.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    let process = graph
        .functions
        .iter()
        .find(|f| f.name == "process")
        .expect("fixture should define process");

    let ty = process
        .return_type
        .as_ref()
        .expect("process should have return_type");

    assert!(
        matches!(ty, Type::Any),
        "expected Any (conflict Int vs String), got {:?}",
        ty
    );
}

// --- Existing fixture: generics.tangle.md (Phase 6b) should now have return_type ---

#[test]
fn test_generics_fixture_process_returns_list_int() {
    let path = fixture_path("generics.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    let process = graph
        .functions
        .iter()
        .find(|f| f.name == "process")
        .expect("fixture should define process");

    let ty = process
        .return_type
        .as_ref()
        .expect("process should now have return_type (Phase 6c)");

    match ty {
        Type::GenericInstance(g) => {
            assert_eq!(g.base, "List", "generics fixture process should return List<Int>");
        }
        other => panic!("expected GenericInstance, got {:?}", other),
    }
}
