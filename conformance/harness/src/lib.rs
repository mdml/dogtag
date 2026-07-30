//! Conformance harness for the Dogtag SDK.
//!
//! Every scenario in `conformance/scenarios/` runs against every fixture
//! profile in `conformance/profiles/`. There is no waiver mechanism: a
//! scenario expressible against only one profile fails the harness and is
//! triaged as either an incomplete configuration model or a personal
//! convention mistaken for an invariant. The mechanical channels for a
//! waiver — schema fields, stray files, a filtered cross product — are
//! enforced structurally (see [`Scenario`] and the strict directory
//! loaders); keeping the prose contracts profile-agnostic still rests on
//! review discipline until execution wiring closes the loop.
//!
//! At M1 every scenario is `pending` and every profile corpus is `scheduled`,
//! so the harness produces the complete scenarios × profiles matrix of
//! pending outcomes and nothing executes. The harness deliberately does not
//! depend on the `dogtag` SDK today (there is nothing to call); when the
//! first scenario graduates to `executable`, execution wiring lands here and
//! consumes only the SDK's public API.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// The exact fixture-profile roster from BETA.md, sorted by name.
///
/// Four profiles, each standing for one persona and together spreading
/// across every axis the configuration seam claims to absorb. The harness
/// requires exactly this roster: a missing profile shrinks the cross product
/// and a surplus profile is a fixture nothing specified.
pub const REQUIRED_PROFILES: [&str; 4] = ["dense", "docs", "records", "starter"];

/// A golden conformance scenario, one per file in `conformance/scenarios/`.
///
/// `deny_unknown_fields` is the structural no-waiver enforcement: there is
/// deliberately no field that could name a profile, so a scenario cannot
/// opt out of any profile even in principle. An added `profiles = [...]`,
/// `skip`, `waive`, or `only` key fails parsing instead of creating an
/// exemption — the place where a personal invariant would hide does not
/// exist in the schema. Tests assert the rejection; see also
/// `conformance/README.md`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Kebab-case identifier; must equal the file's basename without `.toml`.
    pub id: String,
    /// One-line human statement of what the scenario proves.
    pub title: String,
    /// The milestone at which the scenario becomes executable.
    pub milestone: Milestone,
    /// Whether the scenario is a written contract or an executing test.
    pub status: ScenarioStatus,
    /// Given/when/then prose in profile-agnostic terms: behavior binds to
    /// declared capabilities and declared axes, never to any corpus's
    /// vocabulary. No type name, no lifecycle word, no dialect assumption.
    pub contract: String,
}

/// The milestone at which a scenario graduates from contract to executing
/// test. The M0 golden set covers M2–M3; search and mutation scenarios
/// accrue with their own milestones later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
pub enum Milestone {
    /// Open and diagnose: contract loading, capability validation,
    /// structured diagnostics.
    M2,
    /// Read and validate: document model, `check`, `list`, `show`.
    M3,
}

impl fmt::Display for Milestone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Milestone::M2 => f.write_str("M2"),
            Milestone::M3 => f.write_str("M3"),
        }
    }
}

/// Whether a scenario is still prose or already runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScenarioStatus {
    /// A written contract; not yet backed by executing code. All scenarios
    /// are pending at M1.
    Pending,
    /// Backed by executing code. Flipping to this status commits the
    /// scenario to run against every profile — there is no partial
    /// graduation.
    Executable,
}

/// A fixture profile, one per `conformance/profiles/<name>/PROFILE.toml`.
///
/// Same structural rule as [`Scenario`]: `deny_unknown_fields` means a
/// profile has no field with which to exempt itself from any scenario.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Kebab-case name; must equal the profile's directory name.
    pub name: String,
    /// The persona the profile stands for, from the persona table.
    pub persona: String,
    /// The seam axes this profile stresses, from BETA.md's fixture table.
    pub distinguishing_axes: Vec<String>,
    /// Whether the fixture corpus exists yet.
    pub corpus: CorpusStatus,
    /// When the corpus is built (e.g. `"M2"`, `"M4"`, `"pre-E1"`). Free-form
    /// because fixture schedules include pre-experiment gates, not only
    /// numbered milestones.
    pub corpus_milestone: String,
}

/// Whether a profile's fixture corpus has been built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CorpusStatus {
    /// Specified (PROFILE.md) but not yet built. All corpora are scheduled
    /// at M1: the committed vault-contract format is an M2 decision, and a
    /// corpus built before the format exists would freeze a guess.
    Scheduled,
    /// The fixture corpus exists under the profile directory.
    Built,
}

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
    /// A scenario is `executable` and a profile corpus is `built`, but
    /// execution is not wired yet. The harness refuses to report such a pair
    /// as anything, rather than silently marking it pending: graduation is
    /// all-profiles-or-nothing, and the milestone that graduates the first
    /// scenario must land the execution path here.
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
                 but the harness has no execution path yet; wire execution before graduating \
                 scenarios (graduation runs against every profile — no partial graduation)"
            ),
        }
    }
}

impl std::error::Error for HarnessError {}

/// `true` when `s` is non-empty lowercase-ASCII kebab-case
/// (`[a-z0-9]+(-[a-z0-9]+)*`).
pub fn is_kebab_case(s: &str) -> bool {
    !s.is_empty()
        && !s.starts_with('-')
        && !s.ends_with('-')
        && !s.contains("--")
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Root of the `conformance/` directory (parent of the harness crate).
pub fn conformance_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("harness crate lives directly under conformance/")
        .to_path_buf()
}

/// `conformance/scenarios/`.
pub fn scenarios_dir() -> PathBuf {
    conformance_root().join("scenarios")
}

/// `conformance/profiles/`.
pub fn profiles_dir() -> PathBuf {
    conformance_root().join("profiles")
}

/// Parse one scenario document. Exposed so tests (and future tooling) can
/// assert schema behavior — in particular that waiver-shaped fields fail.
pub fn parse_scenario(toml_text: &str) -> Result<Scenario, toml::de::Error> {
    toml::from_str(toml_text)
}

/// Parse one profile document. Same rationale as [`parse_scenario`].
pub fn parse_profile(toml_text: &str) -> Result<Profile, toml::de::Error> {
    toml::from_str(toml_text)
}

/// Load and validate every scenario in `conformance/scenarios/`, sorted by id.
///
/// Validates: the directory contains *only* scenario `*.toml` files (any
/// other entry is a load error — a stray file is where an out-of-band
/// convention would hide); every file parses under the strict schema; `id`
/// equals the file stem; ids are kebab-case and unique; `title` and
/// `contract` are non-empty.
pub fn load_scenarios() -> Result<Vec<Scenario>, HarnessError> {
    load_scenarios_from(&scenarios_dir())
}

/// [`load_scenarios`] against an explicit directory, so tests can exercise
/// the strict loading rules on synthetic trees.
pub fn load_scenarios_from(dir: &Path) -> Result<Vec<Scenario>, HarnessError> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| HarnessError::Io(dir.to_path_buf(), e))? {
        let entry = entry.map_err(|e| HarnessError::Io(dir.to_path_buf(), e))?;
        let path = entry.path();
        if !path.is_file() || path.extension().is_none_or(|ext| ext != "toml") {
            return Err(HarnessError::Invalid(format!(
                "unexpected entry `{}` in {}: the scenarios directory holds only scenario *.toml files",
                path.display(),
                dir.display()
            )));
        }
        paths.push(path);
    }
    paths.sort();

    let mut seen = BTreeSet::new();
    let mut scenarios = Vec::with_capacity(paths.len());
    for path in paths {
        let text = fs::read_to_string(&path).map_err(|e| HarnessError::Io(path.clone(), e))?;
        let scenario = parse_scenario(&text).map_err(|e| HarnessError::Parse(path.clone(), e))?;
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if scenario.id != stem {
            return Err(HarnessError::Invalid(format!(
                "scenario id `{}` does not match filename stem `{stem}`",
                scenario.id
            )));
        }
        if !is_kebab_case(&scenario.id) {
            return Err(HarnessError::Invalid(format!(
                "scenario id `{}` is not kebab-case",
                scenario.id
            )));
        }
        if !seen.insert(scenario.id.clone()) {
            return Err(HarnessError::Invalid(format!(
                "duplicate scenario id `{}`",
                scenario.id
            )));
        }
        if scenario.title.trim().is_empty() {
            return Err(HarnessError::Invalid(format!(
                "scenario `{}` has an empty title",
                scenario.id
            )));
        }
        if scenario.contract.trim().is_empty() {
            return Err(HarnessError::Invalid(format!(
                "scenario `{}` has an empty contract",
                scenario.id
            )));
        }
        scenarios.push(scenario);
    }
    Ok(scenarios)
}

/// Load and validate every profile in `conformance/profiles/`, sorted by name.
///
/// Validates: the directory contains *only* profile subdirectories (a stray
/// file is a load error); each profile directory contains *only*
/// `PROFILE.toml`, `PROFILE.md`, and — once built — a `corpus/` directory;
/// every `PROFILE.toml` parses under the strict schema; `name` equals the
/// directory name and is kebab-case; `persona` and `corpus_milestone` are
/// non-empty; `distinguishing_axes` is non-empty; the declared `corpus`
/// status matches the disk (`built` requires `corpus/` to exist, `scheduled`
/// requires it not to). Roster completeness is asserted by tests against
/// [`REQUIRED_PROFILES`].
pub fn load_profiles() -> Result<Vec<Profile>, HarnessError> {
    load_profiles_from(&profiles_dir())
}

/// [`load_profiles`] against an explicit directory, so tests can exercise
/// the strict loading rules on synthetic trees.
pub fn load_profiles_from(dir: &Path) -> Result<Vec<Profile>, HarnessError> {
    let mut subdirs: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| HarnessError::Io(dir.to_path_buf(), e))? {
        let entry = entry.map_err(|e| HarnessError::Io(dir.to_path_buf(), e))?;
        let path = entry.path();
        if !path.is_dir() {
            return Err(HarnessError::Invalid(format!(
                "unexpected entry `{}` in {}: the profiles directory holds only profile subdirectories",
                path.display(),
                dir.display()
            )));
        }
        subdirs.push(path);
    }
    subdirs.sort();

    let mut profiles = Vec::with_capacity(subdirs.len());
    for subdir in subdirs {
        for entry in fs::read_dir(&subdir).map_err(|e| HarnessError::Io(subdir.clone(), e))? {
            let entry = entry.map_err(|e| HarnessError::Io(subdir.clone(), e))?;
            let entry_path = entry.path();
            let entry_name = entry_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or_default();
            let allowed = (entry_path.is_file()
                && (entry_name == "PROFILE.toml" || entry_name == "PROFILE.md"))
                || (entry_path.is_dir() && entry_name == "corpus");
            if !allowed {
                return Err(HarnessError::Invalid(format!(
                    "unexpected entry `{}` in {}: a profile directory holds only PROFILE.toml, \
                     PROFILE.md, and (once built) a corpus/ directory",
                    entry_path.display(),
                    subdir.display()
                )));
            }
        }

        let path = subdir.join("PROFILE.toml");
        let text = fs::read_to_string(&path).map_err(|e| HarnessError::Io(path.clone(), e))?;
        let profile = parse_profile(&text).map_err(|e| HarnessError::Parse(path.clone(), e))?;
        let dirname = subdir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        if profile.name != dirname {
            return Err(HarnessError::Invalid(format!(
                "profile name `{}` does not match directory name `{dirname}`",
                profile.name
            )));
        }
        if !is_kebab_case(&profile.name) {
            return Err(HarnessError::Invalid(format!(
                "profile name `{}` is not kebab-case",
                profile.name
            )));
        }
        if profile.persona.trim().is_empty() {
            return Err(HarnessError::Invalid(format!(
                "profile `{}` has an empty persona",
                profile.name
            )));
        }
        if profile.distinguishing_axes.is_empty() {
            return Err(HarnessError::Invalid(format!(
                "profile `{}` declares no distinguishing axes",
                profile.name
            )));
        }
        if profile.corpus_milestone.trim().is_empty() {
            return Err(HarnessError::Invalid(format!(
                "profile `{}` has an empty corpus_milestone",
                profile.name
            )));
        }
        let corpus_dir = subdir.join("corpus");
        match profile.corpus {
            CorpusStatus::Built if !corpus_dir.is_dir() => {
                return Err(HarnessError::Invalid(format!(
                    "profile `{}` declares corpus = \"built\" but {} does not exist",
                    profile.name,
                    corpus_dir.display()
                )));
            }
            CorpusStatus::Scheduled if corpus_dir.is_dir() => {
                return Err(HarnessError::Invalid(format!(
                    "profile `{}` declares corpus = \"scheduled\" but {} exists",
                    profile.name,
                    corpus_dir.display()
                )));
            }
            _ => {}
        }
        profiles.push(profile);
    }
    Ok(profiles)
}

/// One cell of the scenarios × profiles cross product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    /// The scenario's id.
    pub scenario_id: String,
    /// The profile's name.
    pub profile_name: String,
    /// What the harness can say about the pair.
    pub outcome: Outcome,
}

/// The outcome of one scenario/profile pair.
///
/// At M1 the only possible outcome is [`Outcome::Pending`]. Pass/fail
/// variants arrive with the execution path, at the milestone that graduates
/// the first scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Not runnable yet: the scenario is still a prose contract, the
    /// profile's corpus is not built, or both.
    Pending {
        /// The scenario's status is still `pending`.
        scenario_pending: bool,
        /// The profile's corpus is not yet built.
        corpus_missing: bool,
    },
}

/// Compute the full cross product: every scenario against every profile,
/// each pair exactly once. No filter parameter exists — the signature is
/// part of the no-waiver enforcement.
///
/// Errors with [`HarnessError::NotExecutable`] if a pair is runnable
/// (executable scenario × built corpus) because execution is not wired yet;
/// the harness refuses to invent an outcome for a pair it should be running.
pub fn report(scenarios: &[Scenario], profiles: &[Profile]) -> Result<Vec<Pair>, HarnessError> {
    let mut pairs = Vec::with_capacity(scenarios.len() * profiles.len());
    for scenario in scenarios {
        for profile in profiles {
            let scenario_pending = scenario.status == ScenarioStatus::Pending;
            let corpus_missing = profile.corpus == CorpusStatus::Scheduled;
            if !scenario_pending && !corpus_missing {
                return Err(HarnessError::NotExecutable {
                    scenario: scenario.id.clone(),
                    profile: profile.name.clone(),
                });
            }
            pairs.push(Pair {
                scenario_id: scenario.id.clone(),
                profile_name: profile.name.clone(),
                outcome: Outcome::Pending {
                    scenario_pending,
                    corpus_missing,
                },
            });
        }
    }
    Ok(pairs)
}

/// Render the cross product as a human-readable matrix: one row per
/// scenario, one column per profile, every cell filled. Printed by the
/// harness's matrix test (`just conformance` runs it with `--nocapture`).
pub fn pending_matrix(scenarios: &[Scenario], profiles: &[Profile], pairs: &[Pair]) -> String {
    let outcome_by_pair: BTreeMap<(&str, &str), &Outcome> = pairs
        .iter()
        .map(|p| {
            (
                (p.scenario_id.as_str(), p.profile_name.as_str()),
                &p.outcome,
            )
        })
        .collect();

    let row_label = |s: &Scenario| format!("{} ({})", s.id, s.milestone);
    let header_label = "scenario (milestone)";
    let label_width = scenarios
        .iter()
        .map(|s| row_label(s).len())
        .chain([header_label.len()])
        .max()
        .unwrap_or(0);
    let cell = |outcome: Option<&&Outcome>| -> &'static str {
        match outcome {
            Some(Outcome::Pending { .. }) => "pending",
            None => "MISSING",
        }
    };
    let col_widths: Vec<usize> = profiles.iter().map(|p| p.name.len().max(7)).collect();

    let mut out = String::new();
    let pending_count = pairs
        .iter()
        .filter(|p| matches!(p.outcome, Outcome::Pending { .. }))
        .count();
    out.push_str(&format!(
        "conformance cross product: {} scenarios x {} profiles = {} pairs ({} pending)\n\n",
        scenarios.len(),
        profiles.len(),
        pairs.len(),
        pending_count
    ));

    out.push_str(&format!("{header_label:<label_width$}"));
    for (profile, width) in profiles.iter().zip(&col_widths) {
        out.push_str(&format!("  {:<width$}", profile.name));
    }
    out.push('\n');
    out.push_str(&"-".repeat(label_width));
    for width in &col_widths {
        out.push_str("  ");
        out.push_str(&"-".repeat(*width));
    }
    out.push('\n');
    for scenario in scenarios {
        out.push_str(&format!("{:<label_width$}", row_label(scenario)));
        for (profile, width) in profiles.iter().zip(&col_widths) {
            let outcome = outcome_by_pair.get(&(scenario.id.as_str(), profile.name.as_str()));
            out.push_str(&format!("  {:<width$}", cell(outcome)));
        }
        out.push('\n');
    }
    out.push('\n');
    out.push_str("corpora: ");
    let corpora: Vec<String> = profiles
        .iter()
        .map(|p| {
            format!(
                "{} ({}, {})",
                p.name,
                match p.corpus {
                    CorpusStatus::Scheduled => "scheduled",
                    CorpusStatus::Built => "built",
                },
                p.corpus_milestone
            )
        })
        .collect();
    out.push_str(&corpora.join(", "));
    out.push('\n');
    out
}
