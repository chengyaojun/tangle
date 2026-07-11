//! F-024: Top-level callable symbol resolution regression test.
//!
//! Root cause: TypeEnv had no `functions` table; resolve_types only
//! collected Callables that were children of Type headings (struct methods).
//! Top-level Callables (not under any struct) were invisible to the checker.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn f024_toplevel_callable_no_symbol_not_found() {
    let run = run_collecting_diagnostics("tests/v03_phase1/fixtures/toplevel_func_call.tangle.md");
    let has_symbol_not_found = run.diagnostics.iter()
        .any(|d| d.code == "TANGLE_SYMBOL_NOT_FOUND");
    assert!(
        !has_symbol_not_found,
        "Top-level callable should be resolvable, got diagnostics:\n{}",
        run.diagnostics.iter().map(|d| format!("  [{}] {}", d.code, d.message)).collect::<Vec<_>>().join("\n")
    );
}
