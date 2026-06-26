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
    },
    /// Run tests
    Test {
        #[arg(long)]
        filter: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Run { file, emit_ir, target } => {
            cli::run::execute(cli::run::RunOptions { file, emit_ir, target });
        }
        Command::Test { filter } => cli::test::execute(filter.as_deref()),
    }
}
