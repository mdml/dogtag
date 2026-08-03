//! Finding a vault root, and what a resolved root is.
//!
//! A vault root is a directory holding the sentinel **file**
//! [`SENTINEL`] — `.dogtag/contract.toml`, not the `.dogtag/` directory. The
//! difference is load-bearing: an empty or half-written `.dogtag/` would
//! otherwise shadow a real parent vault, or the same directory would mean two
//! different things depending on its contents.
//!
//! Three entry points, because one cannot express the decisions behind them:
//!
//! - [`discover`] walks upward from an explicit starting directory and returns
//!   the resolved root *together with the diagnostics discovery itself
//!   produced*, because the symlink-resolution note and the nested-vault
//!   warning both arise on a **successful** discovery.
//! - [`root_at`] verifies that an exact path is a vault root, without walking.
//!   Explicit means exact.
//! - [`inspect_root_trust`] reports whether a resolved root is somewhere its
//!   contract can be trusted.
//!
//! Every one of them is a **pure function of its explicit arguments**: nothing
//! here reads an environment variable, consults the current directory, or
//! holds process-global vault state. That is why [`inspect_root_trust`] takes
//! the home directory as an argument rather than reading `$HOME` — the home
//! directory is an environment fact, and resolving it is the caller's job.
//!
//! Diagnostics raised here are about *directories*, so they carry
//! `location: None` and name the directory in their message.
//! [`crate::diagnostic::FileRef`] has exactly two variants and neither of them
//! is an arbitrary machine path, which is what keeps conformance goldens
//! machine-independent.

mod discover;
mod open;
mod trust;

#[cfg(test)]
pub(crate) mod tree;

use std::borrow::Cow;
use std::io;
use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, KernelDiagnostic, VaultPath};

pub use discover::{Resolved, discover, root_at};
pub use open::{Opened, open};
pub use trust::inspect_root_trust;

/// The directory a vault root holds, beneath which its committed
/// configuration lives.
///
/// Holding this directory is not by itself what makes a directory a vault
/// root: [`SENTINEL`] is.
pub const SENTINEL_DIRECTORY: &str = ".dogtag";

/// The contract file's name inside [`SENTINEL_DIRECTORY`].
pub const CONTRACT_FILE_NAME: &str = "contract.toml";

/// The sentinel whose presence makes a directory a vault root,
/// vault-relative and with forward slashes.
///
/// This is `SENTINEL_DIRECTORY/CONTRACT_FILE_NAME`, written out so it can be
/// used in a message and in a `const` context.
pub const SENTINEL: &str = ".dogtag/contract.toml";

/// A directory verified to be a vault root.
///
/// Deliberately **opaque**. It renders as a path and is compared as one, but
/// there is no constructor from a string: callers never reconstruct a root
/// from a rendering. That costs nothing now and preserves the option of
/// carrying a held directory handle later — the difference between
/// re-resolving a write target from a string and writing through the handle
/// that was verified. Retrofitting it once `VaultRoot` is public API would be
/// a breaking change.
///
/// The path is always canonical, which is what keeps *identity is the path*
/// true: a vault reachable by two routes would otherwise give one note two
/// identities, and every link, diagnostic and eventual index key would fork
/// along with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultRoot {
    path: PathBuf,
}

impl VaultRoot {
    /// A root at an already-canonical, already-verified path.
    ///
    /// Crate-internal on purpose: the only ways to obtain a `VaultRoot` from
    /// outside are [`discover`] and [`root_at`], both of which verify the
    /// sentinel and canonicalize first.
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The canonical directory this root is.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The absolute path of this vault's contract, `<root>/.dogtag/contract.toml`.
    pub fn contract_path(&self) -> PathBuf {
        self.path.join(SENTINEL_DIRECTORY).join(CONTRACT_FILE_NAME)
    }

    /// `path` expressed relative to this root, with forward slashes.
    ///
    /// This is the spelling every in-vault diagnostic uses, so the same fault
    /// in the same corpus renders identically on every machine. Answers
    /// `None` when `path` is not inside the root, and the empty string for the
    /// root itself.
    ///
    /// It is also the **only public door** to a [`VaultPath`], and so to
    /// [`crate::diagnostic::FileRef::InVault`]: a consumer that wants to point
    /// a diagnostic at a file in a vault has to hold the root that file is in,
    /// and the stripping is what proves the path was under it.
    ///
    /// The comparison is lexical: `path` is expected to be canonical already,
    /// because a root is, and because re-resolving a path here would make this
    /// a filesystem operation rather than a rendering.
    pub fn relative(&self, path: &Path) -> Option<VaultPath> {
        VaultPath::under(&self.path, path)
    }

    /// The root as a reader sees it.
    ///
    /// A path that is not valid Unicode renders lossily rather than failing:
    /// this is a rendering, and a root that cannot be named is worse than one
    /// named approximately.
    pub fn display(&self) -> Cow<'_, str> {
        self.path.to_string_lossy()
    }
}

/// What [`discover`] resolved, and what it found on the way.
///
/// The diagnostics travel with the root rather than replacing it, because
/// `discovery.root-resolved-through-symlink` and `discovery.nested-vault` both
/// arise on a *successful* discovery: a bare success value would have nowhere
/// to carry them, and a second private walk to recover them would be a second
/// discovery implementation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Discovered {
    /// The resolved root, absent when discovery could not resolve one.
    pub root: Option<VaultRoot>,
    /// Everything discovery has to say, in the order it was found.
    pub diagnostics: Vec<Diagnostic>,
}

/// The diagnostic every filesystem failure in this module becomes.
///
/// An absent start path, a permission denial on canonicalization, a permission
/// denial probing a directory: each is a diagnostic with an identifier, never
/// a bare error and never a panic.
pub(crate) fn path_unreadable(path: &Path, error: &io::Error) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::DiscoveryPathUnreadable,
        format!("could not read `{}`: {error}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Severity;

    fn root() -> VaultRoot {
        VaultRoot::new(PathBuf::from("/data/vaults/work"))
    }

    #[test]
    fn the_sentinel_is_the_contract_inside_the_sentinel_directory() {
        assert_eq!(
            SENTINEL,
            format!("{SENTINEL_DIRECTORY}/{CONTRACT_FILE_NAME}")
        );
    }

    #[test]
    fn a_root_answers_with_the_path_it_was_built_from() {
        assert_eq!(root().path(), Path::new("/data/vaults/work"));
        assert_eq!(root().display(), "/data/vaults/work");
    }

    #[test]
    fn a_root_names_its_own_contract() {
        assert_eq!(
            root().contract_path(),
            Path::new("/data/vaults/work/.dogtag/contract.toml")
        );
    }

    #[test]
    fn a_path_inside_the_root_is_relative_to_it_with_forward_slashes() {
        let contract = root().contract_path();
        let spelled = root().relative(&contract).expect("inside the root");
        assert_eq!(spelled.as_str(), SENTINEL);
    }

    #[test]
    fn the_root_itself_is_the_empty_relative_path() {
        let spelled = root().relative(Path::new("/data/vaults/work"));
        assert_eq!(spelled.as_ref().map(VaultPath::as_str), Some(""));
    }

    #[test]
    fn a_path_outside_the_root_has_no_vault_relative_spelling() {
        assert!(
            root()
                .relative(Path::new("/data/vaults/other/note.md"))
                .is_none()
        );
        assert!(root().relative(Path::new("relative/note.md")).is_none());
    }

    #[test]
    fn roots_clone_compare_and_format() {
        let copy = root().clone();
        assert_eq!(copy, root());
        assert_ne!(copy, VaultRoot::new(PathBuf::from("/elsewhere")));
        assert!(format!("{:?}", root()).contains("work"));
    }

    #[test]
    fn a_discovery_result_clones_compares_and_formats() {
        let discovered = Discovered {
            root: Some(root()),
            diagnostics: Vec::new(),
        };
        let copy = discovered.clone();
        assert_eq!(copy, discovered);
        assert_ne!(
            copy,
            Discovered {
                root: None,
                diagnostics: Vec::new()
            }
        );
        assert!(format!("{discovered:?}").contains("VaultRoot"));
    }

    #[test]
    fn a_filesystem_failure_names_the_path_and_the_cause() {
        let error = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let diagnostic = path_unreadable(Path::new("/data/vaults"), &error);
        assert_eq!(diagnostic.id.as_str(), "discovery.path-unreadable");
        assert_eq!(diagnostic.severity, Severity::Error);
        assert!(diagnostic.message.contains("/data/vaults"));
        assert!(diagnostic.message.contains("permission denied"));
        assert!(diagnostic.location.is_none());
    }
}
