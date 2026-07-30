//! Harness self-tests: the fixtures parse, the roster is exact, the cross
//! product is complete, and — the load-bearing one — waiver-shaped fields
//! cannot exist in the schema.

use std::collections::BTreeSet;

use dogtag_conformance::{
    CorpusStatus, Milestone, Outcome, Profile, REQUIRED_PROFILES, Scenario, ScenarioStatus,
    load_profiles, load_scenarios, parse_profile, parse_scenario, pending_matrix, report,
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
