//! Synthetic directory trees, for this module's tests and nothing else.
//!
//! Discovery takes an explicit path, which is what makes every branch of it —
//! nested, incomplete, absent, symlinked, competing — reachable against a tree
//! the test builds rather than against whatever directory a test happens to
//! run in.
//!
//! The system temporary directory is the one ambient fact these fixtures use,
//! and it is read here rather than anywhere in the module under test: nothing
//! in [`super`] consults the environment. Because the walk has **no boundary**,
//! "no vault above here" is otherwise a property of the machine, so
//! [`Tree::new`] proves it for every tree it builds and fails loudly when it
//! cannot.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{CONTRACT_FILE_NAME, SENTINEL, SENTINEL_DIRECTORY};

/// A contract body. Nothing at this layer parses it; discovery asks only
/// whether the file is there.
const CONTRACT: &str = "contract_version = 1\n";

/// A name no other tree in this process, or a neighbouring one, will pick.
fn stamp() -> String {
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos());
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{}-{elapsed}-{count}", std::process::id())
}

/// Fails unless nothing at or above `directory` holds a vault sentinel.
///
/// The walk climbs to the filesystem root with no other boundary, so an
/// ancestor holding `.dogtag/contract.toml` — or a bare `.dogtag/`, which
/// halts the walk — would silently change what these tests observe. That must
/// fail loudly rather than pass, or skip, on one developer's directory layout.
pub(crate) fn assert_no_vault_above(directory: &Path) {
    for ancestor in directory.ancestors() {
        assert!(
            !ancestor.join(SENTINEL_DIRECTORY).exists(),
            "`{}` holds `{SENTINEL_DIRECTORY}/`, so discovery from `{}` cannot be asserted \
             against: the upward walk has no boundary, and this ancestor would resolve or halt \
             it. Point the system temporary directory somewhere with no vault above it.",
            ancestor.display(),
            directory.display()
        );
    }
}

/// Sets `path`'s permission bits.
#[cfg(unix)]
pub(crate) fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .expect("a path this test owns accepts a mode");
}

/// A directory tree under the system temporary directory, removed on drop.
pub(crate) struct Tree {
    root: PathBuf,
}

impl Tree {
    /// An empty tree, named for the test that built it.
    pub(crate) fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!("dogtag-vault-{label}-{}", stamp()));
        fs::create_dir_all(&root).expect("the system temporary directory is writable");
        let root = fs::canonicalize(&root).expect("a directory that was just created");
        assert_no_vault_above(&root);
        Self { root }
    }

    /// The tree's own directory, canonical — so a temporary directory that is
    /// itself a symlink cannot be mistaken for one a test built.
    pub(crate) fn path(&self) -> &Path {
        &self.root
    }

    /// Creates `relative` as a directory.
    pub(crate) fn dir(&self, relative: &str) -> PathBuf {
        let path = self.root.join(relative);
        fs::create_dir_all(&path).expect("a directory under a tree this test owns");
        path
    }

    /// Creates `relative` as a vault root: `.dogtag/` and the contract inside it.
    pub(crate) fn vault(&self, relative: &str) -> PathBuf {
        let root = self.incomplete(relative);
        fs::write(root.join(SENTINEL), CONTRACT).expect("a contract under a tree this test owns");
        root
    }

    /// Creates `relative` as a broken vault root: `.dogtag/` and no contract.
    pub(crate) fn incomplete(&self, relative: &str) -> PathBuf {
        let root = self.dir(relative);
        fs::create_dir_all(root.join(SENTINEL_DIRECTORY)).expect("a sentinel directory");
        root
    }

    /// Creates `relative` as a directory whose `.dogtag` is a regular file.
    pub(crate) fn sentinel_as_a_file(&self, relative: &str) -> PathBuf {
        let directory = self.dir(relative);
        fs::write(directory.join(SENTINEL_DIRECTORY), "not a directory").expect("a stray file");
        directory
    }

    /// Creates a symlink at `relative` pointing at `target`.
    #[cfg(unix)]
    pub(crate) fn link(&self, relative: &str, target: &Path) -> PathBuf {
        let link = self.root.join(relative);
        std::os::unix::fs::symlink(target, &link).expect("a symlink under a tree this test owns");
        link
    }

    /// Creates `relative` as a directory the filesystem refuses to answer
    /// about: its `.dogtag` is a symlink to itself, so resolving anything
    /// beneath it is a loop rather than an absence.
    ///
    /// A permission denial would do as well, and is the case this stands in
    /// for, but it would answer differently for a privileged account and turn
    /// a fixed outcome into a property of whoever runs the suite.
    #[cfg(unix)]
    pub(crate) fn unprobeable(&self, relative: &str) -> PathBuf {
        let directory = self.dir(relative);
        let sentinel = directory.join(SENTINEL_DIRECTORY);
        std::os::unix::fs::symlink(SENTINEL_DIRECTORY, &sentinel).expect("a self-referential link");
        directory
    }

    /// A path inside the tree that was never created.
    pub(crate) fn absent(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }
}

impl Drop for Tree {
    fn drop(&mut self) {
        // A tree a test narrowed the mode of is still removable: only the
        // directory's own bits are ever changed, never its parent's.
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tree_is_canonical_and_disappears_when_it_is_dropped() {
        let path = {
            let tree = Tree::new("self-test");
            assert_eq!(
                tree.path(),
                fs::canonicalize(tree.path()).expect("canonical")
            );
            tree.path().to_path_buf()
        };
        assert!(!path.exists());
    }

    #[test]
    #[should_panic(expected = "the upward walk has no boundary")]
    fn a_sentinel_above_a_fixture_fails_loudly_rather_than_passing() {
        let tree = Tree::new("planted-above");
        tree.vault("planted");
        assert_no_vault_above(&tree.dir("planted/inside"));
    }

    #[test]
    fn every_shape_a_test_can_ask_for_is_built_where_it_was_asked_for() {
        let tree = Tree::new("shapes");
        assert!(tree.vault("a").join(SENTINEL).is_file());
        assert!(tree.incomplete("b").join(SENTINEL_DIRECTORY).is_dir());
        assert!(tree.dir("c").is_dir());
        assert!(
            tree.sentinel_as_a_file("d")
                .join(SENTINEL_DIRECTORY)
                .is_file()
        );
        assert!(!tree.absent("e").exists());
    }

    #[test]
    fn a_vault_holds_the_contract_body_the_fixture_writes() {
        let tree = Tree::new("contract-body");
        let contract = tree
            .vault("vault")
            .join(SENTINEL_DIRECTORY)
            .join(CONTRACT_FILE_NAME);
        assert_eq!(fs::read_to_string(contract).expect("readable"), CONTRACT);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_points_where_it_was_aimed() {
        let tree = Tree::new("links");
        let target = tree.dir("target");
        let link = tree.link("shortcut", &target);
        assert_eq!(fs::canonicalize(link).expect("resolvable"), target);
    }

    #[cfg(unix)]
    #[test]
    fn an_unprobeable_directory_refuses_to_answer_about_its_sentinel() {
        let tree = Tree::new("unprobeable");
        let blocked = tree.unprobeable("blocked");
        assert!(fs::metadata(blocked.join(SENTINEL)).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_mode_set_on_a_directory_is_the_mode_it_reports() {
        use std::os::unix::fs::PermissionsExt;

        let tree = Tree::new("modes");
        let directory = tree.dir("open");
        set_mode(&directory, 0o777);
        let mode = fs::metadata(&directory)
            .expect("readable")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o777);
    }
}
