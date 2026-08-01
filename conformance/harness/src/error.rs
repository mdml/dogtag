//! The harness error type: everything that can go wrong loading fixtures or
//! computing the cross product.

use std::fmt;
use std::path::PathBuf;

/// Everything that can go wrong loading or cross-producting the fixtures.
#[derive(Debug)]
pub enum HarnessError {
    /// Filesystem failure reading a scenario or profile.
    Io(PathBuf, std::io::Error),
    /// TOML that does not satisfy the schema — including any attempt to add
    /// a waiver-shaped field.
    Parse(PathBuf, toml::de::Error),
    /// Structurally valid TOML that breaks a harness rule (id mismatch,
    /// duplicate id, non-kebab-case name, empty contract, ...).
    Invalid(String),
    /// A scenario is `executable` and a profile corpus is `built`, but the
    /// scenario has no execution path. The harness refuses to report such a
    /// pair as anything, rather than silently marking it pending: graduation
    /// is all-profiles-or-nothing, so flipping a scenario's status without
    /// landing its case fails the suite instead of quietly passing.
    NotExecutable {
        /// The executable scenario.
        scenario: String,
        /// The built profile.
        profile: String,
    },
}

impl fmt::Display for HarnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HarnessError::Io(path, e) => write!(f, "io error at {}: {e}", path.display()),
            HarnessError::Parse(path, e) => write!(f, "parse error at {}: {e}", path.display()),
            HarnessError::Invalid(msg) => write!(f, "invalid fixture: {msg}"),
            HarnessError::NotExecutable { scenario, profile } => write!(
                f,
                "scenario `{scenario}` is executable and profile `{profile}` has a built corpus, \
                 but the scenario has no execution path; land its case before graduating it \
                 (graduation runs against every profile — no partial graduation)"
            ),
        }
    }
}

impl std::error::Error for HarnessError {}
