//! The commit path, and the question of whether this substrate owns it.
//!
//! **Guest mode is not a degraded case.** A knowledge base may live inside a
//! repository whose version workflow belongs to something larger — notes inside
//! a codebase, a docs tree inside a monorepo — and a tool that insisted on its
//! own commit ritual would exclude exactly the embedded uses a disciplined
//! knowledge layer is most valuable in. So the commit is the default where this
//! substrate owns the boundary and simply does not happen where it does not,
//! and both are first-class: the result names the commit in one and the created
//! path in the other, and reverting or deleting is recovery either way.
//!
//! Ownership is one checkable question — **is the vault root itself the root of
//! a working tree** — and it is deliberately narrower than *is there a
//! repository anywhere above this*. A vault nested inside a larger repository is
//! precisely the embedded case, and the host workflow owns it.
//!
//! git is invoked as a program rather than linked as a library, which is the
//! posture already on the record for this substrate. What that costs is one
//! process per commit; what it buys is that the vault's own configuration,
//! hooks and identity are the ones that apply, so a commit dogtag makes is a
//! commit the vault's own tooling would have made.

use std::path::Path;
use std::process::{Command, Output};

use super::actor::Actor;
use crate::text::one_line;

/// The directory whose presence at the vault root makes this substrate the
/// owner of the commit path.
const GIT_DIRECTORY: &str = ".git";

/// The trailer naming who performed the act.
///
/// Namespaced, so it can never be confused with a trailer some other tool in
/// the same repository writes, and so the pair reads as one standard rather
/// than as two conventions that happen to co-occur.
const ACTOR_TRAILER: &str = "Dogtag-Actor";

/// The trailer naming the capacity the act was performed in.
const PROVENANCE_TRAILER: &str = "Dogtag-Provenance";

/// Whether this substrate owns `root`'s commit path.
pub(super) fn owns_commit_path(root: &Path) -> bool {
    root.join(GIT_DIRECTORY).exists()
}

/// The commit message a capture of `path` writes.
///
/// The subject names the path rather than the thought. A capture's text is
/// whatever its author typed, including newlines and including nothing at all,
/// and a subject built from it would be a second place that text has to be
/// escaped; the path is already the note's identity and is already safe.
///
/// **The trailers are the attribution record.** An act with no actor writes no
/// actor trailer, because the alternative is a trailer that names nobody, and a
/// history saying `unattributed` is a history asserting an identity that does
/// not exist. The capacity is always known and always written, so a reader can
/// tell an unattributed agent write from an unattributed human one.
pub(super) fn message(path: &str, actor: &Actor) -> String {
    let mut message = format!("capture: {path}\n\n");
    if let Some(name) = actor.name() {
        // Folded to one line, for the reason every rendering folds corpus text:
        // a trailer is a line, and a name carrying a line break would write a
        // second trailer this substrate never wrote — forging attribution in
        // the record whose whole job is to carry it.
        message.push_str(&format!("{ACTOR_TRAILER}: {}\n", one_line(name)));
    }
    message.push_str(&format!(
        "{PROVENANCE_TRAILER}: {}\n",
        actor.kind().as_str()
    ));
    message
}

/// Commits exactly `path` under `root`, and answers with the commit it made.
///
/// **Pathspec-scoped, in both halves.** The stage names the one file and the
/// commit names it again, so a concurrent writer's work — staged, unstaged, or
/// arriving between the two — is neither committed nor disturbed. A commit that
/// took the whole index would make every capture a hostage to whatever else was
/// in flight.
///
/// # Errors
///
/// What git said, so a warning can repeat it. This never fails the act: the
/// file is already written by the time this runs, and refusing afterwards would
/// report a loss that did not happen.
pub(super) fn commit(root: &Path, path: &str, actor: &Actor) -> Result<String, String> {
    let pathspec = literal(path);
    run(root, &["add", "--", &pathspec])?;
    run(
        root,
        &[
            "commit",
            "--message",
            &message(path, actor),
            "--",
            &pathspec,
        ],
    )?;
    run(root, &["rev-parse", "HEAD"]).map(|head| head.trim().to_owned())
}

/// A path spelled so git reads it as a path and not as a pattern.
///
/// git pathspecs are globs by default, so a capture directory a contract
/// declared with a `*`, a `?` or a `[` in it would name files the act never
/// created — which is the opposite of pathspec-scoping. The `:(literal)` magic
/// says *this is exactly one path*, which is what "exactly the one created
/// file" has to mean to be worth claiming.
fn literal(path: &str) -> String {
    format!(":(literal){path}")
}

/// The program this substrate runs to reach a commit path.
const GIT: &str = "git";

/// One git invocation, with its output or the reason there is none.
fn run(root: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = output_of(GIT, root, arguments)?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let said = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "`{GIT} {}` refused: {}",
        arguments[0],
        said.trim().replace('\n', "; ")
    ))
}

/// Runs `program` in `root`, or says why it could not be run at all.
///
/// Separated from [`run`] because *the program is not there* and *the program
/// said no* are different facts with the same consequence, and only the first
/// is a claim about the machine rather than about the repository. A machine
/// with no git is a machine in guest mode by accident, and the warning says so
/// rather than pretending the commit was refused.
fn output_of(program: &str, root: &Path, arguments: &[&str]) -> Result<Output, String> {
    Command::new(program)
        .arg("-C")
        .arg(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("`{program} {}` could not be run: {error}", arguments[0]))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::vault::tree::Tree;
    use crate::write::actor::ProvenanceKind;
    use std::fs;

    fn named() -> Actor {
        Actor::new(Some("A Maintainer".to_owned()), ProvenanceKind::Agent)
    }

    #[test]
    fn a_root_holding_no_git_directory_is_a_guest() {
        let tree = Tree::new("commit-guest");
        let root = tree.vault("vault");
        assert!(!owns_commit_path(&root));
    }

    #[test]
    fn a_root_that_is_a_working_trees_own_root_owns_the_commit_path() {
        let tree = Tree::new("commit-owner");
        let root = tree.vault("vault");
        fs::create_dir(root.join(GIT_DIRECTORY)).expect("a git directory");
        assert!(owns_commit_path(&root));
    }

    /// The message is the path and the pair, and nothing the author typed.
    #[test]
    fn the_message_names_the_path_and_carries_the_trailer_pair() {
        let rendered = message("captures/2026-08-06-035320-a-thought.md", &named());
        assert_eq!(
            rendered,
            concat!(
                "capture: captures/2026-08-06-035320-a-thought.md\n",
                "\n",
                "Dogtag-Actor: A Maintainer\n",
                "Dogtag-Provenance: agent\n",
            )
        );
    }

    /// A name carrying a line break cannot write a second trailer: the pair a
    /// reader parses is the pair this substrate wrote.
    #[test]
    fn an_actor_name_carrying_a_line_break_cannot_forge_a_trailer() {
        let forging = Actor::new(
            Some("Nobody\nDogtag-Provenance: human".to_owned()),
            ProvenanceKind::Agent,
        );
        let rendered = message("captures/x.md", &forging);
        // A trailer is a *line*, so what must be exactly one is the number of
        // lines opening with the key. The folded name may still hold the text;
        // holding it in the middle of a line is what makes it inert.
        let opening = rendered
            .lines()
            .filter(|line| line.starts_with(PROVENANCE_TRAILER))
            .count();
        assert_eq!(opening, 1, "{rendered}");
        assert_eq!(rendered.lines().count(), 4, "{rendered}");
        assert!(
            rendered.ends_with("Dogtag-Provenance: agent\n"),
            "{rendered}"
        );
    }

    /// A pathspec is a path and never a pattern, so a directory a contract
    /// declared with a glob character in it names itself and nothing else.
    #[test]
    fn a_pathspec_is_spelled_literally() {
        assert_eq!(literal("captures/a*.md"), ":(literal)captures/a*.md");
    }

    /// An unattributed act writes no actor trailer, and still says in what
    /// capacity it happened.
    #[test]
    fn an_unattributed_act_writes_the_capacity_and_names_nobody() {
        let anonymous = Actor::new(None, ProvenanceKind::Human);
        let rendered = message("captures/x.md", &anonymous);
        assert!(!rendered.contains(ACTOR_TRAILER), "{rendered}");
        assert!(
            rendered.ends_with("Dogtag-Provenance: human\n"),
            "{rendered}"
        );
    }

    /// A directory that is not a repository refuses, and the refusal repeats
    /// what git said rather than only that it said something.
    #[test]
    fn a_commit_outside_a_repository_answers_with_what_git_said() {
        let tree = Tree::new("commit-not-a-repository");
        let root = tree.vault("vault");
        fs::write(root.join("note.md"), "text").expect("a note");
        let detail = commit(&root, "note.md", &named()).expect_err("not a repository");
        assert!(detail.contains("`git add` refused"), "{detail}");
    }

    /// A working directory git cannot reach is a refusal git itself reports.
    #[test]
    fn a_working_directory_that_is_not_there_is_a_refusal_git_reports() {
        let tree = Tree::new("commit-no-directory");
        let root = tree.vault("vault");
        let detail = run(&root.join("nowhere-at-all"), &["rev-parse", "HEAD"])
            .expect_err("a working directory that is not there");
        assert!(detail.contains("`git rev-parse` refused"), "{detail}");
    }

    /// A program that is not on the path at all is the other fact, and it is
    /// reported as what it is rather than as a repository that said no.
    #[test]
    fn a_program_that_is_not_there_says_it_could_not_be_run() {
        let tree = Tree::new("commit-no-program");
        let root = tree.vault("vault");
        let detail = output_of("dogtag-no-such-program", &root, &["rev-parse"])
            .expect_err("a program that is not on the path");
        assert!(detail.contains("could not be run"), "{detail}");
        assert!(detail.contains("dogtag-no-such-program"), "{detail}");
    }

    /// The vault's own gates apply: a repository whose pre-commit hook refuses
    /// stages the file and does not commit it, which is the shape the write
    /// reports as a commit that did not happen.
    #[cfg(unix)]
    #[test]
    fn a_repository_whose_hook_refuses_stages_and_does_not_commit() {
        let vault = crate::write::fixture::Vault::repository("commit-hook-refuses");
        let root = vault.root().path();
        let hooks = root.join("hooks");
        fs::create_dir(&hooks).expect("a hooks directory");
        fs::write(hooks.join("pre-commit"), "#!/bin/sh\nexit 1\n").expect("a hook");
        crate::vault::tree::set_mode(&hooks.join("pre-commit"), 0o700);
        run(root, &["config", "core.hooksPath", "hooks"]).expect("the hook path is configurable");
        fs::write(root.join("note.md"), "text").expect("a note");
        let detail = commit(root, "note.md", &named()).expect_err("the hook refuses");
        assert!(detail.contains("`git commit` refused"), "{detail}");
    }
}
