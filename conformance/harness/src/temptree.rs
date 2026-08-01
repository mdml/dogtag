//! Throwaway directory trees, and the recursive copy the executed scenarios
//! run against.
//!
//! Standard library only, per the dependency policy — no `tempfile` crate. The
//! pattern started life in the harness's own tests and moved here when the
//! executed scenarios needed it too, so both use one implementation rather
//! than two that can drift.
//!
//! Every executed pair runs against a **copy** of a profile's corpus rather
//! than against the checkout. Two reasons, both about a result that would
//! otherwise be a property of the machine: the run writes into the tree
//! (installation records, nested directories, symbolic links, transformed
//! contracts) and must never write into the repository, and the copy's
//! permissions are normalized so a developer's umask cannot decide a
//! conformance result.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// A throwaway directory under the system temp directory, removed on drop.
///
/// The path is canonical, so a temp directory that is itself reached through a
/// symbolic link (`/tmp` on some systems) cannot make a scenario about
/// symbolic links pass or fail for the wrong reason.
#[derive(Debug)]
pub struct TempTree(PathBuf);

impl TempTree {
    /// A fresh tree, named after `label` and unique within the process.
    ///
    /// # Panics
    ///
    /// If the tree cannot be created or canonicalized. A harness that cannot
    /// obtain scratch space has nothing to report.
    pub fn new(label: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "dogtag-conformance-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("temp tree created");
        TempTree(fs::canonicalize(&path).expect("temp tree canonicalized"))
    }

    /// The tree's canonical root.
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Copies the directory `from` to `to`, recursively, normalizing permissions.
///
/// # Errors
///
/// Any filesystem failure, unchanged, so the caller can name the pair it was
/// setting up.
pub fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    set_mode(to, DIRECTORY_MODE)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        copy_entry(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}

/// One entry of a recursive copy: a directory recurses, anything else is a
/// file copy.
fn copy_entry(from: &Path, to: &Path) -> io::Result<()> {
    if from.is_dir() {
        return copy_tree(from, to);
    }
    fs::copy(from, to)?;
    set_mode(to, FILE_MODE)
}

/// The mode every copied directory gets: owner-writable, group- and
/// world-readable, and writable by nobody else.
const DIRECTORY_MODE: u32 = 0o755;

/// The mode every copied file gets.
const FILE_MODE: u32 = 0o644;

/// Fixes a copied entry's permissions, so the checkout's modes — which are
/// whatever the developer's umask made them — reach no conformance result.
#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
}

/// Permissions are not a mode elsewhere; the copy is still a copy.
#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: u32) -> io::Result<()> {
    Ok(())
}
