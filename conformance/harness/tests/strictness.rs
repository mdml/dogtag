//! Strict-schema and strict-loader self-tests, over synthetic trees.
//!
//! The load-bearing pair is the waiver rejection: a scenario has no field with
//! which to name a profile and a profile has none with which to name a
//! scenario, and `deny_unknown_fields` turns either into a parse failure. The
//! rest is the filesystem half of the same rule — a stray entry, a mismatched
//! name, a corpus status that disagrees with the disk, and a built corpus that
//! tried to go back to scheduled are all load errors rather than something
//! silently skipped.

use std::fs;
use std::path::{Path, PathBuf};

use dogtag_conformance::{
    CorpusStatus, HarnessError, TempTree, load_profiles_from, load_scenarios_from, parse_profile,
    parse_scenario,
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

/// The ratchet: a corpus that has been built never returns to `scheduled`.
///
/// The loader's other checks are disk consistency only, so deleting a corpus
/// directory and reverting the status would otherwise be a mechanically valid
/// way to make a failing profile stop failing — which removes it from every
/// scenario at once. The synthetic profile here is named `dense` precisely
/// because that is a name the ratchet knows; the other loader tests use names
/// it does not, so they are unaffected.
#[test]
fn a_built_corpus_may_not_return_to_scheduled() {
    let tree = TempTree::new("corpus-ratchet");
    write_profile_dir(&ProfileDoc::valid("dense"), tree.path());
    expect_invalid(
        load_profiles_from(tree.path()),
        "never returns to scheduled",
    );
    expect_invalid(load_profiles_from(tree.path()), "CORPORA_EVER_BUILT");
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
