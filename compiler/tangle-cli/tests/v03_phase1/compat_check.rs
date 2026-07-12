//! Permanent regression test guarding Phase 1 compatibility: verifies that all
//! 6 examples and 9 test fixtures produce zero diagnostics (no false positives
//! from Call type checking).

use tangle_cli::audit_support::run_collecting_diagnostics;

fn assert_no_diagnostics(fixture: &str) {
    let run = run_collecting_diagnostics(fixture);
    assert!(
        run.diagnostics.is_empty(),
        "Expected zero diagnostics for {}, got {}:\n{}",
        fixture,
        run.diagnostics.len(),
        run.diagnostics
            .iter()
            .map(|d| format!("  [{}] {} (span {:?})", d.code, d.message, d.span))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn examples_produce_zero_diagnostics() {
    let examples = [
        "../../examples/account.tangle.md",
        "../../examples/math-data.tangle.md",
        "../../examples/io-system.tangle.md",
        "../../examples/crypto.tangle.md",
        "../../examples/concurrency.tangle.md",
        "../../examples/collections.tangle.md",
    ];
    for example in &examples {
        assert_no_diagnostics(example);
    }
}

#[test]
fn fixtures_produce_zero_diagnostics() {
    let fixtures = [
        "../../tests/basic/hello.tangle.md",
        "../../tests/basic/expression.tangle.md",
        "../../tests/structs/user.tangle.md",
        "../../tests/rules/feature-toggles.tangle.md",
        "../../tests/rules/decision-tree.tangle.md",
        "../../tests/rules/decision-table.tangle.md",
        "../../tests/rules/approval-flow.tangle.md",
        "../../tests/mvp/order-service.tangle.md",
        "../../tests/errors/payment.tangle.md",
    ];
    for fixture in &fixtures {
        assert_no_diagnostics(fixture);
    }
}
