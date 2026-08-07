//! The plan, the outcome, and the result a write answers with.
//!
//! **The plan is a value.** There is no plan file, no journal, and no
//! two-invocation handshake: a persisted plan is state that can go stale
//! between invocations, and the shape of a one-note write does not need it. A
//! preview is the same value, emitted without applying.
//!
//! **The transaction is the verdict.** Read verbs answer *what is true of the
//! corpus*; a write verb answers *did my act land*. So a result reports both,
//! separately and always: [`WriteResult::landed`] is the act's own answer, and
//! [`WriteResult::diagnostics`] carries everything the corpus had to say —
//! including findings that were already there and are nobody's fault but which
//! a caller triaging its vault wants in the same document.

use crate::diagnostic::{Diagnostic, DiagnosticList, SeverityCounts, VaultPath};

use super::actor::Actor;

/// How an act changes the vault's format compatibility.
///
/// **Deliberately uninhabited.** The transaction's shape is shared across every
/// operation a write verb will ever have, and one of the things a plan must be
/// able to state is that the act moves the contract's version. No M5 operation
/// can: `capture` creates one note in the catch-all and touches no
/// configuration at all. An empty enum says exactly that — the field exists,
/// always resolves to *no impact*, and gains a variant in the change that gains
/// an operation able to produce one — where a placeholder variant would be a
/// value nothing constructs and nothing tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompatibilityImpact {}

/// What a write intends to do, before it does any of it.
///
/// The contents are the fixed list a transaction owes: the actor, the capacity
/// the act is performed in, the file scope it intends to touch, what the corpus
/// already had to say, and what it would do to compatibility.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub(super) actor: Actor,
    pub(super) scope: Vec<VaultPath>,
    pub(super) diagnostics: Vec<Diagnostic>,
    pub(super) compatibility: Option<CompatibilityImpact>,
}

impl Plan {
    /// Who is acting, and in what capacity.
    pub fn actor(&self) -> &Actor {
        &self.actor
    }

    /// Every file the act intends to touch, in vault-relative path order.
    ///
    /// One entry for a capture, which is what makes a partial apply impossible
    /// at this milestone — a property of the operation, recorded as one, and
    /// not a guarantee of the machinery.
    pub fn scope(&self) -> &[VaultPath] {
        &self.scope
    }

    /// What was already true of the corpus before the act.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// What the act would do to the vault's format compatibility.
    ///
    /// Always `None`: see [`CompatibilityImpact`].
    pub fn compatibility(&self) -> Option<CompatibilityImpact> {
        self.compatibility
    }
}

/// How to undo an act that landed.
///
/// Concrete in both modes without a backup copy and without an undo verb, which
/// would be a second mutation this milestone's scope does not admit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recovery {
    /// Revert this commit, which contains exactly the created file.
    Revert {
        /// The commit the act made.
        commit: String,
        /// The file it contains.
        path: VaultPath,
    },
    /// Delete this file. Deleting is recovery for a create.
    Delete {
        /// The file the act created.
        path: VaultPath,
    },
}

impl Recovery {
    /// The file the act created, whichever recovery applies.
    pub fn path(&self) -> &VaultPath {
        match self {
            Self::Revert { path, .. } | Self::Delete { path } => path,
        }
    }
}

/// What a write did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The plan was emitted and nothing was written.
    Previewed,
    /// The file was created, and committed with the one file in it.
    Committed {
        /// The created file.
        path: VaultPath,
        /// The commit that contains it.
        commit: String,
    },
    /// The file was created and not committed.
    ///
    /// Either the substrate does not own this vault's commit path, or it owns
    /// it and the commit did not happen — a warning says which. Both are the
    /// same outcome for the caller: the note is on disk, and deleting it is
    /// recovery.
    Created {
        /// The created file.
        path: VaultPath,
    },
    /// Nothing was written, and the error diagnostics say why.
    Refused,
}

/// What a write answers with: the plan it made, what became of it, and
/// everything anyone had to say about the corpus along the way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WriteResult {
    pub(super) plan: Plan,
    pub(super) outcome: Outcome,
    pub(super) diagnostics: Vec<Diagnostic>,
}

impl WriteResult {
    /// What the act intended.
    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// What became of it.
    pub fn outcome(&self) -> &Outcome {
        &self.outcome
    }

    /// Everything reported, in the diagnostic total order.
    ///
    /// The act's own refusals and warnings and the corpus's findings, in one
    /// list. A caller that wants only the verdict reads [`Self::landed`]; a
    /// caller triaging a vault reads this.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Summary counts per severity.
    pub fn counts(&self) -> SeverityCounts {
        let mut list = DiagnosticList::new();
        list.extend(self.diagnostics.iter().cloned());
        list.counts()
    }

    /// **Whether the act landed**, which is a write verb's whole verdict.
    ///
    /// Deliberately not a function of the diagnostics. A successful capture
    /// into a corpus that already carried errors landed: those findings are
    /// triage, not this transaction's answer, and a verb that reported failure
    /// because its vault was untidy would teach callers to ignore the one
    /// signal a verdict exists to carry. A preview lands nothing and did not
    /// fail: it is `true`, because the act it was asked to perform — emit the
    /// plan, write nothing — is exactly what happened.
    pub fn landed(&self) -> bool {
        !matches!(self.outcome, Outcome::Refused)
    }

    /// How to undo what landed, when something did.
    pub fn recovery(&self) -> Option<Recovery> {
        match &self.outcome {
            Outcome::Committed { path, commit } => Some(Recovery::Revert {
                commit: commit.clone(),
                path: path.clone(),
            }),
            Outcome::Created { path } => Some(Recovery::Delete { path: path.clone() }),
            Outcome::Previewed | Outcome::Refused => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::diagnostic::KernelDiagnostic;
    use crate::vault::VaultRoot;
    use crate::write::actor::ProvenanceKind;
    use std::path::{Path, PathBuf};

    fn path(spelled: &str) -> VaultPath {
        VaultRoot::new(PathBuf::from("/vault"))
            .relative(&Path::new("/vault").join(spelled))
            .expect("a path under the root")
    }

    fn plan(diagnostics: Vec<Diagnostic>) -> Plan {
        Plan {
            actor: Actor::new(Some("A Maintainer".to_owned()), ProvenanceKind::Agent),
            scope: vec![path("captures/one.md")],
            diagnostics,
            compatibility: None,
        }
    }

    fn result(outcome: Outcome, diagnostics: Vec<Diagnostic>) -> WriteResult {
        WriteResult {
            plan: plan(diagnostics.clone()),
            outcome,
            diagnostics,
        }
    }

    fn finding() -> Diagnostic {
        Diagnostic::kernel(KernelDiagnostic::NoteUnknownType, "an old fault")
    }

    #[test]
    fn a_plan_answers_for_the_act_it_describes() {
        let plan = plan(vec![finding()]);
        assert_eq!(plan.actor().name(), Some("A Maintainer"));
        assert_eq!(plan.scope().len(), 1);
        assert_eq!(plan.scope()[0].as_str(), "captures/one.md");
        assert_eq!(plan.diagnostics().len(), 1);
        assert_eq!(plan.compatibility(), None);
        assert_eq!(plan.clone(), plan);
        assert!(format!("{plan:?}").contains("captures/one.md"));
    }

    /// A capture that landed into a corpus that already carried errors landed.
    /// This is the split the record makes, as a test.
    #[test]
    fn the_verdict_is_the_act_and_never_the_corpus() {
        let landed = result(
            Outcome::Created {
                path: path("captures/one.md"),
            },
            vec![finding()],
        );
        assert!(landed.landed());
        assert_eq!(landed.counts().error, 1);
        assert!(!result(Outcome::Refused, vec![finding()]).landed());
    }

    /// A preview performed exactly the act it was asked to perform.
    #[test]
    fn a_preview_lands_and_recovers_nothing() {
        let previewed = result(Outcome::Previewed, Vec::new());
        assert!(previewed.landed());
        assert_eq!(previewed.recovery(), None);
        assert_eq!(previewed.outcome(), &Outcome::Previewed);
        assert_eq!(result(Outcome::Refused, Vec::new()).recovery(), None);
    }

    /// Recovery is concrete in both modes, and both name the created file.
    #[test]
    fn recovery_is_the_commit_where_there_is_one_and_the_file_where_there_is_not() {
        let committed = result(
            Outcome::Committed {
                path: path("captures/one.md"),
                commit: "abc123".to_owned(),
            },
            Vec::new(),
        );
        let recovery = committed.recovery().expect("a committed act recovers");
        assert_eq!(
            recovery,
            Recovery::Revert {
                commit: "abc123".to_owned(),
                path: path("captures/one.md"),
            }
        );
        assert_eq!(recovery.path().as_str(), "captures/one.md");
        let guest = result(
            Outcome::Created {
                path: path("captures/one.md"),
            },
            Vec::new(),
        );
        let recovery = guest.recovery().expect("a created file recovers");
        assert_eq!(
            recovery,
            Recovery::Delete {
                path: path("captures/one.md")
            }
        );
        assert_eq!(recovery.path().as_str(), "captures/one.md");
        assert!(format!("{recovery:?}").contains("Delete"));
    }

    #[test]
    fn a_result_carries_its_plan_and_everything_reported() {
        let landed = result(Outcome::Previewed, vec![finding()]);
        assert_eq!(landed.plan().scope().len(), 1);
        assert_eq!(landed.diagnostics().len(), 1);
        assert_eq!(landed.clone(), landed);
        assert_ne!(landed, result(Outcome::Refused, vec![finding()]));
        assert!(format!("{landed:?}").contains("Previewed"));
    }
}
