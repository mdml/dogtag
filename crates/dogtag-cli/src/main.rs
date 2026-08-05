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
//! Four surfaces:
//!
//! - `dogtag version` — the SDK's version, and nothing else.
//! - `dogtag doctor` — a vault's configuration health check. It opens exactly
//!   two files, writes nothing anywhere, and never refuses to run.
//! - `dogtag contract explain` — the resolved contract rendered as the
//!   instructions an agent follows, and a refusal when it did not resolve.
//! - `dogtag check` — the whole corpus's health: every finding, in order,
//!   with summary counts.
//! - `dogtag list` — body-free corpus summaries with composable SDK-owned
//!   filters.
//! - `dogtag show` — one note's SDK-rendered document model.
//!
//! Exit codes are `0`, `1` and `2`, and no more. Severity alone decides `0`
//! from `1`; `2` is reserved for an argument-parsing failure that produces no
//! diagnostic at all, which is why an unregistered `--vault work` exits `1`
//! even though it arrives as a bad argument.

#![forbid(unsafe_code)]

mod check;
mod doctor;
mod environment;
mod exit;
mod explain;
mod listing;
mod output;
mod preflight;
mod select;
mod show;

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
    /// Report the corpus's health: every finding, with summary counts.
    Check(CheckArgs),
    /// Enumerate a vault's notes with composable filters.
    List(ListArgs),
    /// Render one note's document model.
    Show(ShowArgs),
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
struct CheckArgs {
    #[command(flatten)]
    vault: VaultArg,
    /// The report's format.
    #[arg(long, value_enum, default_value_t = CheckFormat::Text)]
    format: CheckFormat,
    /// Treat warnings as failures — for the exit code only, changing no
    /// rendering and no severity. An `info` finding is never promoted.
    #[arg(long)]
    strict: bool,
}

/// How `check` reports.
#[derive(Clone, Copy, ValueEnum)]
enum CheckFormat {
    /// The summary on stdout; findings as diagnostics on stderr.
    Text,
    /// One structured report carrying the findings and their summary.
    Json,
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
    /// Treat warnings as failures — for the exit code only, changing no
    /// rendering and no severity.
    ///
    /// This surface needs it more than `doctor` does: a nested vault is a
    /// warning, and it means the rendering an agent is about to follow as
    /// instructions came from a different corpus than the one intended.
    #[arg(long)]
    strict: bool,
}

#[derive(Args)]
struct ListArgs {
    #[command(flatten)]
    vault: VaultArg,
    /// The report's format.
    #[arg(long, value_enum, default_value_t = ListFormat::Text)]
    format: ListFormat,
    /// Treat warnings as failures for the exit code only.
    #[arg(long)]
    strict: bool,
    /// Match the bound type exactly.
    #[arg(long = "type", value_name = "NAME")]
    type_name: Option<String>,
    /// Match one literal, complete tag exactly (not a namespace prefix).
    #[arg(long, value_name = "TAG")]
    tag: Option<String>,
    /// Match the declared lifecycle axis value exactly.
    #[arg(long, value_name = "VALUE", conflicts_with = "ordinary")]
    lifecycle: Option<String>,
    /// Match the ordinary state in its declared encoding.
    #[arg(long, conflicts_with = "lifecycle")]
    ordinary: bool,
}

#[derive(Args)]
struct ShowArgs {
    /// A vault-relative path (any `/` or a trailing `.md`) or an unambiguous bare name.
    #[arg(value_name = "REF")]
    reference: String,
    #[command(flatten)]
    vault: VaultArg,
    /// The rendering's format.
    #[arg(long, value_enum, default_value_t = ShowFormat::Text)]
    format: ShowFormat,
    /// Treat warnings as failures — for the exit code only.
    #[arg(long)]
    strict: bool,
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

#[derive(Clone, Copy, ValueEnum)]
enum ListFormat {
    /// One tab-separated summary per line.
    Text,
    /// One structured report carrying summaries and diagnostics.
    Json,
}

/// How `show` renders the document model.
#[derive(Clone, Copy, ValueEnum)]
enum ShowFormat {
    /// SDK-owned human-readable text.
    Text,
    /// The structured result and diagnostic envelope.
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
        Command::Doctor(args) => refuse_empty(environment, args.vault.requested())
            .unwrap_or_else(|| doctor::run(environment, &args)),
        Command::Check(args) => refuse_empty(environment, args.vault.requested())
            .unwrap_or_else(|| check::run(environment, &args)),
        Command::List(args) => refuse_empty(environment, args.vault.requested())
            .unwrap_or_else(|| listing::run(environment, &args)),
        Command::Show(args) => refuse_empty(environment, args.vault.requested())
            .unwrap_or_else(|| show::run(environment, &args)),
        Command::Contract {
            command: ContractCommand::Explain(args),
        } => refuse_empty(environment, args.vault.requested())
            .unwrap_or_else(|| explain::run(environment, &args)),
    }
}

/// Refuses a vault selector that was given but empty.
///
/// An empty selector names no vault, so there is nothing to diagnose and no
/// diagnostic to weigh: it is clap's kind of fault and takes clap's code. The
/// alternative is worse than an error — an empty `DOGTAG_VAULT` treated as
/// unset falls through to discovery and silently resolves whatever vault the
/// working directory sits in.
fn refuse_empty(environment: &Environment, flag: Option<&str>) -> Option<i32> {
    let source = select::empty_selector(flag, environment)?;
    eprintln!("error: {source} is empty; it names a registered vault or a path");
    Some(exit::USAGE)
}

/// Prints the SDK's version, which is the only version this crate knows.
fn version(environment: &Environment) -> i32 {
    let rendering = format!("dogtag {}\n", dogtag::version());
    output::to_stdout(environment, Rendering::verbatim(&rendering));
    exit::SUCCESS
}
