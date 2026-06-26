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
    Run { file: String },
    /// Run tests
    Test {
        #[arg(long)]
        filter: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Run { file } => cli::run::execute(&file),
        Command::Test { filter } => cli::test::execute(filter.as_deref()),
    }
}
