//! The preflight every corpus command shares: select, inspect, open — and the
//! two refusal shapes the surfaces record fixes family-wide.
//!
//! Each command keeps only its own pointer text and its happy path; the
//! loading diagnostics, the selection refusal, and the unresolved-contract
//! refusal are one implementation so the family cannot drift.

use dogtag::diagnostic::{Diagnostic, DiagnosticList, render_plain};
use dogtag::installation::Installation;
use dogtag::vault::{Opened, inspect_root_trust, open};

use crate::environment::Environment;
use crate::exit;
use crate::output::{self, Rendering};
use crate::select::Selected;

/// A selected vault opened, with every loading diagnostic collected.
pub struct Prepared {
    pub opened: Opened,
    pub diagnostics: Vec<Diagnostic>,
}

/// Opens the selected vault and collects the loading path's diagnostics.
pub fn prepare(
    environment: &Environment,
    selected: Selected,
    installation: Installation,
) -> Prepared {
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

/// Refuses a selection that named no usable vault: diagnostics, mapped exit.
pub fn refuse_selection(
    environment: &Environment,
    diagnostics: &[Diagnostic],
    strict: bool,
) -> i32 {
    output::to_stderr(
        environment,
        Rendering::diagnostics(&render_plain(diagnostics)),
    );
    exit::code_for(diagnostics, strict)
}

/// Refuses an unresolved contract exactly as `contract explain`: diagnostics,
/// the command's pointer at `doctor`, exit 1, nothing on stdout.
pub fn refuse_unresolved(environment: &Environment, prepared: &Prepared, pointer: &str) -> i32 {
    output::to_stderr(
        environment,
        Rendering::diagnostics(&render_plain(&prepared.diagnostics)),
    );
    output::to_stderr(environment, Rendering::verbatim(pointer));
    exit::FAILURE
}
