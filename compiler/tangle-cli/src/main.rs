use clap::{Parser, Subcommand};
use tangle_cli::cli;

#[derive(Parser)]
#[command(name = "tangle", version = env!("CARGO_PKG_VERSION"), about = "Tangle compiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a Tangle program
    Run {
        file: String,
        /// Emit IR JSON instead of target code
        #[arg(long)]
        emit_ir: bool,
        /// Target language (js, py, go) [default: js]
        #[arg(long, default_value = "js")]
        target: String,
        /// Enable incremental compilation (skip unchanged files)
        #[arg(long)]
        incremental: bool,
    },
    /// Run tests
    Test {
        #[arg(long)]
        filter: Option<String>,
    },
    /// Start LSP server (stdio)
    Lsp,
    /// Generate documentation HTML
    Doc {
        file: String,
        #[arg(long)]
        output: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Run { file, emit_ir, target, incremental } => {
            cli::run::execute(cli::run::RunOptions { file, emit_ir, target, incremental });
        }
        Command::Test { filter } => cli::test::execute(filter.as_deref()),
        Command::Lsp => {
            let mut server = tangle_cli::lsp::LspServer::new();
            if let Err(e) = server.run() {
                eprintln!("LSP error: {}", e);
            }
        }
        Command::Doc { file, output } => {
            use tangle_cli::frontend::compile_module::{compile_module, CompileModuleInput};
            let source = std::fs::read_to_string(&file).unwrap_or_default();
            let module = compile_module(CompileModuleInput {
                file: file.clone(),
                source,
            });
            let html = tangle_cli::docgen::generate_doc_html(&module, &file);
            if let Some(out) = output {
                let _ = std::fs::write(&out, html);
                println!("Documentation written to {}", out);
            } else {
                println!("{}", html);
            }
        }
    }
}
