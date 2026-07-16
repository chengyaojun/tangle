//! B2: three-host `emit_branch_body` recursion depth limit.
//!
//! `emit_branch_body` previously relied solely on a `visited` set for cycle
//! prevention, with no depth bound. A deeply nested branch chain could exhaust
//! the stack. This test verifies that all three emitters (JS / Py / Go) cap
//! recursion at `MAX_BRANCH_DEPTH` (100) and emit a `// max depth reached`
//! (JS/Go) or `# max depth reached` (Py) comment when the limit is hit.
//!
//! Fixture: `tests/v05_phase5/deep-recursion.tangle.md` — a Mermaid flow graph
//! with a Decision node whose guarded branch target starts a 105-node chain,
//! guaranteeing the recursion depth exceeds MAX_BRANCH_DEPTH.

use std::path::PathBuf;

use tangle_cli::audit_support::run_collecting_ir;
use tangle_cli::codegen::{emit_go, emit_js, emit_python};

fn fixture_path(group: &str, name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests")
        .join(group)
        .join(name)
}

#[test]
fn js_branch_body_depth_limit_emits_comment() {
    let path = fixture_path("v05_phase5", "deep-recursion.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);
    let output = emit_js(&graph, "deep");

    assert!(
        output.contains("// max depth reached"),
        "JS output should contain depth limit comment, got:\n{}",
        output
    );
}

#[test]
fn py_branch_body_depth_limit_emits_comment() {
    let path = fixture_path("v05_phase5", "deep-recursion.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);
    let output = emit_python(&graph, "deep");

    assert!(
        output.contains("# max depth reached"),
        "Py output should contain depth limit comment, got:\n{}",
        output
    );
}

#[test]
fn go_branch_body_depth_limit_emits_comment() {
    let path = fixture_path("v05_phase5", "deep-recursion.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);
    let output = emit_go(&graph, "deep");

    assert!(
        output.contains("// max depth reached"),
        "Go output should contain depth limit comment, got:\n{}",
        output
    );
}

#[test]
fn branch_body_normal_depth_no_comment() {
    // A shallow graph (no deep nesting) must not emit the depth-limit comment.
    let path = fixture_path("basic", "expression.tangle.md");
    let (graph, _diags) = run_collecting_ir(&path);
    let output = emit_js(&graph, "expression");

    assert!(
        !output.contains("max depth reached"),
        "Normal depth should not emit depth comment, got:\n{}",
        output
    );
}
