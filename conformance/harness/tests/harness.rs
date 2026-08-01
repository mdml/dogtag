//! Harness self-tests over the real fixtures: the scenario set and the profile
//! roster are what M2 says they are, graduation is all-or-nothing, and the
//! cross product reports each pair as the two facts about it make it.
//!
//! The strict-schema and strict-loader tests live beside this file in
//! `strictness.rs`, because they are about synthetic trees rather than about
//! the fixtures on disk.

use std::collections::BTreeSet;

use dogtag::contract::{Capability, Contract, Ordinary, load_contract};

use dogtag_conformance::{
    CORPORA_EVER_BUILT, CorpusStatus, Execution, Milestone, NoExecution, Outcome, Pair, Profile,
    REQUIRED_PROFILES, Scenario, ScenarioStatus, SdkExecution, graduated_case_count, load_profiles,
    load_scenarios, matrix, report,
};

#[test]
fn all_scenario_files_parse_with_unique_kebab_case_ids() {
    let scenarios = load_scenarios().expect("every scenario file parses and validates");
    assert_eq!(
        scenarios.len(),
        19,
        "every scenario file on disk is loaded; this count moves with the set"
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
        assert!(
            matches!(scenario.milestone, Milestone::M2 | Milestone::M3),
            "the scenario set covers M2-M3 (`{}` does not)",
            scenario.id
        );
    }
}

/// Graduation is all-or-nothing, so at M2 **every** scenario tagged `M2` is
/// executable and a straggler fails the suite. The M3 scenarios have not
/// graduated ahead of their milestone, and every executable scenario has a
/// case behind it — an executable scenario without one would refuse the whole
/// report, but saying so here names the fault instead.
#[test]
fn every_m2_scenario_has_graduated_and_nothing_has_graduated_early() {
    let scenarios = load_scenarios().expect("scenarios load");
    for scenario in &scenarios {
        let expected = match scenario.milestone {
            Milestone::M2 => ScenarioStatus::Executable,
            Milestone::M3 => ScenarioStatus::Pending,
        };
        assert_eq!(
            scenario.status, expected,
            "`{}` is tagged {} and must be {expected:?}: graduation is all-or-nothing",
            scenario.id, scenario.milestone
        );
    }

    let executable = scenarios
        .iter()
        .filter(|s| s.status == ScenarioStatus::Executable)
        .count();
    assert_eq!(
        executable, 10,
        "the ten M2 scenarios graduated together, all at once"
    );
    assert_eq!(
        graduated_case_count(),
        executable,
        "every graduated scenario has an execution path, and nothing else does"
    );
}

/// The floors the fixture record states for each built corpus.
///
/// The record spells these out and, until now, nothing read them. Both
/// fixtures met them the day they were written; what was missing was anything
/// that would notice them stopping. The absence-versus-named-value pair is the
/// load-bearing one — it is the reason the two profiles exist together, and it
/// is invisible to every other assertion in this suite because the cases adapt
/// to whichever encoding they find.
#[test]
fn each_built_corpus_meets_the_coverage_floor_its_record_states() {
    let dense = contract_of("dense");
    assert!(
        identity_bearing(&dense) >= 2,
        "dense: at least two identity-bearing types"
    );
    assert_eq!(catch_all(&dense), 1, "dense: exactly one catch-all");
    assert!(
        closed_write(&dense) >= 1,
        "dense: at least one closed-write"
    );
    assert!(predicates(&dense) >= 2, "dense: at least two predicates");
    assert!(
        required_predicates(&dense) >= 1,
        "dense: at least one required predicate"
    );
    assert!(
        matches!(ordinary_of(&dense), Some(Ordinary::Absent)),
        "dense: the ordinary state is absence"
    );
    assert_eq!(
        dense.dialect().links().as_str(),
        "wikilink",
        "dense: the wikilink dialect"
    );

    let starter = contract_of("starter");
    assert_eq!(catch_all(&starter), 1, "starter: exactly one catch-all");
    assert!(
        identity_bearing(&starter) >= 1,
        "starter: at least one identity-bearing type"
    );
    assert!(
        matches!(ordinary_of(&starter), Some(Ordinary::Value(_))),
        "starter: the ordinary state is a named value — the other half of the \
         seam axis the two profiles exist to span"
    );
}

/// A built profile's committed contract, loaded the way the SDK loads one.
fn contract_of(profile: &str) -> Contract {
    let path = dogtag_conformance::profiles_dir()
        .join(profile)
        .join("corpus/.dogtag/contract.toml");
    load_contract(&path)
        .contract
        .clone()
        .unwrap_or_else(|_| panic!("the {profile} corpus holds a contract that loads"))
}

fn with(contract: &Contract, capability: Capability) -> usize {
    contract
        .types()
        .iter()
        .filter(|declared| declared.has(capability))
        .count()
}

fn identity_bearing(contract: &Contract) -> usize {
    with(contract, Capability::IdentityBearing)
}

fn catch_all(contract: &Contract) -> usize {
    with(contract, Capability::CatchAll)
}

fn closed_write(contract: &Contract) -> usize {
    with(contract, Capability::ClosedWrite)
}

fn predicates(contract: &Contract) -> usize {
    contract
        .types()
        .iter()
        .map(|declared| declared.relationships().len())
        .sum()
}

fn required_predicates(contract: &Contract) -> usize {
    contract
        .types()
        .iter()
        .flat_map(|declared| declared.relationships())
        .filter(|relationship| relationship.required())
        .count()
}

fn ordinary_of(contract: &Contract) -> Option<&Ordinary> {
    contract.lifecycle().ordinary()
}

#[test]
fn profile_roster_is_exactly_the_beta_roster() {
    let profiles = load_profiles().expect("every profile parses and validates");
    let names: Vec<&str> = profiles.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names, REQUIRED_PROFILES,
        "conformance/profiles/ must hold exactly the four docs/beta.md profiles"
    );
    // A corpus is built exactly when the ratchet says it has ever been built.
    // At M2 that is `dense` and `starter`; `docs` and `records` are scheduled,
    // so M2's cross-profile evidence is two profiles rather than four.
    for profile in &profiles {
        let ever_built = CORPORA_EVER_BUILT.contains(&profile.name.as_str());
        let expected = if ever_built {
            CorpusStatus::Built
        } else {
            CorpusStatus::Scheduled
        };
        assert_eq!(
            profile.corpus,
            expected,
            "profile `{}` is {}named in CORPORA_EVER_BUILT",
            profile.name,
            if ever_built { "" } else { "not " }
        );
    }
}

/// Load the real fixtures and run the real report; the cross-product and
/// matrix tests all start from this triple.
fn m2_report() -> (Vec<Scenario>, Vec<Profile>, Vec<Pair>) {
    let scenarios = load_scenarios().expect("scenarios load");
    let profiles = load_profiles().expect("profiles load");
    let pairs = report(&scenarios, &profiles, &SdkExecution::in_repository())
        .expect("the M2 report succeeds");
    (scenarios, profiles, pairs)
}

#[test]
fn cross_product_is_complete() {
    let (scenarios, profiles, pairs) = m2_report();

    assert_eq!(
        pairs.len(),
        scenarios.len() * profiles.len(),
        "the report is the full cross product"
    );

    // Set equality carries both directions at once: no pair appears twice
    // (the set is as large as the report) and no pair is missing.
    let reported: BTreeSet<(&str, &str)> = pairs
        .iter()
        .map(|p| (p.scenario_id.as_str(), p.profile_name.as_str()))
        .collect();
    assert_eq!(
        reported.len(),
        pairs.len(),
        "no pair appears more than once"
    );
    let expected: BTreeSet<(&str, &str)> = scenarios
        .iter()
        .flat_map(|s| {
            profiles
                .iter()
                .map(move |p| (s.id.as_str(), p.name.as_str()))
        })
        .collect();
    assert_eq!(
        reported, expected,
        "the report holds exactly the scenarios x profiles pairs"
    );
}

/// Every pair's outcome follows from the two facts about it: an executable
/// scenario against a built corpus ran, and everything else is pending for
/// exactly the reasons that make it so.
#[test]
fn every_pair_reports_what_its_two_halves_make_it() {
    let (scenarios, profiles, pairs) = m2_report();
    let runnable = |pair: &Pair| {
        let scenario = scenarios
            .iter()
            .find(|s| s.id == pair.scenario_id)
            .expect("every pair names a loaded scenario");
        let profile = profiles
            .iter()
            .find(|p| p.name == pair.profile_name)
            .expect("every pair names a loaded profile");
        (
            scenario.status == ScenarioStatus::Pending,
            profile.corpus == CorpusStatus::Scheduled,
        )
    };

    let mut ran = 0;
    for pair in &pairs {
        let (scenario_pending, corpus_missing) = runnable(pair);
        match &pair.outcome {
            Outcome::Passed => ran += 1,
            Outcome::Failed { detail } => {
                panic!(
                    "({}, {}) failed: {detail}",
                    pair.scenario_id, pair.profile_name
                )
            }
            Outcome::Pending {
                scenario_pending: reported_scenario,
                corpus_missing: reported_corpus,
            } => {
                assert!(
                    scenario_pending || corpus_missing,
                    "({}, {}) is marked pending for no reason",
                    pair.scenario_id,
                    pair.profile_name
                );
                assert_eq!(
                    (*reported_scenario, *reported_corpus),
                    (scenario_pending, corpus_missing),
                    "({}, {}) reports the wrong reasons",
                    pair.scenario_id,
                    pair.profile_name
                );
            }
        }
    }
    assert_eq!(
        ran,
        10 * 2,
        "the ten graduated scenarios ran against the two built corpora"
    );
}

/// A scenario that has flipped to `executable` — the status that commits it
/// to run against every profile.
fn graduated_scenario() -> Scenario {
    Scenario {
        id: "graduated-scenario".to_string(),
        title: "A scenario that has flipped to executable".to_string(),
        milestone: Milestone::M2,
        status: ScenarioStatus::Executable,
        contract: "Given/when/then.".to_string(),
    }
}

/// A profile whose corpus is in the given state.
fn profile_with_corpus(name: &str, corpus: CorpusStatus) -> Profile {
    Profile {
        name: name.to_string(),
        persona: "a persona for cross-product tests".to_string(),
        distinguishing_axes: vec!["one axis".to_string()],
        corpus,
        corpus_milestone: "M2".to_string(),
    }
}

/// Graduation is all-or-nothing: the harness refuses to produce a report at
/// all when a pair is runnable but the scenario has no execution path, rather
/// than quietly calling it pending. An executor is not a filter, and this is
/// what stops one being used as one.
#[test]
fn runnable_pair_without_execution_path_is_refused() {
    let profile = profile_with_corpus("built-profile", CorpusStatus::Built);
    let err = report(&[graduated_scenario()], &[profile], &NoExecution)
        .expect_err("runnable pair must be refused");
    let message = err.to_string();
    assert!(
        message.contains("graduated-scenario") && message.contains("built-profile"),
        "the refusal names the pair: {message}"
    );
}

/// An execution path that answers for every pair, so the two ran outcomes are
/// reachable in a test without a corpus on disk.
struct Fixed(Result<(), String>);

impl Execution for Fixed {
    fn run(&self, _scenario: &Scenario, _profile: &Profile) -> Option<Result<(), String>> {
        Some(self.0.clone())
    }
}

/// A runnable pair that runs reports the result of running, not a pending
/// placeholder — in both directions.
#[test]
fn a_runnable_pair_reports_what_running_it_produced() {
    let profiles = [profile_with_corpus("built-profile", CorpusStatus::Built)];
    let passed = report(&[graduated_scenario()], &profiles, &Fixed(Ok(())))
        .expect("a pair with an execution path reports");
    assert_eq!(passed[0].outcome, Outcome::Passed);

    let failed = report(
        &[graduated_scenario()],
        &profiles,
        &Fixed(Err("the contract did not resolve".to_string())),
    )
    .expect("a failing pair is still a report, not an error");
    assert_eq!(
        failed[0].outcome,
        Outcome::Failed {
            detail: "the contract did not resolve".to_string()
        }
    );
}

/// Graduating a scenario is not on its own enough to make a pair runnable:
/// an `executable` scenario against a still-scheduled corpus is reported
/// pending, and pending for exactly one reason — the corpus, not the
/// scenario. That is the distinction the matrix has to keep visible.
#[test]
fn executable_scenario_with_a_scheduled_corpus_is_pending_on_the_corpus() {
    let profile = profile_with_corpus("scheduled-profile", CorpusStatus::Scheduled);
    let pairs = report(&[graduated_scenario()], &[profile], &NoExecution)
        .expect("a scheduled corpus is never runnable");
    assert_eq!(pairs.len(), 1, "one scenario times one profile is one pair");
    assert_eq!(
        pairs[0].outcome,
        Outcome::Pending {
            scenario_pending: false,
            corpus_missing: true,
        }
    );
}

/// Prints the human-readable matrix. Run with
/// `cargo test -p dogtag-conformance -- --nocapture` (or `just conformance`)
/// to see it.
#[test]
fn print_matrix() {
    let (scenarios, profiles, pairs) = m2_report();
    let rendered = matrix(&scenarios, &profiles, &pairs);
    // Printed before anything is asserted, so a run that goes red still shows
    // the matrix and the failure details it carries.
    println!("{rendered}");

    // Sanity: every scenario and profile appears in the rendering.
    for scenario in &scenarios {
        assert!(rendered.contains(&scenario.id));
    }
    for profile in &profiles {
        assert!(rendered.contains(&profile.name));
    }
    assert!(!rendered.contains("MISSING"), "no cell is unaccounted for");
    assert!(
        !rendered.contains("IMPOSSIBLE"),
        "no cell is pending for no reason"
    );
    // A skip and a result are different cells, so a run reaching two of four
    // profiles cannot read as a complete matrix. Asserted against the cells
    // rather than the whole rendering: `matrix` always appends a legend that
    // spells every cell word, so `rendered.contains("pass")` was true of a
    // matrix in which nothing had run.
    let body = rendered
        .split_once("\nlegend\n")
        .map_or(rendered.as_str(), |(body, _)| body);
    let cells: Vec<&str> = body
        .lines()
        .flat_map(|line| line.split_whitespace())
        .collect();
    assert!(
        cells.contains(&"pass"),
        "a pair that ran is a cell of its own: {body}"
    );
    assert!(
        cells.contains(&"no-corpus"),
        "and a pair skipped for an unbuilt corpus is a different one: {body}"
    );
    assert!(
        !pairs
            .iter()
            .any(|pair| matches!(pair.outcome, Outcome::Failed { .. })),
        "a failed pair makes the run red; its detail is printed above"
    );
}
