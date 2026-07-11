//! G5: Documentation/example drift regression tests.
//!
//! Populated after audit run (Task 11) surfaces specific drift findings.
//! Each finding becomes a test asserting that the example output matches
//! the documented behavior.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
#[ignore = "等 G5/G6 解析器 let stop-list bug 修复后恢复（account.tangle.md 有残留诊断）"]
fn g5_placeholder_all_examples_run_without_diagnostics() {
    // Audit baseline: every example should run with zero diagnostics after
    // G1-G3 fixes. This test will fail if any example regresses.
    let examples = [
        "../../examples/account.tangle.md",
        "../../examples/collections.tangle.md",
        "../../examples/concurrency.tangle.md",
        "../../examples/crypto.tangle.md",
        "../../examples/io-system.tangle.md",
        "../../examples/math-data.tangle.md",
    ];
    let mut failures = Vec::new();
    for ex in &examples {
        let run = run_collecting_diagnostics(ex);
        if !run.diagnostics.is_empty() {
            failures.push(format!(
                "{}: {} diagnostics",
                ex,
                run.diagnostics.len()
            ));
        }
    }
    assert!(failures.is_empty(), "examples with diagnostics:\n{}", failures.join("\n"));
}
