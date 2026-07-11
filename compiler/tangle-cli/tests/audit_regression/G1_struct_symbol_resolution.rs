//! G1: Struct/type symbol resolution regression tests.
//!
//! Root cause: resolve_types in compiler/tangle-cli/src/checker/resolve.rs
//! only iterated top-level headings, missing types nested under # Program.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn g1_account_example_has_no_diagnostics() {
    let run = run_collecting_diagnostics("../../examples/account.tangle.md");
    assert!(
        run.diagnostics.is_empty(),
        "expected zero diagnostics, got {}:\n{}",
        run.diagnostics.len(),
        run.diagnostics.iter().map(|d| format!("  [{}] {}", d.code, d.message)).collect::<Vec<_>>().join("\n")
    );
}

#[test]
fn g1_struct_symbol_visible_in_main_block() {
    // account.tangle.md: ### Account nested under # AccountDemo
    // #### main references Account.open(100)? — Account must be resolvable.
    let run = run_collecting_diagnostics("../../examples/account.tangle.md");
    let has_symbol_not_found = run.diagnostics.iter()
        .any(|d| d.code == "TANGLE_SYMBOL_NOT_FOUND" && d.message.contains("Account"));
    assert!(
        !has_symbol_not_found,
        "Account symbol should be resolvable in main block"
    );
}
