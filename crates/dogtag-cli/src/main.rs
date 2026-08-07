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
//! Nine surfaces, one of which writes:
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
//! - `dogtag search` — lexical retrieval over the corpus, composed with
//!   `list`'s filters.
//! - `dogtag find` — entity lookup: the one note a name resolves to.
//! - `dogtag capture` — the one mutation: a thought becomes a note.
//!
//! Exit codes are `0`, `1` and `2`, and no more. For every **read** verb,
//! severity alone decides `0` from `1`; `2` is reserved for an
//! argument-parsing failure that produces no diagnostic at all, which is why
//! an unregistered `--vault work` exits `1` even though it arrives as a bad
//! argument. A **write** verb answers a different question — *did my act
//! land* — and its code follows the transaction's verdict instead, which is
//! why `capture` carries no `--strict`. See [`exit`].

#![forbid(unsafe_code)]

mod capture;
mod check;
mod doctor;
mod environment;
mod exit;
mod explain;
mod find;
mod listing;
mod output;
mod preflight;
mod search;
mod select;
mod show;

use std::path::PathBuf;
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
    /// Every verb that opens a vault, flattened to the same level.
    ///
    /// A type of its own rather than seven more variants here, because these
    /// seven share a rule the other two do not: each takes a vault selector,
    /// and each is refused identically when the selector is empty. Flattened,
    /// and declared where those seven were declared, so the command line a
    /// caller types and the order `--help` lists them in are both unchanged.
    #[command(flatten)]
    Vault(VaultCommand),
    /// Work with the vault's committed contract.
    Contract {
        #[command(subcommand)]
        command: ContractCommand,
    },
}

/// A verb that opens a vault.
#[derive(Subcommand)]
enum VaultCommand {
    /// Report a vault's configuration health.
    Doctor(DoctorArgs),
    /// Report the corpus's health: every finding, with summary counts.
    Check(CheckArgs),
    /// Enumerate a vault's notes with composable filters.
    List(ListArgs),
    /// Render one note's document model.
    Show(ShowArgs),
    /// Search the corpus: bodies, paths, titles, and aliases.
    Search(SearchArgs),
    /// Find the one note a name resolves to.
    Find(FindArgs),
    /// Capture a thought as a new, unfiled note.
    Capture(CaptureArgs),
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
struct SearchArgs {
    /// The query: bare words are OR-combined and relevance-ranked, "quoted
    /// phrases" match adjacent words in order, and a trailing * is a prefix
    /// wildcard. Explicit AND, date bounds, tag-text matching, and
    /// link-target matching are not part of the grammar.
    #[arg(value_name = "QUERY")]
    query: String,
    #[command(flatten)]
    vault: VaultArg,
    /// The report's format.
    #[arg(long, value_enum, default_value_t = SearchFormat::Text)]
    format: SearchFormat,
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
    /// Keep at most this many hits, best-ranked first.
    #[arg(long, value_name = "N", default_value_t = 20)]
    limit: usize,
}

/// How `search` reports.
#[derive(Clone, Copy, ValueEnum)]
enum SearchFormat {
    /// One tab-separated hit per line.
    Text,
    /// One structured report carrying hits and diagnostics.
    Json,
}

#[derive(Args)]
struct FindArgs {
    /// The name to find: a case-insensitive match over note names and
    /// aliases, or an exact vault-relative path (any `/` or a trailing
    /// `.md`). An ambiguous name is an error whose diagnostic lists every
    /// candidate.
    #[arg(value_name = "NAME")]
    name: String,
    #[command(flatten)]
    vault: VaultArg,
    /// The report's format.
    #[arg(long, value_enum, default_value_t = FindFormat::Text)]
    format: FindFormat,
    /// Treat warnings as failures for the exit code only.
    #[arg(long)]
    strict: bool,
    /// Narrow the match to notes of this bound type.
    #[arg(long = "type", value_name = "NAME")]
    type_name: Option<String>,
}

/// How `find` reports.
#[derive(Clone, Copy, ValueEnum)]
enum FindFormat {
    /// The found note as one tab-separated summary line.
    Text,
    /// One structured report carrying the note and diagnostics.
    Json,
}

/// What `capture` creates a note from, and how it reports.
///
/// No `--strict`, deliberately. Strictness promotes warnings for the exit code
/// alone, and a write verb's exit code is its transaction's verdict rather than
/// a weighing of severities — so the flag would have nothing to promote and
/// would suggest that a corpus finding could fail a capture that landed.
#[derive(Args)]
#[command(group = clap::ArgGroup::new("thought").required(true).args(["text", "file"]))]
struct CaptureArgs {
    /// The thought to capture. `-` reads it from standard input instead.
    #[arg(value_name = "TEXT")]
    text: Option<String>,
    /// Read the thought from this file rather than from an argument.
    #[arg(long, value_name = "PATH")]
    file: Option<PathBuf>,
    #[command(flatten)]
    vault: VaultArg,
    /// The report's format.
    #[arg(long, value_enum, default_value_t = CaptureFormat::Text)]
    format: CaptureFormat,
    /// Emit the plan and write nothing.
    #[arg(long)]
    preview: bool,
    /// Who is capturing, overriding the installation record's actor.
    #[arg(long, value_name = "NAME")]
    actor: Option<String>,
    /// In what capacity this capture is being performed.
    #[arg(long, value_enum, default_value_t = CaptureProvenance::Human)]
    provenance: CaptureProvenance,
}

/// How `capture` reports.
#[derive(Clone, Copy, ValueEnum)]
enum CaptureFormat {
    /// What the act did, and how to undo it; findings on stderr.
    Text,
    /// One structured document carrying the plan, the outcome, and the
    /// diagnostics.
    Json,
}

/// In what capacity an act is performed, as the invocation spells it.
///
/// The SDK's closed set, spelled once here so the CLI's help text and the
/// commit trailer cannot drift apart.
#[derive(Clone, Copy, ValueEnum)]
enum CaptureProvenance {
    /// A person, acting directly.
    Human,
    /// An agent acting on a person's behalf.
    Agent,
    /// A scheduled or triggered process.
    Automation,
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
        Command::Vault(command) => vault(environment, command),
        Command::Contract { command } => contract(environment, command),
    }
}

/// Runs one vault verb, behind the empty-selector refusal they all share.
///
/// The refusal is stated once, before the verb is chosen, rather than wrapped
/// around each of seven calls: it is the same refusal for the same reason every
/// time, and seven copies of it is seven places for one of them to drift.
fn vault(environment: &Environment, command: VaultCommand) -> i32 {
    if let Some(refused) = refuse_empty(environment, command.requested()) {
        return refused;
    }
    match command {
        VaultCommand::Doctor(args) => doctor::run(environment, &args),
        VaultCommand::Check(args) => check::run(environment, &args),
        VaultCommand::List(args) => listing::run(environment, &args),
        VaultCommand::Show(args) => show::run(environment, &args),
        VaultCommand::Search(args) => search::run(environment, &args),
        VaultCommand::Find(args) => find::run(environment, &args),
        VaultCommand::Capture(args) => capture::run(environment, &args),
    }
}

impl VaultCommand {
    /// The vault selector this verb was given, as it was given.
    fn requested(&self) -> Option<&str> {
        match self {
            Self::Doctor(args) => args.vault.requested(),
            Self::Check(args) => args.vault.requested(),
            Self::List(args) => args.vault.requested(),
            Self::Show(args) => args.vault.requested(),
            Self::Search(args) => args.vault.requested(),
            Self::Find(args) => args.vault.requested(),
            Self::Capture(args) => args.vault.requested(),
        }
    }
}

/// Runs a `contract` subcommand, behind the same empty-selector refusal.
fn contract(environment: &Environment, command: ContractCommand) -> i32 {
    let ContractCommand::Explain(args) = command;
    refuse_empty(environment, args.vault.requested())
        .unwrap_or_else(|| explain::run(environment, &args))
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
