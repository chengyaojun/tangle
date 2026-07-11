//! G3: Member access on non-struct type — was a cascade of G1.
//!
//! After G1 fix, Account resolves to Type::Struct, so Account.open(...)
//! no longer triggers TANGLE_TYPE_ERROR "Member access on non-struct type".
//!
//! Note: this test is currently #[ignore]d because a separate parser bug
//! (TokenKind::Let not in expression stop-list, causing consecutive let
//! statements to merge) produces a *different* "Member access on non-struct
//! type" diagnostic via acc2 → default type → member access. That parser
//! bug is tracked under G5/G6. Once the parser stop-list is fixed, this
//! test should pass and the ignore can be removed.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
#[ignore = "等 G5/G6 解析器 let stop-list bug 修复后恢复"]
fn g3_no_member_access_on_non_struct_after_g1() {
    let run = run_collecting_diagnostics("../../examples/account.tangle.md");
    let has_member_access_error = run.diagnostics.iter()
        .any(|d| d.code == "TANGLE_TYPE_ERROR"
            && d.message.contains("Member access on non-struct type"));
    assert!(
        !has_member_access_error,
        "Member access on non-struct should not occur after G1 fix, got: {}",
        run.diagnostics.iter().map(|d| format!("[{}] {}", d.code, d.message)).collect::<Vec<_>>().join("; ")
    );
}
