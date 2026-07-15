//! G3: Member access on non-struct type — was a cascade of G1.
//!
//! After G1 fix, Account resolves to Type::Struct, so Account.open(...)
//! no longer triggers TANGLE_TYPE_ERROR "Member access on non-struct type".
//! The parser stop-list bug (TokenKind::Let not in expression stop-list)
//! that produced a secondary member-access diagnostic via acc2 was fixed
//! in F-001 (added Let|Const to the stop-list in parser.rs).

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
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
