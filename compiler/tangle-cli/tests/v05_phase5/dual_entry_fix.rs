//! A1-1: top-level dual-entry cleanup regression tests.
//!
//! When a `main` Callable heading exists, `@tangle` code blocks live only in
//! `functions[]` — the top-level `nodes[]` / `edges[]` must stay empty (no
//! dual-entry). Without `main`, the fallback single-function mode keeps the
//! blocks merged at the top level.
//!
//! Fixtures:
//! - `tests/errors/payment.tangle.md` has `#### main` + `#### process` → multi-function mode.
//! - `tests/structs/user.tangle.md` has `#### activate` (Callable, not main) → fallback mode.

use std::path::PathBuf;

use tangle_cli::audit_support::run_collecting_ir;

fn fixture_path(group: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join(group)
        .join(name)
}

#[test]
fn payment_top_level_empty_when_functions_present() {
    let path = fixture_path("errors", "payment.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    // payment has `#### main` + `#### process` → multi-function mode
    assert!(!graph.functions.is_empty(), "payment should have functions[]");
    assert_eq!(graph.functions.len(), 2, "payment should have main + process");

    // Top-level nodes[] must be empty: @tangle blocks live in functions[],
    // and payment has no rules, so nothing is merged into the top-level shell.
    assert!(
        graph.nodes.is_empty(),
        "top-level nodes[] should be empty when functions[] present, got: {:?}",
        graph.nodes.iter().map(|n| &n.label).collect::<Vec<_>>()
    );
}

#[test]
fn payment_functions_array_has_main_and_process() {
    let path = fixture_path("errors", "payment.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    let names: Vec<&str> = graph.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"main"), "functions[] should contain main, got: {:?}", names);
    assert!(names.contains(&"process"), "functions[] should contain process, got: {:?}", names);
}

#[test]
fn fallback_top_level_populated_when_no_main() {
    // user.tangle.md has `#### activate` (Callable, not main) → fallback mode:
    // @tangle block merged into top-level, functions[] stays empty.
    let path = fixture_path("structs", "user.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);

    assert!(graph.functions.is_empty(), "user should have no functions[] (no main)");
    assert!(!graph.nodes.is_empty(), "user top-level nodes[] should be populated (fallback mode)");
}
