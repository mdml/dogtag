//! `dogtag list` — a thin stream-routing consumer of the SDK query.

use dogtag::diagnostic::{DiagnosticList, render_plain};
use dogtag::installation::Installation;
use dogtag::note::{ListFilter, list};
use dogtag::report::{list_json, list_text};

use crate::environment::Environment;
use crate::exit;
use crate::output::{self, Rendering};
use crate::preflight::{prepare, refuse_selection, refuse_unresolved};
use crate::select::{Selected, select};
use crate::{ListArgs, ListFormat};

const POINTER: &str = "\nthe contract did not resolve, so notes cannot be listed: run `dogtag doctor` for the vault's full diagnosis\n";

pub fn run(environment: &Environment, args: &ListArgs) -> i32 {
    let installation = environment.installation();
    match select(environment, args.vault.requested(), &installation) {
        Ok(selected) => run_selected(environment, args, selected, installation),
        Err(refused) => refuse_selection(environment, &refused.diagnostics, args.strict),
    }
}

fn run_selected(
    environment: &Environment,
    args: &ListArgs,
    selected: Selected,
    installation: Installation,
) -> i32 {
    let prepared = prepare(environment, selected, installation);
    let Ok(contract) = prepared.opened.contract() else {
        return refuse_unresolved(environment, &prepared, POINTER);
    };
    let filter = ListFilter {
        type_name: args.type_name.clone(),
        tag: args.tag.clone(),
        lifecycle: args.lifecycle.clone(),
        ordinary: args.ordinary,
    };
    let result = list(prepared.opened.root(), contract, &filter);
    let mut diagnostics = DiagnosticList::new();
    diagnostics.extend(prepared.diagnostics);
    diagnostics.extend(result.diagnostics().iter().cloned());
    let diagnostics = diagnostics.sorted();
    match args.format {
        ListFormat::Text => {
            output::to_stdout(environment, Rendering::verbatim(&list_text(&result)));
            output::to_stderr(
                environment,
                Rendering::diagnostics(&render_plain(&diagnostics)),
            );
        }
        ListFormat::Json => output::to_stdout(
            environment,
            Rendering::verbatim(&list_json(&result, &diagnostics)),
        ),
    }
    exit::code_for(&diagnostics, args.strict)
}
