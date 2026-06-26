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
    pub incremental: bool,
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

    let module_name = Path::new(&opts.file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("main");

    // Check incremental cache if enabled
    if opts.incremental {
        let cache_dir = Path::new(".cache");
        let mut inc_cache = crate::incremental::cache::IncrementalCache::new(cache_dir);
        let ir_cache = crate::ir::cache::IrCache::new(cache_dir);
        let fp = crate::incremental::fingerprint::source_fingerprint(&source);

        if !inc_cache.needs_recompile(&opts.file, fp) {
            // File unchanged — try loading cached IR
            if let Some(cached_graph) = ir_cache.load(&opts.file) {
                if opts.emit_ir {
                    println!("{}", crate::codegen::ir_json::emit_ir_json(&cached_graph));
                    return;
                }
                let code = emit_code(&opts.target, &cached_graph, module_name);
                execute_code(&opts.target, &code);
                return;
            }
        }

        // Run full pipeline
        let graph = run_pipeline(&opts.file, &source);

        // Save IR to cache for next run
        ir_cache.save(&opts.file, &graph);

        if opts.emit_ir {
            println!("{}", crate::codegen::ir_json::emit_ir_json(&graph));
            return;
        }
        let code = emit_code(&opts.target, &graph, module_name);
        execute_code(&opts.target, &code);
    } else {
        // Non-incremental: run full pipeline every time
        let graph = run_pipeline(&opts.file, &source);

        if opts.emit_ir {
            println!("{}", crate::codegen::ir_json::emit_ir_json(&graph));
            return;
        }
        let code = emit_code(&opts.target, &graph, module_name);
        execute_code(&opts.target, &code);
    }
}

/// Run the full 3-stage compilation pipeline, return IR graph
fn run_pipeline(file: &str, source: &str) -> crate::ir::graph::RuleGraph {
    // Stage 1: Front-end
    let module = compile_module(CompileModuleInput {
        file: file.to_string(),
        source: source.to_string(),
    });
    if !module.diagnostics.is_empty() {
        render_diagnostics(&module.diagnostics, source, file);
    }

    // Stage 2: Checker
    let checked = check_module(module);
    if !checked.diagnostics.is_empty() {
        render_diagnostics(&checked.diagnostics, source, file);
    }

    // Stage 3: IR
    let (graph, ir_diags) = compile_to_ir(&checked);
    if !ir_diags.is_empty() {
        render_diagnostics(&ir_diags, source, file);
    }

    graph
}

/// Execute generated code via host runtime
fn execute_code(target: &str, code: &str) {
    match target {
        "js" => {
            let mut child = std::process::Command::new("node")
                .arg("-e")
                .arg(code)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()
                .expect("Failed to execute node. Is Node.js installed?");
            let _ = child.wait();
        }
        "py" => {
            let mut child = std::process::Command::new("python3")
                .arg("-c")
                .arg(code)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn()
                .expect("Failed to execute python3");
            let _ = child.wait();
        }
        "go" => {
            // For Go, write to temp file, run, then clean up
            let dir = std::env::temp_dir().join("tangle_go");
            let _ = std::fs::create_dir_all(&dir);
            let file = dir.join("main.go");
            std::fs::write(&file, code).expect("Failed to write Go temp file");
            let mut child = std::process::Command::new("go")
                .arg("run")
                .arg(&file)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .current_dir(&dir)
                .spawn()
                .expect("Failed to execute go. Is Go installed?");
            let _ = child.wait();
        }
        _ => println!("{}", code),
    }
}

/// Dispatch codegen by target language
fn emit_code(target: &str, graph: &crate::ir::graph::RuleGraph, module_name: &str) -> String {
    match target {
        "js" => crate::codegen::js_emitter::emit_js(graph, module_name),
        "py" => crate::codegen::py_emitter::emit_python(graph, module_name),
        "go" => crate::codegen::go_emitter::emit_go(graph, module_name),
        _ => crate::codegen::js_emitter::emit_js(graph, module_name),
    }
}
