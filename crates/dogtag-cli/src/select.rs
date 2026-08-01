//! Which vault a run is about.
//!
//! Selection is the consumer's, and resolves in one order: `--vault`, then
//! `DOGTAG_VAULT`, then upward discovery from the current directory. Every
//! command reports the root it resolved and how.
//!
//! # An argument's syntax fixes its meaning
//!
//! An argument **containing a path separator, or beginning with `.`, `/` or
//! `~`, is always a path**. It is used exactly, through [`root_at`], and never
//! searched upward: explicit means explicit. **Any other argument is always a
//! registry name**, and never falls back to being a path — an unregistered
//! name is an error.
//!
//! The no-fallback rule is what makes this deterministic.
//! *Path-if-it-exists-else-name* would resolve `--vault work` differently
//! depending on whether a `work` directory happens to sit in the current
//! directory, so one command would select different vaults from different
//! places. *Name-first-else-path* would let registering a vault named `docs`
//! silently change what `--vault docs` means in every script already on the
//! machine. With no fallback, registering a vault can never change what an
//! existing invocation resolves to.
//!
//! # Nothing here reinterprets a vault
//!
//! The three resolutions are the SDK's — [`root_at`], [`resolve_registered`]
//! and [`discover`] — because their failures are kernel diagnostics and
//! because the sentinel must have exactly one implementation. This module
//! chooses *which* of them a run calls, and turns argv and the environment
//! into their explicit arguments.

use std::path::{MAIN_SEPARATOR, PathBuf};

use dogtag::diagnostic::{Diagnostic, DiagnosticList};
use dogtag::installation::{Installation, resolve_registered};
use dogtag::report::{Selection, SelectionRoute};
use dogtag::vault::{Resolved, VaultRoot, discover, root_at};

use crate::environment::Environment;

/// A selection that named no vault, and what refused it.
///
/// The selection is carried even here: *which vault was asked for* is known
/// whether or not one was found, and it is the first thing a reader of a
/// failed run needs. Discarding it was what left `doctor` with nothing to
/// report and no choice but to refuse.
pub struct Unresolved {
    /// Which route was taken, and the argument that drove it.
    pub selection: Selection,
    /// The diagnostics that refused it.
    pub diagnostics: Vec<Diagnostic>,
}

/// A resolved vault, and the decision that resolved it.
pub struct Selected {
    /// The root, verified by the SDK.
    pub root: VaultRoot,
    /// Which route resolved it, and the argument that drove the route.
    pub selection: Selection,
    /// What resolving it had to say — a symlinked root, an ancestor vault.
    pub diagnostics: Vec<Diagnostic>,
}

/// The two routes one source can take.
///
/// A source is a *place* an argument came from, and the argument's syntax
/// decides which of its two routes applies. Keeping the pair together is what
/// makes the flag and the environment variable provably the same rule.
#[derive(Clone, Copy)]
struct Source {
    path: SelectionRoute,
    name: SelectionRoute,
    /// How a usage message spells this source.
    named: &'static str,
}

/// `--vault`, which outranks everything.
const FLAG: Source = Source {
    path: SelectionRoute::FlagPath,
    name: SelectionRoute::FlagName,
    named: "--vault",
};

/// `DOGTAG_VAULT`, consulted only when no flag was given.
const ENVIRONMENT: Source = Source {
    path: SelectionRoute::EnvironmentPath,
    name: SelectionRoute::EnvironmentName,
    named: "DOGTAG_VAULT",
};

/// Resolves the vault this run is about.
///
/// # Errors
///
/// The diagnostics that refused it, every one of them the SDK's own: this
/// module chooses which resolution to call and mints no identifier of its own.
pub fn select(
    environment: &Environment,
    flag: Option<&str>,
    installation: &Installation,
) -> Result<Selected, Unresolved> {
    match given(flag, environment) {
        Some((argument, source)) => explicit(argument, source, environment, installation),
        None => discovered(environment),
    }
}

/// Where a vault selector came from, for a usage message that names it.
///
/// An empty selector is a usage fault rather than a vault fault: there is no
/// vault to diagnose and nothing the SDK could be asked about, so it is
/// clap's kind of error and takes clap's exit code.
pub fn empty_selector(flag: Option<&str>, environment: &Environment) -> Option<&'static str> {
    let (argument, source) = given(flag, environment)?;
    argument.is_empty().then_some(source.named)
}

/// The argument selection was asked to honour, and where it came from.
fn given<'a>(flag: Option<&'a str>, environment: &'a Environment) -> Option<(&'a str, Source)> {
    flag.map(|argument| (argument, FLAG))
        .or_else(|| environment.vault().map(|argument| (argument, ENVIRONMENT)))
}

/// A vault named outright, by whichever of the two routes its syntax picks.
fn explicit(
    argument: &str,
    source: Source,
    environment: &Environment,
    installation: &Installation,
) -> Result<Selected, Unresolved> {
    let (route, resolved) = if is_path(argument) {
        (
            source.path,
            root_at(&absolute(argument, environment)).map_err(one),
        )
    } else {
        (source.name, registered(argument, installation))
    };
    let selection = Selection::new(route, Some(argument.to_owned()));
    // Neither route walks, but either can resolve through a symlink and hand
    // back a root the caller did not type. That info is the SDK's, and this is
    // where it reaches the report.
    let (root, diagnostics) = match resolved {
        Ok(resolved) => resolved.into_parts(),
        Err(diagnostics) => {
            return Err(Unresolved {
                selection,
                diagnostics,
            });
        }
    };
    Ok(Selected {
        root,
        selection,
        diagnostics,
    })
}

/// A path argument as an absolute path.
///
/// The SDK's entry points are pure functions of their arguments, and a
/// relative path is not one: the operating system would resolve it against the
/// *process* working directory rather than the one this run fixed. Joining it
/// here is what keeps that promise true for every other embedder too, since
/// nothing in the SDK is left to consult ambient state on a caller's behalf.
///
/// A `~` is still not expanded — it makes the argument a path and the path is
/// used as written, matching the registry's own no-expansion rule.
fn absolute(argument: &str, environment: &Environment) -> PathBuf {
    environment.current_dir().join(argument)
}

/// Whether an argument names a path rather than a registry entry.
///
/// A `~` is **not** expanded here. It makes the argument a path, and the path
/// is used as written: expansion is the shell's job, and doing it here would
/// make this flag inconsistent with the registry's own rule that a registered
/// path is never expanded.
fn is_path(argument: &str) -> bool {
    argument.starts_with(['.', '/', '~']) || argument.contains(['/', MAIN_SEPARATOR])
}

/// A registry name, refused together with what reading the record had to say.
///
/// The SDK answers every state a record can be in, and its refusal for one that
/// did not load points at the diagnostics reading it produced. This is what puts
/// those in front of a reader: a refusal citing evidence its surface never
/// prints is a dead end, and selection is the one path where nothing else
/// reports the record — a run that resolves a vault reports it through
/// [`dogtag::vault::open`]. A record that loaded, or that was never there, has
/// nothing to add here.
fn registered(name: &str, installation: &Installation) -> Result<Resolved, Vec<Diagnostic>> {
    resolve_registered(name, installation).map_err(|refusal| {
        let mut raised = DiagnosticList::new();
        raised.push(refusal);
        raised.extend(installation.diagnostics().iter().cloned());
        raised.sorted()
    })
}

/// The vault the current directory is inside.
fn discovered(environment: &Environment) -> Result<Selected, Unresolved> {
    let found = discover(environment.current_dir());
    let selection = Selection::new(SelectionRoute::Discovery, None);
    match found.root {
        Some(root) => Ok(Selected {
            root,
            selection,
            diagnostics: found.diagnostics,
        }),
        None => Err(Unresolved {
            selection,
            diagnostics: found.diagnostics,
        }),
    }
}

/// One diagnostic, as the list every refusal here answers with.
fn one(diagnostic: Diagnostic) -> Vec<Diagnostic> {
    vec![diagnostic]
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use dogtag::installation::{load_installation, parse_installation};

    #[test]
    fn an_argument_that_looks_like_a_path_is_one() {
        for argument in [
            "./work",
            "../work",
            ".",
            "/data/vaults/work",
            "~/work",
            "~",
            "vaults/work",
            "work/",
        ] {
            assert!(is_path(argument), "{argument} is a path");
        }
    }

    #[test]
    fn every_other_argument_is_a_registry_name() {
        for argument in ["work", "my-vault", "", "work.notes", "a~b"] {
            assert!(!is_path(argument), "{argument} is a registry name");
        }
    }

    /// Every state a record can be in, refused under the kernel's own
    /// identifier: this crate mints none of its own for any of them.
    #[test]
    fn a_registry_name_is_refused_by_the_sdk_in_every_state_a_record_can_be_in() {
        let states = [
            parse_installation("installation_version = 1\n"),
            parse_installation("installation_version = 1\nstray = true\n"),
            load_installation(Path::new("")),
        ];
        for installation in &states {
            let refusal = resolve_registered("work", installation)
                .expect_err("`work` is registered nowhere in any of these");
            assert_eq!(refusal.id.as_str(), "installation.unknown-vault-name");
            assert!(!refusal.id.as_str().starts_with("ext."));
            let help = refusal.help.expect("the correction is the substance");
            assert!(help.contains("./work"), "{help}");
        }
    }
}
