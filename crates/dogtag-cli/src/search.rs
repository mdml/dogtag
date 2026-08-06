//! `dogtag search` — a thin stream-routing consumer of the SDK scan.

use dogtag::diagnostic::{DiagnosticList, render_plain};
use dogtag::installation::Installation;
use dogtag::note::{ListFilter, SearchRequest, search};
use dogtag::report::{search_json, search_text};

use crate::environment::Environment;
use crate::exit;
use crate::output::{self, Rendering};
use crate::preflight::{prepare, refuse_selection, refuse_unresolved};
use crate::select::{Selected, select};
use crate::{SearchArgs, SearchFormat};

const POINTER: &str = "\nthe contract did not resolve, so the corpus cannot be searched: run \
                       `dogtag doctor` for the vault's full diagnosis\n";

pub fn run(environment: &Environment, args: &SearchArgs) -> i32 {
    let installation = environment.installation();
    match select(environment, args.vault.requested(), &installation) {
        Ok(selected) => run_selected(environment, args, selected, installation),
        Err(refused) => refuse_selection(environment, &refused.diagnostics, args.strict),
    }
}

fn run_selected(
    environment: &Environment,
    args: &SearchArgs,
    selected: Selected,
    installation: Installation,
) -> i32 {
    let prepared = prepare(environment, selected, installation);
    let Ok(contract) = prepared.opened.contract() else {
        return refuse_unresolved(environment, &prepared, POINTER);
    };
    let request = SearchRequest {
        query: args.query.clone(),
        filter: ListFilter {
            type_name: args.type_name.clone(),
            tag: args.tag.clone(),
            lifecycle: args.lifecycle.clone(),
            ordinary: args.ordinary,
        },
        limit: args.limit,
    };
    let result = search(prepared.opened.root(), contract, &request);
    let mut diagnostics = DiagnosticList::new();
    diagnostics.extend(prepared.diagnostics);
    diagnostics.extend(result.diagnostics().iter().cloned());
    let diagnostics = diagnostics.sorted();
    match args.format {
        SearchFormat::Text => {
            output::to_stdout(environment, Rendering::verbatim(&search_text(&result)));
            output::to_stderr(
                environment,
                Rendering::diagnostics(&render_plain(&diagnostics)),
            );
        }
        SearchFormat::Json => output::to_stdout(
            environment,
            Rendering::verbatim(&search_json(&result, &diagnostics)),
        ),
    }
    exit::code_for(&diagnostics, args.strict)
}
