//! `dogtag show` — one note's SDK-owned document-model rendering.

use dogtag::diagnostic::{Diagnostic, DiagnosticList, render_plain};
use dogtag::installation::Installation;
use dogtag::note::read_corpus;
use dogtag::report::{ShowReport, show_json, show_report, show_text};
use dogtag::vault::{Opened, inspect_root_trust, open};

use crate::environment::Environment;
use crate::output::{self, Rendering};
use crate::select::{Selected, select};
use crate::{ShowArgs, ShowFormat, exit};

const POINTER: &str = "\nthe contract did not resolve, so no note can be shown: run \
                       `dogtag doctor` for the vault's full diagnosis\n";

struct Prepared {
    opened: Opened,
    diagnostics: Vec<Diagnostic>,
}

pub fn run(environment: &Environment, args: &ShowArgs) -> i32 {
    let installation = environment.installation();
    match select(environment, args.vault.requested(), &installation) {
        Ok(selected) => show(environment, args, selected, installation),
        Err(refused) => refuse(environment, &refused.diagnostics, args.strict),
    }
}

fn show(
    environment: &Environment,
    args: &ShowArgs,
    selected: Selected,
    installation: Installation,
) -> i32 {
    let prepared = prepare(environment, selected, installation);
    let Ok(contract) = prepared.opened.contract() else {
        output::to_stderr(
            environment,
            Rendering::diagnostics(&render_plain(&prepared.diagnostics)),
        );
        output::to_stderr(environment, Rendering::verbatim(POINTER));
        return exit::FAILURE;
    };
    let corpus = read_corpus(prepared.opened.root(), contract);
    let report = show_report(&corpus, contract, &args.reference, &prepared.diagnostics);
    deliver(environment, args, &report)
}

fn prepare(environment: &Environment, selected: Selected, installation: Installation) -> Prepared {
    let mut collected = DiagnosticList::new();
    collected.extend(selected.diagnostics);
    collected.extend(inspect_root_trust(&selected.root, environment.home()));
    let opened = open(selected.root, installation);
    collected.extend(opened.diagnostics().iter().cloned());
    Prepared {
        opened,
        diagnostics: collected.sorted(),
    }
}

fn deliver(environment: &Environment, args: &ShowArgs, report: &ShowReport) -> i32 {
    match args.format {
        ShowFormat::Text => {
            let result = show_text(report);
            output::to_stdout(environment, Rendering::verbatim(&result));
            output::to_stderr(
                environment,
                Rendering::diagnostics(&render_plain(report.diagnostics())),
            );
        }
        ShowFormat::Json => {
            let result = show_json(report);
            output::to_stdout(environment, Rendering::verbatim(&result));
        }
    }
    exit::code_for(report.diagnostics(), args.strict)
}

fn refuse(environment: &Environment, diagnostics: &[Diagnostic], strict: bool) -> i32 {
    output::to_stderr(
        environment,
        Rendering::diagnostics(&render_plain(diagnostics)),
    );
    exit::code_for(diagnostics, strict)
}
