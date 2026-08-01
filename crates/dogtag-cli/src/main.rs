//! The `dogtag` command-line interface.
//!
//! A thin consumer of the [`dogtag`] SDK's public API: every vault behavior
//! lives in the SDK, and this crate never independently reinterprets vault
//! semantics. What it owns is exactly what the SDK's pure functions refuse to
//! consult on a caller's behalf — argv, the environment, the current
//! directory, which installation record to read, colour, which stream a
//! rendering goes to, and the mapping from severity to an exit code. It
//! carries no version string of its own: everything it reports comes from
//! [`dogtag::version`].
//!
//! Three surfaces:
//!
//! - `dogtag version` — the SDK's version, and nothing else.
//! - `dogtag doctor` — a vault's configuration health check. It opens exactly
//!   two files, writes nothing anywhere, and never refuses to run.
//! - `dogtag contract explain` — the resolved contract rendered as the
//!   instructions an agent follows, and a refusal when it did not resolve.
//!
//! Exit codes are `0`, `1` and `2`, and no more. Severity alone decides `0`
//! from `1`; `2` is reserved for an argument-parsing failure that produces no
//! diagnostic at all, which is why an unregistered `--vault work` exits `1`
//! even though it arrives as a bad argument.

#![forbid(unsafe_code)]

mod doctor;
mod environment;
mod exit;
mod explain;
mod output;
mod select;

use std::process;

use clap::{Args, Parser, Subcommand, ValueEnum};

use environment::Environment;
use output::Rendering;

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
    /// Report a vault's configuration health.
    Doctor(DoctorArgs),
    /// Work with the vault's committed contract.
    Contract {
        #[command(subcommand)]
        command: ContractCommand,
    },
}

#[derive(Subcommand)]
enum ContractCommand {
    /// Render the resolved contract as agent-readable instructions.
    Explain(ExplainArgs),
}

/// Which vault a command is about.
///
/// One flag rather than two, because a registry name and a path are one
/// concept — *which vault* — distinguished by the argument's own syntax with
/// no fallback between them. See [`select`].
#[derive(Args)]
struct VaultArg {
    /// The vault: a registered name, or a path (anything holding a separator
    /// or beginning with `.`, `/` or `~`). Defaults to upward discovery from
    /// the current directory.
    #[arg(long, value_name = "NAME-OR-PATH")]
    vault: Option<String>,
}

impl VaultArg {
    /// The argument as it was given, when it was given.
    ///
    /// It is never rewritten here: a `~` is not expanded into a home
    /// directory, because expansion is the shell's job and doing it here would
    /// make this flag inconsistent with the registry's own no-expansion rule.
    fn requested(&self) -> Option<&str> {
        self.vault.as_deref()
    }
}

#[derive(Args)]
struct DoctorArgs {
    #[command(flatten)]
    vault: VaultArg,
    /// The report's format.
    #[arg(long, value_enum, default_value_t = DoctorFormat::Text)]
    format: DoctorFormat,
    /// Treat warnings as failures — for the exit code only, changing no
    /// rendering and no severity.
    #[arg(long)]
    strict: bool,
}

#[derive(Args)]
struct ExplainArgs {
    #[command(flatten)]
    vault: VaultArg,
    /// The rendering's format.
    #[arg(long, value_enum, default_value_t = ExplainFormat::Markdown)]
    format: ExplainFormat,
    /// Annotate each rendered value with its source and location. The JSON
    /// always carries provenance, so this flag is the Markdown's.
    #[arg(long)]
    provenance: bool,
}

/// How `doctor` reports.
#[derive(Clone, Copy, ValueEnum)]
enum DoctorFormat {
    /// A human report.
    Text,
    /// The structured report, under the SDK's output schema.
    Json,
}

/// How `contract explain` renders.
#[derive(Clone, Copy, ValueEnum)]
enum ExplainFormat {
    /// The generated agent contract.
    Markdown,
    /// The complete resolved model, always carrying provenance.
    Json,
}

fn main() {
    let command = Cli::parse().command;
    process::exit(dispatch(&Environment::from_process(), command));
}

/// Runs one command and answers with its exit code.
///
/// The environment is resolved once, before any command runs, so that the
/// ambient facts a run depends on are fixed for the whole of it.
fn dispatch(environment: &Environment, command: Command) -> i32 {
    match command {
        Command::Version => version(environment),
        Command::Doctor(args) => doctor::run(environment, &args),
        Command::Contract {
            command: ContractCommand::Explain(args),
        } => explain::run(environment, &args),
    }
}

/// Prints the SDK's version, which is the only version this crate knows.
fn version(environment: &Environment) -> i32 {
    let rendering = format!("dogtag {}\n", dogtag::version());
    output::to_stdout(environment, Rendering::verbatim(&rendering));
    exit::SUCCESS
}
