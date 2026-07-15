//! Regression test: ensure MemberAccess returning Type::Function for methods
//! does not introduce false TANGLE_TYPE_ERROR diagnostics across example files.
//! Note: internal return type cannot be verified via diagnostics API; this test
//! guards against regressions where the MemberAccess change breaks existing code.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn member_access_no_false_type_errors_across_examples() {
    let examples = [
        "../../examples/account.tangle.md",
        "../../examples/math-data.tangle.md",
        "../../examples/io-system.tangle.md",
        "../../examples/collections.tangle.md",
    ];

    for example in &examples {
        let run = run_collecting_diagnostics(example);
        let has_member_access_error = run.diagnostics.iter()
            .any(|d| d.code == "TANGLE_TYPE_ERROR" && d.message.contains("Member access"));
        assert!(
            !has_member_access_error,
            "Member access on struct method should not produce type errors in {}: {:?}",
            example,
            run.diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }
}
