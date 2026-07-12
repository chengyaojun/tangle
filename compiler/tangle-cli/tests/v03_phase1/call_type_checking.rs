//! Call expression type checking tests.
//! Verifies arity and parameter type checking with real signatures.

use tangle_cli::audit_support::run_collecting_diagnostics;

fn diags_for(fixture: &str) -> Vec<String> {
    let run = run_collecting_diagnostics(fixture);
    run.diagnostics.iter().map(|d| d.code.clone()).collect()
}

#[test]
fn call_arity_ok_no_diagnostics() {
    let run = run_collecting_diagnostics("tests/v03_phase1/fixtures/call_arity_ok.tangle.md");
    assert!(
        run.diagnostics.is_empty(),
        "Expected zero diagnostics, got {}:\n{}",
        run.diagnostics.len(),
        run.diagnostics.iter().map(|d| format!("  [{}] {}", d.code, d.message)).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn call_arity_wrong_produces_diagnostic() {
    let codes = diags_for("tests/v03_phase1/fixtures/call_arity_wrong.tangle.md");
    assert!(
        codes.contains(&"TANGLE_ARITY_MISMATCH".to_string()),
        "Expected TANGLE_ARITY_MISMATCH for readFile() with 0 args, got: {:?}", codes
    );
}

#[test]
fn call_arg_type_wrong_produces_diagnostic() {
    let codes = diags_for("tests/v03_phase1/fixtures/call_arg_type_wrong.tangle.md");
    assert!(
        codes.contains(&"TANGLE_TYPE_ERROR".to_string()),
        "Expected TANGLE_TYPE_ERROR for readFile(123) where String expected, got: {:?}", codes
    );
}

#[test]
fn call_variadic_ok_no_diagnostics() {
    let run = run_collecting_diagnostics("tests/v03_phase1/fixtures/call_variadic_ok.tangle.md");
    assert!(
        run.diagnostics.is_empty(),
        "Expected zero diagnostics for variadic println with mixed arg types, got {}:\n{}",
        run.diagnostics.len(),
        run.diagnostics.iter().map(|d| format!("  [{}] {}", d.code, d.message)).collect::<Vec<_>>().join("\n")
    );
}
