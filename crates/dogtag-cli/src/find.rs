//! `dogtag find` — a thin stream-routing consumer of the SDK lookup.

use dogtag::diagnostic::{DiagnosticList, render_plain};
use dogtag::installation::Installation;
use dogtag::note::find;
use dogtag::report::{find_json, find_text};

use crate::environment::Environment;
use crate::exit;
use crate::output::{self, Rendering};
use crate::preflight::{prepare, refuse_selection, refuse_unresolved};
use crate::select::{Selected, select};
use crate::{FindArgs, FindFormat};

const POINTER: &str = "\nthe contract did not resolve, so no note can be found: run \
                       `dogtag doctor` for the vault's full diagnosis\n";

pub fn run(environment: &Environment, args: &FindArgs) -> i32 {
    let installation = environment.installation();
    match select(environment, args.vault.requested(), &installation) {
        Ok(selected) => run_selected(environment, args, selected, installation),
        Err(refused) => refuse_selection(environment, &refused.diagnostics, args.strict),
    }
}

fn run_selected(
    environment: &Environment,
    args: &FindArgs,
    selected: Selected,
    installation: Installation,
) -> i32 {
    let prepared = prepare(environment, selected, installation);
    let Ok(contract) = prepared.opened.contract() else {
        return refuse_unresolved(environment, &prepared, POINTER);
    };
    let result = find(
        prepared.opened.root(),
        contract,
        &args.name,
        args.type_name.as_deref(),
    );
    let mut diagnostics = DiagnosticList::new();
    diagnostics.extend(prepared.diagnostics);
    diagnostics.extend(result.diagnostics().iter().cloned());
    let diagnostics = diagnostics.sorted();
    match args.format {
        FindFormat::Text => {
            output::to_stdout(environment, Rendering::verbatim(&find_text(&result)));
            output::to_stderr(
                environment,
                Rendering::diagnostics(&render_plain(&diagnostics)),
            );
        }
        FindFormat::Json => output::to_stdout(
            environment,
            Rendering::verbatim(&find_json(&result, &diagnostics)),
        ),
    }
    exit::code_for(&diagnostics, args.strict)
}
