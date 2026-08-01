//! The local installation record, `$XDG_CONFIG_HOME/dogtag/installation.toml`.
//!
//! The record is never committed, **never required to exist**, and — at this
//! milestone — **read and never written**. Nothing in this module opens the
//! file for writing, creates it, or repairs it.
//!
//! Absence is therefore a *state* rather than a fault: a machine that has never
//! registered a vault has no record, and saying so is a report's job, not a
//! diagnostic's. A file that exists and cannot be read is a different thing
//! entirely, and [`load_installation`] tells the two apart by the I/O error's
//! [`std::io::ErrorKind`].
//!
//! The record owns exactly two things — the vault registry and actor identity.
//! The committed vault contract owns everything else, and that partition is
//! **structural rather than policed**: unknown keys are fatal here, so a record
//! carrying `[[type]]`, `[dialect]`, or `[lifecycle]` is refused by
//! `installation.unknown-key` like any other misspelling. There is deliberately
//! no separate "you tried to supply a contract-owned setting" identifier,
//! because there is no separate mechanism.
//!
//! Every location this module reports names the unexpanded
//! `$XDG_CONFIG_HOME/dogtag/installation.toml` rather than the path that was
//! actually read, so no diagnostic emits an account name.

mod parse;
mod registry;

pub use registry::resolve_registered;

use core::fmt;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

use crate::diagnostic::{Diagnostic, FileRef, KernelDiagnostic, Location};
use crate::provenance::Provenance;

/// Where the record sits beneath the configuration directory.
///
/// A consumer resolves the configuration directory itself — `XDG_CONFIG_HOME`
/// and the home directory are environment facts the kernel never reads — and
/// joins this onto it. It is declared here so that resolving the record is a
/// join rather than a consumer taking [`FileRef::INSTALLATION_RECORD_PATH`]
/// apart to recover the path underneath a rendering: that rendering is
/// human-facing text, and deriving a path from it would break the day it is
/// reworded.
///
/// The two are held together when this crate compiles, by the assertion beside
/// the rendering itself.
pub const RECORD_RELATIVE_PATH: &str = "dogtag/installation.toml";

/// Who this machine attributes work to.
///
/// Nothing at this milestone reads the actor, and the asymmetry is deliberate:
/// it is the field a vault's owner sets once, so reporting its absence now is
/// what keeps provenance from turning up unattributed at the first write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Actor {
    name: String,
}

impl Actor {
    /// The actor's name, exactly as the record writes it.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// One registered vault: a name, and the absolute path it stands for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VaultEntry {
    name: String,
    path: PathBuf,
}

impl VaultEntry {
    /// The registry name, kebab-case and free of path separators.
    ///
    /// Those two properties are what let `--vault <name>` be distinguished from
    /// `--vault <path>` by syntax alone, with no fallback between them.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The registered path, absolute and literal.
    ///
    /// Neither `~` nor `$VAR` is ever expanded, so a registry entry cannot
    /// resolve differently from different directories.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// A record that loaded.
///
/// Holding one is the proof that a version in the supported range was declared
/// and that every rule the record states was kept: there is no way to build one
/// from a file that failed to load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallationRecord {
    installation_version: u32,
    actor: Option<Actor>,
    vaults: Vec<VaultEntry>,
    provenance: Provenance,
}

impl InstallationRecord {
    /// The version the record declares, already classified as supported.
    pub fn installation_version(&self) -> u32 {
        self.installation_version
    }

    /// The declared actor, when the record declares one.
    pub fn actor(&self) -> Option<&Actor> {
        self.actor.as_ref()
    }

    /// Every registered vault, in declaration order.
    pub fn vaults(&self) -> &[VaultEntry] {
        &self.vaults
    }

    /// The entry a registry name stands for, when one is registered.
    ///
    /// Names are unique in a record that loaded — duplicates are a load error —
    /// so this answers with the entry or with nothing, never ambiguously.
    pub fn entry(&self, name: &str) -> Option<&VaultEntry> {
        self.vaults.iter().find(|entry| entry.name == name)
    }

    /// Where each of this record's leaf values was written.
    pub fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

/// What reading the record produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallationState {
    /// No record exists. Not a fault: the record is never required to exist.
    Absent,
    /// A record exists and loaded.
    Loaded(InstallationRecord),
    /// A record exists and could not be used. The diagnostics say why.
    Unusable,
}

impl InstallationState {
    /// The lowercase wire spelling, used by every structured format.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Loaded(_) => "loaded",
            Self::Unusable => "unusable",
        }
    }
}

impl fmt::Display for InstallationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The outcome of reading the installation record: a state, and why.
///
/// The diagnostics stand on their own — a record that loaded may still carry an
/// info-severity note, and one that did not carries the errors that refused it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Installation {
    state: InstallationState,
    diagnostics: Vec<Diagnostic>,
}

impl Installation {
    /// What reading the record produced.
    pub fn state(&self) -> &InstallationState {
        &self.state
    }

    /// Everything reading the record had to say, in the deterministic order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// The loaded record, when one loaded.
    pub fn record(&self) -> Option<&InstallationRecord> {
        match &self.state {
            InstallationState::Loaded(record) => Some(record),
            InstallationState::Absent | InstallationState::Unusable => None,
        }
    }

    /// No record exists, which is not a fault and carries no diagnostic.
    fn absent() -> Self {
        Self {
            state: InstallationState::Absent,
            diagnostics: Vec::new(),
        }
    }

    /// A record exists and cannot be used.
    fn unusable(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            state: InstallationState::Unusable,
            diagnostics,
        }
    }

    /// A record loaded, possibly alongside notes that did not refuse it.
    fn loaded(record: InstallationRecord, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            state: InstallationState::Loaded(record),
            diagnostics,
        }
    }
}

/// Reads the installation record at an explicit path.
///
/// The path is the caller's: this function consults no environment variable and
/// no current directory, so the CLI's `XDG_CONFIG_HOME` resolution stays outside
/// the kernel. The file is opened for reading and never for writing.
///
/// A path that does not exist yields [`InstallationState::Absent`] with no
/// diagnostic at all. Every other I/O failure — a permission denial, a path
/// component that is not a directory — is `installation.unreadable`, because a
/// record that exists and cannot be read is a fault and an absent one is not.
pub fn load_installation(path: &Path) -> Installation {
    match fs::read(path) {
        Ok(bytes) => parse::parse_bytes(&bytes),
        Err(error) if error.kind() == ErrorKind::NotFound => Installation::absent(),
        Err(error) => Installation::unusable(vec![unreadable(&error)]),
    }
}

/// Reads an installation record already held in memory.
///
/// The same walk [`load_installation`] performs, minus the file. Absence has no
/// meaning here — text that exists is text that exists — so this never answers
/// [`InstallationState::Absent`].
pub fn parse_installation(text: &str) -> Installation {
    parse::parse_bytes(text.as_bytes())
}

/// The diagnostic for a record that exists and could not be read.
///
/// The message carries the operating system's own words and never the path:
/// locations always name the unexpanded record, so no account name is emitted.
fn unreadable(error: &Error) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::InstallationUnreadable,
        format!("the installation record could not be read: {error}"),
    )
    .at(Location::whole_file(FileRef::InstallationRecord))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::render_plain;
    use crate::provenance::Source;
    use std::env;
    use std::process;
    use std::time::{SystemTime, UNIX_EPOCH};

    const WORKED_EXAMPLE: &str = concat!(
        "installation_version = 1\n",
        "\n",
        "[actor]\n",
        "name = \"A Maintainer\"\n",
        "\n",
        "[[vault]]\n",
        "name = \"work\"\n",
        "path = \"/data/vaults/work\"\n",
    );

    /// A directory under the system temporary directory, taken away again when
    /// the test ends.
    ///
    /// The SDK never writes the installation record; these tests write files of
    /// their own so that reading one from a real filesystem is exercised, and
    /// none of them is at the record's real location.
    struct Scratch {
        root: PathBuf,
    }

    impl Scratch {
        fn new(label: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("the clock is after the Unix epoch")
                .as_nanos();
            let root = env::temp_dir().join(format!("dogtag-{label}-{}-{unique}", process::id()));
            fs::create_dir_all(&root).expect("a scratch directory");
            Self { root }
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.root.join(name);
            fs::write(&path, bytes).expect("a scratch file");
            path
        }

        fn join(&self, name: &str) -> PathBuf {
            self.root.join(name)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.root).ok();
        }
    }

    fn ids(installation: &Installation) -> Vec<&str> {
        installation
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    fn loaded() -> Installation {
        parse_installation(WORKED_EXAMPLE)
    }

    #[test]
    fn an_absent_record_is_a_state_and_not_a_diagnostic() {
        let scratch = Scratch::new("absent");
        let installation = load_installation(&scratch.join("installation.toml"));
        assert_eq!(installation.state(), &InstallationState::Absent);
        assert!(installation.diagnostics().is_empty());
        assert!(installation.record().is_none());
    }

    #[test]
    fn a_record_that_exists_and_cannot_be_read_is_a_diagnostic() {
        let scratch = Scratch::new("unreadable");
        let blocking = scratch.write("blocking", b"not a directory\n");
        let installation = load_installation(&blocking.join("installation.toml"));
        assert_eq!(installation.state(), &InstallationState::Unusable);
        assert_eq!(ids(&installation), ["installation.unreadable"]);
    }

    #[test]
    fn an_unreadable_record_names_the_unexpanded_path_and_no_account_name() {
        let scratch = Scratch::new("unreadable-path");
        let blocking = scratch.write("blocking", b"not a directory\n");
        let installation = load_installation(&blocking.join("installation.toml"));
        let rendered = render_plain(installation.diagnostics());
        assert!(rendered.contains(FileRef::INSTALLATION_RECORD_PATH));
        assert!(!rendered.contains(&scratch.root.display().to_string()));
    }

    #[test]
    fn a_record_on_disk_loads_through_its_bytes() {
        let scratch = Scratch::new("loads");
        let path = scratch.write("installation.toml", WORKED_EXAMPLE.as_bytes());
        let installation = load_installation(&path);
        assert!(installation.diagnostics().is_empty());
        let record = installation.record().expect("a loaded record");
        assert_eq!(record.installation_version(), 1);
    }

    #[test]
    fn bytes_that_are_not_utf8_are_refused_on_read() {
        let scratch = Scratch::new("utf8");
        let path = scratch.write("installation.toml", b"installation_version = \xff\n");
        let installation = load_installation(&path);
        assert_eq!(ids(&installation), ["installation.invalid-utf8"]);
        assert!(installation.record().is_none());
    }

    #[test]
    fn a_loaded_record_reports_its_actor_and_registry() {
        let installation = loaded();
        let record = installation.record().expect("a loaded record");
        assert_eq!(record.actor().map(Actor::name), Some("A Maintainer"));
        assert_eq!(record.vaults().len(), 1);
        assert_eq!(record.vaults()[0].name(), "work");
        assert_eq!(record.vaults()[0].path(), Path::new("/data/vaults/work"));
    }

    #[test]
    fn a_registry_name_resolves_to_its_entry_or_to_nothing() {
        let installation = loaded();
        let record = installation.record().expect("a loaded record");
        assert_eq!(record.entry("work").map(VaultEntry::name), Some("work"));
        assert!(record.entry("home").is_none());
    }

    #[test]
    fn a_loaded_record_carries_the_provenance_of_every_leaf() {
        let installation = loaded();
        let record = installation.record().expect("a loaded record");
        let entry = record.provenance().get("actor.name").expect("recorded");
        assert_eq!(entry.source, Source::Installation);
        assert_eq!(record.provenance().len(), 4);
    }

    #[test]
    fn only_a_loaded_state_yields_a_record() {
        assert!(Installation::absent().record().is_none());
        assert!(Installation::unusable(Vec::new()).record().is_none());
        assert!(parse_installation("").record().is_none());
    }

    #[test]
    fn states_render_for_structured_output() {
        let record = loaded().record().cloned().expect("a loaded record");
        assert_eq!(InstallationState::Absent.as_str(), "absent");
        assert_eq!(InstallationState::Unusable.to_string(), "unusable");
        assert_eq!(InstallationState::Loaded(record).as_str(), "loaded");
    }

    #[test]
    fn the_model_clones_compares_and_formats() {
        let installation = loaded();
        assert_eq!(installation.clone(), installation);
        assert_ne!(installation, Installation::absent());
        assert!(format!("{installation:?}").contains("A Maintainer"));
        assert!(format!("{}", installation.state()).contains("loaded"));
    }

    #[test]
    fn each_model_type_clones_compares_and_formats() {
        let installation = loaded();
        let record = installation.record().expect("a loaded record");
        let actor = record.actor().expect("an actor");
        let vault = &record.vaults()[0];
        assert_eq!(actor.clone(), *actor);
        assert_eq!(vault.clone(), *vault);
        assert_eq!(record.clone(), *record);
        assert!(format!("{actor:?}{vault:?}{record:?}").contains("/data/vaults/work"));
    }

    #[test]
    fn states_clone_compare_and_format() {
        let states = vec![InstallationState::Absent, InstallationState::Unusable];
        assert_eq!(states.clone(), states);
        assert_ne!(states[0], states[1]);
        assert!(format!("{states:?}").contains("Unusable"));
    }
}
