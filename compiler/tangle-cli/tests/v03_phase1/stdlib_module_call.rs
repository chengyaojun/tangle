//! Verify resolve_stdlib_imports injects real signatures from the registry.
//! After this change, fmt.println should have is_variadic=true and IO.readFile
//! should have a String param — not the old dummy (params=[], returns=String).

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn stdlib_module_import_no_false_diagnostics() {
    let run = run_collecting_diagnostics("tests/v03_phase1/fixtures/stdlib_module_call.tangle.md");
    assert!(
        run.diagnostics.is_empty(),
        "Expected zero diagnostics, got {}:\n{}",
        run.diagnostics.len(),
        run.diagnostics.iter().map(|d| format!("  [{}] {}", d.code, d.message)).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn stdlib_function_import_no_false_diagnostics() {
    let run = run_collecting_diagnostics("tests/v03_phase1/fixtures/stdlib_fn_call.tangle.md");
    assert!(
        run.diagnostics.is_empty(),
        "Expected zero diagnostics, got {}:\n{}",
        run.diagnostics.len(),
        run.diagnostics.iter().map(|d| format!("  [{}] {}", d.code, d.message)).collect::<Vec<_>>().join("\n")
    );
}
