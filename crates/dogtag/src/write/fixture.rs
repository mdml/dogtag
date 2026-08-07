//! Vaults the write tests own, and the acts they perform against them.
//!
//! A write test needs more than a contract: it needs a real directory it may
//! create files in, a resolved contract to write against, and — for the one
//! case that is about committing — a repository. Building those is enough work
//! that doing it per test would bury what each test is actually about, so it is
//! done once here.
//!
//! Every contract is authored in this module, for the reason
//! [`crate::report::fixture`] authors its own: the conformance profiles'
//! committed contracts are the conformance suite's subject, and reaching
//! sideways into that tree would invert a dependency the architecture runs one
//! way.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract::{Contract, DEFAULT_CAPTURE_DIRECTORY, parse_contract};
use crate::vault::tree::Tree;
use crate::vault::{SENTINEL, VaultRoot, root_at};

use super::{Actor, CaptureRequest, CapturedAt, ProvenanceKind, WriteResult};

/// A version-3 contract whose catch-all is born carrying the flag it declares.
///
/// The shape `starter` commits, which is what makes the flagged path the one
/// these tests exercise by default.
pub(crate) const BORN_FLAGGED: &str = concat!(
    "contract_version = 3\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[[flag]]\nproperty = \"needs_triage\"\n",
    "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\"]\n",
    "born-flagged = [\"needs_triage\"]\n",
    "\n  [[type.property]]\n  name = \"needs_triage\"\n  kind = \"boolean\"\n",
);

/// A version-2 contract: no capture seat, no birth state, and a catch-all all
/// the same. The shape `dense` and `docs` commit.
pub(crate) const OLDER: &str = concat!(
    "contract_version = 2\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\"]\n",
);

/// A version-3 contract declaring a capture directory of its own.
pub(crate) const ELSEWHERE: &str = concat!(
    "contract_version = 3\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[capture]\ndirectory = \"unfiled/raw\"\n",
    "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\"]\n",
);

/// A version-3 contract whose capture directory is nested one level down.
///
/// What makes the containment check's timing visible: the outer directory is
/// the one a test can replace with a link, and the inner one is the one the
/// write would create inside it.
pub(crate) const NESTED: &str = concat!(
    "contract_version = 3\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[capture]\ndirectory = \"captures/raw\"\n",
    "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\"]\n",
);

/// A version-3 contract whose catch-all no caller may modify.
pub(crate) const CLOSED: &str = concat!(
    "contract_version = 3\n",
    "\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\", \"closed-write\"]\n",
);

/// The text someone captured.
///
/// A newtype rather than a `&str`, for the reason [`crate::report::fixture`]'s
/// `Body` is one: a fixture hands around several kinds of string — a label, a
/// contract, a vault-relative path — and the one that is *what a person typed*
/// is the one that must never be passed where another was meant, because the
/// write path will happily capture a label.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Thought<'a>(pub(crate) &'a str);

/// A vault a write test owns outright, and its resolved contract.
pub(crate) struct Vault {
    tree: Tree,
    root: VaultRoot,
    contract: Contract,
}

impl Vault {
    /// A vault at `source`, named for the test that built it.
    ///
    /// # Panics
    ///
    /// If the tree cannot be built or the contract does not resolve — either is
    /// a fixture that would make every assertion against it meaningless.
    pub(crate) fn holding(label: &str, source: &str) -> Self {
        let tree = Tree::new(label);
        let path = tree.vault("vault");
        fs::write(path.join(SENTINEL), source).expect("a committed contract");
        let root = root_at(&path)
            .expect("a directory this fixture just made a vault root")
            .into_root();
        let contract = parse_contract(source)
            .contract
            .expect("a contract this fixture expects to resolve");
        Self {
            tree,
            root,
            contract,
        }
    }

    /// The vault every test uses unless it is about something else.
    pub(crate) fn new(label: &str) -> Self {
        Self::holding(label, BORN_FLAGGED)
    }

    /// The same vault, inside a repository whose commit path this substrate
    /// therefore owns.
    ///
    /// The identity is configured on the repository rather than read from the
    /// machine, so a developer with no global git identity runs these tests
    /// exactly as one with an identity does.
    ///
    /// # Panics
    ///
    /// If git is not on the path at all, which every other part of this
    /// repository's tooling already requires.
    pub(crate) fn repository(label: &str) -> Self {
        let vault = Self::new(label);
        let path = vault.root.path();
        for arguments in [
            &["init", "--quiet"][..],
            &["config", "user.name", "A Maintainer"],
            &["config", "user.email", "maintainer@example.invalid"],
            &["config", "commit.gpgsign", "false"],
        ] {
            Command::new("git")
                .arg("-C")
                .arg(path)
                .args(arguments)
                .status()
                .expect("git is on the path")
                .success()
                .then_some(())
                .expect("git prepared the fixture repository");
        }
        vault
    }

    /// The verified root every write resolves its target through.
    pub(crate) fn root(&self) -> &VaultRoot {
        &self.root
    }

    /// The resolved contract the write is judged against.
    pub(crate) fn contract(&self) -> &Contract {
        &self.contract
    }

    /// The tree the vault sits in, which is where a test puts anything that
    /// must be outside it.
    pub(crate) fn tree(&self) -> &Path {
        self.tree.path()
    }

    /// An actor named, or not, acting as an agent.
    fn actor(named: bool) -> Actor {
        let name = named.then(|| "A Maintainer".to_owned());
        Actor::new(name, ProvenanceKind::Agent)
    }

    /// A request for `text`, captured at `seconds`, by a named actor.
    pub(crate) fn request(thought: Thought<'_>, seconds: u64) -> CaptureRequest {
        CaptureRequest::new(
            thought.0,
            CapturedAt::from_unix_seconds(seconds),
            Self::actor(true),
        )
    }

    /// The same, by nobody.
    pub(crate) fn anonymous(thought: Thought<'_>, seconds: u64) -> CaptureRequest {
        CaptureRequest::new(
            thought.0,
            CapturedAt::from_unix_seconds(seconds),
            Self::actor(false),
        )
    }

    /// Captures `text` at a fixed instant.
    pub(crate) fn capture(&self, thought: Thought<'_>) -> WriteResult {
        super::capture(&self.root, &self.contract, &Self::request(thought, 0))
    }

    /// Plans a capture of `text` at the same instant, and writes nothing.
    pub(crate) fn preview(&self, thought: Thought<'_>) -> WriteResult {
        super::plan_capture(&self.root, &self.contract, &Self::request(thought, 0))
    }

    /// The bytes of a file the vault holds.
    ///
    /// # Panics
    ///
    /// If the file is not there, which for a path a result just named would
    /// mean the result named a file the act did not create.
    pub(crate) fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.root.path().join(relative))
            .expect("a file the act under test said it created")
    }

    /// Every note the vault holds, in path order.
    pub(crate) fn notes(&self) -> Vec<String> {
        crate::note::traverse(&self.root)
            .notes()
            .iter()
            .map(|path| path.as_str().to_owned())
            .collect()
    }

    /// Replaces the capture directory with a symbolic link out of the vault,
    /// which is the one way a target this SDK composed can leave the root.
    ///
    /// # Panics
    ///
    /// If the link cannot be made.
    #[cfg(unix)]
    pub(crate) fn link_capture_directory_outside(&self) -> PathBuf {
        let outside = self.tree.dir("outside");
        let link = self.root.path().join(DEFAULT_CAPTURE_DIRECTORY);
        std::os::unix::fs::symlink(&outside, &link).expect("a symbolic link");
        outside
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each authored contract resolves, which is what every fixture built from
    /// one relies on without saying so.
    #[test]
    fn every_authored_contract_resolves() {
        for source in [BORN_FLAGGED, OLDER, ELSEWHERE, CLOSED, NESTED] {
            assert!(parse_contract(source).contract.is_ok());
        }
    }

    #[test]
    fn a_vault_answers_for_its_root_its_contract_and_its_tree() {
        let vault = Vault::holding("fixture-answers", OLDER);
        assert!(vault.root().path().join(SENTINEL).exists());
        assert_eq!(vault.contract().contract_version(), 2);
        assert!(vault.root().path().starts_with(vault.tree()));
        assert_eq!(vault.notes(), Vec::<String>::new());
    }

    #[test]
    fn a_request_is_named_or_anonymous_and_carries_its_instant() {
        let named = Vault::request(Thought("text"), 7);
        assert_eq!(named.actor().name(), Some("A Maintainer"));
        assert_eq!(named.at().unix_seconds(), 7);
        assert_eq!(named.text(), "text");
        assert_eq!(Vault::anonymous(Thought("text"), 7).actor().name(), None);
    }

    #[test]
    fn a_capture_lands_and_is_readable_back_by_the_path_it_named() {
        let vault = Vault::new("fixture-round-trip");
        let result = vault.capture(Thought("a loose thought"));
        let path = result.recovery().expect("it landed");
        assert!(vault.read(path.path().as_str()).contains("a loose thought"));
        assert_eq!(vault.notes(), [path.path().as_str().to_owned()]);
        assert!(vault.preview(Thought("another")).landed());
    }

    #[cfg(unix)]
    #[test]
    fn a_capture_directory_can_be_pointed_out_of_the_vault() {
        let vault = Vault::new("fixture-link");
        let outside = vault.link_capture_directory_outside();
        assert!(outside.starts_with(vault.tree()));
        assert!(!outside.starts_with(vault.root().path()));
    }

    #[test]
    fn a_repository_vault_owns_its_commit_path() {
        let vault = Vault::repository("fixture-repository");
        assert!(super::super::commit::owns_commit_path(vault.root().path()));
    }
}
