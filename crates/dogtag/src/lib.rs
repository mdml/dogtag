//! # dogtag — a PKM SDK for AI agents
//!
//! `dogtag` is the semantic kernel for typed Markdown vaults. Parsing,
//! configuration, identity, relationships, query, validation, mutation,
//! provenance, history, and migration live in this one public core; every
//! official surface — the `dogtag` CLI, MCP server, TUI, CI integrations,
//! agent loops — is a consumer of this API and never independently
//! reinterprets vault semantics.
//!
//! ## The kernel model
//!
//! The core interprets a deliberately small model, and everything else is
//! declared:
//!
//! - A **note** is one file: frontmatter plus body.
//! - A **type** is the single discriminator every note carries — the dispatch
//!   key for structure and validation.
//! - A **property** is a value drawn from a small closed lattice of value
//!   kinds, each required or optional.
//! - A **relationship** is directed, typed, and required to resolve.
//!
//! Everything beyond those four concepts — lifecycle states, orthogonal
//! flags, who may write which type — reaches the core as a *declaration* in
//! the vault's committed contract. The core enforces the declared shape
//! without knowing the vocabulary: it can answer "what is current here"
//! because a corpus named its life axis, not because the kernel knows the
//! word `archived`.
//!
//! ## Capability-bound behavior
//!
//! Configuration binds by capability, never by name. A type declares zero or
//! more capabilities and the core reasons over the declarations:
//!
//! - **Identity-bearing** (any number) marks the types a corpus is *about* —
//!   the targets of entity resolution and structured relationships.
//! - **Catch-all** (exactly one) marks the bottom type that accepts anything,
//!   so capture never blocks on classification.
//! - **Closed-write** (any number) marks types no caller may modify, making
//!   immutable source material a policy on a class rather than a privileged
//!   location.
//!
//! Cardinality is part of the contract and is validated when it loads.
//!
//! ## Current surface
//!
//! This is release `0.1.0-beta.0`, the empty vertical slice: the public API
//! is exactly [`VERSION`] and [`version`]. The model described above is the
//! forthcoming surface, landing milestone by milestone behind the
//! conformance suite; nothing here speculates ahead of it.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// The SDK version, taken verbatim from the crate's package metadata.
///
/// This is the single source of the version string: every official surface
/// (including the `dogtag` CLI) reports this value rather than carrying a
/// version of its own.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Returns the SDK version string.
///
/// Equivalent to reading [`VERSION`]; provided as a function so consumers
/// bind to a call in the public API rather than to a constant's address.
pub fn version() -> &'static str {
    VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_package_metadata() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
        assert_eq!(version(), VERSION);
    }

    #[test]
    fn version_is_a_semver_prerelease() {
        // String-shape check only — deliberately no semver dependency.
        // `<major>.<minor>.<patch>-<prerelease>` with numeric core components
        // and a non-empty prerelease part (the beta ships as a prerelease).
        let v = version();
        let (core, prerelease) = v
            .split_once('-')
            .expect("version must carry a prerelease part at this milestone");
        let components: Vec<&str> = core.split('.').collect();
        assert_eq!(components.len(), 3, "core must be major.minor.patch: {v}");
        for component in components {
            assert!(
                component.parse::<u64>().is_ok(),
                "non-numeric core component in {v}"
            );
        }
        assert!(!prerelease.is_empty(), "empty prerelease part in {v}");
    }
}
