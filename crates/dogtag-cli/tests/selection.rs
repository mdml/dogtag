//! Which vault a run resolves, and how it says so.
//!
//! Five routes exist — the flag and the environment variable each carrying
//! either a path or a registry name, and upward discovery — and every one of
//! them is reported. The rule that keeps them apart is syntactic and has no
//! fallback: an argument holding a separator or opening with `.`, `/` or `~`
//! is always a path, and any other argument is always a registry name.

mod support;

use support::{Finished, STARTER, Tree, dogtag, registering, run};

/// A tree whose vault sits inside the home directory and is registered as
/// `work`.
fn registered(label: &str) -> Tree {
    let tree = Tree::new(label);
    let root = tree.vault("home/vault", STARTER);
    tree.record(&registering("work", &root));
    tree
}

/// The line naming the route, taken from the report.
fn selected_by(finished: &Finished) -> String {
    finished
        .stdout
        .lines()
        .find_map(|line| line.trim_start().strip_prefix("selected by"))
        .map(|route| route.trim().to_owned())
        .unwrap_or_else(|| panic!("no route in the report: {finished:?}"))
}

/// A refusal, held up against everything a refusal has to be: exit `1`,
/// nothing at all on standard output, and every fragment named on standard
/// error.
fn assert_refused(finished: &Finished, fragments: &[&str]) {
    assert_eq!(
        (finished.code, finished.stdout.as_str()),
        (1, ""),
        "{finished:?}"
    );
    for fragment in fragments {
        assert!(
            finished.stderr.contains(fragment),
            "`{fragment}` is missing from the refusal: {finished:?}"
        );
    }
}

#[test]
fn discovery_resolves_the_vault_the_current_directory_is_inside() {
    let tree = registered("route-discovery");
    tree.dir("home/vault/notes/deep");
    let finished = run(dogtag(&tree)
        .arg("doctor")
        .current_dir(tree.home().join("vault/notes/deep")));
    assert_eq!(finished.code, 0, "{finished:?}");
    assert_eq!(
        selected_by(&finished),
        "upward discovery from the current directory"
    );
    assert!(
        finished
            .stdout
            .contains(&tree.home().join("vault").display().to_string()),
        "the nearest root wins: {finished:?}"
    );
}

#[test]
fn a_flag_carrying_a_path_is_used_exactly() {
    let tree = registered("route-flag-path");
    let root = tree.home().join("vault");
    let finished = run(dogtag(&tree).arg("doctor").arg("--vault").arg(&root));
    assert_eq!(finished.code, 0, "{finished:?}");
    assert_eq!(
        selected_by(&finished),
        format!("--vault {}", root.display())
    );
}

#[test]
fn a_relative_argument_is_a_path_because_it_opens_with_a_dot() {
    let tree = registered("route-relative");
    let finished = run(dogtag(&tree)
        .arg("doctor")
        .args(["--vault", "./vault"])
        .current_dir(tree.home()));
    assert_eq!(finished.code, 0, "{finished:?}");
    assert_eq!(selected_by(&finished), "--vault ./vault");
}

#[test]
fn a_flag_carrying_a_bare_name_is_a_registry_lookup() {
    let tree = registered("route-flag-name");
    let finished = run(dogtag(&tree).arg("doctor").args(["--vault", "work"]));
    assert_eq!(finished.code, 0, "{finished:?}");
    assert_eq!(selected_by(&finished), "--vault work (registered)");
}

#[test]
fn the_environment_carries_the_same_two_routes() {
    let tree = registered("route-environment");
    let root = tree.home().join("vault");
    let by_path = run(dogtag(&tree).arg("doctor").env("DOGTAG_VAULT", &root));
    let by_name = run(dogtag(&tree).arg("doctor").env("DOGTAG_VAULT", "work"));
    assert_eq!(by_path.code, 0, "{by_path:?}");
    assert_eq!(by_name.code, 0, "{by_name:?}");
    assert_eq!(
        selected_by(&by_path),
        format!("DOGTAG_VAULT {}", root.display())
    );
    assert_eq!(selected_by(&by_name), "DOGTAG_VAULT work (registered)");
}

#[test]
fn the_flag_outranks_the_environment() {
    let tree = registered("route-order");
    let finished = run(dogtag(&tree)
        .arg("doctor")
        .args(["--vault", "work"])
        .env("DOGTAG_VAULT", "/nowhere/at/all"));
    assert_eq!(finished.code, 0, "{finished:?}");
    assert_eq!(selected_by(&finished), "--vault work (registered)");
}

#[test]
fn the_route_reaches_the_structured_report_too() {
    let tree = registered("route-json");
    let finished = run(dogtag(&tree)
        .arg("doctor")
        .args(["--format", "json", "--vault", "work"]));
    assert_eq!(finished.code, 0, "{finished:?}");
    assert!(
        finished.stdout.contains("\"how\": \"flag-name\""),
        "{finished:?}"
    );
    assert!(
        finished.stdout.contains("\"requested\": \"work\""),
        "the argument is reported exactly as it was given: {finished:?}"
    );
}

#[test]
fn an_unregistered_name_refuses_and_teaches_the_correction() {
    let tree = registered("unregistered");
    let finished = run(dogtag(&tree).arg("doctor").args(["--vault", "notes"]));
    assert_eq!(
        finished.code, 1,
        "an installation-area diagnostic is a failure, not a usage error: {finished:?}"
    );
    assert_eq!(finished.stdout, "", "{finished:?}");
    assert!(
        finished
            .stderr
            .contains("error[installation.unknown-vault-name]"),
        "{finished:?}"
    );
    assert!(
        finished.stderr.contains("./notes"),
        "the correction is not guessable, so the refusal teaches it: {finished:?}"
    );
}

#[test]
fn a_name_on_a_machine_with_no_record_refuses_rather_than_falling_back() {
    let tree = registered("no-record");
    let finished = run(dogtag(&tree)
        .arg("doctor")
        .args(["--vault", "vault"])
        .env_remove("HOME")
        .env_remove("XDG_CONFIG_HOME")
        .current_dir(tree.home()));
    // The kernel answers this state, so no surface mints an identifier for
    // it, and a bare name never means the directory beside it.
    assert_refused(
        &finished,
        &[
            "error[installation.unknown-vault-name]",
            "no installation record exists at all",
            "./vault",
        ],
    );
}

#[test]
fn a_name_against_a_record_that_did_not_load_refuses_under_the_same_identifier() {
    let tree = registered("unusable-record");
    tree.record("installation_version = 1\nstray = true\n");
    let finished = run(dogtag(&tree).arg("doctor").args(["--vault", "work"]));
    // The refusal points at the diagnostics reading the record reported, so
    // they are reported beside it rather than promised and withheld.
    assert_refused(
        &finished,
        &[
            "error[installation.unknown-vault-name]",
            "did not load",
            "error[installation.unknown-key]",
        ],
    );
}

/// No surface may mint an identifier for something the kernel models. The
/// selection failures are the place that pressure showed up, so the whole
/// stream is held up against the namespace rather than one message.
#[test]
fn no_selection_refusal_carries_a_consumer_identifier() {
    let tree = registered("no-consumer-identifier");
    tree.record("installation_version = 1\nstray = true\n");
    let refusals = [
        run(dogtag(&tree).arg("doctor").args(["--vault", "work"])),
        run(dogtag(&tree)
            .arg("doctor")
            .args(["--vault", "notes"])
            .env_remove("HOME")
            .env_remove("XDG_CONFIG_HOME")),
        run(dogtag(&tree)
            .arg("contract")
            .arg("explain")
            .args(["--vault", "notes"])),
    ];
    for finished in &refusals {
        assert_eq!(finished.code, 1, "{finished:?}");
        assert!(!finished.stderr.contains("ext."), "{finished:?}");
    }
}

#[test]
fn a_path_argument_is_never_searched_upward() {
    let tree = registered("exact");
    tree.dir("home/vault/notes");
    let finished = run(dogtag(&tree)
        .arg("doctor")
        .arg("--vault")
        .arg(tree.home().join("vault/notes")));
    assert_eq!(finished.code, 1, "{finished:?}");
    assert_eq!(finished.stdout, "", "{finished:?}");
    assert!(
        finished
            .stderr
            .contains("error[discovery.not-a-vault-root]"),
        "the vault one directory above must not be resolved: {finished:?}"
    );
}

#[test]
fn a_tilde_makes_an_argument_a_path_and_is_not_expanded() {
    let tree = registered("tilde");
    let finished = run(dogtag(&tree).arg("doctor").args(["--vault", "~/vault"]));
    assert_eq!(finished.code, 1, "{finished:?}");
    assert!(
        finished.stderr.contains("error[discovery.path-unreadable]"),
        "expansion is the shell's job: the path is used as written: {finished:?}"
    );
    assert!(finished.stderr.contains("~/vault"), "{finished:?}");
}

#[test]
fn an_ancestor_vault_is_reported_without_changing_what_was_resolved() {
    let tree = Tree::new("nested");
    tree.vault("home/outer", STARTER);
    let inner = tree.vault("home/outer/inner", STARTER);
    let finished = run(dogtag(&tree).arg("doctor").current_dir(&inner));
    assert_eq!(finished.code, 0, "nesting is legal: {finished:?}");
    assert!(
        finished
            .stdout
            .contains(&format!("root          {}", inner.display())),
        "the nearest root wins: {finished:?}"
    );
    assert!(
        finished.stdout.contains("warning[discovery.nested-vault]"),
        "and the ancestor is named: {finished:?}"
    );
}
