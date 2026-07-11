use tangle_cli::audit_support::run_collecting_diagnostics;

#[test]
fn smoke_test_helper_returns_diagnostics_for_account_example() {
    // cargo test runs with cwd = package dir (compiler/tangle-cli), matching the
    // existing pattern in src/frontend/compile_module.rs (`../../tests/rules/...`).
    // The account example lives at the worktree root under `examples/`.
    let run = run_collecting_diagnostics("../../examples/account.tangle.md");
    // 当前已知 account.tangle.md 有虚假诊断，所以 diagnostics 非空
    assert!(!run.diagnostics.is_empty(), "expected false-positive diagnostics before G1 fix");
    println!("Captured {} diagnostics", run.diagnostics.len());
    for d in &run.diagnostics {
        println!("  [{}] {}", d.code, d.message);
    }
}
