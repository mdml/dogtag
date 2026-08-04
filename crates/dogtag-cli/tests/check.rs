//! `dogtag check` against the built binary.

mod support;

use support::{Finished, TOO_NEW, Tree, dogtag, run, well_formed_json};

const CONTRACT: &str = concat!(
    "contract_version = 2\n\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[tags]\nproperty = \"tags\"\n",
    "\n[[type]]\nname = \"work\"\ncapabilities = [\"identity-bearing\"]\n",
    "  [[type.property]]\n  name = \"tags\"\n  kind = \"list\"\n  of = \"string\"\n",
    "  [[type.relationship]]\n  predicate = \"relates\"\n",
    "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
);

fn vault(label: &str, contract: &str) -> Tree {
    let tree = Tree::new(label);
    tree.vault("home/vault", contract);
    tree.write("home/vault/b.md", "plain\n");
    tree.write(
        "home/vault/a.md",
        "---\ntype: work\ntags: [topic/x]\n---\n# A\n",
    );
    tree
}

fn check(tree: &Tree, arguments: &[&str]) -> Finished {
    run(dogtag(tree)
        .arg("check")
        .args(arguments)
        .current_dir(tree.home().join("vault")))
}

#[test]
fn a_clean_corpus_reports_no_findings_and_exits_zero() {
    let tree = vault("check-clean", CONTRACT);
    let finished = check(&tree, &[]);
    assert_eq!(finished.code, 0, "{finished:?}");
    assert_eq!(finished.stdout, "no findings\n");
    assert_eq!(finished.stderr, "");
}

#[test]
fn findings_summarize_on_stdout_and_diagnose_on_stderr() {
    let tree = vault("check-findings", CONTRACT);
    tree.write(
        "home/vault/c.md",
        "---\ntype: work\nrelates: \"[[missing]]\"\n---\n# C\n",
    );
    let finished = check(&tree, &[]);
    assert_eq!(finished.code, 1, "{finished:?}");
    assert!(
        finished
            .stdout
            .starts_with("findings: 1 error(s), 0 warning(s), 0 info\n"),
        "{finished:?}"
    );
    assert!(finished.stdout.contains("link.dangling-typed-link  1"));
    assert!(finished.stderr.contains("error[link.dangling-typed-link]"));
}

#[test]
fn json_is_one_document_with_summary_tally_and_diagnostics() {
    let tree = vault("check-json", CONTRACT);
    tree.write(
        "home/vault/c.md",
        "---\ntype: work\nrelates: \"[[missing]]\"\n---\n# C\n",
    );
    let finished = check(&tree, &["--format", "json"]);
    assert_eq!(finished.code, 1, "{finished:?}");
    assert!(well_formed_json(&finished.stdout));
    assert!(finished.stdout.contains("\"report\": \"check\""));
    assert!(finished.stdout.contains("\"summary\":"));
    assert!(finished.stdout.contains("\"by_identifier\":"));
    assert!(
        finished
            .stdout
            .contains("\"id\": \"link.dangling-typed-link\"")
    );
    assert!(finished.stdout.contains("\"count\": 1"));
    assert!(finished.stdout.contains("\"diagnostics\":"));
    assert_eq!(finished.stderr, "");
}

#[test]
fn strict_changes_only_the_exit_decision() {
    let tree = vault("check-strict", CONTRACT);
    tree.write("home/vault/warn.md", "\u{feff}plain\n");
    let ordinary = check(&tree, &[]);
    let strict = check(&tree, &["--strict"]);
    assert_eq!((ordinary.code, strict.code), (0, 1), "{ordinary:?}");
    assert_eq!(ordinary.stdout, strict.stdout);
    assert_eq!(ordinary.stderr, strict.stderr);
}

#[test]
fn an_unresolved_contract_refuses_exactly_as_contract_explain() {
    let tree = vault("check-broken", TOO_NEW);
    let finished = check(&tree, &[]);
    assert_eq!(finished.code, 1, "{finished:?}");
    assert_eq!(finished.stdout, "");
    assert!(finished.stderr.contains("cannot be checked"));
    assert!(finished.stderr.contains("run `dogtag doctor`"));
}

#[test]
fn an_unknown_vault_name_is_a_selection_refusal() {
    let tree = Tree::new("check-selection");
    let missing = run(dogtag(&tree).args(["check", "--vault", "unknown"]));
    assert_eq!(missing.code, 1, "{missing:?}");
    assert!(missing.stderr.contains("installation.unknown-vault-name"));
}
