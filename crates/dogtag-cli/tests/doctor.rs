//! `dogtag doctor` against the built binary.
//!
//! Every run here is a child process whose home directory, configuration
//! directory and current directory are the test's, so nothing depends on the
//! machine the suite runs on.

mod support;

use support::{
    DENSE, Finished, STARTER, TOO_NEW, Tree, dogtag, registering, run, well_formed_json,
};

/// A tree holding a vault inside the home directory, with a record
/// registering it. Inside the home directory on purpose: a root outside it is
/// a warning, and these tests are about everything else.
fn vault(label: &str, contract: &str) -> Tree {
    let tree = Tree::new(label);
    let root = tree.vault("home/vault", contract);
    tree.record(&registering("work", &root));
    tree
}

/// `doctor` run from inside that vault.
fn doctor(tree: &Tree, arguments: &[&str]) -> Finished {
    run(dogtag(tree)
        .arg("doctor")
        .args(arguments)
        .current_dir(tree.home().join("vault")))
}

#[test]
fn a_clean_vault_reports_every_section_and_exits_0() {
    let tree = vault("clean", STARTER);
    let finished = doctor(&tree, &[]);
    assert_eq!(finished.code, 0, "{finished:?}");
    assert_eq!(finished.stderr, "", "a clean run says nothing on stderr");
    for line in [
        "upward discovery from the current directory",
        ".dogtag/contract.toml",
        "2 (current; supported 1..=2)",
        "3 declared",
        "catch-all          note",
        "lifecycle       axis \"status\", ordinary state is \"active\"",
        "links: wikilink",
        "$XDG_CONFIG_HOME/dogtag/installation.toml",
        "this vault is registered as \"work\"",
        "no diagnostics",
    ] {
        assert!(finished.stdout.contains(line), "no `{line}`: {finished:?}");
    }
    assert!(
        finished
            .stdout
            .contains(&tree.home().join("vault").display().to_string()),
        "the resolved root is the one fact every other line is about: {finished:?}"
    );
}

/// The dense fixture exercises the parts the starter one cannot: many types,
/// a lifecycle whose ordinary state is an absence, and flags.
#[test]
fn a_dense_vault_reports_its_own_shape() {
    let tree = vault("dense", DENSE);
    let finished = doctor(&tree, &[]);
    assert_eq!(finished.code, 0, "{finished:?}");
    assert!(
        finished
            .stdout
            .contains("ordinary state is the absence of a value"),
        "{finished:?}"
    );
    assert!(finished.stdout.contains("declared"), "{finished:?}");
}

#[test]
fn an_unusable_contract_still_reports_the_root_and_the_registry() {
    let tree = vault("too-new", TOO_NEW);
    let finished = doctor(&tree, &[]);
    assert_eq!(finished.code, 1, "an error is a failure: {finished:?}");
    let reason = "not evaluated (the contract declares a version above the supported range 1..=2)";
    for line in [
        "3 (too new; supported 1..=2)",
        "state         loaded",
        "types           not evaluated",
        "lifecycle       not evaluated",
        "dialect         not evaluated",
        reason,
        "error[compat.contract-too-new]",
    ] {
        assert!(finished.stdout.contains(line), "no `{line}`: {finished:?}");
    }
    assert!(
        finished
            .stdout
            .contains(&tree.home().join("vault").display().to_string()),
        "a vault that cannot be read is exactly when the root matters: {finished:?}"
    );
}

#[test]
fn the_json_report_is_one_document_carrying_the_settled_shape() {
    let tree = vault("json", STARTER);
    let finished = doctor(&tree, &["--format", "json"]);
    assert_eq!(finished.code, 0, "{finished:?}");
    assert!(
        well_formed_json(&finished.stdout),
        "not one JSON document: {finished:?}"
    );
    for fragment in [
        "\"schema_version\": 3",
        "\"report\": \"doctor\"",
        "\"how\": \"discovery\"",
        "\"requested\": null",
        "\"path\": \".dogtag/contract.toml\"",
        "\"state\": \"loaded\"",
        "\"classification\": \"current\"",
        "\"registry_entry\"",
        "\"sections\"",
        "\"evaluated\": true",
        "\"diagnostics\": []",
        "\"summary\"",
    ] {
        assert!(
            finished.stdout.contains(fragment),
            "no `{fragment}`: {finished:?}"
        );
    }
}

#[test]
fn an_unusable_contract_marks_each_json_section_not_evaluated() {
    let tree = vault("json-too-new", TOO_NEW);
    let finished = doctor(&tree, &["--format", "json"]);
    assert_eq!(finished.code, 1, "{finished:?}");
    assert!(well_formed_json(&finished.stdout), "{finished:?}");
    assert!(
        finished.stdout.contains("\"evaluated\": false"),
        "{finished:?}"
    );
    assert!(finished.stdout.contains("\"reason\":"), "{finished:?}");
    assert!(
        !finished.stdout.contains("\"evaluated\": true"),
        "one section cannot be evaluated when the contract did not resolve: {finished:?}"
    );
}

/// A root outside the home directory is a warning, which is the severity
/// `--strict` exists to promote.
#[test]
fn strict_promotes_a_warning_to_a_failure_and_changes_nothing_else() {
    let tree = Tree::new("strict");
    let root = tree.vault("outside/vault", STARTER);
    tree.record(&registering("work", &root));

    let ordinary = run(dogtag(&tree).arg("doctor").current_dir(&root));
    let strict = run(dogtag(&tree)
        .arg("doctor")
        .arg("--strict")
        .current_dir(&root));

    assert_eq!(ordinary.code, 0, "a warning is not a failure: {ordinary:?}");
    assert_eq!(strict.code, 1, "under strict it is: {strict:?}");
    assert_eq!(
        ordinary.stdout, strict.stdout,
        "strict changes the exit code and nothing else"
    );
    assert!(
        ordinary
            .stdout
            .contains("warning[discovery.root-outside-home]"),
        "{ordinary:?}"
    );
}

#[test]
fn doctor_writes_nothing_anywhere() {
    let tree = vault("read-only", STARTER);
    let before = tree.listing();
    let finished = doctor(&tree, &[]);
    assert_eq!(finished.code, 0, "{finished:?}");
    assert_eq!(
        tree.listing(),
        before,
        "doctor opens two files and creates none"
    );
}

#[test]
fn a_vault_that_cannot_be_resolved_is_still_reported_on() {
    // `doctor` never refuses. A selection that named nothing is exactly when a
    // reader most needs what is known — whether a record exists, what it
    // declares, what was looked for — and a `--format json` consumer parsing
    // this stream during the parallel run must get a document either way.
    let tree = Tree::new("no-vault");
    support::assert_no_vault_above(&tree.home());
    let finished = run(dogtag(&tree).arg("doctor"));
    assert_eq!(finished.code, 1, "{finished:?}");
    assert!(finished.stdout.contains("none resolved"), "{finished:?}");
    assert!(
        finished.stdout.contains("error[discovery.no-vault-found]"),
        "{finished:?}"
    );
}

#[test]
fn a_vault_that_cannot_be_resolved_still_yields_a_parseable_document() {
    let tree = Tree::new("no-vault-json");
    support::assert_no_vault_above(&tree.home());
    let finished = run(dogtag(&tree).arg("doctor").args(["--format", "json"]));
    assert_eq!(finished.code, 1, "{finished:?}");
    assert!(
        support::well_formed_json(&finished.stdout),
        "a consumer parses one shape whether or not a vault resolved: {finished:?}"
    );
    assert!(finished.stdout.contains("\"root\": null"), "{finished:?}");
    assert!(
        finished.stdout.contains("\"evaluated\": false"),
        "{finished:?}"
    );
}

#[test]
fn the_record_is_read_from_the_configuration_directory_the_environment_names() {
    let tree = vault("xdg", STARTER);
    let elsewhere = Tree::new("xdg-elsewhere");
    let finished = run(dogtag(&tree)
        .arg("doctor")
        .env("XDG_CONFIG_HOME", elsewhere.config())
        .current_dir(tree.home().join("vault")));
    assert_eq!(finished.code, 0, "{finished:?}");
    assert!(
        finished.stdout.contains("state         absent"),
        "the record is read from where the environment says, not from the home directory: \
         {finished:?}"
    );
}

#[test]
fn without_that_variable_the_record_sits_under_dot_config() {
    let tree = vault("default-config", STARTER);
    let finished = run(dogtag(&tree)
        .arg("doctor")
        .env_remove("XDG_CONFIG_HOME")
        .current_dir(tree.home().join("vault")));
    assert_eq!(finished.code, 0, "{finished:?}");
    assert!(
        finished
            .stdout
            .contains("this vault is registered as \"work\""),
        "{finished:?}"
    );
}

#[test]
fn a_machine_with_no_home_directory_reports_an_absent_record() {
    let tree = vault("no-home", STARTER);
    let finished = run(dogtag(&tree)
        .arg("doctor")
        .env_remove("HOME")
        .env_remove("XDG_CONFIG_HOME")
        .current_dir(tree.home().join("vault")));
    assert_eq!(
        finished.code, 0,
        "absence is a state, not a fault: {finished:?}"
    );
    assert!(
        finished.stdout.contains("state         absent"),
        "{finished:?}"
    );
    assert!(
        !finished.stdout.contains("discovery.root-outside-home"),
        "with no home directory there is nothing to judge the root against: {finished:?}"
    );
}

/// The report carries diagnostics, so it is the one stdout rendering colour
/// can reach — and a piped run reaches it with none, however `NO_COLOR` is set.
#[test]
fn the_text_report_carries_no_escape_sequence_when_it_is_piped() {
    let tree = vault("colour", TOO_NEW);
    let suppressed = run(dogtag(&tree)
        .arg("doctor")
        .env("NO_COLOR", "1")
        .current_dir(tree.home().join("vault")));
    let ordinary = doctor(&tree, &[]);
    assert!(!suppressed.has_escapes(), "{suppressed:?}");
    assert!(!ordinary.has_escapes(), "{ordinary:?}");
    assert_eq!(
        suppressed.stdout, ordinary.stdout,
        "colour changes escape sequences and nothing else"
    );
}

#[test]
fn an_argument_the_parser_refuses_exits_2_with_no_diagnostic() {
    let tree = vault("usage", STARTER);
    let finished = doctor(&tree, &["--format", "bogus"]);
    assert_eq!(
        finished.code, 2,
        "2 is reserved for an argument that produces no diagnostic: {finished:?}"
    );
    assert_eq!(finished.stdout, "", "{finished:?}");
    assert!(finished.stderr.contains("--format"), "{finished:?}");
}
