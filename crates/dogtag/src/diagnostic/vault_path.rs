//! How a path inside a vault is spelled, and the two doors that produce one.
//!
//! An in-vault path reaches a diagnostic, the plain-text rendering and the
//! JSON as text, so the spelling *is* the guarantee: relative to the vault
//! root and written with forward slashes, so the same fault in the same corpus
//! renders identically on every machine and no machine path reaches structured
//! output.
//!
//! The payload is private and there is deliberately **no constructor from an
//! arbitrary string**, not even a shape-validating one. There are two doors
//! and both are crate-internal: `under`, which strips a real root's prefix off
//! a path and is what [`crate::vault::VaultRoot::relative`] is, and `kernel`,
//! which takes a `&'static str` so that a literal or a `const` in this crate
//! reaches it and a value read from a file or an argument does not.
//!
//! What holding one proves is worth stating exactly, because it is less than
//! it looks. It does **not** prove the file exists — nothing here touches the
//! filesystem. It does **not** prove the path belongs to the vault a given
//! report is about: a path stripped against root A can still be attached to a
//! diagnostic about root B, because a `VaultPath` does not remember which root
//! it came from. What it proves is the one thing rendering depends on: no
//! caller-supplied runtime string, and in particular no absolute path, reaches
//! output through this type.

use core::fmt;
use std::path::Path;

/// A path inside a vault: relative to the root, with forward slashes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VaultPath(String);

impl VaultPath {
    /// `path` expressed relative to `root`, or `None` when it is not inside it.
    ///
    /// The comparison is lexical: both paths are expected to be canonical
    /// already, because a vault root is, and re-resolving here would make a
    /// rendering into a filesystem operation.
    pub(crate) fn under(root: &Path, path: &Path) -> Option<Self> {
        let relative = path.strip_prefix(root).ok()?;
        let spelling = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        Some(Self(spelling))
    }

    /// One of the kernel's own committed paths.
    ///
    /// The `&'static str` is the restriction: a literal or a `const` reaches
    /// this, and a value read from a file or an argument does not.
    pub(crate) fn kernel(path: &'static str) -> Self {
        Self(path.to_owned())
    }

    /// The path as it is written in output.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VaultPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::CONTRACT_PATH;
    use core::cmp::Ordering;
    use std::path::PathBuf;

    fn root() -> PathBuf {
        PathBuf::from("/data/vaults/work")
    }

    #[test]
    fn a_path_inside_the_root_is_relative_to_it_with_forward_slashes() {
        let contract = root().join(".dogtag").join("contract.toml");
        let spelled = VaultPath::under(&root(), &contract).expect("inside the root");
        assert_eq!(spelled.as_str(), ".dogtag/contract.toml");
        assert_eq!(spelled.to_string(), ".dogtag/contract.toml");
    }

    #[test]
    fn the_root_itself_is_the_empty_spelling() {
        let spelled = VaultPath::under(&root(), &root()).expect("the root is inside itself");
        assert_eq!(spelled.as_str(), "");
    }

    #[test]
    fn a_path_outside_the_root_has_no_vault_relative_spelling() {
        assert!(VaultPath::under(&root(), Path::new("/data/vaults/other/note.md")).is_none());
        assert!(VaultPath::under(&root(), Path::new("relative/note.md")).is_none());
    }

    #[test]
    fn a_kernel_path_is_already_the_spelling_it_renders_as() {
        let spelled = VaultPath::kernel(CONTRACT_PATH);
        assert_eq!(spelled.as_str(), CONTRACT_PATH);
        assert!(!CONTRACT_PATH.contains('\\'), "forward slashes only");
        assert!(!Path::new(CONTRACT_PATH).is_absolute(), "vault-relative");
    }

    #[test]
    fn spellings_sort_by_path_and_clone_and_format() {
        let a = VaultPath::kernel(".dogtag/a.toml");
        let b = VaultPath::kernel(".dogtag/b.toml");
        assert_eq!(a.cmp(&b), Ordering::Less);
        assert_eq!(a.clone(), a);
        assert!(format!("{a:?}").contains("a.toml"));
    }
}
