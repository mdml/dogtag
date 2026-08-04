//! `dogtag show` against the built binary.

mod support;

use support::{STARTER, Tree, dogtag, run, well_formed_json};

fn vault(tree: &Tree) -> std::path::PathBuf {
    let root = tree.vault("vault", STARTER);
    tree.write(
        "vault/people/ada.md",
        "---\ntype: person\nfull_name: Ada Lovelace\nstatus: active\n---\n# Ada Lovelace\n\nBody.\n",
    );
    root
}

#[test]
fn text_show_renders_the_note_through_the_sdk() {
    let tree = Tree::new("show-text");
    let root = vault(&tree);
    let finished = run(dogtag(&tree)
        .current_dir(&root)
        .args(["show", "people/ada"]));
    assert_eq!(finished.code, 0, "{}", finished.stderr);
    assert!(finished.stdout.contains("path: people/ada.md"));
    assert!(finished.stdout.contains("title: Ada Lovelace"));
    assert!(finished.stdout.contains("body:\n# Ada Lovelace\n\nBody."));
}

#[test]
fn json_show_is_one_document_with_the_result_and_diagnostics() {
    let tree = Tree::new("show-json");
    let root = vault(&tree);
    let finished = run(dogtag(&tree)
        .current_dir(&root)
        .args(["show", "ada", "--format", "json"]));
    assert_eq!(finished.code, 0, "{}", finished.stderr);
    assert!(well_formed_json(&finished.stdout));
    assert!(finished.stdout.contains("\"report\": \"show\""));
    assert!(finished.stdout.contains("\"path\": \"people/ada.md\""));
    assert!(finished.stdout.contains("\"diagnostics\":"));
    assert!(finished.stdout.contains("\"summary\":"));
    assert_eq!(finished.stderr, "");
}

#[test]
fn a_missing_reference_uses_the_link_diagnostic_and_exit_one() {
    let tree = Tree::new("show-missing");
    let root = vault(&tree);
    let finished = run(dogtag(&tree).current_dir(&root).args(["show", "missing"]));
    assert_eq!(finished.code, 1);
    assert_eq!(finished.stdout, "");
    assert!(finished.stderr.contains("error[link.target-not-found]"));
}

#[test]
fn json_keeps_a_missing_reference_in_one_structured_envelope() {
    let tree = Tree::new("show-missing-json");
    let root = vault(&tree);
    let finished = run(dogtag(&tree)
        .current_dir(&root)
        .args(["show", "missing", "--format", "json"]));
    assert_eq!(finished.code, 1);
    assert!(well_formed_json(&finished.stdout));
    assert!(finished.stdout.contains("\"note\": null"));
    assert!(finished.stdout.contains("link.target-not-found"));
    assert_eq!(finished.stderr, "");
}

#[test]
fn an_unresolved_contract_refuses_exactly_without_a_report() {
    let tree = Tree::new("show-contract-refusal");
    let root = tree.vault("vault", "contract_version = 99\n");
    let finished = run(dogtag(&tree)
        .current_dir(&root)
        .args(["show", "anything", "--format", "json"]));
    assert_eq!(finished.code, 1);
    assert_eq!(finished.stdout, "");
    assert!(finished.stderr.contains("run `dogtag doctor`"));
}

#[test]
fn a_vault_that_does_not_resolve_refuses_with_the_sdk_diagnostic() {
    let tree = Tree::new("show-vault-refusal");
    let finished = run(dogtag(&tree).args(["show", "anything", "--vault", "./absent"]));
    assert_eq!(finished.code, 1);
    assert_eq!(finished.stdout, "");
    assert!(
        finished.stderr.contains("error[discovery.path-unreadable]"),
        "{}",
        finished.stderr
    );
}
