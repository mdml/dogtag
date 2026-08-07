//! `dogtag search` against the built binary.

mod support;

use support::{Finished, TOO_NEW, Tree, dogtag, run, well_formed_json};

const CONTRACT: &str = concat!(
    "contract_version = 3\n\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\naxis = \"stage\"\nordinary = { absent = true }\n",
    "\n[tags]\nproperty = \"tags\"\n",
    "\n[[type]]\nname = \"work\"\ncapabilities = [\"identity-bearing\"]\n",
    "  [[type.property]]\n  name = \"stage\"\n  kind = \"enum\"\n  values = [\"active\", \"done\"]\n",
    "  [[type.property]]\n  name = \"tags\"\n  kind = \"list\"\n  of = \"string\"\n",
    "  [[type.property]]\n  name = \"aliases\"\n  kind = \"list\"\n  of = \"string\"\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
);

fn vault(label: &str, contract: &str) -> Tree {
    let tree = Tree::new(label);
    tree.vault("home/vault", contract);
    tree.write(
        "home/vault/engines.md",
        "---\ntype: work\nstage: active\ntags: [topic/x]\n---\n# Engines\n\nThe analytical engine.\n",
    );
    tree.write(
        "home/vault/babbage.md",
        "---\ntype: work\naliases: [\"Engine Father\"]\n---\nprose\n",
    );
    tree.write("home/vault/plain.md", "nothing to find\n");
    tree
}

fn search(tree: &Tree, arguments: &[&str]) -> Finished {
    run(dogtag(tree)
        .arg("search")
        .args(arguments)
        .current_dir(tree.home().join("vault")))
}

#[test]
fn text_is_relevance_ordered_with_a_path_tie_break() {
    let tree = vault("search-text", CONTRACT);
    let found = search(&tree, &["engine"]);
    assert_eq!(found.code, 0, "{found:?}");
    assert_eq!(
        found.stdout,
        "babbage.md\twork\tEngine Father\nengines.md\twork\t# Engines  The analytical engine.\n",
        "the alias identity match outranks the body match, and the window folds to one line"
    );
    assert_eq!(found.stderr, "");
}

#[test]
fn filters_compose_and_the_limit_caps_the_hits() {
    let tree = vault("search-filters", CONTRACT);
    let filtered = search(
        &tree,
        &[
            "engine",
            "--type",
            "work",
            "--tag",
            "topic/x",
            "--lifecycle",
            "active",
        ],
    );
    assert_eq!(filtered.code, 0, "{filtered:?}");
    assert_eq!(
        filtered.stdout,
        "engines.md\twork\t# Engines  The analytical engine.\n"
    );
    let limited = search(&tree, &["engine", "--limit", "1"]);
    assert_eq!(limited.stdout, "babbage.md\twork\tEngine Father\n");
    let none = search(&tree, &["engine", "--limit", "0"]);
    assert_eq!((none.code, none.stdout.as_str()), (0, ""), "{none:?}");
}

#[test]
fn json_is_one_document_with_hits_and_diagnostics() {
    let tree = vault("search-json", CONTRACT);
    let finished = search(&tree, &["engine", "--format", "json"]);
    assert!(well_formed_json(&finished.stdout), "{finished:?}");
    assert!(finished.stdout.contains("\"report\": \"search\""));
    assert!(finished.stdout.contains("\"hits\": ["));
    assert!(
        finished
            .stdout
            .contains("\"snippet\": \"# Engines\\n\\nThe analytical engine.\"")
    );
    assert_eq!(finished.stderr, "");
}

#[test]
fn an_empty_result_is_a_result() {
    let tree = vault("search-empty", CONTRACT);
    let found = search(&tree, &["absent"]);
    assert_eq!(
        (found.code, found.stdout.as_str(), found.stderr.as_str()),
        (0, "", "")
    );
}

#[test]
fn an_invalid_query_is_a_diagnostic_not_a_usage_error() {
    let tree = vault("search-invalid", CONTRACT);
    let unbalanced = search(&tree, &["\"never closed"]);
    assert_eq!(unbalanced.code, 1, "{unbalanced:?}");
    assert_eq!(unbalanced.stdout, "");
    assert!(unbalanced.stderr.contains("search.invalid-query"));
    let wordless = search(&tree, &["…"]);
    assert_eq!(wordless.code, 1, "{wordless:?}");
    assert!(wordless.stderr.contains("search.invalid-query"));
    let json = search(&tree, &["\"never closed", "--format", "json"]);
    assert_eq!(json.code, 1, "{json:?}");
    assert!(well_formed_json(&json.stdout), "{json:?}");
    assert!(json.stdout.contains("\"hits\": []"));
    assert!(json.stdout.contains("search.invalid-query"));
}

#[test]
fn lifecycle_flags_are_exclusive_and_no_axis_is_a_diagnostic() {
    let exclusive = vault("search-exclusive", CONTRACT);
    let usage = search(
        &exclusive,
        &["engine", "--ordinary", "--lifecycle", "active"],
    );
    assert_eq!(usage.code, 2, "{usage:?}");
    let none = CONTRACT.replace(
        "axis = \"stage\"\nordinary = { absent = true }",
        "none = true",
    );
    let no_axis = vault("search-no-axis", &none);
    let refused = search(&no_axis, &["engine", "--ordinary"]);
    assert_eq!(refused.code, 1, "{refused:?}");
    assert_eq!(refused.stdout, "");
    assert!(refused.stderr.contains("note.lifecycle-axis-absent"));
}

#[test]
fn unresolved_contract_and_selection_refuse_without_a_result() {
    let broken_tree = vault("search-broken", TOO_NEW);
    let broken = search(&broken_tree, &["engine"]);
    assert_eq!(broken.code, 1, "{broken:?}");
    assert_eq!(broken.stdout, "");
    assert!(broken.stderr.contains("run `dogtag doctor`"));
    let tree = Tree::new("search-selection");
    let missing = run(dogtag(&tree).args(["search", "engine", "--vault", "unknown"]));
    assert_eq!(missing.code, 1, "{missing:?}");
    assert!(missing.stderr.contains("installation.unknown-vault-name"));
}

#[test]
fn strict_changes_only_the_exit_decision() {
    let tree = vault("search-strict", CONTRACT);
    tree.write("home/vault/warn.md", "\u{feff}engine\n");
    let ordinary = search(&tree, &["engine"]);
    let strict = search(&tree, &["engine", "--strict"]);
    assert_eq!((ordinary.code, strict.code), (0, 1));
    assert_eq!(ordinary.stdout, strict.stdout);
    assert_eq!(ordinary.stderr, strict.stderr);
}
