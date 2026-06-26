use crate::frontend::compile_module::{compile_module, CompileModuleInput};
use crate::checker::check_module::check_module;
use crate::ir::compile_to_ir::compile_to_ir;
use crate::diagnostic::render_diagnostics;
use std::fs;
use std::path::Path;

pub struct RunOptions {
    pub file: String,
    pub emit_ir: bool,
    pub target: String,
}

pub fn execute(opts: RunOptions) {
    let path = Path::new(&opts.file);
    if !path.exists() {
        eprintln!("Error: file '{}' not found", opts.file);
        std::process::exit(1);
    }

    let source = match fs::read_to_string(&opts.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };

    // Stage 1: Front-end (Markdown -> TangleModule)
    let module = compile_module(CompileModuleInput {
        file: opts.file.clone(),
        source: source.clone(),
    });

    if !module.diagnostics.is_empty() {
        render_diagnostics(&module.diagnostics, &source, &opts.file);
    }

    // Stage 2: Checker (type checking)
    let checked = check_module(module);

    if !checked.diagnostics.is_empty() {
        render_diagnostics(&checked.diagnostics, &source, &opts.file);
    }

    // Stage 3: IR lowering
    let (graph, ir_diags) = compile_to_ir(&checked);
    if !ir_diags.is_empty() {
        render_diagnostics(&ir_diags, &source, &opts.file);
    }

    let module_name = Path::new(&opts.file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    // Stage 4a: IR JSON export (if --emit-ir)
    if opts.emit_ir {
        println!("{}", crate::codegen::ir_json::emit_ir_json(&graph));
        return;
    }

    // Stage 4b: Codegen dispatch by target
    let code = match opts.target.as_str() {
        "js" => crate::codegen::js_emitter::emit_js(&graph, module_name),
        "py" => crate::codegen::py_emitter::emit_python(&graph, module_name),
        "go" => crate::codegen::go_emitter::emit_go(&graph, module_name),
        _ => crate::codegen::js_emitter::emit_js(&graph, module_name),
    };
    println!("{}", code);
}
