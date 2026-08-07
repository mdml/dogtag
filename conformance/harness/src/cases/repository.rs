//! Turning a corpus copy into a repository, and reading one back.
//!
//! One scenario is about the commit, so one scenario needs a repository. The
//! copies are not repositories by default and that is the point — every other
//! write case therefore exercises **guest mode** by construction, which is a
//! first-class product stance rather than a degraded case, and asserting git
//! state in all of them would have made the commit-less path the exception.
//!
//! Constructing one is a derivation like any other: it happens inside the
//! per-pair copy, the checked-in corpora are untouched, and the identity is
//! configured on the repository rather than read from the machine — so a
//! contributor with no global git identity runs this case exactly as one with
//! an identity does.

use std::path::Path;
use std::process::Command;

use super::corpus::Corpus;
use super::expect::{Checked, require};

/// What a fresh repository must be told before it can commit.
const PREPARATION: &[&[&str]] = &[
    &["init", "--quiet"],
    &["config", "user.name", "A Conformance Run"],
    &["config", "user.email", "conformance@example.invalid"],
    &["config", "commit.gpgsign", "false"],
];

/// Makes `corpus`'s copy a repository whose commit path the SDK therefore owns.
///
/// # Errors
///
/// Whatever git said, or the reason it could not be run — either of which makes
/// the case's subject absent rather than failing.
pub fn construct(corpus: &Corpus) -> Checked {
    for arguments in PREPARATION {
        git(corpus.root(), arguments)?;
    }
    require(corpus.root().join(".git").exists(), || {
        "constructing a repository left no `.git` at the copy's root, so the SDK would not own \
         its commit path and this case would test guest mode instead"
            .to_owned()
    })
}

/// One git invocation inside `root`, with its output.
///
/// # Errors
///
/// What git said, so a failing case repeats it rather than only reporting that
/// something went wrong.
pub fn git(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("`git {}` could not be run: {error}", arguments[0]))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    Err(format!(
        "`git {}` refused: {}",
        arguments[0],
        String::from_utf8_lossy(&output.stderr)
            .trim()
            .replace('\n', "; ")
    ))
}

/// Everything one commit says: its message, and the paths it contains.
///
/// # Errors
///
/// What git said about a commit that is not there or cannot be read.
pub fn contents(root: &Path, commit: &str) -> Result<(String, Vec<String>), String> {
    let message = git(root, &["show", "--no-patch", "--format=%B", commit])?;
    let files = git(root, &["show", "--name-only", "--format=", commit])?;
    let paths = files
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    Ok((message, paths))
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::corpus::{Corpus, NO_AXIS};

    /// A copy that has not been constructed is a guest, and git says so rather
    /// than the harness guessing.
    #[test]
    fn a_copy_that_is_not_a_repository_answers_with_what_git_said() {
        let corpus = Corpus::holding("repository-absent", NO_AXIS);
        let detail = git(corpus.root(), &["rev-parse", "HEAD"])
            .expect_err("a directory that is not a repository");
        assert!(detail.contains("`git rev-parse` refused"), "{detail}");
    }

    /// A constructed copy owns its commit path, and its first commit says what
    /// it contains.
    #[test]
    fn a_constructed_copy_commits_and_reports_what_the_commit_holds() {
        let corpus = Corpus::holding("repository-constructed", NO_AXIS);
        construct(&corpus).expect("a repository");
        std::fs::write(corpus.root().join("planted.md"), "text").expect("a note");
        git(corpus.root(), &["add", "--", "planted.md"]).expect("staging");
        git(corpus.root(), &["commit", "--message", "planted"]).expect("committing");
        let head = git(corpus.root(), &["rev-parse", "HEAD"]).expect("a head");
        let (message, paths) = contents(corpus.root(), head.trim()).expect("the commit");
        assert!(message.contains("planted"), "{message}");
        assert_eq!(paths, ["planted.md"]);
    }

    /// A commit that is not there is a refusal git reports, so a case reading
    /// one back says what happened rather than panicking.
    #[test]
    fn a_commit_that_is_not_there_says_what_git_said() {
        let corpus = Corpus::holding("repository-no-commit", NO_AXIS);
        construct(&corpus).expect("a repository");
        let detail =
            contents(corpus.root(), "HEAD").expect_err("a repository with no commit in it");
        assert!(detail.contains("`git show` refused"), "{detail}");
    }
}
