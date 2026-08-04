//! `dogtag list` against the built binary.

mod support;

use support::{Finished, TOO_NEW, Tree, dogtag, run, well_formed_json};

const CONTRACT: &str = concat!(
    "contract_version = 2\n\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\naxis = \"stage\"\nordinary = { absent = true }\n",
    "\n[tags]\nproperty = \"tags\"\n",
    "\n[[type]]\nname = \"work\"\ncapabilities = [\"identity-bearing\"]\n",
    "  [[type.property]]\n  name = \"stage\"\n  kind = \"enum\"\n  values = [\"active\", \"done\"]\n",
    "  [[type.property]]\n  name = \"tags\"\n  kind = \"list\"\n  of = \"string\"\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
);

fn vault(label: &str, contract: &str) -> Tree {
    let tree = Tree::new(label);
    tree.vault("home/vault", contract);
    tree.write("home/vault/b.md", "plain\n");
    tree.write(
        "home/vault/a.md",
        "---\ntype: work\nstage: active\ntags: [topic/x]\n---\n# A\n",
    );
    tree
}

fn list(tree: &Tree, arguments: &[&str]) -> Finished {
    run(dogtag(tree)
        .arg("list")
        .args(arguments)
        .current_dir(tree.home().join("vault")))
}

#[test]
fn text_is_sorted_and_filters_compose() {
    let tree = vault("list-text", CONTRACT);
    let all = list(&tree, &[]);
    assert_eq!(all.code, 0, "{all:?}");
    assert_eq!(all.stdout, "a.md\twork\tactive\nb.md\tcapture\n");
    assert_eq!(all.stderr, "");
    let filtered = list(
        &tree,
        &[
            "--type",
            "work",
            "--tag",
            "topic/x",
            "--lifecycle",
            "active",
        ],
    );
    assert_eq!(filtered.stdout, "a.md\twork\tactive\n");
}

#[test]
fn json_is_one_document_with_notes_and_diagnostics() {
    let tree = vault("list-json", CONTRACT);
    let finished = list(&tree, &["--format", "json"]);
    assert!(well_formed_json(&finished.stdout), "{finished:?}");
    assert!(finished.stdout.contains("\"report\": \"list\""));
    assert!(finished.stdout.contains("\"notes\": ["));
    assert_eq!(finished.stderr, "");
}

#[test]
fn lifecycle_flags_are_exclusive_and_no_axis_is_a_diagnostic() {
    let exclusive = vault("list-exclusive", CONTRACT);
    let usage = list(&exclusive, &["--ordinary", "--lifecycle", "active"]);
    assert_eq!(usage.code, 2, "{usage:?}");
    let none = CONTRACT.replace(
        "axis = \"stage\"\nordinary = { absent = true }",
        "none = true",
    );
    let no_axis = vault("list-no-axis", &none);
    let refused = list(&no_axis, &["--ordinary"]);
    assert_eq!(refused.code, 1, "{refused:?}");
    assert_eq!(refused.stdout, "");
    assert!(refused.stderr.contains("note.lifecycle-axis-absent"));
}

#[test]
fn unresolved_contract_and_selection_refuse_without_a_result() {
    let broken_tree = vault("list-broken", TOO_NEW);
    let broken = list(&broken_tree, &[]);
    assert_eq!(broken.code, 1, "{broken:?}");
    assert_eq!(broken.stdout, "");
    assert!(broken.stderr.contains("run `dogtag doctor`"));
    let tree = Tree::new("list-selection");
    let missing = run(dogtag(&tree).args(["list", "--vault", "unknown"]));
    assert_eq!(missing.code, 1, "{missing:?}");
    assert!(missing.stderr.contains("installation.unknown-vault-name"));
}

#[test]
fn strict_changes_only_the_exit_decision() {
    let tree = vault("list-strict", CONTRACT);
    tree.write("home/vault/warn.md", "\u{feff}plain\n");
    let ordinary = list(&tree, &[]);
    let strict = list(&tree, &["--strict"]);
    assert_eq!((ordinary.code, strict.code), (0, 1));
    assert_eq!(ordinary.stdout, strict.stdout);
    assert_eq!(ordinary.stderr, strict.stderr);
}
