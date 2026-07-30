//! Integration tests for the `commit-lint` binary, run against the built
//! executable via `CARGO_BIN_EXE_commit-lint` (no external test-harness
//! crates, per the dependency policy).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};

/// A throwaway directory under the system temp dir, removed on drop. Same
/// pattern as the conformance harness's tests — standard library only.
struct TempTree(PathBuf);

impl TempTree {
    fn new(label: &str) -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "commit-lint-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("temp tree created");
        TempTree(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_commit-lint"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run the commit-lint binary")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr was not UTF-8")
}

/// Run `git` in a fixture repository, failing loudly rather than silently
/// producing a repository in an unexpected state.
fn git(tree: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(tree)
        .env("GIT_AUTHOR_NAME", "test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .expect("failed to run git");
    assert!(output.status.success(), "git {args:?} failed: {output:?}");
}

/// A repository with one valid commit on top of a root commit, plus whatever
/// subjects the caller adds.
fn repo_with_commits(label: &str, subjects: &[&str]) -> TempTree {
    let tree = TempTree::new(label);
    git(tree.path(), &["init", "--quiet", "--initial-branch=main"]);
    fs::write(tree.path().join("file.txt"), "root\n").expect("file written");
    git(tree.path(), &["add", "-A"]);
    git(
        tree.path(),
        &["commit", "--quiet", "-m", "chore: root commit"],
    );
    for (i, subject) in subjects.iter().enumerate() {
        fs::write(tree.path().join("file.txt"), format!("change {i}\n")).expect("file written");
        git(tree.path(), &["add", "-A"]);
        git(tree.path(), &["commit", "--quiet", "-m", subject]);
    }
    tree
}

/// File mode: a conforming message is accepted and says nothing.
#[test]
fn a_valid_message_file_is_accepted_quietly() {
    let tree = TempTree::new("valid-file");
    let path = tree.path().join("COMMIT_EDITMSG");
    fs::write(&path, "feat(cli): add the thing\n").expect("message written");

    let output = run(&[path.to_str().expect("utf-8 path")], tree.path());
    assert!(output.status.success(), "expected exit 0: {output:?}");
    assert!(output.stderr.is_empty(), "expected silence: {output:?}");
}

/// File mode: a non-conforming message is rejected, and the diagnostic names
/// both the offending file and the specific rule broken.
#[test]
fn an_invalid_message_file_is_rejected_with_a_reason() {
    let tree = TempTree::new("invalid-file");
    let path = tree.path().join("COMMIT_EDITMSG");
    fs::write(&path, "wip: something\n").expect("message written");

    let output = run(&[path.to_str().expect("utf-8 path")], tree.path());
    assert_eq!(output.status.code(), Some(1), "expected exit 1: {output:?}");
    let stderr = stderr_of(&output);
    assert!(
        stderr.contains("COMMIT_EDITMSG"),
        "names the file: {stderr}"
    );
    assert!(
        stderr.contains("`wip` is not an allowed type"),
        "names the rule: {stderr}"
    );
}

/// Each rule's diagnostic survives the trip through the binary, not just the
/// library: the hook's usefulness is entirely in what it prints.
#[test]
fn rejections_name_the_specific_rule_broken() {
    let tree = TempTree::new("rule-messages");
    let cases = [
        ("feat(): x\n", "scope parentheses are empty"),
        ("feat add the thing\n", "separator is missing"),
        ("fixup! feat: x\n", "autosquashed"),
        (
            "feat: x\n\nBREAKING CHANGE:\n",
            "breaking-change footer has no description",
        ),
    ];
    for (message, expected) in cases {
        let path = tree.path().join("COMMIT_EDITMSG");
        fs::write(&path, message).expect("message written");
        let output = run(&[path.to_str().expect("utf-8 path")], tree.path());
        assert_eq!(output.status.code(), Some(1), "for {message:?}: {output:?}");
        let stderr = stderr_of(&output);
        assert!(
            stderr.contains(expected),
            "for {message:?} expected {expected:?} in: {stderr}"
        );
    }
}

/// A message file that does not exist is a usage failure (exit 2), distinct
/// from a commit that simply does not conform (exit 1).
#[test]
fn a_missing_message_file_exits_two() {
    let tree = TempTree::new("missing-file");
    let output = run(&["definitely-not-here.txt"], tree.path());
    assert_eq!(output.status.code(), Some(2), "expected exit 2: {output:?}");
    assert!(stderr_of(&output).contains("could not read"));
}

/// Wrong arguments are a usage failure, not a validation verdict.
#[test]
fn bad_arguments_exit_two_with_usage() {
    let tree = TempTree::new("bad-args");
    for args in [vec![], vec!["--range"], vec!["--nonsense", "x"]] {
        let output = run(&args, tree.path());
        assert_eq!(output.status.code(), Some(2), "for {args:?}: {output:?}");
        assert!(stderr_of(&output).contains("usage:"), "for {args:?}");
    }
}

/// Range mode: every commit in the range is checked, and a clean range
/// reports how many it verified.
#[test]
fn a_clean_range_reports_the_number_of_commits_checked() {
    let tree = repo_with_commits("clean-range", &["feat: one", "fix(cli): two"]);
    let output = run(&["--range", "HEAD~2..HEAD"], tree.path());
    assert!(output.status.success(), "expected exit 0: {output:?}");
    let stdout = String::from_utf8(output.stdout).expect("stdout was not UTF-8");
    assert!(
        stdout.contains("2 commit(s)"),
        "counts the commits: {stdout}"
    );
}

/// Range mode is what makes the local hook non-optional: a commit that
/// bypassed the hook is still caught, and named by hash and subject.
#[test]
fn a_bad_commit_anywhere_in_the_range_fails_the_check() {
    let tree = repo_with_commits("dirty-range", &["feat: fine", "nope: bad", "fix: fine too"]);
    let output = run(&["--range", "HEAD~3..HEAD"], tree.path());
    assert_eq!(output.status.code(), Some(1), "expected exit 1: {output:?}");
    let stderr = stderr_of(&output);
    assert!(stderr.contains("nope: bad"), "names the commit: {stderr}");
    assert!(
        stderr.contains("not an allowed type"),
        "names the rule: {stderr}"
    );
    assert!(
        !stderr.contains("feat: fine"),
        "does not implicate the good commits: {stderr}"
    );
}

/// Merge commits carry no authored subject of their own, so they are skipped
/// rather than failed.
#[test]
fn merge_commits_are_skipped() {
    let tree = repo_with_commits("merges", &["feat: on main"]);
    git(
        tree.path(),
        &["checkout", "--quiet", "-b", "side", "HEAD~1"],
    );
    fs::write(tree.path().join("other.txt"), "side\n").expect("file written");
    git(tree.path(), &["add", "-A"]);
    git(
        tree.path(),
        &["commit", "--quiet", "-m", "feat: on the side"],
    );
    git(tree.path(), &["checkout", "--quiet", "main"]);
    git(
        tree.path(),
        &[
            "merge",
            "--quiet",
            "--no-ff",
            "-m",
            "Merge branch 'side'",
            "side",
        ],
    );

    // HEAD~2 is the root commit along the first-parent walk, so this range
    // spans both branch commits and the merge that joined them.
    let output = run(&["--range", "HEAD~2..HEAD"], tree.path());
    assert!(
        output.status.success(),
        "the merge commit's non-conforming subject must be skipped: {output:?}"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout was not UTF-8");
    assert!(
        stdout.contains("2 commit(s)"),
        "the two authored commits are checked and the merge is not: {stdout}"
    );
}

/// An unusable range is a tool failure (exit 2), not a verdict about commits.
#[test]
fn an_invalid_range_exits_two() {
    let tree = repo_with_commits("bad-range", &[]);
    let output = run(&["--range", "no-such-ref..HEAD"], tree.path());
    assert_eq!(output.status.code(), Some(2), "expected exit 2: {output:?}");
    assert!(stderr_of(&output).contains("failed"));
}
