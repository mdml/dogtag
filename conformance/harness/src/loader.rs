//! Strict fixture loaders for `conformance/scenarios/` and
//! `conformance/profiles/`.
//!
//! Strict means the loaders reject anything they did not expect: a stray
//! file, a mismatched name, a corpus status that disagrees with the disk.
//! This is the filesystem half of the no-waiver rule — an out-of-band
//! convention has nowhere to hide because every unexpected entry is a load
//! error, never something silently skipped.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::HarnessError;
use crate::schema::{
    CorpusStatus, Profile, Scenario, is_kebab_case, parse_profile, parse_scenario,
};
use crate::{profiles_dir, scenarios_dir};

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
    let mut seen = BTreeSet::new();
    let mut scenarios = Vec::new();
    for path in scenario_files(dir)? {
        let scenario = load_scenario_file(&path)?;
        if !seen.insert(scenario.id.clone()) {
            return Err(HarnessError::Invalid(format!(
                "duplicate scenario id `{}`",
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
/// [`crate::REQUIRED_PROFILES`].
pub fn load_profiles() -> Result<Vec<Profile>, HarnessError> {
    load_profiles_from(&profiles_dir())
}

/// [`load_profiles`] against an explicit directory, so tests can exercise
/// the strict loading rules on synthetic trees.
pub fn load_profiles_from(dir: &Path) -> Result<Vec<Profile>, HarnessError> {
    let mut profiles = Vec::new();
    for subdir in profile_dirs(dir)? {
        profiles.push(load_profile_dir(&subdir)?);
    }
    Ok(profiles)
}

/// Read a directory's entries, sorted, mapping failures to
/// [`HarnessError::Io`].
fn sorted_entries(dir: &Path) -> Result<Vec<PathBuf>, HarnessError> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| HarnessError::Io(dir.to_path_buf(), e))? {
        let entry = entry.map_err(|e| HarnessError::Io(dir.to_path_buf(), e))?;
        paths.push(entry.path());
    }
    paths.sort();
    Ok(paths)
}

/// The one strict directory scan: every entry must satisfy `allowed`, and
/// any other entry fails the load with `expectation` naming what the
/// directory may hold. Nothing is ever silently skipped — a stray entry is
/// where an out-of-band convention would hide.
fn strict_entries(
    dir: &Path,
    allowed: impl Fn(&Path) -> bool,
    expectation: &str,
) -> Result<Vec<PathBuf>, HarnessError> {
    let paths = sorted_entries(dir)?;
    for path in &paths {
        if !allowed(path) {
            return Err(HarnessError::Invalid(format!(
                "unexpected entry `{}` in {}: {expectation}",
                path.display(),
                dir.display()
            )));
        }
    }
    Ok(paths)
}

/// The scenarios directory holds only scenario `*.toml` files; anything
/// else is a load error.
fn scenario_files(dir: &Path) -> Result<Vec<PathBuf>, HarnessError> {
    strict_entries(
        dir,
        |path| path.is_file() && path.extension().is_some_and(|ext| ext == "toml"),
        "the scenarios directory holds only scenario *.toml files",
    )
}

/// Read, parse, and validate one scenario file.
fn load_scenario_file(path: &Path) -> Result<Scenario, HarnessError> {
    let text = fs::read_to_string(path).map_err(|e| HarnessError::Io(path.to_path_buf(), e))?;
    let scenario = parse_scenario(&text).map_err(|e| HarnessError::Parse(path.to_path_buf(), e))?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    validate_scenario(&scenario, stem)?;
    Ok(scenario)
}

/// Field-level scenario checks: id matches the filename, is kebab-case, and
/// the human-facing fields are non-empty.
fn validate_scenario(scenario: &Scenario, stem: &str) -> Result<(), HarnessError> {
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
    Ok(())
}

/// The profiles directory holds only profile subdirectories; a stray file
/// is a load error.
fn profile_dirs(dir: &Path) -> Result<Vec<PathBuf>, HarnessError> {
    strict_entries(
        dir,
        |path| path.is_dir(),
        "the profiles directory holds only profile subdirectories",
    )
}

/// Load, parse, and validate one profile directory.
fn load_profile_dir(subdir: &Path) -> Result<Profile, HarnessError> {
    check_profile_entries(subdir)?;
    let path = subdir.join("PROFILE.toml");
    let text = fs::read_to_string(&path).map_err(|e| HarnessError::Io(path.clone(), e))?;
    let profile = parse_profile(&text).map_err(|e| HarnessError::Parse(path.clone(), e))?;
    let dirname = subdir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    validate_profile(&profile, dirname)?;
    check_corpus_consistency(&profile, &subdir.join("corpus"))?;
    Ok(profile)
}

/// A profile directory holds only `PROFILE.toml`, `PROFILE.md`, and (once
/// built) a `corpus/` directory; anything else is a load error.
fn check_profile_entries(subdir: &Path) -> Result<(), HarnessError> {
    strict_entries(
        subdir,
        is_allowed_profile_entry,
        "a profile directory holds only PROFILE.toml, PROFILE.md, and (once built) a corpus/ \
         directory",
    )?;
    Ok(())
}

/// `true` for exactly the entries a profile directory may contain.
fn is_allowed_profile_entry(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    (path.is_file() && (name == "PROFILE.toml" || name == "PROFILE.md"))
        || (path.is_dir() && name == "corpus")
}

/// Field-level profile checks: name matches the directory, is kebab-case,
/// and the declared metadata is non-empty.
fn validate_profile(profile: &Profile, dirname: &str) -> Result<(), HarnessError> {
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
    Ok(())
}

/// The declared corpus status must match the disk: `built` requires
/// `corpus/` to exist, `scheduled` requires it not to. A mismatch in either
/// direction is a lie the loader refuses to load.
fn check_corpus_consistency(profile: &Profile, corpus_dir: &Path) -> Result<(), HarnessError> {
    match profile.corpus {
        CorpusStatus::Built if !corpus_dir.is_dir() => Err(HarnessError::Invalid(format!(
            "profile `{}` declares corpus = \"built\" but {} does not exist",
            profile.name,
            corpus_dir.display()
        ))),
        CorpusStatus::Scheduled if corpus_dir.is_dir() => Err(HarnessError::Invalid(format!(
            "profile `{}` declares corpus = \"scheduled\" but {} exists",
            profile.name,
            corpus_dir.display()
        ))),
        _ => Ok(()),
    }
}
