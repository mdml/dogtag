//! `dogtag capture` against the built binary.
//!
//! The one mutation, so these tests assert what a *write* verb owes that a read
//! verb does not: that a preview leaves the tree byte-identical, that the act
//! creates exactly one file and says where, that the exit code follows the
//! transaction rather than the corpus, and that the thought reaches the note
//! unchanged whichever of the three inputs it arrived by.

mod support;

use std::process::Stdio;

use support::{Finished, TOO_NEW, Tree, dogtag, registering, run, well_formed_json};

/// The `starter` shape: a catch-all born carrying the triage flag it declares,
/// and a capture directory written out rather than defaulted.
const CONTRACT: &str = concat!(
    "contract_version = 3\n\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[[flag]]\nproperty = \"needs_triage\"\n",
    "\n[capture]\ndirectory = \"captures\"\n",
    "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\"]\n",
    "born-flagged = [\"needs_triage\"]\n",
    "  [[type.property]]\n  name = \"needs_triage\"\n  kind = \"boolean\"\n",
);

/// The same corpus at the version below the seats: no capture table to declare
/// a directory in, and no birth state.
const OLDER: &str = concat!(
    "contract_version = 2\n\n[dialect]\nlinks = \"wikilink\"\n",
    "\n[lifecycle]\nnone = true\n",
    "\n[[type]]\nname = \"note\"\ncapabilities = [\"catch-all\"]\n",
);

fn vault(label: &str, contract: &str) -> Tree {
    let tree = Tree::new(label);
    let root = tree.vault("home/vault", contract);
    tree.record(&registering("work", &root));
    tree
}

fn capture(tree: &Tree, arguments: &[&str]) -> Finished {
    run(dogtag(tree)
        .arg("capture")
        .args(arguments)
        .current_dir(tree.home().join("vault")))
}

/// The vault-relative path of the one note the vault holds, or a panic naming
/// what it holds instead.
fn only_note(tree: &Tree) -> String {
    let notes: Vec<String> = tree
        .listing()
        .into_iter()
        .filter(|path| path.ends_with(".md") && path.contains("vault/"))
        .collect();
    assert_eq!(notes.len(), 1, "exactly one note: {notes:?}");
    notes.into_iter().next().expect("the one note")
}

#[test]
fn a_capture_creates_one_note_and_says_where_it_is() {
    let tree = vault("capture-lands", CONTRACT);
    let finished = capture(&tree, &["a loose thought"]);
    assert_eq!(finished.code, 0, "{finished:?}");
    assert!(
        finished.stdout.contains("captured      captures/"),
        "{finished:?}"
    );
    assert!(
        finished.stdout.contains("recover by    deleting captures/"),
        "{finished:?}"
    );
    let note = only_note(&tree);
    assert!(note.contains("/captures/"), "{note}");
    let contents = std::fs::read_to_string(tree.path().join(&note)).expect("the note");
    assert_eq!(contents, "---\nneeds_triage: true\n---\na loose thought");
}

/// The preview writes nothing at all — not the note, not the directory — and
/// the whole tree is byte-identical afterward.
#[test]
fn a_preview_writes_nothing_anywhere() {
    let tree = vault("capture-preview", CONTRACT);
    let before = tree.listing();
    let finished = capture(&tree, &["--preview", "a loose thought"]);
    assert_eq!(finished.code, 0, "{finished:?}");
    assert!(
        finished.stdout.starts_with("preview: nothing written\n"),
        "{finished:?}"
    );
    assert!(
        finished.stdout.contains("would create   captures/"),
        "{finished:?}"
    );
    assert_eq!(tree.listing(), before, "a preview left the tree changed");
}

/// The three inputs, each carrying the same thought to the same body.
#[test]
fn the_thought_arrives_by_argument_by_file_or_by_standard_input() {
    let tree = vault("capture-inputs", CONTRACT);
    let path = tree.write("thought.txt", "from a file\n");
    let from_file = capture(&tree, &["--file", path.to_str().expect("a path")]);
    assert_eq!(from_file.code, 0, "{from_file:?}");

    let mut command = dogtag(&tree);
    command
        .arg("capture")
        .arg("-")
        .current_dir(tree.home().join("vault"))
        .stdin(Stdio::piped());
    let mut child = command.spawn().expect("the dogtag binary runs");
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("a piped standard input");
        stdin
            .write_all(b"from standard input\n")
            .expect("the thought");
    }
    let output = child.wait_with_output().expect("the binary finishes");
    assert!(output.status.success(), "{output:?}");

    let bodies: Vec<String> = tree
        .listing()
        .into_iter()
        .filter(|path| path.contains("/captures/"))
        .map(|path| std::fs::read_to_string(tree.path().join(path)).expect("a capture"))
        .collect();
    assert_eq!(bodies.len(), 2, "{bodies:?}");
    assert!(
        bodies.iter().any(|body| body.ends_with("from a file\n")),
        "{bodies:?}"
    );
    assert!(
        bodies
            .iter()
            .any(|body| body.ends_with("from standard input\n")),
        "{bodies:?}"
    );
}

/// The body is the bytes that arrived, and the frontmatter is only what the
/// contract says a note of this type is born carrying — which at a version with
/// no birth state is nothing at all.
#[test]
fn a_version_without_the_seats_captures_into_the_default_with_no_frontmatter() {
    let tree = vault("capture-older", OLDER);
    let finished = capture(&tree, &["a loose thought"]);
    assert_eq!(finished.code, 0, "{finished:?}");
    let note = only_note(&tree);
    assert!(note.contains("/captures/"), "{note}");
    let contents = std::fs::read_to_string(tree.path().join(&note)).expect("the note");
    assert_eq!(contents, "a loose thought");
}

/// The structured document is one JSON document on standard output and nothing
/// else, whichever way the act went.
#[test]
fn the_json_is_one_document_carrying_the_plan_and_the_verdict() {
    let tree = vault("capture-json", CONTRACT);
    let finished = capture(&tree, &["--format", "json", "a loose thought"]);
    assert_eq!(finished.code, 0, "{finished:?}");
    assert_eq!(finished.stderr, "", "the JSON run says nothing on stderr");
    assert!(well_formed_json(&finished.stdout), "{finished:?}");
    for fragment in [
        "\"schema_version\": 4",
        "\"report\": \"capture\"",
        "\"landed\": true",
        "\"outcome\": \"created\"",
        "\"provenance\": \"human\"",
        "\"compatibility\": null",
        "\"action\": \"delete\"",
    ] {
        assert!(
            finished.stdout.contains(fragment),
            "no `{fragment}`: {finished:?}"
        );
    }
}

/// The structured document carries the *whole* run, not only the act: what the
/// loading path had to say reaches it and its summary, exactly as it reaches
/// every other verb's document.
///
/// Held against a vault below the current contract version, because that is the
/// shape two of the three committed corpora are in and the classification they
/// earn is the one a parallel week's diff would otherwise silently lose.
#[test]
fn the_json_carries_what_the_loading_path_reported_and_counts_it() {
    let tree = vault("capture-json-loading", OLDER);
    let finished = capture(&tree, &["--format", "json", "a loose thought"]);
    assert_eq!(finished.code, 0, "{finished:?}");
    assert!(
        finished.stdout.contains("compat.newer-format-available"),
        "{finished:?}"
    );
    // The same classification the text run puts on stderr, counted rather than
    // merely listed: the summary is what a diff reads first.
    assert!(finished.stdout.contains("\"info\": 1"), "{finished:?}");
    let text = capture(&tree, &["a loose thought"]);
    assert!(
        text.stderr.contains("compat.newer-format-available"),
        "{text:?}"
    );
}

/// The invocation narrows what the record says, and the capacity is the
/// invocation's alone — the record has nowhere to carry one.
///
/// Three rows: what the record says on its own, what the invocation says
/// instead, and the third capacity, so every member of the closed set is
/// reached by the surface that spells it.
#[test]
fn the_invocation_narrows_the_record_and_names_the_capacity() {
    let tree = vault("capture-actor", CONTRACT);
    let cases: &[(&[&str], &str, &str)] = &[
        (&[], "A Maintainer", "human"),
        (
            &["--actor", "Somebody Else", "--provenance", "agent"],
            "Somebody Else",
            "agent",
        ),
        (
            &["--provenance", "automation"],
            "A Maintainer",
            "automation",
        ),
    ];
    for (arguments, name, provenance) in cases {
        let mut given = vec!["--format", "json"];
        given.extend_from_slice(arguments);
        given.push("a loose thought");
        let finished = capture(&tree, &given);
        let observed = (
            finished.code,
            finished.stdout.contains(&format!("\"name\": \"{name}\"")),
            finished
                .stdout
                .contains(&format!("\"provenance\": \"{provenance}\"")),
        );
        assert_eq!(observed, (0, true, true), "{arguments:?}: {finished:?}");
    }
}

/// A vault selector naming no registered vault refuses the act, with the
/// installation-area diagnostic and nothing written.
#[test]
fn a_selector_that_names_no_vault_refuses_the_act() {
    let tree = vault("capture-no-such-vault", CONTRACT);
    let before = tree.listing();
    let finished = capture(&tree, &["--vault", "nonesuch", "a loose thought"]);
    assert_eq!(finished.code, 1, "{finished:?}");
    assert_eq!(finished.stdout, "");
    assert!(
        finished.stderr.contains("installation.unknown-vault-name"),
        "{finished:?}"
    );
    assert_eq!(tree.listing(), before, "a refusal left the tree changed");
}

/// A thought that is not text at all cannot be captured, and the refusal is the
/// same kind of fault as a file that is not there: nothing was read.
#[test]
fn a_thought_that_is_not_utf8_is_a_usage_fault() {
    let tree = vault("capture-not-utf8", CONTRACT);
    let mut command = dogtag(&tree);
    command
        .arg("capture")
        .arg("-")
        .current_dir(tree.home().join("vault"))
        .stdin(Stdio::piped());
    let mut child = command.spawn().expect("the dogtag binary runs");
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("a piped standard input");
        let _ = stdin.write_all(&[0xff, 0xfe, 0xfd]);
    }
    let output = child.wait_with_output().expect("the binary finishes");
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
}

/// Two things a capture reports without either of them changing its verdict:
/// an act nobody can be attributed with, and a corpus that was already broken
/// before the act. Both warn or report on standard error, both leave the note
/// on standard output, and both exit `0`.
///
/// Held together because they are one claim — *the transaction is the verdict*
/// — asserted from its two sides, and because a reader comparing them sees at
/// once that the only difference is which identifier arrives.
#[test]
fn what_a_capture_reports_never_changes_whether_it_landed() {
    let unattributed = Tree::new("capture-unattributed");
    unattributed.vault("home/vault", CONTRACT);

    let broken = vault("capture-verdict", CONTRACT);
    broken.write("home/vault/wrong.md", "---\ntype: nonesuch\n---\nbody\n");

    for (tree, identifier) in [
        (&unattributed, "write.actor-unattributed"),
        (&broken, "note.unknown-type"),
    ] {
        let finished = capture(tree, &["a loose thought"]);
        let observed = (
            finished.code,
            finished.stderr.contains(identifier),
            finished.stdout.contains("captured      captures/"),
        );
        assert_eq!(observed, (0, true, true), "{identifier}: {finished:?}");
    }
}

/// An unresolved contract is the family refusal, identical at every door: the
/// diagnostics, the pointer at `doctor`, and nothing on standard output.
#[test]
fn an_unresolved_contract_refuses_toward_doctor() {
    let tree = vault("capture-unresolved", TOO_NEW);
    let before = tree.listing();
    let finished = capture(&tree, &["a loose thought"]);
    assert_eq!(finished.code, 1, "{finished:?}");
    assert_eq!(finished.stdout, "");
    assert!(
        finished.stderr.contains("compat.contract-too-new"),
        "{finished:?}"
    );
    assert!(finished.stderr.contains("dogtag doctor"), "{finished:?}");
    assert_eq!(tree.listing(), before, "a refusal left the tree changed");
}

/// The thought is required: `capture` with none of the three inputs is clap's
/// kind of fault and takes clap's code.
#[test]
fn a_capture_with_no_thought_at_all_is_a_usage_fault() {
    let tree = vault("capture-no-thought", CONTRACT);
    let finished = capture(&tree, &[]);
    assert_eq!(finished.code, 2, "{finished:?}");
    assert_eq!(finished.stdout, "");
}

/// A file that is not there is the same kind of fault, for the same reason:
/// nothing was read, so there is nothing to say about a vault.
#[test]
fn a_thought_file_that_is_not_there_is_a_usage_fault() {
    let tree = vault("capture-no-file", CONTRACT);
    let finished = capture(&tree, &["--file", "nowhere-at-all.txt"]);
    assert_eq!(finished.code, 2, "{finished:?}");
    assert_eq!(finished.stdout, "");
    assert!(
        finished.stderr.contains("could not be read"),
        "{finished:?}"
    );
}

/// Two captures of one thought both land, and the second wears the suffix.
#[test]
fn two_captures_of_one_thought_both_land() {
    let tree = vault("capture-collision", CONTRACT);
    assert_eq!(capture(&tree, &["twice"]).code, 0);
    assert_eq!(capture(&tree, &["twice"]).code, 0);
    let captures: Vec<String> = tree
        .listing()
        .into_iter()
        .filter(|path| path.contains("/captures/"))
        .collect();
    assert_eq!(captures.len(), 2, "{captures:?}");
}

/// No colour reaches structured output, and a piped run carries no escapes at
/// all — the rule every surface in this crate follows.
#[test]
fn a_piped_run_carries_no_escape_sequences() {
    let tree = vault("capture-no-colour", CONTRACT);
    let finished = capture(&tree, &["a loose thought"]);
    assert!(!finished.has_escapes(), "{finished:?}");
}
