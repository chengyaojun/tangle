//! Test-only entry point that runs the compile pipeline and returns
//! structured diagnostics. Not part of the public API.

use std::path::Path;

use crate::frontend::compile_module::{compile_module, CompileModuleInput};
use crate::checker::check_module::check_module;
use crate::ir::compile_to_ir::compile_to_ir;
use crate::ir::graph::RuleGraph;
use crate::model::TangleDiagnostic;

#[derive(Debug, Clone)]
pub struct TestRun {
    pub exit_code: i32,
    pub diagnostics: Vec<TangleDiagnostic>,
    pub stdout: String,
    pub stderr: String,
}

/// Run frontend → checker → IR pipeline on a file, collecting all diagnostics.
/// Does NOT execute the generated code. Suitable for audit regression tests.
pub fn run_collecting_diagnostics(file: &str) -> TestRun {
    let source = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            return TestRun {
                exit_code: 1,
                diagnostics: vec![],
                stdout: String::new(),
                stderr: format!("Error reading file: {}", e),
            };
        }
    };

    let mut all_diags: Vec<TangleDiagnostic> = Vec::new();

    let module = compile_module(CompileModuleInput {
        file: file.to_string(),
        source: source.clone(),
    });
    all_diags.extend(module.diagnostics.clone());

    let checked = check_module(module);
    all_diags.extend(checked.diagnostics.clone());

    let (_graph, ir_diags) = compile_to_ir(&checked);
    all_diags.extend(ir_diags);

    let exit_code = if all_diags.is_empty() { 0 } else { 1 };
    TestRun {
        exit_code,
        diagnostics: all_diags,
        stdout: String::new(),
        stderr: String::new(),
    }
}

/// Run frontend → checker → IR pipeline and return the resulting `RuleGraph`
/// together with all diagnostics. Like `run_collecting_diagnostics` but exposes
/// the IR graph so structural regression tests can assert on `nodes` / `functions`.
pub fn run_collecting_ir(file: impl AsRef<Path>) -> (RuleGraph, Vec<TangleDiagnostic>) {
    let file = file.as_ref();
    let source = std::fs::read_to_string(file).unwrap_or_else(|e| {
        panic!("failed to read {}: {}", file.display(), e)
    });

    let mut all_diags: Vec<TangleDiagnostic> = Vec::new();

    let module = compile_module(CompileModuleInput {
        file: file.to_string_lossy().into_owned(),
        source,
    });
    all_diags.extend(module.diagnostics.clone());

    let checked = check_module(module);
    all_diags.extend(checked.diagnostics.clone());

    let (graph, ir_diags) = compile_to_ir(&checked);
    all_diags.extend(ir_diags);

    (graph, all_diags)
}
