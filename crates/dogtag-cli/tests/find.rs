//! `dogtag find` against the built binary.

mod support;

use support::{Finished, TOO_NEW, Tree, dogtag, run, well_formed_json};

const CONTRACT: &str = concat!(
    "contract_version = 3\n\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\naxis = \"stage\"\nordinary = { absent = true }\n",
    "\n[[type]]\nname = \"work\"\ncapabilities = [\"identity-bearing\"]\n",
    "  [[type.property]]\n  name = \"stage\"\n  kind = \"enum\"\n  values = [\"active\", \"done\"]\n",
    "  [[type.property]]\n  name = \"aliases\"\n  kind = \"list\"\n  of = \"string\"\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
);

fn vault(label: &str, contract: &str) -> Tree {
    let tree = Tree::new(label);
    tree.vault("home/vault", contract);
    tree.write(
        "home/vault/engines/analytical.md",
        "---\ntype: work\nstage: active\naliases: [\"The Engine\"]\n---\n# Analytical\n",
    );
    tree.write("home/vault/2025/daily.md", "# Daily\n");
    tree.write("home/vault/2026/daily.md", "# Daily\n");
    tree
}

fn find(tree: &Tree, arguments: &[&str]) -> Finished {
    run(dogtag(tree)
        .arg("find")
        .args(arguments)
        .current_dir(tree.home().join("vault")))
}

#[test]
fn an_unambiguous_name_answers_one_summary_line_case_insensitively() {
    let tree = vault("find-text", CONTRACT);
    let by_name = find(&tree, &["ANALYTICAL"]);
    assert_eq!(by_name.code, 0, "{by_name:?}");
    assert_eq!(by_name.stdout, "engines/analytical.md\twork\tactive\n");
    assert_eq!(by_name.stderr, "");
    let by_alias = find(&tree, &["the engine"]);
    assert_eq!(by_alias.stdout, "engines/analytical.md\twork\tactive\n");
}

#[test]
fn an_ambiguous_name_is_an_error_whose_diagnostic_lists_every_candidate() {
    let tree = vault("find-ambiguous", CONTRACT);
    let refused = find(&tree, &["daily"]);
    assert_eq!(refused.code, 1, "{refused:?}");
    assert_eq!(refused.stdout, "");
    assert!(refused.stderr.contains("link.ambiguous-reference"));
    assert!(refused.stderr.contains("2025/daily.md"));
    assert!(refused.stderr.contains("2026/daily.md"));
    let qualified = find(&tree, &["2025/daily"]);
    assert_eq!(qualified.code, 0, "{qualified:?}");
    assert_eq!(qualified.stdout, "2025/daily.md\tcapture\n");
}

#[test]
fn the_type_filter_narrows_and_an_absent_name_is_target_not_found() {
    let tree = vault("find-type", CONTRACT);
    let narrowed = find(&tree, &["daily", "--type", "work"]);
    assert_eq!(narrowed.code, 1, "{narrowed:?}");
    assert!(narrowed.stderr.contains("link.target-not-found"));
    assert!(narrowed.stderr.contains("of type `work`"));
    let absent = find(&tree, &["babbage"]);
    assert_eq!(absent.code, 1, "{absent:?}");
    assert_eq!(absent.stdout, "");
    assert!(absent.stderr.contains("link.target-not-found"));
}

#[test]
fn json_is_one_document_with_the_note_or_an_explicit_null() {
    let tree = vault("find-json", CONTRACT);
    let found = find(&tree, &["analytical", "--format", "json"]);
    assert!(well_formed_json(&found.stdout), "{found:?}");
    assert!(found.stdout.contains("\"report\": \"find\""));
    assert!(found.stdout.contains("\"path\": \"engines/analytical.md\""));
    assert_eq!(found.stderr, "");
    let refused = find(&tree, &["daily", "--format", "json"]);
    assert_eq!(refused.code, 1, "{refused:?}");
    assert!(well_formed_json(&refused.stdout), "{refused:?}");
    assert!(refused.stdout.contains("\"note\": null"));
    assert!(refused.stdout.contains("link.ambiguous-reference"));
    assert_eq!(refused.stderr, "");
}

#[test]
fn unresolved_contract_and_selection_refuse_without_a_result() {
    let broken_tree = vault("find-broken", TOO_NEW);
    let broken = find(&broken_tree, &["analytical"]);
    assert_eq!(broken.code, 1, "{broken:?}");
    assert_eq!(broken.stdout, "");
    assert!(broken.stderr.contains("run `dogtag doctor`"));
    let tree = Tree::new("find-selection");
    let missing = run(dogtag(&tree).args(["find", "analytical", "--vault", "unknown"]));
    assert_eq!(missing.code, 1, "{missing:?}");
    assert!(missing.stderr.contains("installation.unknown-vault-name"));
}

#[test]
fn strict_changes_only_the_exit_decision() {
    let tree = vault("find-strict", CONTRACT);
    tree.write("home/vault/warn.md", "\u{feff}plain\n");
    let ordinary = find(&tree, &["analytical"]);
    let strict = find(&tree, &["analytical", "--strict"]);
    assert_eq!((ordinary.code, strict.code), (0, 1));
    assert_eq!(ordinary.stdout, strict.stdout);
    assert_eq!(ordinary.stderr, strict.stderr);
}
