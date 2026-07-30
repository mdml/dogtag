//! The `commit-lint` binary: validate commit messages as Conventional Commits.
//!
//! Two modes, matching the two moments the rule is enforced:
//!
//! ```text
//! commit-lint <path>            validate the message in a file (commit-msg hook)
//! commit-lint --range <RANGE>   validate every commit in a git range (CI)
//! ```
//!
//! The range form is what makes the local hook non-optional: a contributor who
//! bypasses the hook with `--no-verify` still has to face the same check over
//! the whole pull request before it can merge.

#![forbid(unsafe_code)]

use std::fs;
use std::process::{Command, ExitCode};

use commit_lint::validate;

/// Field and record separators for the `git log` format below — control
/// characters cannot occur in a commit message, so parsing never guesses.
const UNIT: char = '\u{1f}';
const RECORD: char = '\u{1e}';

const USAGE: &str = "usage: commit-lint <message-file> | commit-lint --range <RANGE>";

/// One commit to check: its identity for reporting, and its message.
struct Commit {
    label: String,
    message: String,
}

/// Read the commits in a git range, skipping merges — a merge commit has no
/// authored message of its own to hold to the standard.
fn commits_in_range(range: &str) -> Result<Vec<Commit>, String> {
    let output = Command::new("git")
        .args([
            "log",
            "--no-merges",
            &format!("--format=%h %s{UNIT}%B{RECORD}"),
            range,
        ])
        .output()
        .map_err(|e| format!("could not run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git log {range} failed: {}", stderr.trim()));
    }

    let text = String::from_utf8(output.stdout)
        .map_err(|e| format!("git produced non-UTF-8 output: {e}"))?;

    Ok(text
        .split(RECORD)
        .filter_map(|record| record.split_once(UNIT))
        .map(|(label, message)| Commit {
            label: label.trim().to_string(),
            message: message.to_string(),
        })
        .filter(|commit| !commit.label.is_empty())
        .collect())
}

/// Check every commit, reporting each failure, and return whether all passed.
fn check_all(commits: &[Commit]) -> bool {
    let mut ok = true;
    for commit in commits {
        if let Err(problem) = validate(&commit.message) {
            eprintln!("commit-lint: {}: {problem}", commit.label);
            ok = false;
        }
    }
    ok
}

/// Check the single message a commit-msg hook hands over.
fn check_message_file(path: &str) -> Result<bool, String> {
    let message = fs::read_to_string(path).map_err(|e| format!("could not read {path}: {e}"))?;
    Ok(check_all(&[Commit {
        label: path.to_string(),
        message,
    }]))
}

/// Check every authored commit in a range, reporting the count when clean so
/// a passing CI log still shows what was covered.
fn check_range(range: &str) -> Result<bool, String> {
    let commits = commits_in_range(range)?;
    let passed = check_all(&commits);
    if passed {
        println!(
            "commit-lint: {} commit(s) are Conventional Commits.",
            commits.len()
        );
    }
    Ok(passed)
}

fn run(args: &[String]) -> Result<bool, String> {
    match args {
        [path] if !path.starts_with('-') => check_message_file(path),
        [flag, range] if flag == "--range" => check_range(range),
        _ => Err(USAGE.to_string()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => {
            eprintln!(
                "commit-lint: see https://www.conventionalcommits.org — subjects are \
                 `<type>[(scope)][!]: <description>`"
            );
            ExitCode::FAILURE
        }
        Err(message) => {
            eprintln!("commit-lint: {message}");
            ExitCode::from(2)
        }
    }
}
