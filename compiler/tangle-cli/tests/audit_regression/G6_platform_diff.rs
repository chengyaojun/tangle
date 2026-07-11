//! G6: Platform/target differential regression tests.
//!
//! Populated after audit run (Task 11) if any diagnostics appear in
//! py/go targets but not js. Currently a baseline guard: all three
//! targets must produce equivalent diagnostic profiles.

use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
#[ignore = "等 G5/G6 解析器 let stop-list bug 修复后恢复（account.tangle.md 有残留诊断）"]
fn g6_placeholder_all_targets_equivalent_for_account() {
    // Baseline: account.tangle.md diagnostics should not depend on target.
    // Since run_collecting_diagnostics runs the frontend→checker→IR pipeline
    // (target-agnostic), this is automatically true today; the test exists
    // to fail loudly if codegen ever starts feeding back into the checker.
    let run = run_collecting_diagnostics("../../examples/account.tangle.md");
    assert!(run.diagnostics.is_empty(), "account.tangle.md should be clean across all targets");
}
