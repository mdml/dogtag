//! `dogtag check` — a thin stream-routing consumer of the SDK's corpus report.

use dogtag::installation::Installation;
use dogtag::note::read_corpus;
use dogtag::report::{check_json, check_report, check_text};

use dogtag::diagnostic::render_plain;

use crate::environment::Environment;
use crate::exit;
use crate::output::{self, Rendering};
use crate::preflight::{prepare, refuse_selection, refuse_unresolved};
use crate::select::{Selected, select};
use crate::{CheckArgs, CheckFormat};

const POINTER: &str = "\nthe contract did not resolve, so the corpus cannot be checked: run \
                       `dogtag doctor` for the vault's full diagnosis\n";

pub fn run(environment: &Environment, args: &CheckArgs) -> i32 {
    let installation = environment.installation();
    match select(environment, args.vault.requested(), &installation) {
        Ok(selected) => check(environment, args, selected, installation),
        Err(refused) => refuse_selection(environment, &refused.diagnostics, args.strict),
    }
}

fn check(
    environment: &Environment,
    args: &CheckArgs,
    selected: Selected,
    installation: Installation,
) -> i32 {
    let prepared = prepare(environment, selected, installation);
    let Ok(contract) = prepared.opened.contract() else {
        return refuse_unresolved(environment, &prepared, POINTER);
    };
    let corpus = read_corpus(prepared.opened.root(), contract);
    let report = check_report(&corpus, &prepared.diagnostics);
    match args.format {
        CheckFormat::Text => {
            output::to_stdout(environment, Rendering::verbatim(&check_text(&report)));
            output::to_stderr(
                environment,
                Rendering::diagnostics(&render_plain(report.diagnostics())),
            );
        }
        CheckFormat::Json => {
            output::to_stdout(environment, Rendering::verbatim(&check_json(&report)));
        }
    }
    exit::code_for(report.diagnostics(), args.strict)
}
