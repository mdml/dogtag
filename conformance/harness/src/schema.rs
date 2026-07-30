//! The fixture schemas: scenarios, profiles, and their strict parsers.
//!
//! Both document types carry `deny_unknown_fields`, which is the structural
//! half of the no-waiver rule: the field with which a scenario could scope
//! itself to a profile (or a profile exempt itself from a scenario) does not
//! exist, and adding one fails parsing instead of creating an exemption.

use std::fmt;

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

/// Parse one scenario document. Exposed so tests (and future tooling) can
/// assert schema behavior — in particular that waiver-shaped fields fail.
pub fn parse_scenario(toml_text: &str) -> Result<Scenario, toml::de::Error> {
    toml::from_str(toml_text)
}

/// Parse one profile document. Same rationale as [`parse_scenario`].
pub fn parse_profile(toml_text: &str) -> Result<Profile, toml::de::Error> {
    toml::from_str(toml_text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `is_kebab_case` accepts exactly `[a-z0-9]+(-[a-z0-9]+)*`: lowercase
    /// ASCII words joined by single hyphens, digits allowed.
    #[test]
    fn kebab_case_accepts_lowercase_hyphenated_words() {
        for id in ["a", "m2", "starter", "missing-required-property", "a-1-b"] {
            assert!(is_kebab_case(id), "`{id}` is kebab-case");
        }
    }

    /// Each structural rejection is its own branch: empty input, a leading
    /// or trailing hyphen, and a doubled hyphen are all refused even though
    /// every character is individually legal.
    #[test]
    fn kebab_case_rejects_misplaced_hyphens() {
        for id in ["", "-leading", "trailing-", "double--dash"] {
            assert!(!is_kebab_case(id), "`{id}` is not kebab-case");
        }
    }

    /// The character-class branch: anything outside lowercase ASCII, digits,
    /// and `-` is refused — uppercase, underscores, dots, spaces, Unicode.
    #[test]
    fn kebab_case_rejects_foreign_characters() {
        for id in ["Upper", "snake_case", "dotted.id", "with space", "naïve"] {
            assert!(!is_kebab_case(id), "`{id}` is not kebab-case");
        }
    }
}
