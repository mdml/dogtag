//! `dogtag doctor` — a vault's configuration health check.
//!
//! It opens **exactly two files**: the vault's contract and the installation
//! record. No note is read and no directory under the vault root is
//! enumerated — that line is where this milestone ends, and crossing it for a
//! progress counter would decide a traversal policy by accident. It writes
//! nothing, anywhere, including the installation record it reads.
//!
//! It never refuses to run. A contract that did not resolve costs the
//! contract-dependent sections and nothing else: the resolved root, how it was
//! selected, the installation state and the version classification are all
//! still reported, and each section says *not evaluated* with the reason. The
//! one thing it cannot report on is a vault it could not resolve at all.

use dogtag::diagnostic::{Diagnostic, render_plain};
use dogtag::installation::Installation;
use dogtag::report::{DoctorReport, doctor_json, doctor_report, doctor_text};
use dogtag::vault::{inspect_root_trust, open};

use crate::environment::Environment;
use crate::output::{self, Rendering};
use crate::select::{Selected, select};
use crate::{DoctorArgs, DoctorFormat, exit};

/// Reports on the selected vault, and answers with the run's exit code.
pub fn run(environment: &Environment, args: &DoctorArgs) -> i32 {
    let installation = environment.installation();
    match select(environment, args.vault.requested(), &installation) {
        Ok(selected) => report(environment, args, selected, installation),
        Err(diagnostics) => refuse(environment, &diagnostics, args.strict),
    }
}

/// Builds the report, writes it, and weighs what it found.
fn report(
    environment: &Environment,
    args: &DoctorArgs,
    selected: Selected,
    installation: Installation,
) -> i32 {
    let mut raised = selected.diagnostics;
    // Trust is inspected on every command, because a contract planted in any
    // ancestor of where this ran is text an agent may be handed as the vault's
    // rules. The home directory it is judged against is an environment fact,
    // which is why the SDK takes it as an argument instead of reading it.
    raised.extend(inspect_root_trust(&selected.root, environment.home()));
    let opened = open(selected.root, installation);
    let report = doctor_report(&opened, selected.selection, &raised);
    write(environment, args.format, &report);
    exit::code(report.counts(), args.strict)
}

/// Writes the report to standard output, in the format that was asked for.
fn write(environment: &Environment, format: DoctorFormat, report: &DoctorReport) {
    match format {
        DoctorFormat::Text => {
            output::to_stdout(environment, Rendering::diagnostics(&doctor_text(report)));
        }
        DoctorFormat::Json => {
            output::to_stdout(environment, Rendering::verbatim(&doctor_json(report)));
        }
    }
}

/// The one thing `doctor` cannot report on: a vault it could not resolve.
///
/// There is no report to write, so nothing reaches standard output at all —
/// a consumer piping `--format json` receives valid JSON or nothing, never a
/// refusal it cannot parse. The diagnostics name what was looked for and what
/// was not found.
fn refuse(environment: &Environment, diagnostics: &[Diagnostic], strict: bool) -> i32 {
    output::to_stderr(
        environment,
        Rendering::diagnostics(&render_plain(diagnostics)),
    );
    exit::code_for(diagnostics, strict)
}
