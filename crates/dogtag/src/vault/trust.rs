//! Whether a resolved root is somewhere its contract can be trusted.
//!
//! A discovered contract is trusted exactly as far as the directory tree it
//! was found in, and [`super::discover`] has no boundary, so that tree is not
//! always the user's.
//!
//! This is live now rather than deferred to the milestone that writes,
//! because `contract explain` renders the contract as **instructions an agent
//! follows** and every string value in a valid contract is free text. A
//! `.dogtag/contract.toml` planted in any ancestor of where dogtag runs — in
//! an extracted archive whose top level carries one, in a world-writable
//! directory on a shared host, on a network mount — becomes attacker-authored
//! text presented to an agent as the vault's rules. Fatal unknown keys bound
//! the structure such a contract can inject; they bound nothing about its
//! content.
//!
//! Neither check changes what discovery resolves, and neither is a boundary:
//! every candidate boundary was considered and rejected. They reduce a planted
//! contract to something *visible*, not to something impossible, which is the
//! whole claim being made.

use std::fs;
use std::path::Path;

use crate::diagnostic::{Diagnostic, KernelDiagnostic};

use super::{VaultRoot, path_unreadable};

/// Warns about a resolved root whose location undermines its contract.
///
/// Two warnings, each costing one `stat`: the root is outside `home`, or the
/// root directory grants write to the group or to others.
///
/// `home` is an argument because the home directory is an environment fact and
/// this SDK never reads `$HOME` — the caller resolves it and passes it in.
/// When it is `None` the outside-home check does not fire: a caller that could
/// not resolve a home directory has said nothing about the root.
///
/// The permission check is a no-op on platforms without a Unix mode. macOS and
/// Linux are the supported platforms.
pub fn inspect_root_trust(root: &VaultRoot, home: Option<&Path>) -> Vec<Diagnostic> {
    let mut diagnostics: Vec<Diagnostic> = home
        .and_then(|home| outside_home(root.path(), home))
        .into_iter()
        .collect();
    diagnostics.extend(permissions(root.path()));
    diagnostics
}

fn outside_home(root: &Path, home: &Path) -> Option<Diagnostic> {
    if under(root, home) {
        return None;
    }
    Some(
        Diagnostic::kernel(
            KernelDiagnostic::DiscoveryRootOutsideHome,
            format!(
                "the vault root `{}` is outside the home directory `{}`, so its contract was \
                 not necessarily authored by whoever is running this command",
                root.display(),
                home.display()
            ),
        )
        .with_help(
            "read the contract before acting on it: `contract explain` renders it as \
             instructions, and every string in it is free text",
        ),
    )
}

/// Whether `root` is inside `home`.
///
/// A root is always canonical; a supplied home directory need not be, and a
/// home reached through a symlink is ordinary. So the literal comparison is
/// tried first and a canonical one second. Canonicalizing the home is a
/// convenience and never a failure: a home directory that does not resolve is
/// simply not a prefix of anything.
fn under(root: &Path, home: &Path) -> bool {
    root.starts_with(home) || fs::canonicalize(home).is_ok_and(|home| root.starts_with(home))
}

#[cfg(unix)]
fn permissions(root: &Path) -> Vec<Diagnostic> {
    use std::os::unix::fs::PermissionsExt;

    match fs::metadata(root) {
        Ok(metadata) => group_or_world_writable(root, metadata.permissions().mode())
            .into_iter()
            .collect(),
        Err(error) => vec![path_unreadable(root, &error)],
    }
}

#[cfg(not(unix))]
fn permissions(_root: &Path) -> Vec<Diagnostic> {
    Vec::new()
}

/// Warns when `mode` grants write beyond the owner.
///
/// The sticky bit is deliberately not an exemption. It stops one account
/// replacing another's existing file, which is not the question here: a vault
/// root with no `.dogtag/` yet, or a `.dogtag/` an attacker owns, still admits
/// a planted contract.
#[cfg(unix)]
fn group_or_world_writable(root: &Path, mode: u32) -> Option<Diagnostic> {
    /// Group-write and other-write.
    const BEYOND_OWNER: u32 = 0o022;
    /// The permission bits worth showing a reader.
    const PERMISSIONS: u32 = 0o7777;

    if mode & BEYOND_OWNER == 0 {
        return None;
    }
    Some(
        Diagnostic::kernel(
            KernelDiagnostic::DiscoveryRootGroupOrWorldWritable,
            format!(
                "the vault root `{}` is writable beyond its owner (mode {:04o}), so another \
                 account on this host can plant or rewrite its contract",
                root.display(),
                mode & PERMISSIONS
            ),
        )
        .with_help("restrict the vault root to its owner, or move it somewhere only you can write"),
    )
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::vault::tree::{Tree, set_mode};
    use std::path::PathBuf;

    /// A root owned by whoever built the tree, and by nobody else.
    fn owned_root(tree: &Tree, relative: &str) -> VaultRoot {
        let path = tree.vault(relative);
        set_mode(&path, 0o755);
        VaultRoot::new(path)
    }

    fn ids(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    #[test]
    fn a_root_inside_the_home_directory_draws_no_warning() {
        let tree = Tree::new("trust-inside-home");
        let root = owned_root(&tree, "home/vault");
        let home = tree.dir("home");
        assert!(inspect_root_trust(&root, Some(&home)).is_empty());
    }

    #[test]
    fn a_root_outside_the_home_directory_is_a_warning_naming_both() {
        let tree = Tree::new("trust-outside-home");
        let root = owned_root(&tree, "elsewhere/vault");
        let home = tree.dir("home");
        let diagnostics = inspect_root_trust(&root, Some(&home));
        assert_eq!(ids(&diagnostics), ["discovery.root-outside-home"]);
        assert!(diagnostics[0].message.contains(&root.display().to_string()));
        assert!(diagnostics[0].message.contains(&home.display().to_string()));
        assert!(diagnostics[0].location.is_none());
    }

    #[test]
    fn a_home_directory_reached_through_a_symlink_still_contains_its_vaults() {
        let tree = Tree::new("trust-linked-home");
        let root = owned_root(&tree, "real/home/vault");
        let home = tree.link("home", &tree.dir("real/home"));
        assert!(inspect_root_trust(&root, Some(&home)).is_empty());
    }

    #[test]
    fn a_home_directory_that_does_not_resolve_cannot_contain_anything() {
        let tree = Tree::new("trust-absent-home");
        let root = owned_root(&tree, "vault");
        let home = tree.absent("no-such-home");
        assert_eq!(
            ids(&inspect_root_trust(&root, Some(&home))),
            ["discovery.root-outside-home"]
        );
    }

    #[test]
    fn no_home_directory_means_the_outside_home_check_does_not_fire() {
        let tree = Tree::new("trust-no-home");
        let root = owned_root(&tree, "elsewhere/vault");
        assert!(inspect_root_trust(&root, None).is_empty());
    }

    #[test]
    fn a_group_or_world_writable_root_is_a_warning_naming_the_mode() {
        let tree = Tree::new("trust-writable");
        let root = owned_root(&tree, "vault");
        set_mode(root.path(), 0o777);
        let diagnostics = inspect_root_trust(&root, None);
        assert_eq!(
            ids(&diagnostics),
            ["discovery.root-group-or-world-writable"]
        );
        assert!(diagnostics[0].message.contains("0777"));
    }

    #[test]
    fn a_group_writable_root_is_enough_to_warn() {
        let tree = Tree::new("trust-group-writable");
        let root = owned_root(&tree, "vault");
        set_mode(root.path(), 0o775);
        assert_eq!(
            ids(&inspect_root_trust(&root, None)),
            ["discovery.root-group-or-world-writable"]
        );
    }

    #[test]
    fn both_warnings_arrive_together_when_both_apply() {
        let tree = Tree::new("trust-both");
        let root = owned_root(&tree, "elsewhere/vault");
        set_mode(root.path(), 0o777);
        assert_eq!(
            ids(&inspect_root_trust(&root, Some(&tree.dir("home")))),
            [
                "discovery.root-outside-home",
                "discovery.root-group-or-world-writable"
            ]
        );
    }

    #[test]
    fn a_root_the_filesystem_no_longer_answers_about_is_a_diagnostic() {
        let root = VaultRoot::new(PathBuf::from("/dogtag-no-such-vault-root"));
        assert_eq!(
            ids(&inspect_root_trust(&root, None)),
            ["discovery.path-unreadable"]
        );
    }
}
