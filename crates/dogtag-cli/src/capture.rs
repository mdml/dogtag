//! `dogtag capture` — a thin stream-routing consumer of the SDK's write
//! transaction.
//!
//! What this module owns is exactly what the SDK's pure functions refuse to
//! consult on a caller's behalf: which of the three inputs the thought arrives
//! by, the clock the capture's identity derives from, who is acting and in what
//! capacity, and which stream each rendering goes to. The transaction itself,
//! the bytes it writes, the name it writes them under and the commit it makes
//! are all the SDK's.
//!
//! **The exit code is the transaction's verdict, not the run's severity**, and
//! that is a deliberate departure from the rule every read verb follows. See
//! [`crate::exit::for_write`].

use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::time::SystemTime;

use dogtag::diagnostic::{DiagnosticList, render_plain};
use dogtag::installation::Installation;
use dogtag::report::{capture_json, capture_text};
use dogtag::write::{
    Actor, CaptureRequest, CapturedAt, ProvenanceKind, WriteResult, capture, plan_capture,
};

use crate::environment::Environment;
use crate::exit;
use crate::output::{self, Rendering};
use crate::preflight::{Prepared, prepare, refuse_selection, refuse_unresolved};
use crate::select::{Selected, select};
use crate::{CaptureArgs, CaptureFormat, CaptureProvenance};

const POINTER: &str = "\nthe contract did not resolve, so nothing can be captured into this \
                       vault: run `dogtag doctor` for the vault's full diagnosis\n";

/// The argument that means *the thought is on standard input*.
const STDIN: &str = "-";

pub fn run(environment: &Environment, args: &CaptureArgs) -> i32 {
    let installation = environment.installation();
    match select(environment, args.vault.requested(), &installation) {
        Ok(selected) => run_selected(environment, args, selected, installation),
        // A selection that named no usable vault refuses the act. There is no
        // `--strict` on a write verb to promote anything, so the flag is
        // false: severity decides nothing here.
        Err(refused) => refuse_selection(environment, &refused.diagnostics, false),
    }
}

fn run_selected(
    environment: &Environment,
    args: &CaptureArgs,
    selected: Selected,
    installation: Installation,
) -> i32 {
    let Some(text) = thought(args) else {
        return exit::USAGE;
    };
    let actor = Actor::new(named(args, &installation), args.provenance.kind());
    let request = CaptureRequest::new(text, CapturedAt::at(SystemTime::now()), actor);
    let prepared = prepare(environment, selected, installation);
    let Ok(contract) = prepared.opened.contract() else {
        return refuse_unresolved(environment, &prepared, POINTER);
    };
    let act = if args.preview { plan_capture } else { capture };
    let result = act(prepared.opened.root(), contract, &request);
    render(environment, args, &prepared, &result)
}

/// The thought, from whichever of the three inputs carries it.
///
/// `None` is a fault that produced no diagnostic, because nothing was read and
/// so there is nothing to say about a vault: it is the same kind of thing as an
/// argument clap refuses, and takes the same code.
fn thought(args: &CaptureArgs) -> Option<String> {
    match args.file.as_deref() {
        Some(path) => from_file(path),
        // One of the two is always present — clap requires the group — so the
        // fallback is the spelling that means standard input rather than a
        // case this reaches.
        None => spelled(args.text.as_deref().unwrap_or(STDIN)),
    }
}

/// The thought a positional argument carries, or the one standard input does.
fn spelled(text: &str) -> Option<String> {
    if text == STDIN {
        return from_stdin();
    }
    Some(text.to_owned())
}

fn from_file(path: &Path) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) => {
            eprintln!("error: `{}` could not be read: {error}", path.display());
            None
        }
    }
}

/// The thought, read from standard input in full.
///
/// Read as bytes into a string rather than by lines, because the captured text
/// becomes the body byte for byte: reading by lines would decide what a line
/// ending is, and deciding that is exactly what a writing surface must not do
/// to bytes it did not semantically touch.
fn from_stdin() -> Option<String> {
    let mut text = String::new();
    match io::stdin().read_to_string(&mut text) {
        Ok(_) => Some(text),
        Err(error) => {
            eprintln!("error: standard input could not be read: {error}");
            None
        }
    }
}

/// Who is acting: the invocation's actor where it names one, and the
/// installation record's otherwise.
///
/// Local narrows, never defines. The invocation may name someone the record
/// does not — an agent acting for a person, a shared machine — and the record
/// is where an unconfigured installation's silence comes from. `None` from both
/// is a state rather than a fault, and the SDK warns about it rather than
/// refusing.
fn named(args: &CaptureArgs, installation: &Installation) -> Option<String> {
    args.actor
        .clone()
        .or_else(|| Some(installation.record()?.actor()?.name().to_owned()))
}

/// Routes the two renderings and answers with the transaction's verdict.
fn render(
    environment: &Environment,
    args: &CaptureArgs,
    prepared: &Prepared,
    result: &WriteResult,
) -> i32 {
    let mut diagnostics = DiagnosticList::new();
    diagnostics.extend(prepared.diagnostics.iter().cloned());
    diagnostics.extend(result.diagnostics().iter().cloned());
    let diagnostics = diagnostics.sorted();
    match args.format {
        CaptureFormat::Text => {
            output::to_stdout(environment, Rendering::verbatim(&capture_text(result)));
            output::to_stderr(
                environment,
                Rendering::diagnostics(&render_plain(&diagnostics)),
            );
        }
        // One JSON document on standard output and nothing else, so a consumer
        // piping it receives valid JSON or nothing at all — and it carries the
        // *whole* run, the loading path included. A structured document that
        // dropped what only the text run showed would answer a different
        // question from the run beside it, and the structured one is what a
        // parallel week diffs.
        CaptureFormat::Json => {
            output::to_stdout(
                environment,
                Rendering::verbatim(&capture_json(result, &prepared.diagnostics)),
            );
        }
    }
    exit::for_write(result.landed())
}

impl CaptureProvenance {
    /// The SDK's kind this spelling names.
    fn kind(self) -> ProvenanceKind {
        match self {
            Self::Human => ProvenanceKind::Human,
            Self::Agent => ProvenanceKind::Agent,
            Self::Automation => ProvenanceKind::Automation,
        }
    }
}
