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
use tangle_cli::codegen::{emit_python, emit_go};

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

// --- A1-3/4: Py/Go multi-function emission ---

#[test]
fn py_multi_function_emits_main_and_process() {
    // payment has `#### main` + `#### process` → multi-function mode.
    let path = fixture_path("errors", "payment.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);
    let output = emit_python(&graph, "payment");

    assert!(output.contains("def main("), "Py output should contain def main, got: {}", output);
    assert!(output.contains("def process("), "Py output should contain def process, got: {}", output);
    assert!(
        output.contains("if __name__ == '__main__'"),
        "Py output should contain if __name__ == __main__ entry, got: {}", output
    );
}

#[test]
fn py_single_function_fallback_when_no_main() {
    // user has `#### activate` (no main) → fallback single-function mode.
    // Fallback emits `def user()` (not `def main()`/`def process()`).
    let path = fixture_path("structs", "user.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);
    let output = emit_python(&graph, "user");

    assert!(!output.contains("def main("), "user should not emit def main, got: {}", output);
    assert!(!output.contains("def process("), "user should not emit def process, got: {}", output);
}

#[test]
fn go_multi_function_emits_main_and_process() {
    let path = fixture_path("errors", "payment.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);
    let output = emit_go(&graph, "payment");

    assert!(output.contains("func main("), "Go output should contain func main, got: {}", output);
    assert!(output.contains("func process("), "Go output should contain func process, got: {}", output);
}

#[test]
fn go_single_function_fallback_when_no_main() {
    // user has no main → fallback single-function mode emits `func User()` + `func main()` entry.
    // The distinguishing factor: no `func process()` (only multi-function mode emits process).
    let path = fixture_path("structs", "user.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);
    let output = emit_go(&graph, "user");

    assert!(!output.contains("func process("), "user should not emit func process, got: {}", output);
}
