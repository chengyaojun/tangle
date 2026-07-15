//! G7: Heading prefix recognition — `Prefix: Identifier` pattern.
//!
//! Findings F-003 / F-004 / F-005 from the initial audit: headings like
//! `##### Error: PayFailed` and `##### Rule: Approval` were flagged with
//! `TANGLE_HEADING_MULTI_WORD` because the multi-word check in
//! `frontend/headings.rs` did not recognize Tangle's built-in prefix
//! convention (`Error:` / `Rule:` / `Type:` / `Command:` / `Query:` /
//! `Test:`) and did not extract the trailing identifier as the symbol name.
//!
//! After the F-003 fix, the parser recognizes `^(Prefix)\s*:\s*(<ident>)$`
//! and uses `<ident>` as the symbol name, skipping the multi-word warning.

use tangle_cli::audit_support::run_collecting_diagnostics;

fn has_multi_word_diag(run: &tangle_cli::audit_support::TestRun) -> bool {
    run.diagnostics.iter().any(|d| d.code == "TANGLE_HEADING_MULTI_WORD")
}

#[test]
fn g7_payment_fixture_no_heading_multi_word() {
    let run = run_collecting_diagnostics("../../tests/errors/payment.tangle.md");
    assert!(
        !has_multi_word_diag(&run),
        "payment.tangle.md should not emit TANGLE_HEADING_MULTI_WORD after F-003 fix, got: {}",
        run.diagnostics.iter()
            .map(|d| format!("[{}] {}", d.code, d.message))
            .collect::<Vec<_>>().join("; ")
    );
}

#[test]
fn g7_order_service_fixture_no_heading_multi_word_for_prefix_headings() {
    let run = run_collecting_diagnostics("../../tests/mvp/order-service.tangle.md");
    // F-004 (a) `# Order Service` is a genuine multi-word top-level title
    // and SHOULD still be flagged. We only assert that the prefix-form
    // headings (`Error: PayFailed`, `Error: Timeout`) are NOT flagged.
    let prefix_diag_count = run.diagnostics.iter()
        .filter(|d| d.code == "TANGLE_HEADING_MULTI_WORD")
        .filter(|d| d.message.contains("Error:"))
        .count();
    assert_eq!(
        prefix_diag_count, 0,
        "prefix-form Error: headings should not be flagged after F-003 fix, got: {}",
        run.diagnostics.iter()
            .filter(|d| d.code == "TANGLE_HEADING_MULTI_WORD")
            .map(|d| d.message.clone())
            .collect::<Vec<_>>().join("; ")
    );
}

#[test]
fn g7_rule_fixtures_no_heading_multi_word() {
    let fixtures = [
        "../../tests/rules/approval-flow.tangle.md",
        "../../tests/rules/decision-table.tangle.md",
        "../../tests/rules/decision-tree.tangle.md",
        "../../tests/rules/feature-toggles.tangle.md",
    ];
    for ex in &fixtures {
        let run = run_collecting_diagnostics(ex);
        assert!(
            !has_multi_word_diag(&run),
            "{} should not emit TANGLE_HEADING_MULTI_WORD after F-003 fix, got: {}",
            ex,
            run.diagnostics.iter()
                .map(|d| format!("[{}] {}", d.code, d.message))
                .collect::<Vec<_>>().join("; ")
        );
    }
}
