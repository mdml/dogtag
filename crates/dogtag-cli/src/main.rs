//! The `dogtag` command-line interface.
//!
//! A thin consumer of the [`dogtag`] SDK's public API: the CLI composes SDK
//! operations and never independently reinterprets vault semantics. It
//! carries no version string of its own — everything it reports comes from
//! [`dogtag::version`].

use clap::{Parser, Subcommand};

/// dogtag — a PKM SDK for AI agents.
#[derive(Parser)]
#[command(
    name = "dogtag",
    version = dogtag::version(),
    about = "dogtag — a PKM SDK for AI agents",
    arg_required_else_help = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print the dogtag version.
    Version,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Version => println!("dogtag {}", dogtag::version()),
    }
}
