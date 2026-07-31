//! Harness self-tests: the fixtures parse, the roster is exact, the cross
//! product is complete, and — the load-bearing one — waiver-shaped fields
//! cannot exist in the schema.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use dogtag_conformance::{
    CorpusStatus, HarnessError, Milestone, Outcome, Pair, Profile, REQUIRED_PROFILES, Scenario,
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
        "conformance/profiles/ must hold exactly the four docs/beta.md profiles"
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

/// Load the real fixtures and compute the M1 report; the cross-product and
/// matrix tests all start from this triple.
fn m1_report() -> (Vec<Scenario>, Vec<Profile>, Vec<Pair>) {
    let scenarios = load_scenarios().expect("scenarios load");
    let profiles = load_profiles().expect("profiles load");
    let pairs = report(&scenarios, &profiles).expect("the M1 report succeeds");
    (scenarios, profiles, pairs)
}

#[test]
fn cross_product_is_complete() {
    let (scenarios, profiles, pairs) = m1_report();

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

#[test]
fn all_pairs_pending_at_m1() {
    let (_scenarios, _profiles, pairs) = m1_report();
    for pair in &pairs {
        // Irrefutable at M1: Pending is the only outcome variant until the
        // execution path lands.
        let Outcome::Pending {
            scenario_pending,
            corpus_missing,
        } = pair.outcome;
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
/// all when a pair is runnable but execution is not wired, rather than
/// quietly calling it pending.
#[test]
fn runnable_pair_without_execution_path_is_refused() {
    let profile = profile_with_corpus("built-profile", CorpusStatus::Built);
    let err =
        report(&[graduated_scenario()], &[profile]).expect_err("runnable pair must be refused");
    let message = err.to_string();
    assert!(
        message.contains("graduated-scenario") && message.contains("built-profile"),
        "the refusal names the pair: {message}"
    );
}

/// Graduating a scenario is not on its own enough to make a pair runnable:
/// an `executable` scenario against a still-scheduled corpus is reported
/// pending, and pending for exactly one reason — the corpus, not the
/// scenario. Only when both halves are ready does the refusal above fire.
#[test]
fn executable_scenario_with_a_scheduled_corpus_is_pending_on_the_corpus() {
    let profile = profile_with_corpus("scheduled-profile", CorpusStatus::Scheduled);
    let pairs =
        report(&[graduated_scenario()], &[profile]).expect("a scheduled corpus is never runnable");
    assert_eq!(pairs.len(), 1, "one scenario times one profile is one pair");
    let Outcome::Pending {
        scenario_pending,
        corpus_missing,
    } = pairs[0].outcome;
    assert!(
        !scenario_pending,
        "the scenario has graduated to executable"
    );
    assert!(corpus_missing, "the corpus is still scheduled");
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

/// A scenario document and the filename to write it under, field by field,
/// so a test can spoil exactly one field-level rule with `..valid(id)` and
/// leave the rest satisfied.
struct ScenarioDoc<'a> {
    /// The file's basename without `.toml`; the loader requires `id == stem`.
    stem: &'a str,
    id: &'a str,
    title: &'a str,
    contract: &'a str,
}

impl<'a> ScenarioDoc<'a> {
    /// A document that satisfies every rule the loader enforces.
    fn valid(id: &'a str) -> Self {
        ScenarioDoc {
            stem: id,
            id,
            title: "A minimal scenario for schema tests",
            contract: "Given a corpus. When nothing happens. Then nothing is reported.",
        }
    }

    /// Render and write the document into a scenarios directory.
    fn write_into(&self, dir: &Path) {
        let body = format!(
            "id = \"{}\"\ntitle = \"{}\"\nmilestone = \"M2\"\n\
             status = \"pending\"\ncontract = \"{}\"\n",
            self.id, self.title, self.contract
        );
        fs::write(dir.join(format!("{}.toml", self.stem)), body).expect("scenario written");
    }
}

/// The profile counterpart of [`ScenarioDoc`]: `dirname` is the profile
/// directory, which the loader requires to equal `name`.
struct ProfileDoc<'a> {
    dirname: &'a str,
    name: &'a str,
    persona: &'a str,
    axes: &'a str,
    corpus: &'a str,
    milestone: &'a str,
}

impl<'a> ProfileDoc<'a> {
    /// A document that satisfies every rule the loader enforces.
    fn valid(name: &'a str) -> Self {
        ProfileDoc {
            dirname: name,
            name,
            persona: "a strictness-test persona",
            axes: "[\"one axis\"]",
            corpus: "scheduled",
            milestone: "M2",
        }
    }

    /// Render and write the document as `<dirname>/PROFILE.toml`, returning
    /// the profile directory.
    fn write_into(&self, profiles_dir: &Path) -> PathBuf {
        let dir = profiles_dir.join(self.dirname);
        fs::create_dir_all(&dir).expect("profile dir created");
        let body = format!(
            "name = \"{}\"\npersona = \"{}\"\n\
             distinguishing_axes = {}\ncorpus = \"{}\"\n\
             corpus_milestone = \"{}\"\n",
            self.name, self.persona, self.axes, self.corpus, self.milestone
        );
        fs::write(dir.join("PROFILE.toml"), body).expect("PROFILE.toml written");
        dir
    }
}

/// A profile document declaring `corpus = "built"` — the status the loader
/// requires a `corpus/` directory on disk to back.
fn built_profile_doc() -> ProfileDoc<'static> {
    ProfileDoc {
        corpus: "built",
        ..ProfileDoc::valid("minimal")
    }
}

/// A complete profile directory: `PROFILE.toml` plus the `PROFILE.md` the
/// loader also permits.
fn write_profile_dir(doc: &ProfileDoc<'_>, profiles_dir: &Path) -> PathBuf {
    let dir = doc.write_into(profiles_dir);
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

/// Assert a load failed with [`HarnessError::Io`] at `path`, and that the
/// rendering names both the path being read and the cause the operating
/// system gave — an unlabelled "io error" would not be actionable.
fn expect_io<T: std::fmt::Debug>(result: Result<T, HarnessError>, path: &Path) {
    let err = result.expect_err("a filesystem failure must not load");
    match &err {
        HarnessError::Io(at, cause) => {
            assert_eq!(at, path, "the io error names the path it was reading");
            let message = err.to_string();
            assert!(
                message.starts_with(&format!("io error at {}: ", path.display())),
                "the rendering leads with the path: {message}"
            );
            assert!(
                message.ends_with(&cause.to_string()),
                "the rendering carries the cause `{cause}`: {message}"
            );
        }
        other => panic!(
            "expected HarnessError::Io at {}, got {other:?}",
            path.display()
        ),
    }
}

/// The [`HarnessError::Parse`] counterpart of [`expect_io`]: the rendering
/// names the offending file and repeats the TOML parser's own complaint.
fn expect_parse<T: std::fmt::Debug>(result: Result<T, HarnessError>, path: &Path) {
    let err = result.expect_err("malformed TOML must not load");
    match &err {
        HarnessError::Parse(at, cause) => {
            assert_eq!(at, path, "the parse error names the file it was parsing");
            let message = err.to_string();
            assert!(
                message.starts_with(&format!("parse error at {}: ", path.display())),
                "the rendering leads with the path: {message}"
            );
            assert!(
                message.ends_with(&cause.to_string()),
                "the rendering carries the parser's complaint: {message}"
            );
        }
        other => panic!(
            "expected HarnessError::Parse at {}, got {other:?}",
            path.display()
        ),
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
    write_profile_dir(&ProfileDoc::valid("minimal"), tree.path());
    fs::write(tree.path().join("README.md"), "stray\n").expect("stray file written");
    expect_invalid(load_profiles_from(tree.path()), "README.md");
}

/// A profile directory holds only PROFILE.toml, PROFILE.md, and (once
/// built) a corpus/ directory.
#[test]
fn stray_entry_in_a_profile_dir_is_a_load_error() {
    let tree = TempTree::new("profile-stray");
    let dir = write_profile_dir(&ProfileDoc::valid("minimal"), tree.path());
    fs::write(dir.join("extra.txt"), "stray\n").expect("stray file written");
    expect_invalid(load_profiles_from(tree.path()), "extra.txt");
}

/// The declared corpus status must match the disk: `built` without a
/// corpus/ directory is a lie, and the loader rejects it.
#[test]
fn built_corpus_without_corpus_dir_is_a_load_error() {
    let tree = TempTree::new("corpus-built-missing");
    write_profile_dir(&built_profile_doc(), tree.path());
    expect_invalid(load_profiles_from(tree.path()), "built");
}

/// The symmetric direction: a corpus/ directory on disk with the status
/// still `scheduled` is also a mismatch.
#[test]
fn scheduled_corpus_with_corpus_dir_is_a_load_error() {
    let tree = TempTree::new("corpus-scheduled-present");
    let dir = write_profile_dir(&ProfileDoc::valid("minimal"), tree.path());
    fs::create_dir_all(dir.join("corpus")).expect("corpus dir created");
    expect_invalid(load_profiles_from(tree.path()), "scheduled");
}

/// And the consistent pairing loads: `built` with a corpus/ directory.
#[test]
fn built_corpus_with_corpus_dir_loads() {
    let tree = TempTree::new("corpus-built-present");
    let dir = write_profile_dir(&built_profile_doc(), tree.path());
    fs::create_dir_all(dir.join("corpus")).expect("corpus dir created");
    let profiles = load_profiles_from(tree.path()).expect("consistent profile loads");
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].corpus, CorpusStatus::Built);
}

/// Every place the loaders touch the disk reports its failure as
/// [`HarnessError::Io`] rather than skipping the entry: a missing directory,
/// a fixture file whose bytes are not UTF-8, and a profile directory with no
/// `PROFILE.toml` at all.
#[test]
fn filesystem_failures_are_io_errors_naming_the_path_and_cause() {
    let missing = TempTree::new("io-missing-dir");
    let absent = missing.path().join("not-created");
    expect_io(load_scenarios_from(&absent), &absent);

    let binary = TempTree::new("io-not-utf8");
    let file = binary.path().join("minimal-scenario.toml");
    fs::write(&file, b"id = \"minimal-scenario\"\n\xff\xfe\n").expect("scenario written");
    expect_io(load_scenarios_from(binary.path()), &file);

    let headless = TempTree::new("io-no-profile-toml");
    let dir = headless.path().join("minimal");
    fs::create_dir_all(&dir).expect("profile dir created");
    fs::write(dir.join("PROFILE.md"), "# specified, not declared\n").expect("PROFILE.md written");
    expect_io(
        load_profiles_from(headless.path()),
        &dir.join("PROFILE.toml"),
    );
}

/// Malformed TOML is a [`HarnessError::Parse`] on both fixture kinds, naming
/// the file and repeating the parser's complaint — the strict schemas'
/// rejections stay diagnosable rather than collapsing into "did not load".
#[test]
fn malformed_toml_is_a_parse_error_naming_the_path_and_cause() {
    let scenarios = TempTree::new("parse-scenario");
    let file = scenarios.path().join("minimal-scenario.toml");
    fs::write(&file, "id = \n").expect("scenario written");
    expect_parse(load_scenarios_from(scenarios.path()), &file);

    let profiles = TempTree::new("parse-profile");
    let dir = profiles.path().join("minimal");
    fs::create_dir_all(&dir).expect("profile dir created");
    fs::write(dir.join("PROFILE.toml"), "name = \n").expect("PROFILE.toml written");
    expect_parse(
        load_profiles_from(profiles.path()),
        &dir.join("PROFILE.toml"),
    );
}

/// [`HarnessError::Invalid`] renders with an `invalid fixture:` label and
/// its message verbatim, so a refusal is still readable when it surfaces as
/// a plain error string rather than a matched variant.
#[test]
fn invalid_fixture_errors_render_with_a_label_and_their_message() {
    let tree = TempTree::new("invalid-display");
    fs::write(tree.path().join("NOTES.md"), "stray\n").expect("stray file written");
    let message = load_scenarios_from(tree.path())
        .expect_err("a stray entry must not load")
        .to_string();
    assert!(
        message.starts_with("invalid fixture: "),
        "the rendering is labelled: {message}"
    );
    assert!(
        message.contains("NOTES.md"),
        "the rendering names the offending entry: {message}"
    );
}

/// Each field-level scenario rule refuses on its own terms: the id must
/// equal the filename stem, the id must be kebab-case, and the two
/// human-facing fields must carry something other than whitespace. Every
/// document below is valid TOML under the strict schema and breaks exactly
/// one rule, so each message identifies the rule it broke.
#[test]
fn each_scenario_field_rule_refuses_with_its_own_message() {
    let cases = [
        (
            ScenarioDoc {
                stem: "renamed-scenario",
                ..ScenarioDoc::valid("other-id")
            },
            "does not match filename stem",
        ),
        (
            ScenarioDoc::valid("not_kebab_scenario"),
            "is not kebab-case",
        ),
        (
            ScenarioDoc {
                title: "   ",
                ..ScenarioDoc::valid("blank-title")
            },
            "has an empty title",
        ),
        (
            ScenarioDoc {
                contract: "  ",
                ..ScenarioDoc::valid("blank-contract")
            },
            "has an empty contract",
        ),
    ];
    for (doc, needle) in cases {
        let tree = TempTree::new(doc.stem);
        doc.write_into(tree.path());
        expect_invalid(load_scenarios_from(tree.path()), needle);
    }
}

/// The profile side of the same contract: name matches the directory, name
/// is kebab-case, persona and corpus_milestone say something, and a profile
/// that stresses no axis is a fixture nothing specified.
#[test]
fn each_profile_field_rule_refuses_with_its_own_message() {
    let cases = [
        (
            ProfileDoc {
                dirname: "renamed-profile",
                ..ProfileDoc::valid("other-name")
            },
            "does not match directory name",
        ),
        (ProfileDoc::valid("not_kebab_profile"), "is not kebab-case"),
        (
            ProfileDoc {
                persona: "   ",
                ..ProfileDoc::valid("blank-persona")
            },
            "has an empty persona",
        ),
        (
            ProfileDoc {
                axes: "[]",
                ..ProfileDoc::valid("no-axes")
            },
            "declares no distinguishing axes",
        ),
        (
            ProfileDoc {
                milestone: "  ",
                ..ProfileDoc::valid("blank-milestone")
            },
            "has an empty corpus_milestone",
        ),
    ];
    for (doc, needle) in cases {
        let tree = TempTree::new(doc.dirname);
        doc.write_into(tree.path());
        expect_invalid(load_profiles_from(tree.path()), needle);
    }
}

/// Prints the human-readable pending matrix. Run with
/// `cargo test -p dogtag-conformance -- --nocapture` (or `just conformance`)
/// to see it.
#[test]
fn print_pending_matrix() {
    let (scenarios, profiles, pairs) = m1_report();
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
