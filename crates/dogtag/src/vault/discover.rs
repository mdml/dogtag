//! The upward walk, and exact-path verification.
//!
//! The walk starts at an explicit directory, canonicalizes it, and climbs
//! **physically** to the filesystem root. There is no other boundary: not
//! `.git`, not `$HOME`, not a mount point. `.git` would hardcode one version
//! layer into the kernel's most basic operation and refuse a vault that is not
//! a repository; `$HOME` is arbitrary and simply inapplicable to a vault under
//! a mounted share. One rule with no exceptions is also the only version that
//! stays explainable — the cost is stated in this module's decision record and
//! mitigated by [`super::inspect_root_trust`], not by adding a boundary here.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, KernelDiagnostic};

use super::{
    CONTRACT_FILE_NAME, Discovered, SENTINEL, SENTINEL_DIRECTORY, VaultRoot, path_unreadable,
};

/// Resolves the vault root at or above `start`.
///
/// The nearest directory holding [`SENTINEL`] wins, and the walk continues
/// past it to the filesystem root so an ancestor vault can be reported. A
/// directory holding `.dogtag/` without the contract inside it **halts a walk
/// that has not yet found a root**: it is a broken vault root, not a non-root,
/// and delegating upward from one would silently act on the wrong corpus while
/// reporting success. Above the resolved root it delegates to nothing, so the
/// scan climbs past it and any ancestor vault is still reported.
///
/// `start` is expected to be absolute. A relative path is resolved by the
/// operating system against the process's current directory, which is exactly
/// the ambient fact this SDK declines to consult on a caller's behalf: the CLI
/// resolves the current directory and passes an absolute path in.
///
/// Every failure is carried in [`Discovered::diagnostics`] with an identifier.
/// Nothing here panics, and nothing here returns a bare error.
pub fn discover(start: &Path) -> Discovered {
    match fs::canonicalize(start) {
        Ok(canonical) => walk(&Start {
            requested: start,
            canonical,
        }),
        Err(error) => unresolved(path_unreadable(start, &error)),
    }
}

/// Verifies that `path` is itself a vault root, without walking.
///
/// This is what an explicitly named vault resolves through, so it **never**
/// searches upward: a directory inside a vault is refused rather than
/// silently resolving the vault around it. The path is still canonicalized,
/// because a root reached by two routes would otherwise give one note two
/// identities.
///
/// # Errors
///
/// `discovery.not-a-vault-root` when the path holds no sentinel,
/// `discovery.incomplete-vault-root` when it holds `.dogtag/` without the
/// contract, and `discovery.path-unreadable` when the filesystem refuses the
/// question.
#[expect(
    clippy::result_large_err,
    reason = "this SDK has exactly one error type by decision: every foreseeable failure is a \
              Diagnostic with an identifier, carrying a message, a location and evidence, so it \
              is over the lint's threshold by construction. Boxing it would push that \
              consequence onto every consumer and every binding of a public API whose shape is \
              fixed."
)]
pub fn root_at(path: &Path) -> Result<VaultRoot, Diagnostic> {
    let canonical = match fs::canonicalize(path) {
        Ok(canonical) => canonical,
        Err(error) => return Err(path_unreadable(path, &error)),
    };
    match probe(&canonical) {
        Ok(Probe::Root) => Ok(VaultRoot::new(canonical)),
        Ok(Probe::Incomplete) => Err(incomplete_root(&canonical)),
        Ok(Probe::NotARoot) => Err(not_a_vault_root(&canonical)),
        Err(error) => Err(path_unreadable(&canonical, &error)),
    }
}

/// Where a walk began: what the caller asked for, and what it canonicalized to.
struct Start<'a> {
    requested: &'a Path,
    canonical: PathBuf,
}

/// A discovery that resolved nothing, carrying the reason it did not.
fn unresolved(diagnostic: Diagnostic) -> Discovered {
    Discovered {
        root: None,
        diagnostics: vec![diagnostic],
    }
}

fn walk(start: &Start<'_>) -> Discovered {
    match search(&start.canonical) {
        Search::Found(found) => resolved(start, found),
        Search::Halted(diagnostic) => unresolved(diagnostic),
    }
}

/// A successful discovery, and the diagnostics reaching it produced.
fn resolved(start: &Start<'_>, found: Found) -> Discovered {
    let mut diagnostics = Vec::new();
    if start.requested != start.canonical {
        diagnostics.push(canonical_root(start, &found.root));
    }
    diagnostics.extend(
        found
            .ancestors
            .iter()
            .map(|ancestor| nested_vault(ancestor)),
    );
    Discovered {
        root: Some(VaultRoot::new(found.root)),
        diagnostics,
    }
}

/// A resolved root, with every ancestor that also holds the sentinel.
struct Found {
    root: PathBuf,
    ancestors: Vec<PathBuf>,
}

enum Search {
    Found(Found),
    Halted(Diagnostic),
}

/// The physical upward walk, from `from` to the filesystem root.
///
/// It does not stop when it finds the nearest root. Completing the walk is the
/// only thing that makes the nested-vault warning possible at all: if a
/// consumer performed a second private walk to find the ancestor, there would
/// be two discovery implementations, and every other consumer would silently
/// lose the warning.
fn search(from: &Path) -> Search {
    let mut walk = Walk::default();
    for directory in from.ancestors() {
        match probe(directory) {
            Ok(Probe::Root) => walk.record(directory),
            Ok(Probe::NotARoot) => {}
            // Before a root is settled, a broken root and a directory the
            // filesystem refuses to answer about both halt the walk: climbing
            // past either would resolve a vault above a directory that might
            // itself have been the right one.
            Ok(Probe::Incomplete) if !walk.has_root() => {
                return Search::Halted(incomplete_root(directory));
            }
            Err(error) if !walk.has_root() => {
                return Search::Halted(path_unreadable(directory, &error));
            }
            // Above the settled root, neither can change what discovery
            // resolves, so the scan runs on rather than ending here.
            Ok(Probe::Incomplete) | Err(_) => {}
        }
    }
    walk.finish(no_vault_found(from))
}

/// What the walk has seen so far.
#[derive(Default)]
struct Walk {
    root: Option<PathBuf>,
    ancestors: Vec<PathBuf>,
}

impl Walk {
    /// Records a directory holding the sentinel. The first one is the resolved
    /// root — nearest wins — and every later one is an ancestor vault.
    fn record(&mut self, directory: &Path) {
        let directory = directory.to_path_buf();
        if self.root.is_none() {
            self.root = Some(directory);
        } else {
            self.ancestors.push(directory);
        }
    }

    /// Whether the nearest root has been settled.
    ///
    /// This is what scopes the halt rule to the phase before it: see
    /// [`Walk::finish`].
    fn has_root(&self) -> bool {
        self.root.is_some()
    }

    /// Ends the walk at the filesystem root, reporting `absent` only when no
    /// root was resolved.
    ///
    /// The walk reaches the filesystem root **whatever it passes on the way**,
    /// and the halt rule is scoped to the phase before a root is recorded. That
    /// is where halting earns its cost: continuing past a broken root, or past
    /// a directory the filesystem will not answer about, would resolve a vault
    /// above it and act on the wrong corpus while reporting success.
    ///
    /// After the nearest root is settled there is no such delegation left to
    /// prevent, and ending the scan there would cost the one thing completing
    /// the walk exists to produce: an ancestor vault sitting above a broken or
    /// unreadable directory would never be found, so `discovery.nested-vault`
    /// would not be raised for it, and the exit code a `--strict` unattended run
    /// turns that warning into would say the vault is fine.
    ///
    /// The passed-over directory is deliberately **not** reported. Both
    /// identifiers for it carry error severity, and a vault that resolved
    /// cleanly must load with no diagnostic at any severity — so naming it here
    /// would fail a healthy vault over a stray directory that changed nothing.
    fn finish(self, absent: Diagnostic) -> Search {
        match self.root {
            Some(root) => Search::Found(Found {
                root,
                ancestors: self.ancestors,
            }),
            None => Search::Halted(absent),
        }
    }
}

/// What one directory is, as far as discovery is concerned.
enum Probe {
    /// It holds the sentinel: a vault root.
    Root,
    /// It holds `.dogtag/` but no contract inside it: a broken vault root.
    Incomplete,
    /// It holds no `.dogtag/` at all: an ordinary directory.
    NotARoot,
}

fn probe(directory: &Path) -> io::Result<Probe> {
    let sentinel_directory = directory.join(SENTINEL_DIRECTORY);
    if exists_as(
        &sentinel_directory.join(CONTRACT_FILE_NAME),
        fs::Metadata::is_file,
    )? {
        return Ok(Probe::Root);
    }
    if exists_as(&sentinel_directory, fs::Metadata::is_dir)? {
        return Ok(Probe::Incomplete);
    }
    Ok(Probe::NotARoot)
}

/// Whether `path` exists and is the kind `is_wanted` accepts.
///
/// An absent path is an answer rather than a failure, and so is a path whose
/// parent turned out not to be a directory — a `.dogtag` that is a regular
/// file makes its directory an ordinary one. Anything else the filesystem says
/// is a real failure and becomes `discovery.path-unreadable`.
fn exists_as(path: &Path, is_wanted: fn(&fs::Metadata) -> bool) -> io::Result<bool> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(is_wanted(&metadata)),
        Err(error) if is_absent(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

fn is_absent(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
    )
}

fn no_vault_found(start: &Path) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::DiscoveryNoVaultFound,
        format!(
            "no vault root at or above `{}`: no directory between it and the filesystem root \
             holds `{SENTINEL}`",
            start.display()
        ),
    )
    .with_help(format!(
        "a vault root is the directory holding `{SENTINEL}`; name one explicitly, or run from \
         inside one"
    ))
}

fn incomplete_root(directory: &Path) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::DiscoveryIncompleteVaultRoot,
        format!(
            "`{}` holds `{SENTINEL_DIRECTORY}/` but no `{SENTINEL}`: it is a broken vault root \
             rather than an ordinary directory, so the search stops here instead of delegating \
             to a vault above it",
            directory.display()
        ),
    )
    .with_help(format!(
        "write `{SENTINEL}` there, or remove `{SENTINEL_DIRECTORY}/` so the search can continue \
         above it"
    ))
}

fn not_a_vault_root(path: &Path) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::DiscoveryNotAVaultRoot,
        format!(
            "`{}` is not a vault root: it holds no `{SENTINEL}`, and an explicitly named path is \
             used exactly and never searched upward",
            path.display()
        ),
    )
    .with_help("name the vault root itself, or discover one from a directory inside it")
}

fn nested_vault(ancestor: &Path) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::DiscoveryNestedVault,
        format!(
            "the ancestor `{}` also holds `{SENTINEL}`; the nearest root wins",
            ancestor.display()
        ),
    )
    .with_help("nesting is legal: every path reported is relative to the nearest root")
}

fn canonical_root(start: &Start<'_>, root: &Path) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::DiscoveryRootResolvedThroughSymlink,
        format!(
            "the vault root resolved to `{}`, reached from the requested path `{}`: the start \
             path was canonicalized before the walk",
            root.display(),
            start.requested.display()
        ),
    )
    .with_help(
        "every path this SDK reports is relative to the canonical root, so one note never has \
         two identities",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::tree::{Tree, assert_no_vault_above};

    fn ids(discovered: &Discovered) -> Vec<&str> {
        discovered
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    fn root_of(discovered: &Discovered) -> &Path {
        discovered
            .root
            .as_ref()
            .expect("discovery resolved a root")
            .path()
    }

    fn message(discovered: &Discovered) -> &str {
        &discovered.diagnostics[0].message
    }

    fn names(text: &str, path: &Path) -> bool {
        text.contains(&path.display().to_string())
    }

    #[test]
    fn a_vault_root_discovers_itself() {
        let tree = Tree::new("discovers-itself");
        let root = tree.vault("vault");
        let discovered = discover(&root);
        assert_eq!(root_of(&discovered), root);
        assert!(discovered.diagnostics.is_empty());
    }

    #[test]
    fn discovery_climbs_from_a_nested_directory() {
        let tree = Tree::new("nested-start");
        let root = tree.vault("vault");
        let discovered = discover(&tree.dir("vault/a/b/c"));
        assert_eq!(root_of(&discovered), root);
        assert!(discovered.diagnostics.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_root_reached_through_a_symlink_resolves_canonically() {
        let tree = Tree::new("symlink");
        let root = tree.vault("real/vault");
        let link = tree.link("shortcut", &root);
        let discovered = discover(&link);
        assert_eq!(root_of(&discovered), root);
        assert_eq!(
            ids(&discovered),
            ["discovery.root-resolved-through-symlink"]
        );
        assert!(names(message(&discovered), &link));
        assert!(names(message(&discovered), &root));
    }

    #[test]
    fn an_incomplete_root_halts_the_walk_beneath_a_real_one() {
        let tree = Tree::new("incomplete-beneath");
        tree.vault("outer");
        let broken = tree.incomplete("outer/inner");
        let discovered = discover(&tree.dir("outer/inner/work"));
        assert!(discovered.root.is_none());
        assert_eq!(ids(&discovered), ["discovery.incomplete-vault-root"]);
        assert!(names(message(&discovered), &broken));
    }

    #[test]
    fn an_incomplete_root_halts_the_walk_with_no_real_one_above() {
        let tree = Tree::new("incomplete-alone");
        let broken = tree.incomplete("solo");
        let discovered = discover(&broken);
        assert!(discovered.root.is_none());
        assert_eq!(ids(&discovered), ["discovery.incomplete-vault-root"]);
    }

    #[test]
    fn nested_vaults_resolve_the_nearest_and_name_the_ancestor() {
        let tree = Tree::new("nested-vaults");
        let outer = tree.vault("outer");
        let inner = tree.vault("outer/vendored/inner");
        let discovered = discover(&tree.dir("outer/vendored/inner/notes"));
        assert_eq!(root_of(&discovered), inner);
        assert_eq!(ids(&discovered), ["discovery.nested-vault"]);
        assert!(names(message(&discovered), &outer));
    }

    #[test]
    fn every_ancestor_vault_is_named_once_nearest_first() {
        let tree = Tree::new("nested-twice");
        let outer = tree.vault("outer");
        let middle = tree.vault("outer/middle");
        let inner = tree.vault("outer/middle/inner");
        let discovered = discover(&inner);
        assert_eq!(root_of(&discovered), inner);
        assert_eq!(
            ids(&discovered),
            ["discovery.nested-vault", "discovery.nested-vault"]
        );
        assert!(names(&discovered.diagnostics[0].message, &middle));
        assert!(names(&discovered.diagnostics[1].message, &outer));
    }

    #[test]
    fn an_incomplete_ancestor_above_a_resolved_root_changes_nothing() {
        let tree = Tree::new("incomplete-above");
        tree.incomplete("stray");
        let root = tree.vault("stray/vault");
        let discovered = discover(&root);
        assert_eq!(root_of(&discovered), root);
        assert!(discovered.diagnostics.is_empty());
    }

    #[test]
    fn an_ancestor_vault_above_an_incomplete_root_is_still_named() {
        let tree = Tree::new("incomplete-between");
        let ancestor = tree.vault("outer");
        tree.incomplete("outer/stray");
        let root = tree.vault("outer/stray/inner");
        let discovered = discover(&tree.dir("outer/stray/inner/notes"));
        assert_eq!(root_of(&discovered), root);
        assert_eq!(
            ids(&discovered),
            ["discovery.nested-vault"],
            "the walk continues to the filesystem root, and the broken root it \
             climbed past is not itself reported"
        );
        assert!(names(message(&discovered), &ancestor));
    }

    #[cfg(unix)]
    #[test]
    fn an_ancestor_vault_above_an_unreadable_directory_is_still_named() {
        let tree = Tree::new("unreadable-between");
        let ancestor = tree.vault("outer");
        tree.unprobeable("outer/blocked");
        let root = tree.vault("outer/blocked/inner");
        let discovered = discover(&root);
        assert_eq!(root_of(&discovered), root);
        assert_eq!(ids(&discovered), ["discovery.nested-vault"]);
        assert!(names(message(&discovered), &ancestor));
    }

    #[test]
    fn an_absent_start_path_is_a_diagnostic_rather_than_a_panic() {
        let tree = Tree::new("absent-start");
        let discovered = discover(&tree.absent("nowhere"));
        assert!(discovered.root.is_none());
        assert_eq!(ids(&discovered), ["discovery.path-unreadable"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_directory_the_filesystem_refuses_to_probe_is_a_diagnostic() {
        let tree = Tree::new("unprobeable");
        let blocked = tree.unprobeable("blocked");
        let discovered = discover(&blocked);
        assert!(discovered.root.is_none());
        assert_eq!(ids(&discovered), ["discovery.path-unreadable"]);
        assert!(names(message(&discovered), &blocked));
    }

    #[test]
    fn finding_nothing_names_the_start_and_the_sentinel() {
        let tree = Tree::new("no-vault");
        let start = tree.dir("empty");
        assert_no_vault_above(&start);
        let discovered = discover(&start);
        assert!(discovered.root.is_none());
        assert_eq!(ids(&discovered), ["discovery.no-vault-found"]);
        assert!(names(message(&discovered), &start));
        assert!(message(&discovered).contains(SENTINEL));
    }

    #[test]
    fn a_dogtag_file_rather_than_a_directory_is_an_ordinary_directory() {
        let tree = Tree::new("dogtag-is-a-file");
        let root = tree.vault("vault");
        let start = tree.sentinel_as_a_file("vault/odd");
        assert_eq!(root_of(&discover(&start)), root);
    }

    #[test]
    fn root_at_accepts_a_vault_root() {
        let tree = Tree::new("root-at-root");
        let root = tree.vault("vault");
        let verified = root_at(&root).expect("the path is a vault root");
        assert_eq!(verified.path(), root);
    }

    #[test]
    fn root_at_refuses_an_ordinary_directory() {
        let tree = Tree::new("root-at-ordinary");
        let refusal = root_at(&tree.dir("plain")).expect_err("not a vault root");
        assert_eq!(refusal.id.as_str(), "discovery.not-a-vault-root");
        assert!(refusal.location.is_none());
    }

    #[test]
    fn root_at_never_resolves_the_vault_a_directory_sits_in() {
        let tree = Tree::new("root-at-inside");
        tree.vault("vault");
        let inside = tree.dir("vault/notes");
        let refusal = root_at(&inside).expect_err("explicit means exact");
        assert_eq!(refusal.id.as_str(), "discovery.not-a-vault-root");
        assert!(names(&refusal.message, &inside));
    }

    #[test]
    fn root_at_refuses_a_broken_vault_root() {
        let tree = Tree::new("root-at-incomplete");
        let refusal = root_at(&tree.incomplete("broken")).expect_err("a broken root");
        assert_eq!(refusal.id.as_str(), "discovery.incomplete-vault-root");
    }

    #[test]
    fn root_at_reports_an_absent_path() {
        let tree = Tree::new("root-at-absent");
        let refusal = root_at(&tree.absent("nowhere")).expect_err("nothing there");
        assert_eq!(refusal.id.as_str(), "discovery.path-unreadable");
    }

    #[cfg(unix)]
    #[test]
    fn root_at_reports_a_directory_it_cannot_probe() {
        let tree = Tree::new("root-at-unprobeable");
        let refusal = root_at(&tree.unprobeable("blocked")).expect_err("unprobeable");
        assert_eq!(refusal.id.as_str(), "discovery.path-unreadable");
    }

    #[cfg(unix)]
    #[test]
    fn root_at_canonicalizes_the_path_it_verifies() {
        let tree = Tree::new("root-at-symlink");
        let root = tree.vault("real/vault");
        let verified = root_at(&tree.link("shortcut", &root)).expect("a vault root");
        assert_eq!(verified.path(), root);
    }
}
