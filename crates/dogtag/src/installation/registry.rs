//! Resolving a registry name against an explicitly supplied record.
//!
//! `--vault work` and `--vault ./work` are different questions, and the first
//! one is answered here. An argument holding a path separator, or beginning
//! with `.`, `/` or `~`, is **always** a path; any other argument is **always**
//! a registry name. There is **no fallback between them**, which is what makes
//! selection deterministic: *path-if-it-exists-else-name* would resolve
//! `--vault work` differently depending on whether a `work` directory happens
//! to sit in the current directory, and *name-first-else-path* would let
//! registering a vault named `docs` silently change what `--vault docs` means
//! in every script already on the machine.
//!
//! The consequence is that a bare name can never mean the directory of the same
//! name, so the refusal for an unregistered name has to *teach* the correction
//! rather than merely report the absence.
//!
//! Resolution lives in the SDK rather than in the CLI because its failures are
//! `installation.*` diagnostics, and only the SDK may mint an identifier
//! outside the `ext.` namespace. The [`Installation`] arrives as an argument,
//! so the no-ambient-state rule survives intact: the caller still decides
//! *which* record, using `XDG_CONFIG_HOME`.
//!
//! It is the whole [`Installation`] rather than a loaded [`InstallationRecord`]
//! because a name can be asked about on a machine that has no record and on one
//! whose record did not load, and a function taking a record cannot be called
//! in either state. A consumer left to answer those two would have to mint an
//! identifier in the `ext.` namespace for a situation the kernel already
//! models — and then every binding would answer the same situation with an
//! identifier of its own, which is exactly the drift shared diagnostic
//! identifiers exist to prevent.

use crate::diagnostic::{Diagnostic, FileRef, KernelDiagnostic, Location, Related};
use crate::vault::{self, Resolved, SENTINEL};

use super::{Installation, InstallationRecord, InstallationState, VaultEntry};

/// What an unknown name means on a machine that has never had a record.
const NO_RECORD: &str =
    ": no installation record exists at all, so no vault can be registered under any name";

/// What it means where a record exists and could not be used.
const UNUSABLE_RECORD: &str = ": the installation record did not load, so the name cannot be \
                               resolved from it — the diagnostics reported when it was read say \
                               why";

/// What it means where a record loaded and registers nothing.
const NO_VAULTS: &str = ": the installation record registers no vaults at all";

/// Resolves a registry name against the installation a caller read.
///
/// The entry's path is verified through [`vault::root_at`], so there is exactly
/// one implementation of what the sentinel is and the registered path is used
/// **exactly**: a registered name never searches upward and never falls through
/// to discovery. A registry entry pointing inside a vault is a broken entry,
/// not a shorthand for the vault around it.
///
/// All three states a record can be in are answerable here, which is the reason
/// the argument is the [`Installation`]: a name asked about on a machine with no
/// record, or against a record that did not load, is refused with the same
/// kernel identifier as a name that is simply not registered, and the message
/// says which of the three it was.
///
/// This is a pure function of its two arguments. It reads no environment
/// variable and no current directory; the only filesystem access is the
/// verification of the registered path.
///
/// # Errors
///
/// `installation.unknown-vault-name` when nothing answers the name: no entry
/// carries it, the record registers nothing at all, no record exists, or the
/// record did not load. Which of the four it was travels in the message rather
/// than in an identifier of its own, because a caller matching on the
/// identifier recovers the same way from all four.
/// `installation.vault-path-not-a-root` when the entry's path is absent, or
/// exists and is not a vault root; the discovery diagnostic that established it
/// is mapped into the installation area, because the caller asked about a
/// registry entry and that is what failed.
#[expect(
    clippy::result_large_err,
    reason = "this SDK has exactly one error type by decision: every foreseeable failure is a \
              Diagnostic with an identifier, carrying a message, a location and evidence, so it \
              is over the lint's threshold by construction. Boxing it would push that \
              consequence onto every consumer and every binding of a public API whose shape is \
              fixed."
)]
pub fn resolve_registered(name: &str, installation: &Installation) -> Result<Resolved, Diagnostic> {
    match installation.state() {
        InstallationState::Loaded(record) => registered(name, record),
        InstallationState::Absent => Err(unknown_vault_name(name, NO_RECORD)),
        InstallationState::Unusable => Err(unknown_vault_name(name, UNUSABLE_RECORD)),
    }
}

/// The name resolved against a record that loaded.
#[expect(
    clippy::result_large_err,
    reason = "this answers with the same Result as `resolve_registered`, for the reason recorded \
              there"
)]
fn registered(name: &str, record: &InstallationRecord) -> Result<Resolved, Diagnostic> {
    let Some(entry) = record.entry(name) else {
        return Err(unknown_vault_name(name, emptiness(record)));
    };
    vault::root_at(entry.path()).map_err(|cause| not_a_root(entry, record, &cause))
}

/// The refusal for a name no registry answered, with `because` saying why.
///
/// The help line is the substance rather than a courtesy: the mistake is easy
/// and the correction is not guessable from the error alone.
///
/// The location is the record either way. Where none exists it is still where
/// one would be read from, and naming it is how a reader learns that a registry
/// is what was consulted — the rendering is the unexpanded
/// `$XDG_CONFIG_HOME/dogtag/installation.toml`, so it emits no account name.
fn unknown_vault_name(name: &str, because: &str) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::InstallationUnknownVaultName,
        format!("no vault is registered as `{name}`{because}"),
    )
    .at(Location::whole_file(FileRef::InstallationRecord))
    .with_help(format!(
        "a bare argument is always a registry name and never a path, with no fallback between \
         them, so `{name}` can never mean the directory `{name}` — write `./{name}` for the \
         directory, or register the vault under that name"
    ))
}

/// Whether the record registers nothing at all, said as a message fragment.
///
/// An empty registry keeps the same identifier: a caller matching on
/// `installation.unknown-vault-name` handles "you have registered nothing" and
/// "you have not registered that" with the same recovery, and a second
/// identifier would only make the exhaustive set longer.
///
/// The registered names are deliberately **not** listed, and neither is how
/// many there are. A registry enumerates every vault its owner has registered,
/// by chosen name and absolute path; this SDK is agent-facing, so answering a
/// question about one name with the whole inventory would put the home
/// directory's layout into an agent's context and its provider's logs.
fn emptiness(record: &InstallationRecord) -> &'static str {
    if record.vaults().is_empty() {
        NO_VAULTS
    } else {
        ""
    }
}

/// The refusal for an entry whose path is not a vault root.
///
/// The discovery diagnostic that established this is carried as unlocated
/// evidence rather than as the answer: its identifier would tell a caller that
/// *discovery* failed, when what failed is a registry entry the caller named.
/// Distinguishing an absent path from a directory with no sentinel from a
/// broken root is still worth keeping, so the reason travels as evidence.
///
/// The diagnostic points at the path the entry writes, and cites the entry
/// itself as related evidence. Both spans come from the record's own
/// provenance, which is the only thing that knows them: a [`VaultEntry`] holds
/// the values and not the bytes they were written in.
fn not_a_root(entry: &VaultEntry, record: &InstallationRecord, cause: &Diagnostic) -> Diagnostic {
    let name = entry.name();
    let registered = Related {
        location: written(record, &format!("vault.{name}.name")),
        message: format!("`{name}` is registered here"),
    };
    Diagnostic {
        location: written(record, &format!("vault.{name}.path")),
        ..Diagnostic::kernel(
            KernelDiagnostic::InstallationVaultPathNotARoot,
            format!(
                "the registry entry `{name}` names `{}`, which is not a vault root",
                entry.path().display()
            ),
        )
        .with_related(registered)
        .with_related(Related::new(cause.message.clone()))
        .with_help(format!(
            "a registered name resolves to its registered path exactly and never falls through \
             to upward discovery — point the entry at the directory holding `{SENTINEL}`, or \
             remove the entry"
        ))
    }
}

/// Where a leaf of a registry entry is written, as the record recorded it.
fn written(record: &InstallationRecord, key: &str) -> Option<Location> {
    record
        .provenance()
        .get(key)
        .and_then(|entry| entry.location.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Severity, render_plain};
    use crate::installation::{load_installation, parse_installation};
    use crate::vault::SENTINEL_DIRECTORY;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A directory name no other call in this process will pick.
    ///
    /// Every directory a test asks for is named here rather than by the test,
    /// because none of these tests cares what a directory is called — only what
    /// is or is not inside it.
    fn next_name() -> String {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        format!("entry-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// A fresh directory under `parent`.
    fn made(parent: &Path) -> PathBuf {
        let path = parent.join(next_name());
        fs::create_dir_all(&path).expect("a directory under a tree this test owns");
        path
    }

    /// A directory tree under the system temporary directory, taken away again
    /// when the test that built it ends.
    ///
    /// Resolution verifies through the filesystem, so a registry entry can only
    /// be exercised against directories that really exist. Nothing here is the
    /// installation record's real location, and the SDK writes none of it.
    struct Tree {
        root: PathBuf,
    }

    impl Tree {
        /// An empty tree, named for the test that built it.
        ///
        /// The root is canonical, so a temporary directory that is itself a
        /// symlink cannot make a resolved root differ from the registered path.
        fn new(label: &str) -> Self {
            let elapsed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos());
            let root = std::env::temp_dir().join(format!(
                "dogtag-registry-{label}-{}-{elapsed}-{}",
                std::process::id(),
                next_name()
            ));
            fs::create_dir_all(&root).expect("the system temporary directory is writable");
            let root = fs::canonicalize(&root).expect("a directory that was just created");
            Self { root }
        }

        /// An ordinary directory: no sentinel of any kind.
        fn plain(&self) -> PathBuf {
            made(&self.root)
        }

        /// A directory holding `.dogtag/` and no contract: a broken vault root.
        fn incomplete(&self) -> PathBuf {
            let path = self.plain();
            fs::create_dir_all(path.join(SENTINEL_DIRECTORY)).expect("a sentinel directory");
            path
        }

        /// A vault root: `.dogtag/` with a contract inside it. Nothing at this
        /// layer parses the contract; resolution asks only whether it is there.
        fn vault(&self) -> PathBuf {
            let path = self.incomplete();
            fs::write(path.join(SENTINEL), "contract_version = 1\n").expect("a contract");
            path
        }

        /// A path inside the tree that was never created.
        fn absent(&self) -> PathBuf {
            self.root.join(next_name())
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// A machine whose record loaded, registering each name at its path.
    ///
    /// The record is written and parsed rather than assembled, so the spans the
    /// refusals cite are the ones a real record would carry.
    fn registry(entries: &[(&str, &Path)]) -> Installation {
        let mut source = String::from("installation_version = 1\n");
        for (name, path) in entries {
            source.push_str(&format!(
                "\n[[vault]]\nname = \"{name}\"\npath = \"{}\"\n",
                path.display()
            ));
        }
        let installation = parse_installation(&source);
        assert!(
            installation.diagnostics().is_empty(),
            "the fixture record must load cleanly"
        );
        installation
    }

    /// A machine that has never registered a vault, and so has no record.
    fn no_record(tree: &Tree) -> Installation {
        let installation = load_installation(&tree.absent());
        assert_eq!(installation.state(), &InstallationState::Absent);
        installation
    }

    /// A machine whose record exists and could not be used.
    fn unusable_record() -> Installation {
        let installation = parse_installation("installation_version = 1\nstray = true\n");
        assert_eq!(installation.state(), &InstallationState::Unusable);
        installation
    }

    /// The refusal a name nothing answers produces, whatever the state.
    fn refuse_name(name: &str, installation: &Installation) -> Diagnostic {
        let refusal =
            resolve_registered(name, installation).expect_err("nothing answers this name");
        assert_eq!(refusal.id.as_str(), "installation.unknown-vault-name");
        assert_eq!(refusal.severity, Severity::Error);
        refusal
    }

    /// The refusal a registered but unusable path produces.
    fn refuse_path(path: &Path) -> Diagnostic {
        let installation = registry(&[("work", path)]);
        let refusal =
            resolve_registered("work", &installation).expect_err("the path is not a root");
        assert_eq!(refusal.id.as_str(), "installation.vault-path-not-a-root");
        assert_eq!(refusal.severity, Severity::Error);
        refusal
    }

    fn line_of(location: Option<&Location>) -> u32 {
        location
            .expect("a location")
            .span
            .expect("a span")
            .start
            .line
    }

    #[test]
    fn a_registered_name_resolves_to_the_root_it_names() {
        let tree = Tree::new("resolves");
        let path = tree.vault();
        let root = resolve_registered("work", &registry(&[("work", &path)]))
            .expect("the entry names a vault root");
        assert_eq!(root.root().path(), path);
    }

    #[test]
    fn an_unknown_name_teaches_the_spelling_that_means_a_directory() {
        let tree = Tree::new("unknown");
        let refusal = refuse_name("home", &registry(&[("work", &tree.vault())]));
        assert!(refusal.message.contains("`home`"));
        assert!(refusal.help.expect("a help line").contains("`./home`"));
    }

    #[test]
    fn an_unknown_name_is_located_at_the_record_and_lists_no_inventory() {
        let tree = Tree::new("no-inventory");
        let registry = registry(&[("work", &tree.vault()), ("notes", &tree.vault())]);
        let refusal = refuse_name("home", &registry);
        assert_eq!(
            refusal.location,
            Some(Location::whole_file(FileRef::InstallationRecord))
        );
        assert!(!refusal.message.contains("work"));
        assert!(!refusal.message.contains("notes"));
        assert!(refusal.related.is_empty());
    }

    #[test]
    fn an_empty_registry_says_so_under_the_same_identifier() {
        let refusal = refuse_name("work", &registry(&[]));
        assert!(refusal.message.contains("registers no vaults at all"));
    }

    #[test]
    fn a_machine_with_no_record_at_all_is_answered_here_rather_than_by_a_consumer() {
        let tree = Tree::new("absent");
        let refusal = refuse_name("work", &no_record(&tree));
        assert!(
            refusal
                .message
                .contains("no installation record exists at all"),
            "{}",
            refusal.message
        );
        assert!(
            refusal
                .message
                .contains("no vault can be registered under any name"),
            "{}",
            refusal.message
        );
        assert!(refusal.help.expect("a help line").contains("`./work`"));
    }

    #[test]
    fn a_record_that_did_not_load_points_at_the_diagnostics_that_said_why() {
        let installation = unusable_record();
        let refusal = refuse_name("work", &installation);
        assert!(
            refusal.message.contains("did not load"),
            "{}",
            refusal.message
        );
        assert!(
            refusal.message.contains("the diagnostics"),
            "{}",
            refusal.message
        );
        assert!(
            !installation.diagnostics().is_empty(),
            "the diagnostics the refusal points at have to exist"
        );
    }

    #[test]
    fn no_refusal_here_emits_an_account_name() {
        let tree = Tree::new("unexpanded");
        let states = [no_record(&tree), unusable_record(), registry(&[])];
        for installation in &states {
            let rendered = render_plain(&[refuse_name("work", installation)]);
            assert!(
                rendered.contains(FileRef::INSTALLATION_RECORD_PATH),
                "{rendered}"
            );
            assert!(
                !rendered.contains(&tree.root.display().to_string()),
                "{rendered}"
            );
        }
    }

    #[test]
    fn a_registered_path_that_is_not_there_is_an_installation_diagnostic() {
        let tree = Tree::new("absent");
        let missing = tree.absent();
        let refusal = refuse_path(&missing);
        assert!(refusal.message.contains(&missing.display().to_string()));
    }

    #[test]
    fn a_registered_path_holding_no_sentinel_is_refused_rather_than_searched() {
        let tree = Tree::new("ordinary");
        let refusal = refuse_path(&tree.plain());
        assert!(refusal.related[1].message.contains(SENTINEL));
        assert!(refusal.related[1].location.is_none());
    }

    #[test]
    fn a_registered_path_inside_a_vault_never_falls_through_to_the_vault() {
        let tree = Tree::new("inside");
        let refusal = refuse_path(&made(&tree.vault()));
        assert!(refusal.help.expect("a help line").contains(SENTINEL));
    }

    #[test]
    fn a_registered_path_with_a_sentinel_directory_and_no_contract_is_refused() {
        let tree = Tree::new("broken");
        let refusal = refuse_path(&tree.incomplete());
        assert!(refusal.related[1].message.contains(SENTINEL_DIRECTORY));
    }

    #[test]
    fn the_refusal_cites_the_registry_entry_as_related_evidence() {
        let tree = Tree::new("evidence");
        let refusal = refuse_path(&tree.plain());
        let entry = &refusal.related[0];
        assert_eq!(entry.message, "`work` is registered here");
        assert_eq!(
            entry.location.as_ref().map(|at| &at.file),
            Some(&FileRef::InstallationRecord)
        );
        // The fixture writes `name` on the line above `path`, and the refusal
        // points at the path while the evidence cites the entry.
        assert_eq!(line_of(entry.location.as_ref()), 4);
        assert_eq!(line_of(refusal.location.as_ref()), 5);
        assert_eq!(
            refusal.location.as_ref().map(|at| &at.file),
            Some(&FileRef::InstallationRecord)
        );
    }
}
