//! Safe mutation: the write transaction, and its one operation.
//!
//! A write is plan, apply, report. The plan is a value the caller can be handed
//! without anything happening; applying it creates the file and, where this
//! substrate owns the commit path, commits exactly that file; the report says
//! what landed, what the corpus has to say, and how to undo it.
//!
//! # Capture is exempt, structurally
//!
//! Nothing stands between a thought and its capture — no required fields, no
//! validation, no commit gate — and every mutation at this milestone is
//! nevertheless validated after it lands. Those two compose without machinery,
//! and the composition is not a carve-out:
//!
//! - A capture omits `type`, so it binds to the **catch-all**.
//! - From contract version 2 the catch-all **may require nothing**.
//! - Therefore a capture cannot fail a contract rule *by construction*.
//!
//! The symmetry is the price and is stated as one: a capture receives none of
//! the vault's services either — no template, no defaults, no lifecycle,
//! because the catch-all is axis-less under the named-ordinary composition. You
//! may ignore the vault's rules only by also forgoing what the vault provides.
//!
//! So post-write validation runs — the same shared loading, traversal and
//! validation path `check`, `list`, `show`, `search` and `find` read by — and
//! **reports**. It never rolls back and never refuses a capture. Refusals exist
//! only for the non-lint non-negotiables: a target that cannot be written, one
//! that resolves outside the vault root, and an ownership violation.
//!
//! # Every write goes through the verified handle
//!
//! A target is built from a [`VaultRoot`], never re-resolved from a string, and
//! the built target is checked against the root *after* its directory exists —
//! because the one thing a path this SDK composed cannot rule out by spelling
//! is a capture directory that is a symbolic link out of the vault. Paths
//! derived from the environment are discovery inputs and never write targets.

mod actor;
mod commit;
mod identity;
mod note;
mod plan;

#[cfg(test)]
pub(crate) mod fixture;

use std::fs;
use std::path::{Path, PathBuf};

use crate::contract::{Capability, Contract, DEFAULT_CAPTURE_DIRECTORY};
use crate::diagnostic::{
    Diagnostic, DiagnosticList, FileRef, KernelDiagnostic, Location, Severity, VaultPath,
};
use crate::vault::VaultRoot;

pub use actor::{Actor, CapturedAt, ProvenanceKind};
pub use plan::{CompatibilityImpact, Outcome, Plan, Recovery, WriteResult};

/// How many names a collision may try before the write gives up.
///
/// A bound rather than an unbounded search: a directory that answers *taken* to
/// every name is a directory something is wrong with, and a write that spun
/// there forever would be worse than one that says so. Reached only by a second
/// that already holds this many notes captured in it.
const COLLISION_LIMIT: usize = 1000;

/// What a capture is asked to create.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureRequest {
    text: String,
    at: CapturedAt,
    actor: Actor,
}

impl CaptureRequest {
    /// A capture of `text`, made at `at`, by `actor`.
    ///
    /// `text` becomes the note's body byte for byte. It is not trimmed, not
    /// normalized, and not given a trailing newline: a writing surface
    /// preserves every byte it did not semantically touch, and it touched none
    /// of these.
    pub fn new(text: impl Into<String>, at: CapturedAt, actor: Actor) -> Self {
        Self {
            text: text.into(),
            at,
            actor,
        }
    }

    /// The captured text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The instant the capture's identity derives from.
    pub fn at(&self) -> CapturedAt {
        self.at
    }

    /// Who is capturing, and in what capacity.
    pub fn actor(&self) -> &Actor {
        &self.actor
    }
}

/// Plans a capture and writes nothing.
///
/// The plan is the same value [`capture`] would have applied, so a preview and
/// the act that follows it cannot disagree about what was intended. It costs
/// one corpus read, which is what makes the diagnostics in it the ones the act
/// would have started from.
pub fn plan_capture(
    root: &VaultRoot,
    contract: &Contract,
    request: &CaptureRequest,
) -> WriteResult {
    let (plan, acted) = prepare(root, contract, request);
    let outcome = if refused(&acted) {
        Outcome::Refused
    } else {
        Outcome::Previewed
    };
    let corpus = plan.diagnostics.clone();
    report(plan, outcome, acted, corpus)
}

/// Plans a capture and applies it, in one act.
///
/// One act rather than two, because *capture is instant* is a product stance
/// and a two-invocation handshake would put a round trip between a thought and
/// its landing. A caller that wants to look first calls [`plan_capture`].
pub fn capture(root: &VaultRoot, contract: &Contract, request: &CaptureRequest) -> WriteResult {
    let (plan, mut acted) = prepare(root, contract, request);
    let before = plan.diagnostics.clone();
    if refused(&acted) {
        return report(plan, Outcome::Refused, acted, before);
    }
    let Some(target) = resolve(root, contract, &mut acted) else {
        return report(plan, Outcome::Refused, acted, before);
    };
    let minted = Minted {
        name: identity::file_name(request.at(), request.text()),
        contents: note::document(note::birth_flags(contract), request.text()),
    };
    let Some(created) = write(root, &target, &minted, &mut acted) else {
        return report(plan, Outcome::Refused, acted, before);
    };
    let outcome = settle(root, &created, request, &mut acted);
    // The shared read path, over the corpus the act just changed. Uniform with
    // every other door: it reports, and nothing it reports can undo the write.
    // It supersedes the plan's reading rather than joining it — the corpus has
    // one state, and the newer reading is of the corpus the caller now has.
    let after = crate::note::read_corpus(root, contract)
        .diagnostics()
        .to_vec();
    report(plan, outcome, acted, after)
}

/// Whether what the act reported stops it.
///
/// Severity decides, and only an error does: the unattributed warning is the
/// standing example of something a write says and does not stop for.
fn refused(acted: &[Diagnostic]) -> bool {
    acted
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

/// The plan, and the refusals that stop it before anything is written.
fn prepare(
    root: &VaultRoot,
    contract: &Contract,
    request: &CaptureRequest,
) -> (Plan, Vec<Diagnostic>) {
    let known = crate::note::read_corpus(root, contract)
        .diagnostics()
        .to_vec();
    let scope = intended(root, contract, request);
    let plan = Plan {
        actor: request.actor().clone(),
        scope: scope.into_iter().collect(),
        diagnostics: known,
        compatibility: None,
    };
    let mut acted = closed_write(contract);
    acted.extend(unattributed(request.actor()));
    (plan, acted)
}

/// The warning an act with nobody to attribute it to carries.
///
/// Known before the write and reported by a preview as well as by the act, so
/// a caller looking first is told the same thing the act would have told it.
/// Never a refusal: `doctor`'s advance warning about an unconfigured
/// installation resolves here, at the moment it starts to cost something, and
/// what it costs is attribution rather than the thought.
fn unattributed(actor: &Actor) -> Option<Diagnostic> {
    if actor.is_attributed() {
        return None;
    }
    Some(
        Diagnostic::kernel(
            KernelDiagnostic::WriteActorUnattributed,
            "this capture is recorded as unattributed: no actor is configured",
        )
        .with_help(
            "an actor is named in the installation record, or per invocation; the capture lands \
             either way",
        ),
    )
}

/// The one file a capture intends to create, as the plan states it.
///
/// A plan states the target it *intends*, which is the uncollided name: what a
/// collision changes is which name the act settles on, and a plan that named
/// the settled one would have had to touch the filesystem to make a claim it
/// makes before touching anything.
fn intended(root: &VaultRoot, contract: &Contract, request: &CaptureRequest) -> Option<VaultPath> {
    let directory = capture_directory(contract);
    let name = identity::file_name(request.at(), request.text());
    root.relative(&root.path().join(directory).join(name))
}

/// Where this vault's captures land.
///
/// The contract's declaration where its version has a seat for one, and the
/// default where it does not: a version-1 or version-2 vault captures into the
/// same directory a version-3 vault declaring nothing does, because the seats
/// configure the verb rather than enable it.
fn capture_directory(contract: &Contract) -> &str {
    contract
        .capture()
        .map_or(DEFAULT_CAPTURE_DIRECTORY, |capture| capture.directory())
}

/// The ownership refusal: a catch-all no caller may modify.
///
/// The one contract-shaped refusal a capture can earn, and it is not a lint. A
/// corpus whose bottom type is closed-write has said that nothing may create
/// one, and creating one anyway would make the capability a suggestion.
fn closed_write(contract: &Contract) -> Vec<Diagnostic> {
    contract
        .catch_all()
        .filter(|declared| declared.has(Capability::ClosedWrite))
        .map(|declared| {
            Diagnostic::kernel(
                KernelDiagnostic::WriteClosedWrite,
                format!(
                    "the catch-all type `{}` declares `closed-write`, so no caller may create one",
                    declared.name()
                ),
            )
            .with_help(
                "capture creates a note in the catch-all; a corpus that closes its catch-all to \
                 writers has no type a capture may land in",
            )
        })
        .into_iter()
        .collect()
}

/// Where the capture will be written, with its directory made.
///
/// The containment check runs **before** anything is created, against the
/// deepest part of the path that already exists. Creating first and checking
/// afterwards would already have written a directory outside the vault by the
/// time it refused — and *every write resolves its target through the verified
/// handle* has to include the directories a write makes on its way, or it is a
/// claim about the last step only.
///
/// The path is then canonical, which is the other half: containment is judged
/// lexically, and a lexical judgement about a directory that is a symbolic link
/// answers *inside* about a directory that is not.
fn resolve(root: &VaultRoot, contract: &Contract, acted: &mut Vec<Diagnostic>) -> Option<Target> {
    let declared = capture_directory(contract).to_owned();
    let directory = root.path().join(&declared);
    if root.relative(&anchored(root, &directory)).is_none() {
        acted.push(outside_vault(&declared));
        return None;
    }
    if let Err(error) = fs::create_dir_all(&directory) {
        acted.push(unwritable(&format!(
            "the capture directory `{declared}` could not be created: {error}"
        )));
        return None;
    }
    Some(Target {
        directory: fs::canonicalize(&directory).unwrap_or(directory),
        declared,
    })
}

/// The canonical form of the deepest ancestor of `path` that exists.
///
/// The walk is **up** rather than down because that is where the answer is: the
/// components that do not exist yet cannot be links, and the ones that do can.
/// The vault root is an ancestor of every capture target and is itself canonical
/// and present, so the walk never climbs past it in practice — the fallback is
/// the root for the same reason, and is a value rather than a branch because
/// there is no case to distinguish.
fn anchored(root: &VaultRoot, path: &Path) -> PathBuf {
    path.ancestors()
        .find_map(|existing| fs::canonicalize(existing).ok())
        .unwrap_or(root.path().to_path_buf())
}

/// The note a capture is about to write: what it is called, and what is in it.
struct Minted {
    name: String,
    contents: String,
}

/// The directory a capture is about to be written into.
struct Target {
    /// Where it is, canonical wherever the filesystem would say.
    directory: PathBuf,
    /// What the contract called it, which is how a refusal names it.
    declared: String,
}

/// Writes the note, taking the first name nothing else holds.
///
/// A collision appends a suffix rather than refusing, and never overwrites: the
/// file is created exclusively, so two writers racing for one name both land.
///
/// **This is the containment check**, and it is per candidate rather than per
/// directory because the vault-relative spelling a diagnostic needs and the
/// proof that the target is inside the root are the same act: obtaining a
/// [`VaultPath`] *is* the stripping that proves the path was under the root.
fn write(
    root: &VaultRoot,
    target: &Target,
    minted: &Minted,
    acted: &mut Vec<Diagnostic>,
) -> Option<VaultPath> {
    for attempt in 0..COLLISION_LIMIT {
        let candidate = if attempt == 0 {
            target.directory.join(&minted.name)
        } else {
            target
                .directory
                .join(identity::nth(&minted.name, attempt + 1))
        };
        let Some(spelled) = root.relative(&candidate) else {
            acted.push(outside_vault(&target.declared));
            return None;
        };
        match create(&candidate, &minted.contents) {
            Created::Wrote => return Some(spelled),
            Created::Taken => continue,
            Created::Refused(error) => {
                acted.push(unwritable(&format!(
                    "`{spelled}` could not be written: {error}",
                    spelled = spelled.as_str()
                )));
                return None;
            }
        }
    }
    acted.push(unwritable(&format!(
        "no name was free for this capture after {COLLISION_LIMIT} attempts"
    )));
    None
}

/// The refusal a target outside the vault root earns.
fn outside_vault(declared: &str) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::WriteTargetOutsideVault,
        format!("the capture directory `{declared}` resolves outside the vault root"),
    )
    .with_help(
        "a capture lands inside the vault it is captured into; a directory that is a link out \
         of it would be written and then invisible, because the corpus walk does not follow links",
    )
}

/// What creating one candidate answered.
enum Created {
    /// The file did not exist and now holds the capture.
    Wrote,
    /// Something already holds that name.
    Taken,
    /// The write itself failed.
    Refused(std::io::Error),
}

/// Creates `candidate` exclusively, so an existing file is never overwritten.
fn create(candidate: &Path, contents: &str) -> Created {
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(candidate)
    {
        Ok(mut file) => {
            use std::io::Write;
            // The refusal arm is a constructor rather than a block, so a write
            // that fails after the file opened — a full device, a revoked
            // mount — reports as the same kind of thing without adding a
            // branch nothing in a working filesystem can reach.
            file.write_all(contents.as_bytes())
                .map_or_else(Created::Refused, |()| Created::Wrote)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Created::Taken,
        Err(error) => Created::Refused(error),
    }
}

/// Commits the created file where this substrate owns the commit path.
///
/// A commit that does not happen is a warning and never a refusal: the note is
/// already on disk, so refusing afterwards would report a loss that did not
/// happen, and the result names the created path as the recovery it is —
/// exactly what it names where the substrate never owned the commit path.
fn settle(
    root: &VaultRoot,
    created: &VaultPath,
    request: &CaptureRequest,
    reported: &mut Vec<Diagnostic>,
) -> Outcome {
    if !commit::owns_commit_path(root.path()) {
        return Outcome::Created {
            path: created.clone(),
        };
    }
    match commit::commit(root.path(), created.as_str(), request.actor()) {
        Ok(commit) => Outcome::Committed {
            path: created.clone(),
            commit,
        },
        Err(said) => {
            reported.push(
                Diagnostic::kernel(
                    KernelDiagnostic::WriteCommitFailed,
                    format!("the capture landed and was not committed: {said}"),
                )
                .at(Location::whole_file(FileRef::InVault(created.clone())))
                .with_help("the note is on disk; deleting it is recovery"),
            );
            Outcome::Created {
                path: created.clone(),
            }
        }
    }
}

/// Every filesystem refusal of the write itself, under one identifier.
fn unwritable(message: &str) -> Diagnostic {
    Diagnostic::kernel(KernelDiagnostic::WriteTargetUnwritable, message)
}

/// The result, with every diagnostic in the deterministic total order.
///
/// `corpus` is one reading of the corpus and never two: the plan's where
/// nothing was written, and the post-write one where something was. Joining
/// both would report every pre-existing finding twice and make a capture look
/// as though it had doubled its vault's faults.
fn report(
    plan: Plan,
    outcome: Outcome,
    acted: Vec<Diagnostic>,
    corpus: Vec<Diagnostic>,
) -> WriteResult {
    let mut collected = DiagnosticList::new();
    collected.extend(corpus);
    collected.extend(acted);
    WriteResult {
        plan,
        outcome,
        diagnostics: collected.sorted(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::diagnostic::Severity;
    use fixture::{CLOSED, ELSEWHERE, OLDER, Thought, Vault};

    /// Every identifier a result reported, in the order the total order put
    /// them.
    fn ids(result: &WriteResult) -> Vec<&str> {
        result
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    /// The commit a result made, where it made one.
    ///
    /// Both arms are reached: a guest-mode act answers `None`, which is what
    /// makes the question worth asking of a result rather than assuming.
    fn committed(result: &WriteResult) -> Option<&str> {
        match result.outcome() {
            Outcome::Committed { commit, .. } => Some(commit),
            Outcome::Previewed | Outcome::Created { .. } | Outcome::Refused => None,
        }
    }

    /// The path a result says it created.
    fn created(result: &WriteResult) -> String {
        result
            .recovery()
            .expect("the act landed")
            .path()
            .as_str()
            .to_owned()
    }

    /// The capture lands in the default directory, bound to the catch-all by
    /// the absence of a discriminator, and is visible to the corpus walk.
    #[test]
    fn a_capture_lands_unfiled_in_the_default_directory() {
        let vault = Vault::new("write-lands-unfiled");
        let result = vault.capture(Thought("a loose thought"));
        assert!(result.landed());
        let path = created(&result);
        assert!(path.starts_with("captures/"), "{path}");
        assert_eq!(vault.notes(), std::slice::from_ref(&path));
        let corpus = crate::note::read_corpus(vault.root(), vault.contract());
        let note = corpus
            .notes()
            .iter()
            .find(|note| note.path.as_str() == path)
            .expect("the corpus holds the note the act named");
        assert_eq!(note.binding().bound_by(), "catch-all");
    }

    /// A contract that declares a directory is obeyed; one whose version has no
    /// seat to declare one lands in the same default. The seats configure the
    /// verb rather than enable it, as two vaults.
    #[test]
    fn the_declared_directory_is_obeyed_and_an_absent_seat_takes_the_default() {
        let declared = Vault::holding("write-declared-directory", ELSEWHERE);
        assert!(created(&declared.capture(Thought("a thought"))).starts_with("unfiled/raw/"));
        let older = Vault::holding("write-older-version", OLDER);
        let path = created(&older.capture(Thought("a thought")));
        assert!(path.starts_with("captures/"), "{path}");
        assert!(older.read(&path).contains("a thought"));
    }

    /// The body is the captured bytes and nothing else — no trailing newline
    /// this surface did not receive, and no frontmatter where nothing is
    /// stamped.
    #[test]
    fn the_body_is_verbatim_and_the_frontmatter_is_only_what_is_stamped() {
        let flagged = Vault::new("write-verbatim-flagged");
        let path = created(&flagged.capture(Thought("first\nsecond")));
        assert_eq!(
            flagged.read(&path),
            "---\nneeds_triage: true\n---\nfirst\nsecond"
        );
        let plain = Vault::holding("write-verbatim-plain", OLDER);
        let path = created(&plain.capture(Thought("first\nsecond")));
        assert_eq!(plain.read(&path), "first\nsecond");
    }

    /// Every shape the reader calls a fence, captured and read back through
    /// the ordinary read path: the body that comes out is the body that went
    /// in, whichever line terminator or trailing whitespace it opened with.
    ///
    /// End to end rather than over the emitted bytes, because the claim is
    /// about what the *reader* makes of what the writer wrote, and only the
    /// reader can answer that.
    #[test]
    fn every_fence_shaped_thought_round_trips_through_the_read_path() {
        let vault = Vault::holding("write-fence-round-trip", OLDER);
        for (nth, text) in [
            "---",
            "--- ",
            "---\t",
            "---\nkey: value\n---\nthe real body\n",
            "--- \ntype: person\n---\nthe real body\n",
            "---\rtype: person\r---\rthe real body\r",
        ]
        .into_iter()
        .enumerate()
        {
            let request = CaptureRequest::new(
                text,
                CapturedAt::from_unix_seconds(nth as u64),
                Actor::new(Some("A Maintainer".to_owned()), ProvenanceKind::Agent),
            );
            let result = capture(vault.root(), vault.contract(), &request);
            assert!(result.landed(), "{text:?}");
            let path = created(&result);
            let corpus = crate::note::read_corpus(vault.root(), vault.contract());
            let note = corpus
                .notes()
                .iter()
                .find(|note| note.path.as_str() == path)
                .expect("the corpus holds the note the act named");
            assert_eq!(note.body(), text, "the body did not round-trip: {text:?}");
        }
    }

    /// Two captures that would take one name both land, and neither
    /// overwrites the other.
    #[test]
    fn a_collision_appends_a_suffix_and_loses_neither_thought() {
        let vault = Vault::new("write-collision");
        let first = created(&vault.capture(Thought("twice")));
        let second = created(&vault.capture(Thought("twice")));
        assert_ne!(first, second);
        assert!(second.ends_with("-2.md"), "{second}");
        assert_eq!(vault.notes().len(), 2);
        let third = created(&vault.capture(Thought("twice")));
        assert!(third.ends_with("-3.md"), "{third}");
    }

    /// The plan is a value: previewing writes nothing at all, and says what it
    /// would have created.
    #[test]
    fn a_preview_writes_nothing_and_names_what_it_would_have_created() {
        let vault = Vault::new("write-preview");
        let result = vault.preview(Thought("a loose thought"));
        let observed = (
            result.landed(),
            result.outcome(),
            result.recovery(),
            vault.notes(),
            vault.root().path().join("captures").exists(),
        );
        assert_eq!(
            observed,
            (true, &Outcome::Previewed, None, Vec::new(), false),
            "a preview lands, writes nothing, and leaves no directory behind"
        );
        let intended = result.plan().scope();
        assert_eq!(intended.len(), 1);
        assert!(intended[0].as_str().starts_with("captures/"));
    }

    /// The plan a preview emits is the plan the act applies: the same scope,
    /// the same actor, the same absence of compatibility impact.
    #[test]
    fn the_preview_and_the_act_agree_about_what_was_intended() {
        let vault = Vault::new("write-preview-agrees");
        let previewed = vault.preview(Thought("a loose thought"));
        let applied = vault.capture(Thought("a loose thought"));
        let agreed = (
            previewed.plan().scope(),
            previewed.plan().actor(),
            applied.plan().compatibility(),
        );
        assert_eq!(
            agreed,
            (applied.plan().scope(), applied.plan().actor(), None)
        );
        assert_eq!(created(&applied), previewed.plan().scope()[0].as_str());
    }

    /// The same input twice produces the same plan, so a caller can diff two
    /// runs; two acts still produce two notes.
    #[test]
    fn a_repeated_plan_is_identical_and_two_acts_are_two_notes() {
        let vault = Vault::new("write-repeat");
        assert_eq!(
            vault.preview(Thought("a loose thought")).plan(),
            vault.preview(Thought("a loose thought")).plan()
        );
        vault.capture(Thought("a loose thought"));
        vault.capture(Thought("a loose thought"));
        assert_eq!(vault.notes().len(), 2);
    }

    /// A corpus that already carried an error is reported and does not change
    /// the verdict: the act landed.
    #[test]
    fn a_pre_existing_corpus_error_rides_the_result_and_is_not_the_verdict() {
        let vault = Vault::new("write-pre-existing-error");
        std::fs::write(
            vault.root().path().join("wrong.md"),
            "---\ntype: nonesuch\n---\nbody\n",
        )
        .expect("a note the contract does not admit");
        let result = vault.capture(Thought("a loose thought"));
        assert!(result.landed());
        assert!(ids(&result).contains(&"note.unknown-type"));
        assert!(result.counts().error > 0);
    }

    /// The post-write reading is of the corpus the caller now has, and it is
    /// one reading rather than two: a pre-existing finding appears once.
    #[test]
    fn the_corpus_is_reported_once_and_after_the_act() {
        let vault = Vault::new("write-one-reading");
        std::fs::write(
            vault.root().path().join("wrong.md"),
            "---\ntype: nonesuch\n---\nbody\n",
        )
        .expect("a note the contract does not admit");
        let result = vault.capture(Thought("a loose thought"));
        let unknown = ids(&result)
            .iter()
            .filter(|id| **id == "note.unknown-type")
            .count();
        assert_eq!(unknown, 1);
    }

    /// A missing actor warns and does not gate: the write lands, provenance is
    /// unattributed, and a preview says the same thing in advance.
    #[test]
    fn a_capture_without_an_actor_warns_and_still_lands() {
        let vault = Vault::new("write-unattributed");
        let request = Vault::anonymous(Thought("a loose thought"), 0);
        let result = capture(vault.root(), vault.contract(), &request);
        assert!(result.landed());
        assert!(ids(&result).contains(&"write.actor-unattributed"));
        assert_eq!(result.counts().error, 0);
        let previewed = plan_capture(vault.root(), vault.contract(), &request);
        assert!(ids(&previewed).contains(&"write.actor-unattributed"));
        assert!(previewed.landed());
    }

    /// An ownership violation is one of the three refusals, and it refuses
    /// before anything is written — including before the directory is made.
    #[test]
    fn a_closed_write_catch_all_refuses_the_act_entirely() {
        let vault = Vault::holding("write-closed", CLOSED);
        let result = vault.capture(Thought("a loose thought"));
        let observed = (
            result.landed(),
            result.outcome(),
            ids(&result),
            vault.notes(),
            vault.root().path().join("captures").exists(),
        );
        assert_eq!(
            observed,
            (
                false,
                &Outcome::Refused,
                vec!["write.closed-write"],
                Vec::new(),
                false
            ),
            "an ownership refusal writes nothing at all, not even the directory"
        );
        assert!(!vault.preview(Thought("a loose thought")).landed());
    }

    /// The severity rule the verdict follows: an error among the act's own
    /// diagnostics refuses, and a warning does not.
    #[test]
    fn only_an_error_of_the_acts_own_refuses_it() {
        let error = Diagnostic::kernel(KernelDiagnostic::WriteClosedWrite, "an ownership fault");
        let warning = Diagnostic::kernel(
            KernelDiagnostic::WriteActorUnattributed,
            "nobody in particular",
        );
        let severities = (error.severity, warning.severity);
        assert_eq!(severities, (Severity::Error, Severity::Warning));
        let verdicts = (refused(&[error]), refused(&[warning]), refused(&[]));
        assert_eq!(
            verdicts,
            (true, false, false),
            "an error refuses; a warning and silence do not"
        );
    }

    /// A capture directory that is a symbolic link out of the vault is refused
    /// rather than followed: a note written through one would be written and
    /// then invisible.
    #[cfg(unix)]
    #[test]
    fn a_capture_directory_pointing_out_of_the_vault_is_refused() {
        let vault = Vault::new("write-outside");
        let outside = vault.link_capture_directory_outside();
        let result = vault.capture(Thought("a loose thought"));
        assert!(!result.landed());
        assert_eq!(ids(&result), ["write.target-outside-vault"]);
        assert_eq!(
            std::fs::read_dir(&outside)
                .expect("the directory outside")
                .count(),
            0
        );
    }

    /// The per-candidate containment check is defence in depth: [`resolve`]
    /// refuses an outside directory before anything is created, so this guard
    /// answers only for a target that became outside afterwards — a link made
    /// between the two. Exercised directly, because a race is not a fixture.
    #[cfg(unix)]
    #[test]
    fn a_candidate_outside_the_root_is_refused_and_written_nowhere() {
        let vault = Vault::new("write-candidate-outside");
        let outside = vault.link_capture_directory_outside();
        let target = Target {
            directory: outside.clone(),
            declared: "captures".to_owned(),
        };
        let minted = Minted {
            name: "note.md".to_owned(),
            contents: "a loose thought".to_owned(),
        };
        let mut acted = Vec::new();
        let written = write(vault.root(), &target, &minted, &mut acted);
        assert_eq!(written, None);
        assert_eq!(
            acted
                .iter()
                .map(|diagnostic| diagnostic.id.as_str())
                .collect::<Vec<&str>>(),
            ["write.target-outside-vault"]
        );
        assert!(!outside.join("note.md").exists());
    }

    /// A capture directory nested under a link out of the vault is refused
    /// **before** anything is created, so the refusal leaves no directory
    /// behind it outside the root.
    #[cfg(unix)]
    #[test]
    fn a_capture_directory_under_a_link_is_refused_before_anything_is_made() {
        let vault = Vault::holding("write-outside-nested", fixture::NESTED);
        let outside = vault.link_capture_directory_outside();
        let result = vault.capture(Thought("a loose thought"));
        assert!(!result.landed());
        assert_eq!(ids(&result), ["write.target-outside-vault"]);
        assert_eq!(
            std::fs::read_dir(&outside)
                .expect("the directory outside")
                .count(),
            0,
            "the refusal made a directory outside the vault"
        );
    }

    /// A directory that cannot be made is the other refusal, and it names the
    /// cause rather than only the fact.
    #[cfg(unix)]
    #[test]
    fn a_capture_directory_that_cannot_be_made_is_refused_with_the_reason() {
        let vault = Vault::new("write-unwritable");
        crate::vault::tree::set_mode(vault.root().path(), 0o500);
        let result = vault.capture(Thought("a loose thought"));
        crate::vault::tree::set_mode(vault.root().path(), 0o700);
        assert!(!result.landed());
        assert_eq!(ids(&result), ["write.target-unwritable"]);
        let reported = &result.diagnostics()[0];
        assert!(reported.message.contains("could not be created"));
    }

    /// The directory exists and the file cannot be created in it: the other
    /// half of the unwritable refusal, and the one that happens after the
    /// target has been resolved.
    #[cfg(unix)]
    #[test]
    fn a_note_that_cannot_be_created_is_refused_with_the_reason() {
        let vault = Vault::new("write-file-unwritable");
        let captures = vault.root().path().join("captures");
        std::fs::create_dir(&captures).expect("the capture directory");
        crate::vault::tree::set_mode(&captures, 0o500);
        let result = vault.capture(Thought("a loose thought"));
        crate::vault::tree::set_mode(&captures, 0o700);
        assert!(!result.landed());
        assert_eq!(ids(&result), ["write.target-unwritable"]);
        assert!(
            result.diagnostics()[0]
                .message
                .contains("could not be written")
        );
    }

    /// The collision bound is a bound rather than an unbounded search, and the
    /// refusal says so rather than spinning.
    #[test]
    fn a_second_that_is_entirely_taken_refuses_rather_than_spinning() {
        let vault = Vault::new("write-collision-limit");
        let captures = vault.root().path().join("captures");
        std::fs::create_dir(&captures).expect("the capture directory");
        let name = "1970-01-01-000000-crowded.md";
        std::fs::write(captures.join(name), "taken").expect("the first name");
        for nth in 2..=COLLISION_LIMIT {
            let taken = identity::nth(name, nth);
            std::fs::write(captures.join(taken), "taken").expect("every other name");
        }
        let result = vault.capture(Thought("crowded"));
        assert!(!result.landed());
        assert_eq!(ids(&result), ["write.target-unwritable"]);
        assert!(result.diagnostics()[0].message.contains("no name was free"));
    }

    /// Where the substrate owns the commit path, the act commits exactly the
    /// created file, the result names the commit, and reverting it is recovery.
    #[test]
    fn a_capture_into_a_repository_commits_at_birth_with_the_trailer_pair() {
        let vault = Vault::repository("write-commit");
        let result = vault.capture(Thought("a loose thought"));
        assert!(result.landed());
        let path = created(&result);
        let commit = committed(&result)
            .expect("a repository vault commits")
            .to_owned();
        assert!(!commit.is_empty());
        assert_eq!(
            result.recovery(),
            Some(Recovery::Revert {
                commit: commit.clone(),
                path: vault
                    .root()
                    .relative(&vault.root().path().join(&path))
                    .expect("under the root"),
            })
        );
        let shown = git(
            vault.root().path(),
            &["show", "--name-only", "--format=%B", &commit],
        );
        assert!(shown.contains("Dogtag-Actor: A Maintainer"), "{shown}");
        assert!(shown.contains("Dogtag-Provenance: agent"), "{shown}");
        assert!(shown.contains(&path), "{shown}");
    }

    /// The commit is pathspec-scoped: a concurrent writer's file, present and
    /// even staged, is neither committed nor disturbed.
    #[test]
    fn the_commit_takes_the_created_file_and_nothing_else() {
        let vault = Vault::repository("write-commit-scope");
        std::fs::write(
            vault.root().path().join("theirs.md"),
            "somebody else's work\n",
        )
        .expect("a concurrent writer's file");
        git(vault.root().path(), &["add", "--", "theirs.md"]);
        let result = vault.capture(Thought("mine"));
        let path = created(&result);
        let commit = committed(&result)
            .expect("a repository vault commits")
            .to_owned();
        let files = git(
            vault.root().path(),
            &["show", "--name-only", "--format=", &commit],
        );
        assert_eq!(files.trim(), path, "the commit holds exactly the one file");
        assert!(vault.root().path().join("theirs.md").exists());
    }

    /// A commit that cannot happen is a warning and not a refusal: the note is
    /// on disk, and deleting it is recovery.
    #[test]
    fn a_commit_that_fails_leaves_the_note_and_says_so() {
        let vault = Vault::new("write-commit-fails");
        // A `.git` that is not a repository: the substrate owns the commit path
        // by the rule that decides ownership, and git refuses.
        std::fs::create_dir(vault.root().path().join(".git")).expect("a hollow git directory");
        let result = vault.capture(Thought("a loose thought"));
        assert!(result.landed());
        assert!(ids(&result).contains(&"write.commit-failed"));
        assert_eq!(result.counts().error, 0);
        let path = created(&result);
        assert_eq!(committed(&result), None);
        assert!(vault.read(&path).contains("a loose thought"));
    }

    /// One git invocation, for the assertions that read a repository back.
    fn git(root: &Path, arguments: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .arg("-C")
            .arg(root)
            .args(arguments)
            .output()
            .expect("git is on the path");
        output
            .status
            .success()
            .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
            .expect("git answered the question this assertion asked")
    }

    /// A request answers for what it was built from, and compares and formats.
    #[test]
    fn a_request_carries_its_text_its_instant_and_its_actor() {
        let request = Vault::request(Thought("a loose thought"), 42);
        let carried = (
            request.text(),
            request.at().unix_seconds(),
            request.actor().kind(),
        );
        assert_eq!(carried, ("a loose thought", 42, ProvenanceKind::Agent));
        assert_eq!(request.clone(), request);
        assert_ne!(request, Vault::anonymous(Thought("a loose thought"), 42));
        assert!(format!("{request:?}").contains("a loose thought"));
    }
}
