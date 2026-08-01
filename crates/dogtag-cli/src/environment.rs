//! The ambient facts, resolved once, at the edge.
//!
//! Every SDK entry point this crate calls is a pure function of its explicit
//! arguments: none reads an environment variable, consults the current
//! directory, or holds process-global vault state. Resolving those facts is
//! therefore the consumer's whole job on this side of the boundary, and it is
//! done here, in one place, so that no command reaches for the process
//! environment on its own.
//!
//! Two of these are attacker-reachable rather than merely convenient.
//! `XDG_CONFIG_HOME` redirects which registry a run resolves names against, so
//! whoever controls an invocation's environment redirects it without needing a
//! filesystem write; `DOGTAG_VAULT` selects the vault outright. Neither is
//! trusted further than the diagnostics that report what was resolved, which
//! is why every command prints the root it opened and how it was chosen.

use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use dogtag::installation::{Installation, RECORD_RELATIVE_PATH, load_installation};

/// The variable naming the vault to select, when no `--vault` is given.
const VAULT: &str = "DOGTAG_VAULT";

/// The variable naming the directory the installation record sits under.
const CONFIG_HOME: &str = "XDG_CONFIG_HOME";

/// The variable naming the user's home directory.
///
/// This is read directly rather than through a home-directory helper that
/// falls back to the password database: a fallback would make a run that
/// unsets `HOME` read a real record, which is neither hermetic under test nor
/// what unsetting it means.
const HOME: &str = "HOME";

/// The variable whose presence suppresses colour.
const NO_COLOR: &str = "NO_COLOR";

/// The default configuration directory, relative to the home directory, on the
/// supported platforms.
const CONFIG_DEFAULT: &str = ".config";

/// What the process supplies.
pub struct Environment {
    current_dir: PathBuf,
    home: Option<PathBuf>,
    record: Option<PathBuf>,
    vault: Option<String>,
    colour: bool,
}

impl Environment {
    /// Resolves every ambient fact this crate is allowed to consult.
    pub fn from_process() -> Self {
        let home = variable(HOME).map(PathBuf::from);
        Self {
            // A current directory that cannot be read leaves the empty path,
            // which names no directory: discovery then refuses with
            // `discovery.path-unreadable` rather than searching from somewhere
            // this run did not choose.
            current_dir: env::current_dir().unwrap_or_default(),
            record: record_path(config_home(home.as_deref())),
            home,
            // Read lossily rather than discarded when it is not valid UTF-8:
            // a variable that selects a vault must never be *silently*
            // ignored, because ignoring it falls back to discovery and
            // resolves a different vault than the one that was asked for. A
            // mangled argument fails loudly instead, naming what it looked
            // for. (The report carries the argument as a `String`, so there
            // is nothing faithful to carry either way.)
            // Read *without* the empty filter the other variables get. An
            // empty selector is a selector: treating it as unset falls through
            // to discovery and resolves whatever vault the working directory
            // sits in, which is the silent wrong-vault resolution the comment
            // above forbids. An unfilled slot in a CI or cron template is
            // exactly how it arrives, and `doctor --strict` would report that
            // wrong vault healthy and exit 0.
            vault: env::var_os(VAULT).map(|value| value.to_string_lossy().into_owned()),
            colour: variable(NO_COLOR).is_none(),
        }
    }

    /// Where upward discovery starts.
    pub fn current_dir(&self) -> &Path {
        &self.current_dir
    }

    /// The home directory root trust is judged against, when there is one.
    ///
    /// `None` says nothing about a root rather than claiming it is outside a
    /// home directory, which is what the SDK's trust check does with it.
    pub fn home(&self) -> Option<&Path> {
        self.home.as_deref()
    }

    /// The vault named by the environment, when one is.
    pub fn vault(&self) -> Option<&str> {
        self.vault.as_deref()
    }

    /// Whether colour is permitted at all, before any stream is consulted.
    pub fn colour(&self) -> bool {
        self.colour
    }

    /// Reads the installation record this machine's environment names.
    ///
    /// The record is opened for reading and never for writing. A machine with
    /// no home directory has nowhere to keep one, and the empty path names no
    /// file, so the record reads as *absent* — which is exactly what that
    /// machine's state is, and absence is a state rather than a fault.
    pub fn installation(&self) -> Installation {
        load_installation(self.record.as_deref().unwrap_or(Path::new("")))
    }
}

/// A variable's value, treating an empty one as unset.
///
/// Every variable read *through this function* names a location or suppresses
/// colour, and an empty string is neither a location nor a presence worth
/// honouring — `NO_COLOR=` means what `NO_COLOR` unset means, by the
/// convention the variable is named for. `DOGTAG_VAULT` deliberately does not
/// come through here: see [`Environment::from_process`].
fn variable(name: &str) -> Option<OsString> {
    env::var_os(name).filter(|value| !value.is_empty())
}

/// The directory the installation record sits under.
///
/// `$XDG_CONFIG_HOME` when set, and `~/.config` otherwise, on the supported
/// platforms. Both are the consumer's to resolve: the SDK takes the record's
/// path as an argument precisely so that this decision stays out of the kernel.
fn config_home(home: Option<&Path>) -> Option<PathBuf> {
    variable(CONFIG_HOME)
        .map(PathBuf::from)
        .or_else(|| default_config_home(home))
}

/// Where the configuration directory sits when no variable names it.
fn default_config_home(home: Option<&Path>) -> Option<PathBuf> {
    home.map(|home| home.join(CONFIG_DEFAULT))
}

/// The record's path, by expanding the one variable the SDK's rendering names.
///
/// The SDK renders the record as `$XDG_CONFIG_HOME/dogtag/installation.toml`
/// and never expands it — an emitted path must never carry an account name.
/// Expanding it is this crate's job, and everything below the variable comes
/// from [`RECORD_RELATIVE_PATH`] rather than from that rendering: the rendering
/// is text a reader sees, and taking a path back out of it would break the day
/// it is reworded.
fn record_path(config_home: Option<PathBuf>) -> Option<PathBuf> {
    config_home.map(|directory| directory.join(RECORD_RELATIVE_PATH))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_record_sits_where_the_sdk_says_it_does() {
        assert_eq!(
            Path::new(RECORD_RELATIVE_PATH),
            Path::new("dogtag/installation.toml"),
            "the SDK's constant is the only declaration of this path"
        );
    }

    /// The default location, resolved without consulting the process
    /// environment: a developer who has `XDG_CONFIG_HOME` set must not change
    /// what this test measures.
    #[test]
    fn a_home_directory_with_no_variable_keeps_the_record_under_dot_config() {
        let resolved = record_path(default_config_home(Some(Path::new("/home/someone"))));
        assert_eq!(
            resolved,
            Some(PathBuf::from(
                "/home/someone/.config/dogtag/installation.toml"
            ))
        );
    }

    #[test]
    fn a_machine_with_no_home_directory_has_no_record_path() {
        assert_eq!(default_config_home(None), None);
        assert_eq!(record_path(None), None);
    }
}
