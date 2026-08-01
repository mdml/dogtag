//! `dogtag contract explain` — the vault's rules, as an agent receives them.
//!
//! The Markdown rendering is the generated agent contract: the artifact whose
//! whole purpose is that it cannot drift from the contract it is generated
//! from. It is the SDK's, not this crate's, so that the CLI, the MCP server
//! and a language binding cannot each grow their own and hand an agent a
//! different contract depending on which door it entered by.
//!
//! **When the contract does not resolve, this refuses.** Explaining a contract
//! that did not resolve would be a fiction handed to an agent as the vault's
//! rules, so nothing at all reaches standard output, the diagnostics go to
//! standard error, and the refusal points at `doctor`, which is the surface
//! that reports on a broken vault.
//!
//! At this milestone it writes to standard output only. Generating a vault's
//! agent instructions *on disk* is a write, and belongs to the milestone that
//! performs writes.

use dogtag::contract::Contract;
use dogtag::diagnostic::{Diagnostic, DiagnosticList, render_plain};
use dogtag::installation::Installation;
use dogtag::report::{contract_json, contract_markdown};
use dogtag::vault::{Opened, VaultRoot, inspect_root_trust, open};

use crate::environment::Environment;
use crate::output::{self, Rendering};
use crate::select::{Selected, select};
use crate::{ExplainArgs, ExplainFormat, exit};

/// What a reader is pointed at when there is nothing to explain.
///
/// The blank line before it separates it from the last diagnostic block, of
/// which there is always at least one: a contract that did not resolve always
/// says why.
const POINTER: &str = "\nthe contract did not resolve, so there is nothing to explain: run \
                       `dogtag doctor` for the vault's full diagnosis\n";

/// An opened vault and everything the run had to say about it.
struct Explained {
    opened: Opened,
    diagnostics: Vec<Diagnostic>,
}

/// Renders the selected vault's contract, and answers with the exit code.
pub fn run(environment: &Environment, args: &ExplainArgs) -> i32 {
    let installation = environment.installation();
    match select(environment, args.vault.requested(), &installation) {
        Ok(selected) => explain(environment, args, selected, installation),
        Err(diagnostics) => refuse(environment, &diagnostics, args.strict),
    }
}

/// Renders the contract, or refuses because there is none to render.
fn explain(
    environment: &Environment,
    args: &ExplainArgs,
    selected: Selected,
    installation: Installation,
) -> i32 {
    let explained = explained(environment, selected, installation);
    let rendering = explained
        .opened
        .contract()
        .ok()
        .map(|contract| render(args, explained.opened.root(), contract));
    output::to_stderr(
        environment,
        Rendering::diagnostics(&render_plain(&explained.diagnostics)),
    );
    match rendering {
        Some(text) => delivered(environment, &text, &explained.diagnostics, args.strict),
        None => refused(environment),
    }
}

/// Opens the vault and collects everything said about it into one order.
fn explained(
    environment: &Environment,
    selected: Selected,
    installation: Installation,
) -> Explained {
    let mut collected = DiagnosticList::new();
    collected.extend(selected.diagnostics);
    // Live here as much as in `doctor`, and for this command's own reason:
    // this is the surface that renders a contract as instructions, so a
    // contract planted above where the command ran is the case the warning
    // exists for.
    collected.extend(inspect_root_trust(&selected.root, environment.home()));
    let opened = open(selected.root, installation);
    collected.extend(opened.diagnostics().iter().cloned());
    Explained {
        opened,
        diagnostics: collected.sorted(),
    }
}

/// The contract in the format that was asked for.
fn render(args: &ExplainArgs, root: &VaultRoot, contract: &Contract) -> String {
    match args.format {
        ExplainFormat::Markdown => contract_markdown(root, contract, args.provenance),
        ExplainFormat::Json => contract_json(root, contract),
    }
}

/// The rendering, written to standard output and weighed by what was raised.
///
/// The contract resolved, so the rendering is not a fiction — but a run that
/// raised an error is a run that failed, and severity alone decides that.
fn delivered(
    environment: &Environment,
    rendering: &str,
    diagnostics: &[Diagnostic],
    strict: bool,
) -> i32 {
    output::to_stdout(environment, Rendering::verbatim(rendering));
    exit::code_for(diagnostics, strict)
}

/// The refusal: nothing on standard output, and the way forward on standard
/// error.
fn refused(environment: &Environment) -> i32 {
    output::to_stderr(environment, Rendering::verbatim(POINTER));
    exit::FAILURE
}

/// A vault that could not be resolved at all.
fn refuse(environment: &Environment, diagnostics: &[Diagnostic], strict: bool) -> i32 {
    output::to_stderr(
        environment,
        Rendering::diagnostics(&render_plain(diagnostics)),
    );
    exit::code_for(diagnostics, strict)
}
