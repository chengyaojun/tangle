use crate::frontend::compile_module::{compile_module, CompileModuleInput};
use crate::checker::check_module::check_module;
use crate::ir::compile_to_ir::compile_to_ir;
use crate::codegen::js_emitter::emit_js;
use crate::diagnostic::render_diagnostics;
use std::fs;
use std::path::Path;

pub fn execute(file: &str) {
    let path = Path::new(file);
    if !path.exists() {
        eprintln!("Error: file '{}' not found", file);
        std::process::exit(1);
    }

    let source = match fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    // Stage 1: Front-end (Markdown -> TangleModule)
    let module = compile_module(CompileModuleInput {
        file: file.to_string(),
        source: source.clone(),
    });

    if !module.diagnostics.is_empty() {
        render_diagnostics(&module.diagnostics, &source, file);
    }

    // Stage 2: Checker (type checking)
    let checked = check_module(module);

    if !checked.diagnostics.is_empty() {
        render_diagnostics(&checked.diagnostics, &source, file);
    }

    // Stage 3: IR lowering
    let (graph, ir_diags) = compile_to_ir(&checked);
    if !ir_diags.is_empty() {
        render_diagnostics(&ir_diags, &source, file);
    }

    // Stage 4: Codegen (IR -> JS)
    let module_name = Path::new(file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");
    let js = emit_js(&graph, module_name);
    println!("{}", js);
}
