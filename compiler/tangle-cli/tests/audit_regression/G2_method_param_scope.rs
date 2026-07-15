//! G2: Method parameter scope regression tests.
//!
//! Root cause: check_module built block_env with receiver but did not
//! inject the method's params into variables, so method body references
//! to declared parameters were reported as SYMBOL_NOT_FOUND.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn g2_method_params_resolvable_in_body() {
    let run = run_collecting_diagnostics("../../examples/account.tangle.md");
    let param_not_found = run.diagnostics.iter()
        .any(|d| d.code == "TANGLE_SYMBOL_NOT_FOUND"
            && (d.message.contains("account") || d.message.contains("amount")));
    assert!(
        !param_not_found,
        "method params 'account' / 'amount' should be resolvable in deposit body, got: {}",
        run.diagnostics.iter().map(|d| format!("[{}] {}", d.code, d.message)).collect::<Vec<_>>().join("; ")
    );
}
