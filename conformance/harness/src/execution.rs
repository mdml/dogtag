//! How an executable scenario is actually run against a profile.
//!
//! An execution path is **not a filter**. It cannot decline a pair: a runnable
//! pair (an `executable` scenario against a `built` corpus) whose scenario has
//! no case behind it is [`crate::HarnessError::NotExecutable`], never a silent
//! skip. That is what keeps graduation all-or-nothing — there is no partial
//! graduation and no per-profile rollout, and an executor is the wrong shape
//! to smuggle one in through.

use std::path::PathBuf;

use crate::cases;
use crate::schema::{Profile, Scenario};

/// The execution path [`crate::report`] runs a runnable pair through.
pub trait Execution {
    /// Run one scenario against one profile.
    ///
    /// `None` means **this scenario has no execution path**, which the report
    /// turns into [`crate::HarnessError::NotExecutable`]. It is never a skip,
    /// and it is not a place to decline a profile: a scenario answers `None`
    /// for every profile or for none of them.
    ///
    /// `Some(Ok(()))` is a pass; `Some(Err(detail))` is a failure, and
    /// `detail` is what the matrix prints beneath itself.
    fn run(&self, scenario: &Scenario, profile: &Profile) -> Option<Result<(), String>>;
}

/// No execution path at all.
///
/// Every pair answers `None`, so a runnable pair is refused with
/// [`crate::HarnessError::NotExecutable`]. This is the state the harness was
/// in before the M2 scenarios graduated, kept so tests can assert the refusal
/// still fires — it is a way to have no executor, not a way to skip a pair.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoExecution;

impl Execution for NoExecution {
    fn run(&self, _scenario: &Scenario, _profile: &Profile) -> Option<Result<(), String>> {
        None
    }
}

/// The real execution path: each graduated scenario's case, run against a
/// temporary copy of the profile's corpus.
///
/// The profile directory is resolved from a root the caller supplies rather
/// than from a field on [`Profile`]. That is deliberate — the profile schema
/// has no field with which to name anything, and it must not gain one.
#[derive(Clone, Debug)]
pub struct SdkExecution {
    profiles_root: PathBuf,
}

impl SdkExecution {
    /// An execution path resolving profiles under `profiles_root`.
    pub fn new(profiles_root: PathBuf) -> Self {
        Self { profiles_root }
    }

    /// An execution path against the repository's own `conformance/profiles/`.
    pub fn in_repository() -> Self {
        Self::new(crate::profiles_dir())
    }
}

impl Default for SdkExecution {
    fn default() -> Self {
        Self::in_repository()
    }
}

impl Execution for SdkExecution {
    fn run(&self, scenario: &Scenario, profile: &Profile) -> Option<Result<(), String>> {
        let case = cases::case_for(&scenario.id)?;
        let corpus = self.profiles_root.join(&profile.name).join("corpus");
        Some(cases::run(case, &corpus, &scenario.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The default execution path resolves the repository's own fixtures, so
    /// a caller that names no root still runs against the profiles rather than
    /// against a directory that happens to be the process's own.
    #[test]
    fn the_default_execution_path_resolves_the_repository_profiles() {
        assert_eq!(SdkExecution::default().profiles_root, crate::profiles_dir());
    }

    /// A scenario with no case has no execution path, whichever profile it is
    /// asked about — the answer is about the scenario, never about the pair.
    #[test]
    fn a_scenario_with_no_case_has_no_execution_path() {
        assert!(cases::case_for("a-scenario-nothing-implements").is_none());
    }
}
