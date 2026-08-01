//! `dogtag contract explain` against the built binary.
//!
//! The Markdown is the generated agent contract, so these tests assert what an
//! agent receives: the vault it is about, every declaration the contract
//! makes, and — when the contract did not resolve — nothing at all.

mod support;

use support::{
    DENSE, Finished, STARTER, TOO_NEW, Tree, dogtag, registering, run, well_formed_json,
};

/// A tree whose vault sits inside the home directory and is registered.
fn vault(label: &str, contract: &str) -> Tree {
    let tree = Tree::new(label);
    let root = tree.vault("home/vault", contract);
    tree.record(&registering("work", &root));
    tree
}

/// `contract explain` run from inside that vault.
fn explain(tree: &Tree, arguments: &[&str]) -> Finished {
    run(dogtag(tree)
        .args(["contract", "explain"])
        .args(arguments)
        .current_dir(tree.home().join("vault")))
}

#[test]
fn the_markdown_names_the_vault_and_renders_every_declaration() {
    let tree = vault("markdown", STARTER);
    let finished = explain(&tree, &[]);
    assert_eq!(finished.code, 0, "{finished:?}");
    assert_eq!(finished.stderr, "", "{finished:?}");
    assert!(
        finished
            .stdout
            .contains(&format!("`{}`", tree.home().join("vault").display())),
        "an agent reading piped output receives the provenance with the instructions: {finished:?}"
    );
    for fragment in [
        "# Vault contract",
        "## Types",
        "### `note` — catch-all",
        "### `person` — identity-bearing",
        "| `full_name` | string | yes |",
        "enum (`active`, `archived`)",
        "list of string",
        "## Lifecycle",
        "## Flags",
        "## Dialect",
        "wikilink",
    ] {
        assert!(
            finished.stdout.contains(fragment),
            "no `{fragment}`: {finished:?}"
        );
    }
}

#[test]
fn a_contract_with_no_flags_says_so_rather_than_omitting_the_section() {
    let tree = vault("no-flags", STARTER);
    let finished = explain(&tree, &[]);
    assert!(
        finished.stdout.contains("This contract declares no flags."),
        "{finished:?}"
    );
}

#[test]
fn a_dense_contract_renders_its_flags_and_relationships() {
    let tree = vault("dense-markdown", DENSE);
    let finished = explain(&tree, &[]);
    assert_eq!(finished.code, 0, "{finished:?}");
    for fragment in [
        "`needs_rework` — a boolean property, orthogonal to the life axis.",
        "| relationship | required |",
        "The life axis is the property `standing`.",
    ] {
        assert!(
            finished.stdout.contains(fragment),
            "no `{fragment}`: {finished:?}"
        );
    }
}

#[test]
fn provenance_is_opt_in_and_annotates_nothing_without_the_flag() {
    let tree = vault("provenance", STARTER);
    let without = explain(&tree, &[]);
    let with = explain(&tree, &["--provenance"]);
    assert_eq!(with.code, 0, "{with:?}");
    assert!(
        !without.stdout.contains("(contract)") && !without.stdout.contains("(default"),
        "with the flag off, no annotation appears anywhere: {without:?}"
    );
    assert!(
        with.stdout
            .contains("- `dialect.links` — `.dogtag/contract.toml:"),
        "{with:?}"
    );
    assert!(
        with.stdout.contains("(contract)"),
        "a declared value names where it was written: {with:?}"
    );
    assert!(
        with.stdout.contains("(default, contract version 1)"),
        "a defaulted value names the contract version that defines it: {with:?}"
    );
    assert!(
        with.stdout.len() > without.stdout.len(),
        "annotations are additions to the same rendering"
    );
}

#[test]
fn the_json_is_one_document_that_always_carries_provenance() {
    let tree = vault("explain-json", STARTER);
    let finished = explain(&tree, &["--format", "json"]);
    assert_eq!(finished.code, 0, "{finished:?}");
    assert!(
        well_formed_json(&finished.stdout),
        "not one JSON document: {finished:?}"
    );
    for fragment in [
        "\"schema_version\": 1",
        "\"report\": \"contract\"",
        "\"contract_version\": 1",
        "\"links\": \"wikilink\"",
        "\"provenance\"",
        "\"source\": \"contract\"",
        "\"source\": \"default\"",
        "\"name\": \"person\"",
    ] {
        assert!(
            finished.stdout.contains(fragment),
            "no `{fragment}`: {finished:?}"
        );
    }
}

#[test]
fn a_contract_that_did_not_resolve_is_never_explained() {
    let tree = vault("refuse", TOO_NEW);
    let finished = explain(&tree, &[]);
    assert_eq!(finished.code, 1, "{finished:?}");
    assert_eq!(
        finished.stdout, "",
        "a partially resolved contract presented as the vault's rules is worse than no answer: \
         {finished:?}"
    );
    assert!(
        finished.stderr.contains("error[compat.contract-too-new]"),
        "{finished:?}"
    );
    assert!(
        finished.stderr.contains("run `dogtag doctor`"),
        "the refusal points at the surface that reports on a broken vault: {finished:?}"
    );
}

#[test]
fn a_vault_that_cannot_be_resolved_refuses_the_same_way() {
    let tree = Tree::new("explain-no-vault");
    support::assert_no_vault_above(&tree.home());
    let finished = run(dogtag(&tree).args(["contract", "explain"]));
    assert_eq!(finished.code, 1, "{finished:?}");
    assert_eq!(finished.stdout, "", "{finished:?}");
    assert!(
        finished.stderr.contains("error[discovery.no-vault-found]"),
        "{finished:?}"
    );
}

#[test]
fn explain_selects_a_vault_the_same_way_doctor_does() {
    let tree = vault("explain-selection", STARTER);
    let finished = run(dogtag(&tree).args(["contract", "explain", "--vault", "work"]));
    assert_eq!(finished.code, 0, "{finished:?}");
    assert!(
        finished.stdout.starts_with("# Vault contract"),
        "{finished:?}"
    );
}

/// The contract resolved, so the rendering is not a fiction and is delivered
/// — but a run that raised an error is a run that failed, because severity
/// alone decides the exit code.
#[test]
fn an_unusable_record_fails_the_run_without_withholding_the_rendering() {
    let tree = vault("explain-record", STARTER);
    tree.record("installation_version = 9\n");
    let finished = explain(&tree, &[]);
    assert_eq!(finished.code, 1, "{finished:?}");
    assert!(
        finished.stdout.starts_with("# Vault contract"),
        "{finished:?}"
    );
    assert!(
        finished
            .stderr
            .contains("error[compat.installation-too-new]"),
        "{finished:?}"
    );
}

/// A warning does not stop the rendering, and does not fail the run either:
/// severity alone decides, and `explain` has no `--strict`.
#[test]
fn a_root_outside_the_home_directory_is_reported_beside_the_rendering() {
    let tree = Tree::new("explain-trust");
    let root = tree.vault("outside/vault", STARTER);
    let finished = run(dogtag(&tree)
        .args(["contract", "explain"])
        .current_dir(&root));
    assert_eq!(finished.code, 0, "{finished:?}");
    assert!(
        finished.stdout.starts_with("# Vault contract"),
        "{finished:?}"
    );
    assert!(
        finished
            .stderr
            .contains("warning[discovery.root-outside-home]"),
        "the warning belongs beside the instructions, not inside them: {finished:?}"
    );
    assert!(
        !finished.stdout.contains("root-outside-home"),
        "and never inside the rendering itself: {finished:?}"
    );
}

#[test]
fn piped_output_carries_no_escape_sequence_whether_or_not_colour_is_suppressed() {
    let tree = vault("colour", TOO_NEW);
    let suppressed = run(dogtag(&tree)
        .args(["contract", "explain"])
        .env("NO_COLOR", "1")
        .current_dir(tree.home().join("vault")));
    let ordinary = explain(&tree, &[]);
    assert!(
        !suppressed.has_escapes(),
        "NO_COLOR is honoured: {suppressed:?}"
    );
    assert!(
        !ordinary.has_escapes(),
        "and a stream that is not a terminal takes no colour either: {ordinary:?}"
    );
    assert_eq!(
        (suppressed.stdout, suppressed.stderr),
        (ordinary.stdout, ordinary.stderr),
        "colour changes escape sequences and nothing else"
    );
}

#[test]
fn strict_makes_a_nested_vault_fail_on_the_surface_an_agent_reads() {
    // `contract explain` renders the contract as instructions an agent
    // follows. A nested vault is a warning, and it means those instructions
    // came from a different corpus than the one intended — which is the
    // reason `--strict` exists, and this is the surface that needed it most.
    let nested = Tree::new("explain-strict");
    nested.vault("home/outer", STARTER);
    let inner = nested.vault("home/outer/inner", STARTER);
    let finished = run(dogtag(&nested)
        .args(["contract", "explain", "--strict"])
        .current_dir(&inner));
    assert_eq!(finished.code, 1, "{finished:?}");
    assert!(
        finished.stderr.contains("warning[discovery.nested-vault]"),
        "{finished:?}"
    );
}

#[test]
fn strict_decides_the_exit_code_and_changes_nothing_else() {
    let nested = Tree::new("explain-strict-bytes");
    nested.vault("home/outer", STARTER);
    let inner = nested.vault("home/outer/inner", STARTER);
    let ordinary = run(dogtag(&nested)
        .args(["contract", "explain"])
        .current_dir(&inner));
    let strict = run(dogtag(&nested)
        .args(["contract", "explain", "--strict"])
        .current_dir(&inner));
    assert_eq!(ordinary.code, 0, "the warning alone is not a failure");
    assert_eq!(
        (ordinary.stdout, ordinary.stderr),
        (strict.stdout, strict.stderr)
    );
}
