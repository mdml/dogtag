//! Harness self-tests: the fixtures parse, the roster is exact, the cross
//! product is complete, and — the load-bearing one — waiver-shaped fields
//! cannot exist in the schema.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use dogtag_conformance::{
    CorpusStatus, HarnessError, Milestone, Outcome, Profile, REQUIRED_PROFILES, Scenario,
    ScenarioStatus, load_profiles, load_profiles_from, load_scenarios, load_scenarios_from,
    parse_profile, parse_scenario, pending_matrix, report,
};

/// A minimal scenario document that must parse; the waiver-rejection tests
/// append one extra key to it and demand failure.
const MINIMAL_SCENARIO: &str = r#"
id = "minimal-scenario"
title = "A minimal scenario for schema tests"
milestone = "M2"
status = "pending"
contract = "Given a corpus. When nothing happens. Then nothing is reported."
"#;

const MINIMAL_PROFILE: &str = r#"
name = "minimal"
persona = "a schema-test persona"
distinguishing_axes = ["one axis"]
corpus = "scheduled"
corpus_milestone = "M2"
"#;

#[test]
fn all_scenario_files_parse_with_unique_kebab_case_ids() {
    let scenarios = load_scenarios().expect("every scenario file parses and validates");
    assert_eq!(
        scenarios.len(),
        11,
        "the M0 golden set is exactly eleven scenarios"
    );

    // load_scenarios already enforces id == filename stem, kebab-case, and
    // uniqueness; re-assert uniqueness here so the property is stated in a
    // test, not only in the loader.
    let ids: BTreeSet<&str> = scenarios.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids.len(), scenarios.len(), "scenario ids are unique");

    for scenario in &scenarios {
        assert!(
            dogtag_conformance::is_kebab_case(&scenario.id),
            "scenario id `{}` is kebab-case",
            scenario.id
        );
        assert_eq!(
            scenario.status,
            ScenarioStatus::Pending,
            "at M1 every scenario is pending (`{}` is not)",
            scenario.id
        );
        assert!(
            matches!(scenario.milestone, Milestone::M2 | Milestone::M3),
            "the M0 golden set covers M2-M3 (`{}` does not)",
            scenario.id
        );
    }
}

#[test]
fn profile_roster_is_exactly_the_beta_roster() {
    let profiles = load_profiles().expect("every profile parses and validates");
    let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names, REQUIRED_PROFILES,
        "conformance/profiles/ must hold exactly the four BETA.md profiles"
    );
    for profile in &profiles {
        assert_eq!(
            profile.corpus,
            CorpusStatus::Scheduled,
            "at M1 every corpus is scheduled, not built (`{}` is not)",
            profile.name
        );
    }
}

/// The no-waiver test. A scenario schema with any field that could name,
/// skip, or scope a profile must not exist; serde's `deny_unknown_fields`
/// turns each such key into a parse failure. If this test ever starts
/// failing, someone widened the schema — that is the exact change the
/// conformance rule forbids.
#[test]
fn waiver_shaped_fields_fail_scenario_parsing() {
    assert!(
        parse_scenario(MINIMAL_SCENARIO).is_ok(),
        "the minimal scenario itself must parse, or the rejections below prove nothing"
    );

    let waiver_keys = [
        r#"profiles = ["dense"]"#,
        r#"skip = ["dense"]"#,
        r#"waive = "dense""#,
        r#"only = ["starter"]"#,
        r#"except = ["records"]"#,
    ];
    for key in waiver_keys {
        let doc = format!("{MINIMAL_SCENARIO}{key}\n");
        assert!(
            parse_scenario(&doc).is_err(),
            "a scenario with `{key}` must fail to parse: profile scoping has no syntax"
        );
    }
}

/// Symmetric rejection on the profile side: a profile cannot exempt itself
/// from scenarios either.
#[test]
fn waiver_shaped_fields_fail_profile_parsing() {
    assert!(
        parse_profile(MINIMAL_PROFILE).is_ok(),
        "the minimal profile itself must parse, or the rejections below prove nothing"
    );

    let waiver_keys = [
        r#"scenarios = ["missing-required-property-diagnostic"]"#,
        r#"skip = ["missing-required-property-diagnostic"]"#,
        r#"waive = "all""#,
        r#"only = ["contract-loads-with-provenance"]"#,
    ];
    for key in waiver_keys {
        let doc = format!("{MINIMAL_PROFILE}{key}\n");
        assert!(
            parse_profile(&doc).is_err(),
            "a profile with `{key}` must fail to parse: scenario scoping has no syntax"
        );
    }
}

#[test]
fn cross_product_is_complete_and_all_pending_at_m1() {
    let scenarios = load_scenarios().expect("scenarios load");
    let profiles = load_profiles().expect("profiles load");
    let pairs = report(&scenarios, &profiles).expect("the M1 report succeeds");

    assert_eq!(
        pairs.len(),
        scenarios.len() * profiles.len(),
        "the report is the full cross product"
    );

    let mut seen = BTreeSet::new();
    for pair in &pairs {
        assert!(
            seen.insert((pair.scenario_id.clone(), pair.profile_name.clone())),
            "pair ({}, {}) appears more than once",
            pair.scenario_id,
            pair.profile_name
        );
    }
    for scenario in &scenarios {
        for profile in &profiles {
            assert!(
                seen.contains(&(scenario.id.clone(), profile.name.clone())),
                "pair ({}, {}) is missing from the report",
                scenario.id,
                profile.name
            );
        }
    }

    for pair in &pairs {
        match pair.outcome {
            Outcome::Pending {
                scenario_pending,
                corpus_missing,
            } => {
                assert!(
                    scenario_pending || corpus_missing,
                    "({}, {}) is marked pending for no reason",
                    pair.scenario_id,
                    pair.profile_name
                );
                // At M1 specifically, both hold for every pair.
                assert!(
                    scenario_pending && corpus_missing,
                    "at M1 every scenario is pending AND every corpus is scheduled \
                     (({}, {}) disagrees)",
                    pair.scenario_id,
                    pair.profile_name
                );
            }
        }
    }
}

/// Graduation is all-or-nothing: the harness refuses to produce a report at
/// all when a pair is runnable but execution is not wired, rather than
/// quietly calling it pending.
#[test]
fn runnable_pair_without_execution_path_is_refused() {
    let scenario = Scenario {
        id: "graduated-scenario".to_string(),
        title: "A scenario that has flipped to executable".to_string(),
        milestone: Milestone::M2,
        status: ScenarioStatus::Executable,
        contract: "Given/when/then.".to_string(),
    };
    let profile = Profile {
        name: "built-profile".to_string(),
        persona: "a persona with a built corpus".to_string(),
        distinguishing_axes: vec!["one axis".to_string()],
        corpus: CorpusStatus::Built,
        corpus_milestone: "M2".to_string(),
    };
    let err = report(&[scenario], &[profile]).expect_err("runnable pair must be refused");
    let message = err.to_string();
    assert!(
        message.contains("graduated-scenario") && message.contains("built-profile"),
        "the refusal names the pair: {message}"
    );
}

/// A throwaway directory under the system temp dir, removed on drop.
/// Standard library only, per the dependency policy — no `tempfile`.
struct TempTree(PathBuf);

impl TempTree {
    fn new(label: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "dogtag-conformance-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("temp tree created");
        TempTree(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn write_profile_dir(profiles_dir: &Path, name: &str, corpus: &str) -> PathBuf {
    let dir = profiles_dir.join(name);
    fs::create_dir_all(&dir).expect("profile dir created");
    fs::write(
        dir.join("PROFILE.toml"),
        format!(
            "name = \"{name}\"\npersona = \"a strictness-test persona\"\n\
             distinguishing_axes = [\"one axis\"]\ncorpus = \"{corpus}\"\n\
             corpus_milestone = \"M2\"\n"
        ),
    )
    .expect("PROFILE.toml written");
    fs::write(dir.join("PROFILE.md"), "# strictness-test profile\n").expect("PROFILE.md written");
    dir
}

fn expect_invalid<T: std::fmt::Debug>(result: Result<T, HarnessError>, needle: &str) {
    match result {
        Err(HarnessError::Invalid(message)) => assert!(
            message.contains(needle),
            "error message should mention `{needle}`: {message}"
        ),
        other => panic!("expected HarnessError::Invalid mentioning `{needle}`, got {other:?}"),
    }
}

/// The scenarios directory holds only scenario `*.toml` files: a stray file
/// (or subdirectory) is a load error, not something silently skipped.
#[test]
fn stray_entry_in_scenarios_dir_is_a_load_error() {
    let tree = TempTree::new("scenarios-stray");
    fs::write(tree.path().join("minimal-scenario.toml"), MINIMAL_SCENARIO)
        .expect("scenario written");
    fs::write(tree.path().join("NOTES.md"), "stray\n").expect("stray file written");
    expect_invalid(load_scenarios_from(tree.path()), "NOTES.md");

    let subdir_tree = TempTree::new("scenarios-subdir");
    fs::create_dir_all(subdir_tree.path().join("drafts")).expect("stray subdir created");
    expect_invalid(load_scenarios_from(subdir_tree.path()), "drafts");
}

/// The profiles directory holds only profile subdirectories.
#[test]
fn stray_file_in_profiles_dir_is_a_load_error() {
    let tree = TempTree::new("profiles-stray");
    write_profile_dir(tree.path(), "minimal", "scheduled");
    fs::write(tree.path().join("README.md"), "stray\n").expect("stray file written");
    expect_invalid(load_profiles_from(tree.path()), "README.md");
}

/// A profile directory holds only PROFILE.toml, PROFILE.md, and (once
/// built) a corpus/ directory.
#[test]
fn stray_entry_in_a_profile_dir_is_a_load_error() {
    let tree = TempTree::new("profile-stray");
    let dir = write_profile_dir(tree.path(), "minimal", "scheduled");
    fs::write(dir.join("extra.txt"), "stray\n").expect("stray file written");
    expect_invalid(load_profiles_from(tree.path()), "extra.txt");
}

/// The declared corpus status must match the disk: `built` without a
/// corpus/ directory is a lie, and the loader rejects it.
#[test]
fn built_corpus_without_corpus_dir_is_a_load_error() {
    let tree = TempTree::new("corpus-built-missing");
    write_profile_dir(tree.path(), "minimal", "built");
    expect_invalid(load_profiles_from(tree.path()), "built");
}

/// The symmetric direction: a corpus/ directory on disk with the status
/// still `scheduled` is also a mismatch.
#[test]
fn scheduled_corpus_with_corpus_dir_is_a_load_error() {
    let tree = TempTree::new("corpus-scheduled-present");
    let dir = write_profile_dir(tree.path(), "minimal", "scheduled");
    fs::create_dir_all(dir.join("corpus")).expect("corpus dir created");
    expect_invalid(load_profiles_from(tree.path()), "scheduled");
}

/// And the consistent pairing loads: `built` with a corpus/ directory.
#[test]
fn built_corpus_with_corpus_dir_loads() {
    let tree = TempTree::new("corpus-built-present");
    let dir = write_profile_dir(tree.path(), "minimal", "built");
    fs::create_dir_all(dir.join("corpus")).expect("corpus dir created");
    let profiles = load_profiles_from(tree.path()).expect("consistent profile loads");
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].corpus, CorpusStatus::Built);
}

/// Prints the human-readable pending matrix. Run with
/// `cargo test -p dogtag-conformance -- --nocapture` (or `just conformance`)
/// to see it.
#[test]
fn print_pending_matrix() {
    let scenarios = load_scenarios().expect("scenarios load");
    let profiles = load_profiles().expect("profiles load");
    let pairs = report(&scenarios, &profiles).expect("the M1 report succeeds");
    let matrix = pending_matrix(&scenarios, &profiles, &pairs);

    // Sanity: every scenario and profile appears in the rendering.
    for scenario in &scenarios {
        assert!(matrix.contains(&scenario.id));
    }
    for profile in &profiles {
        assert!(matrix.contains(&profile.name));
    }
    assert!(!matrix.contains("MISSING"), "no cell is unaccounted for");

    println!("{matrix}");
}
