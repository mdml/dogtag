//! Conventional Commits validation for this repository's history.
//!
//! The repository has always declared Conventional Commits; this crate is
//! what makes the declaration enforceable, in the commit-msg hook locally and
//! over a pull request's whole range in CI. It implements the subset of
//! [Conventional Commits 1.0.0](https://www.conventionalcommits.org) the
//! repository actually uses — a closed type list, optional scopes, and both
//! breaking-change forms — and deliberately nothing else: stylistic rules the
//! specification does not mandate (subject casing, trailing punctuation, line
//! width) are left to review, because a validator that rejects a legitimate
//! message teaches people to bypass it.
//!
//! Standard library only, in keeping with the dependency policy.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;

/// The commit types this repository permits, and nothing else.
///
/// A closed list is the point: an unrecognized type is nearly always a typo
/// or an invented category, and either way the release notes would silently
/// drop the commit into "other".
pub const ALLOWED_TYPES: [&str; 10] = [
    "feat", "fix", "docs", "test", "refactor", "perf", "build", "ci", "chore", "revert",
];

/// The separator the specification requires between header and description.
const SEPARATOR: &str = ": ";

/// The two spellings of the breaking-change footer token.
const BREAKING_TOKENS: [&str; 2] = ["BREAKING CHANGE", "BREAKING-CHANGE"];

/// A parsed, valid commit header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header<'a> {
    /// The commit type, guaranteed to be one of [`ALLOWED_TYPES`].
    pub kind: &'a str,
    /// The optional scope, guaranteed non-empty when present.
    pub scope: Option<&'a str>,
    /// Whether the commit declares a breaking change, by either the `!`
    /// marker or a `BREAKING CHANGE:` footer.
    pub breaking: bool,
    /// The description, guaranteed non-blank.
    pub description: &'a str,
}

/// Why a commit message is not a valid Conventional Commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// The message has no non-comment content.
    Empty,
    /// The subject line lacks the `": "` separator.
    MissingSeparator,
    /// The type is not one of [`ALLOWED_TYPES`].
    UnknownType(String),
    /// A scope was opened but the header is malformed around it.
    MalformedScope(String),
    /// The scope parentheses are empty.
    EmptyScope,
    /// The description after the separator is blank.
    EmptyDescription,
    /// The message is a `fixup!`/`squash!` commit, which must be squashed
    /// before it can land.
    Autosquash,
    /// A breaking-change footer token carries no description.
    EmptyBreakingFooter,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Problem::Empty => f.write_str("the commit message is empty"),
            Problem::MissingSeparator => write!(
                f,
                "the subject must be `<type>[(scope)][!]{SEPARATOR}<description>` \
                 (the `{SEPARATOR}` separator is missing)"
            ),
            Problem::UnknownType(found) => write!(
                f,
                "`{found}` is not an allowed type; use one of: {}",
                ALLOWED_TYPES.join(", ")
            ),
            Problem::MalformedScope(found) => {
                write!(
                    f,
                    "`{found}` has a malformed scope; write `type(scope): ...`"
                )
            }
            Problem::EmptyScope => f.write_str("the scope parentheses are empty"),
            Problem::EmptyDescription => f.write_str("the description is empty"),
            Problem::Autosquash => {
                f.write_str("`fixup!`/`squash!` commits must be autosquashed before landing")
            }
            Problem::EmptyBreakingFooter => {
                f.write_str("the breaking-change footer has no description")
            }
        }
    }
}

/// The scissors line git writes in verbose mode; nothing after it is part of
/// the message.
const SCISSORS: &str = "# ------------------------ >8";

/// The message's content lines, with exactly what git discards discarded:
/// comment lines, and everything from the scissors line on. The commit-msg
/// hook is handed the raw editor buffer, so skipping this would fail every
/// commit made with `--verbose`.
fn content_lines(message: &str) -> impl Iterator<Item = &str> {
    message
        .lines()
        .take_while(|line| !line.starts_with(SCISSORS))
        .filter(|line| !line.starts_with('#'))
}

/// The first non-blank content line: the subject the specification governs.
fn subject_of(message: &str) -> Option<&str> {
    content_lines(message).find(|line| !line.trim().is_empty())
}

/// Split a header into its type/scope part and the `!` breaking marker.
fn split_breaking_marker(head: &str) -> (&str, bool) {
    match head.strip_suffix('!') {
        Some(rest) => (rest, true),
        None => (head, false),
    }
}

/// Parse the `type` or `type(scope)` portion ahead of the separator.
fn parse_kind_and_scope(head: &str) -> Result<(&str, Option<&str>), Problem> {
    let Some(open) = head.find('(') else {
        return Ok((head, None));
    };
    let Some(close) = head.strip_suffix(')') else {
        return Err(Problem::MalformedScope(head.to_string()));
    };
    let scope = &close[open + 1..];
    if scope.is_empty() {
        return Err(Problem::EmptyScope);
    }
    Ok((&head[..open], Some(scope)))
}

/// Whether any footer line declares a breaking change, validating the token
/// when one is present.
fn breaking_footer(message: &str) -> Result<bool, Problem> {
    let mut found = false;
    for line in content_lines(message) {
        for token in BREAKING_TOKENS {
            let Some(rest) = line.strip_prefix(token) else {
                continue;
            };
            let Some(description) = rest.strip_prefix(':') else {
                continue;
            };
            if description.trim().is_empty() {
                return Err(Problem::EmptyBreakingFooter);
            }
            found = true;
        }
    }
    Ok(found)
}

/// Validate one commit message, returning its parsed header.
///
/// Comments are stripped first, so this accepts the raw buffer a commit-msg
/// hook is handed as readily as the stored message `git log` prints.
pub fn validate(message: &str) -> Result<Header<'_>, Problem> {
    let subject = subject_of(message).ok_or(Problem::Empty)?;

    if subject.starts_with("fixup!") || subject.starts_with("squash!") {
        return Err(Problem::Autosquash);
    }

    let (head, description) = subject
        .split_once(SEPARATOR)
        .ok_or(Problem::MissingSeparator)?;
    let description = description.trim();
    if description.is_empty() {
        return Err(Problem::EmptyDescription);
    }

    let (head, marked_breaking) = split_breaking_marker(head);
    let (kind, scope) = parse_kind_and_scope(head)?;
    if !ALLOWED_TYPES.contains(&kind) {
        return Err(Problem::UnknownType(kind.to_string()));
    }

    Ok(Header {
        kind,
        scope,
        breaking: marked_breaking || breaking_footer(message)?,
        description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(message: &str) -> Header<'_> {
        validate(message).expect("message should be valid")
    }

    fn problem(message: &str) -> Problem {
        validate(message).expect_err("message should be rejected")
    }

    #[test]
    fn plain_subject_parses_into_its_parts() {
        let h = header("feat: add the thing");
        assert_eq!(h.kind, "feat");
        assert_eq!(h.scope, None);
        assert_eq!(h.description, "add the thing");
        assert!(!h.breaking);
    }

    #[test]
    fn every_allowed_type_is_accepted() {
        for kind in ALLOWED_TYPES {
            let message = format!("{kind}: a description");
            assert_eq!(header(&message).kind, kind, "type `{kind}` must be allowed");
        }
    }

    #[test]
    fn scopes_and_breaking_markers_parse_in_every_combination() {
        let cases = [
            ("feat(cli): x", Some("cli"), false),
            ("feat!: x", None, true),
            ("feat(cli)!: x", Some("cli"), true),
            (
                "fix(conformance harness): x",
                Some("conformance harness"),
                false,
            ),
        ];
        for (message, scope, breaking) in cases {
            let h = header(message);
            assert_eq!(h.scope, scope, "scope of `{message}`");
            assert_eq!(h.breaking, breaking, "breaking flag of `{message}`");
        }
    }

    #[test]
    fn a_breaking_change_footer_marks_the_commit_breaking() {
        let message = "feat: add the thing\n\nBREAKING CHANGE: the old thing is gone\n";
        assert!(header(message).breaking);
        let hyphenated = "feat: add the thing\n\nBREAKING-CHANGE: the old thing is gone\n";
        assert!(header(hyphenated).breaking);
    }

    /// Only the footer *token* declares a breaking change. Prose that merely
    /// begins with the same words does not, or every commit discussing
    /// breaking changes would be marked as one.
    #[test]
    fn breaking_change_prose_without_the_colon_is_not_a_footer() {
        let message = concat!(
            "docs: explain the policy\n\n",
            "BREAKING CHANGES are listed in the release notes, not here.\n"
        );
        assert!(
            !header(message).breaking,
            "a line starting with the token but lacking `:` is prose, not a footer"
        );
    }

    #[test]
    fn a_description_may_itself_contain_the_separator() {
        assert_eq!(
            header("docs: note this: and that").description,
            "note this: and that"
        );
    }

    #[test]
    fn comments_and_scissors_content_are_ignored() {
        let message = "# please enter a message\nfeat: real subject\n";
        assert_eq!(header(message).description, "real subject");

        let scissors = concat!(
            "feat: real subject\n",
            "# ------------------------ >8 ------------------------\n",
            "nonsense that git would discard\n"
        );
        assert_eq!(header(scissors).kind, "feat");
    }

    #[test]
    fn each_malformed_message_reports_its_own_problem() {
        let cases = [
            ("", Problem::Empty),
            ("#only a comment\n", Problem::Empty),
            ("feat add the thing", Problem::MissingSeparator),
            ("feat:no space", Problem::MissingSeparator),
            (
                "wip: add the thing",
                Problem::UnknownType("wip".to_string()),
            ),
            (
                "Feat: add the thing",
                Problem::UnknownType("Feat".to_string()),
            ),
            ("feat(): x", Problem::EmptyScope),
            (
                "feat(cli: x",
                Problem::MalformedScope("feat(cli".to_string()),
            ),
            ("feat:    ", Problem::EmptyDescription),
            ("fixup! feat: x", Problem::Autosquash),
            ("squash! feat: x", Problem::Autosquash),
            (
                "feat: x\n\nBREAKING CHANGE:   \n",
                Problem::EmptyBreakingFooter,
            ),
        ];
        for (message, expected) in cases {
            assert_eq!(problem(message), expected, "for message {message:?}");
        }
    }

    #[test]
    fn every_problem_renders_an_actionable_message() {
        let rendered = [
            Problem::Empty,
            Problem::MissingSeparator,
            Problem::UnknownType("wip".to_string()),
            Problem::MalformedScope("feat(cli".to_string()),
            Problem::EmptyScope,
            Problem::EmptyDescription,
            Problem::Autosquash,
            Problem::EmptyBreakingFooter,
        ];
        for problem in rendered {
            let text = problem.to_string();
            assert!(!text.is_empty(), "{problem:?} renders nothing");
            assert!(
                text.chars().next().is_some_and(char::is_lowercase) || text.starts_with('`'),
                "{problem:?} should read as a lowercase clause: {text}"
            );
        }
        assert!(
            Problem::UnknownType("wip".into())
                .to_string()
                .contains("feat")
        );
    }

    #[test]
    fn the_subject_is_the_first_non_blank_line() {
        assert_eq!(header("\n\nfeat: after blanks").description, "after blanks");
    }

    #[test]
    fn trailing_whitespace_on_the_subject_does_not_break_parsing() {
        assert_eq!(header("feat: trailing   ").description, "trailing");
    }
}
