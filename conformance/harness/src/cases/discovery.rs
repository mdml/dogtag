//! The three discovery cases, and the hermeticity they depend on.
//!
//! **These cases need a synthetic tree the run controls, not a per-profile
//! input.** The profile contributes a contract that discovery never parses,
//! because the sentinel is a path test; what varies between them is nothing.
//! They are honestly one case each rather than four, and saying so is part of
//! the rule — pretending the cross product multiplies them would claim a
//! coverage this suite does not have.
//!
//! The upward walk has **no boundary**: not `.git`, not `$HOME`, not a mount
//! point. So "no vault above here" is otherwise a property of the machine, and
//! one developer's directory layout would be reaching a conformance result.
//! Every case below proves no ancestor of its tree holds the sentinel before
//! asserting anything, and fails loudly with an explanatory message rather
//! than passing or silently skipping if one does.

use std::fs;
use std::path::Path;

use dogtag::vault::{
    CONTRACT_FILE_NAME, Discovered, SENTINEL, SENTINEL_DIRECTORY, discover, root_at,
};

use crate::temptree::TempTree;

use super::corpus::Corpus;
use super::expect::{Checked, ids, rendered, require, require_contains};

/// `vault-root-discovered-from-a-nested-path`.
pub fn nested_path_discovery(corpus: &Corpus) -> Checked {
    hermetic(corpus.root())?;
    let nested = corpus.root().join("area").join("beneath").join("deep");
    make_dir(&nested)?;

    let direct = discover(&nested);
    let root = resolved(&direct, "discovery from a nested directory")?;
    // The walk stopped at the nearest ancestor holding the contract, and
    // nothing but the starting path decided which one that was: the answer is
    // a directory inside a tree this run built moments ago, which no current
    // directory, environment variable or process-global state names. That is
    // as far as an outside observer can prove the purity claim; the SDK's own
    // tests prove it from the inside.
    require(root == corpus.root(), || {
        format!(
            "discovery from {} resolved {} rather than the enclosing root {}",
            nested.display(),
            root.display(),
            corpus.root().display()
        )
    })?;
    require(direct.diagnostics.is_empty(), || {
        format!(
            "discovery from a path that is already canonical reported {}",
            rendered(&direct.diagnostics)
        )
    })?;
    through_a_symbolic_link(corpus, &nested, root)
}

/// The same nested directory reached through a symbolic link resolves to the
/// same canonical root, and the difference between the requested path and the
/// canonical one is reported rather than left silent.
fn through_a_symbolic_link(corpus: &Corpus, nested: &Path, root: &Path) -> Checked {
    let link = corpus.tree().join("link-to-the-nested-directory");
    symlink(nested, &link)?;
    let via = discover(&link);
    let through = resolved(&via, "discovery through a symbolic link")?;
    require(through == root, || {
        format!(
            "the symbolic link resolved {} rather than {}",
            through.display(),
            root.display()
        )
    })?;
    require(
        ids(&via.diagnostics).contains(&"discovery.root-resolved-through-symlink"),
        || {
            format!(
                "discovery through a symbolic link must report the difference, but reported {}",
                rendered(&via.diagnostics)
            )
        },
    )
}

/// The identifier a walk halted by a damaged vault root carries.
const INCOMPLETE_ROOT: &str = "discovery.incomplete-vault-root";

/// The identifier a walk that reached the filesystem root carries.
const NO_VAULT_FOUND: &str = "discovery.no-vault-found";

/// `incomplete-vault-root-halts-discovery`.
pub fn incomplete_root_halts(corpus: &Corpus) -> Checked {
    hermetic(corpus.root())?;
    let incomplete = corpus.root().join("half-configured");
    make_dir(&incomplete.join(SENTINEL_DIRECTORY))?;
    // The walk halted rather than delegating upward: had it continued it would
    // have resolved the enclosing root, and reported success against a corpus
    // the caller never named.
    halted_at(
        &discover(&incomplete),
        INCOMPLETE_ROOT,
        &incomplete,
        "discovery from an incomplete root",
    )
}

/// A walk that halted: it resolved nothing, reported **exactly** `id`, and
/// named `at` while doing so.
///
/// Both halting cases have this shape, and the word carrying them is *exactly*.
/// A walk that resolved a root delegated to a corpus the caller never named. A
/// walk that reported a second identifier, or the other halting identifier,
/// leaves a reader unable to tell a damaged vault from an absent one — and the
/// two call for different actions. Each case demanding its own identifier
/// exactly is what keeps the two apart: were the SDK ever to report one
/// identifier for both, one of the two cases would fail.
fn halted_at(found: &Discovered, id: &str, at: &Path, subject: &str) -> Checked {
    require(found.root.is_none(), || {
        format!("{subject} must resolve nothing: a root resolved here is a corpus nobody named")
    })?;
    require(ids(&found.diagnostics) == vec![id], || {
        format!(
            "{subject} must report exactly `{id}`, but reported {}",
            rendered(&found.diagnostics)
        )
    })?;
    require_contains(
        &rendered(&found.diagnostics),
        &at.display().to_string(),
        subject,
    )
}

/// `explicit-vault-root-is-used-exactly`.
pub fn explicit_root_used_exactly(corpus: &Corpus) -> Checked {
    hermetic(corpus.root())?;
    let resolved = root_at(corpus.root()).map_err(|diagnostic| {
        format!(
            "an explicit vault root must be accepted, but was refused with {}",
            diagnostic.id.as_str()
        )
    })?;
    let root = resolved.root();
    require(root.path() == corpus.root(), || {
        format!(
            "an explicit root is used as given, but `{}` verified as `{}`",
            corpus.root().display(),
            root.path().display()
        )
    })?;
    // The corpus copy is already canonical, so resolving it reports nothing:
    // the symlink info is for a root the caller did not type.
    require(resolved.diagnostics().is_empty(), || {
        format!(
            "a canonical explicit root reports nothing, but reported {:?}",
            ids(resolved.diagnostics())
        )
    })?;

    let inside = corpus.root().join("inside-the-vault");
    make_dir(&inside)?;
    let refusal = root_at(&inside)
        .err()
        .ok_or_else(|| "a directory inside a vault is not a vault root".to_owned())?;
    require(refusal.id.as_str() == "discovery.not-a-vault-root", || {
        format!(
            "an explicit non-root must be refused with `discovery.not-a-vault-root`, not {}",
            refusal.id.as_str()
        )
    })?;
    no_vault_above()
}

/// Discovery from a directory with no vault above it, in a tree this run
/// controls entirely.
fn no_vault_above() -> Checked {
    let tree = TempTree::new("no-vault-above");
    let start = tree.path().join("start").join("here");
    make_dir(&start)?;
    hermetic(&start)?;

    let found = discover(&start);
    halted_at(
        &found,
        NO_VAULT_FOUND,
        &start,
        "a tree with no vault above it",
    )?;
    // And it names the sentinel it looked for, so a reader learns what would
    // have made the directory a vault root.
    require_contains(
        &rendered(&found.diagnostics),
        SENTINEL,
        "the no-vault-found diagnostic",
    )
}

/// Proves no directory strictly above `start` holds the sentinel.
///
/// `start` itself is excluded because two of the three callers pass a vault
/// root, which of course holds it; what must be true in every case is that
/// nothing *above* the tree this run built does.
///
/// Failing here is not a scenario failure but a harness failure, and it says
/// so: the tree the walk runs in stopped being one the run controls, and no
/// result from it would mean anything.
fn hermetic(start: &Path) -> Checked {
    for ancestor in start.ancestors().skip(1) {
        let sentinel = ancestor.join(SENTINEL_DIRECTORY).join(CONTRACT_FILE_NAME);
        require(!sentinel.is_file(), || {
            format!(
                "HARNESS NOT HERMETIC: {} holds {SENTINEL}, which is above the tree this run \
                 controls. The upward walk has no boundary, so a discovery result taken here \
                 would be a property of this machine rather than of the SDK. Remove that vault \
                 or run with a temp directory outside it.",
                ancestor.display()
            )
        })?;
    }
    Ok(())
}

/// The root a discovery resolved, or why it resolved none.
fn resolved<'a>(discovered: &'a Discovered, subject: &str) -> Result<&'a Path, String> {
    discovered
        .root
        .as_ref()
        .map(|root| root.path())
        .ok_or_else(|| {
            format!(
                "{subject} resolved no root, reporting {}",
                rendered(&discovered.diagnostics)
            )
        })
}

/// Creates a directory and everything above it.
fn make_dir(path: &Path) -> Checked {
    fs::create_dir_all(path).map_err(|error| format!("creating {} failed: {error}", path.display()))
}

/// A symbolic link at `link` pointing at `target`.
#[cfg(unix)]
fn symlink(target: &Path, link: &Path) -> Checked {
    std::os::unix::fs::symlink(target, link)
        .map_err(|error| format!("linking {} failed: {error}", link.display()))
}

/// The scenario is about symbolic links, so a platform without them cannot run
/// it. It says so rather than passing.
#[cfg(not(unix))]
fn symlink(_target: &Path, _link: &Path) -> Checked {
    Err("this scenario needs symbolic links, which this platform does not provide".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    use dogtag::diagnostic::{Diagnostic, KernelDiagnostic};

    use super::super::corpus::NO_AXIS;

    /// A directory a synthetic diagnostic is about.
    const SOMEWHERE: &str = "/a/directory/this/run/named";

    /// A walk that halted, as the SDK reports one.
    fn halted(kind: KernelDiagnostic, message: &str) -> Discovered {
        Discovered {
            root: None,
            diagnostics: vec![Diagnostic::kernel(kind, message)],
        }
    }

    /// A second vault at `at`, inside `corpus` — the shape each case below is
    /// written to notice.
    fn vault_inside(corpus: &Corpus, at: &str) {
        let sentinel = corpus.root().join(at).join(SENTINEL_DIRECTORY);
        make_dir(&sentinel).expect("a nested sentinel directory");
        fs::write(sentinel.join(CONTRACT_FILE_NAME), NO_AXIS).expect("a nested contract");
    }

    /// The detail a case answered, or a panic naming what it accepted instead.
    fn refusal(checked: Checked) -> String {
        checked.expect_err("the case must not pass against a tree it did not expect")
    }

    /// The walk stops at the *nearest* root, so a vault nested beneath the
    /// corpus would answer a different root than the case is about. It says
    /// which root it got rather than reporting a success against a corpus
    /// nobody named.
    #[test]
    fn a_vault_nested_beneath_the_root_fails_the_discovery_case() {
        let corpus = Corpus::holding("discovery-nested-vault", NO_AXIS);
        vault_inside(&corpus, "area/beneath/deep");
        let detail = refusal(nested_path_discovery(&corpus));
        assert!(
            detail.contains("rather than the enclosing root"),
            "the failure says what went wrong: {detail}"
        );
        assert!(
            detail.contains("deep"),
            "the failure names the root it resolved: {detail}"
        );
    }

    /// A path that reaches the same directory through a symbolic link is not
    /// the canonical path, and discovery says so. The case asserts the silence
    /// of the canonical route, so a link on that route fails it.
    #[test]
    fn a_nested_path_reached_through_a_link_is_not_a_canonical_path() {
        let corpus = Corpus::holding("discovery-linked-area", NO_AXIS);
        let elsewhere = corpus.root().join("elsewhere");
        make_dir(&elsewhere.join("beneath").join("deep")).expect("a directory to link at");
        symlink(&elsewhere, &corpus.root().join("area")).expect("a link into the vault");
        let detail = refusal(nested_path_discovery(&corpus));
        assert!(
            detail.contains("already canonical reported"),
            "the failure says what went wrong: {detail}"
        );
        assert!(
            detail.contains("discovery.root-resolved-through-symlink"),
            "the failure carries what was reported: {detail}"
        );
    }

    /// The incomplete-root case is about a walk that halts. A directory that
    /// turns out to hold a whole vault resolves one, and the case fails rather
    /// than reading the resolution as a halt.
    #[test]
    fn a_complete_vault_where_an_incomplete_one_belongs_fails_the_case() {
        let corpus = Corpus::holding("discovery-completed-half", NO_AXIS);
        vault_inside(&corpus, "half-configured");
        let detail = refusal(incomplete_root_halts(&corpus));
        assert!(
            detail.contains("must resolve nothing"),
            "the failure says what was expected: {detail}"
        );
    }

    /// An explicitly named root that is not a root at all fails the case, and
    /// the failure carries the identifier the SDK refused it with.
    #[test]
    fn a_corpus_that_is_not_a_vault_root_fails_the_explicit_root_case() {
        let corpus = Corpus::empty("discovery-not-a-root");
        let detail = refusal(explicit_root_used_exactly(&corpus));
        assert!(
            detail.contains("an explicit vault root must be accepted"),
            "the failure says what was expected: {detail}"
        );
        assert!(
            detail.contains("discovery.not-a-vault-root"),
            "the failure carries the identifier: {detail}"
        );
    }

    /// A directory inside the vault that is *itself* a damaged root is refused
    /// under a different identifier, and the case insists on the one it is
    /// about: a damaged vault and a non-root call for different actions.
    #[test]
    fn a_damaged_directory_inside_the_vault_is_refused_under_its_own_identifier() {
        let corpus = Corpus::holding("discovery-damaged-inside", NO_AXIS);
        let damaged = corpus
            .root()
            .join("inside-the-vault")
            .join(SENTINEL_DIRECTORY);
        make_dir(&damaged).expect("a sentinel directory with no contract in it");
        let detail = refusal(explicit_root_used_exactly(&corpus));
        assert!(
            detail.contains("must be refused with `discovery.not-a-vault-root`"),
            "the failure says which refusal was expected: {detail}"
        );
        assert!(
            detail.contains("discovery.incomplete-vault-root"),
            "the failure names the refusal that arrived: {detail}"
        );
    }

    /// Hermeticity is proved before anything is asserted, and a tree the run
    /// does not control fails the **harness** rather than the scenario — with
    /// the ancestor named and the remedy stated.
    #[test]
    fn a_tree_under_a_vault_is_not_hermetic_and_says_so() {
        let corpus = Corpus::holding("discovery-not-hermetic", NO_AXIS);
        let detail = refusal(hermetic(&corpus.root().join("beneath")));
        assert!(
            detail.contains("HARNESS NOT HERMETIC"),
            "the failure is labelled as the harness's: {detail}"
        );
        assert!(
            detail.contains(&corpus.root().display().to_string()),
            "the failure names the ancestor: {detail}"
        );
        assert!(
            detail.contains("Remove that vault"),
            "the failure says what to do: {detail}"
        );
    }

    /// A walk that resolved a root did not halt, whatever else it reported —
    /// and a root resolved from a directory the caller named is a corpus the
    /// caller did not.
    #[test]
    fn a_walk_that_resolved_a_root_did_not_halt() {
        let corpus = Corpus::holding("discovery-halted-but-resolved", NO_AXIS);
        let found = Discovered {
            root: Some(root_at(corpus.root()).expect("a vault root").into_root()),
            diagnostics: Vec::new(),
        };
        let detail = refusal(halted_at(
            &found,
            NO_VAULT_FOUND,
            corpus.root(),
            "a tree with no vault above it",
        ));
        assert!(
            detail.contains("must resolve nothing"),
            "the failure says what was expected: {detail}"
        );
    }

    /// A damaged vault and an absent one must not be read as each other, so
    /// each case demands its own identifier exactly and names what arrived.
    #[test]
    fn a_walk_reporting_the_other_halting_identifier_is_refused() {
        let found = halted(KernelDiagnostic::DiscoveryNoVaultFound, SOMEWHERE);
        let detail = refusal(halted_at(
            &found,
            INCOMPLETE_ROOT,
            Path::new(SOMEWHERE),
            "discovery from an incomplete root",
        ));
        assert!(
            detail.contains("must report exactly `discovery.incomplete-vault-root`"),
            "the failure names the identifier: {detail}"
        );
        assert!(
            detail.contains("discovery.no-vault-found"),
            "the failure names what arrived: {detail}"
        );
    }

    /// A halting diagnostic that does not name the directory it is about is
    /// unactionable: the walk has no boundary, so *somewhere above here* is
    /// not an answer.
    #[test]
    fn a_walk_that_does_not_name_the_directory_is_refused() {
        let found = halted(KernelDiagnostic::DiscoveryNoVaultFound, "no vault found");
        let detail = refusal(halted_at(
            &found,
            NO_VAULT_FOUND,
            Path::new(SOMEWHERE),
            "a tree with no vault above it",
        ));
        assert!(
            detail.contains(&format!("must carry `{SOMEWHERE}`")),
            "the failure names the directory it wanted: {detail}"
        );
    }

    /// A discovery that resolved nothing reports what it said instead, because
    /// *no root* on its own does not say which of the ways it happened.
    #[test]
    fn a_discovery_that_resolved_no_root_says_what_it_reported() {
        let discovered = Discovered {
            root: None,
            diagnostics: Vec::new(),
        };
        let detail = resolved(&discovered, "discovery from a nested directory")
            .expect_err("a discovery with no root resolved none");
        assert!(
            detail.contains("discovery from a nested directory resolved no root"),
            "the failure names the subject: {detail}"
        );
        assert!(
            detail.contains("reporting nothing"),
            "the failure says what was reported: {detail}"
        );
    }
}
