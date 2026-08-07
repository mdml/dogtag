//! Plain-text rendering for a write transaction's result.
//!
//! Three lines at most, and every one of them is something a caller acts on:
//! what the act did, where the note is, and how to undo it. The findings
//! themselves are diagnostics and travel on the diagnostic stream, exactly as
//! `check`'s do; this rendering is the result.

use crate::text::one_line;
use crate::write::{Outcome, Recovery, WriteResult};

/// Renders what a write did, as SDK-owned plain text.
///
/// A preview says what it would have created rather than what it created, and
/// it is the one outcome with no recovery line, because there is nothing to
/// recover from.
pub fn capture_text(result: &WriteResult) -> String {
    let mut rendered = String::new();
    match result.outcome() {
        Outcome::Previewed => {
            rendered.push_str("preview: nothing written\n");
            for intended in result.plan().scope() {
                rendered.push_str(&format!("would create   {}\n", intended.as_str()));
            }
        }
        Outcome::Committed { path, commit } => {
            rendered.push_str(&format!("captured      {}\n", path.as_str()));
            rendered.push_str(&format!("committed     {}\n", one_line(commit)));
        }
        Outcome::Created { path } => {
            rendered.push_str(&format!("captured      {}\n", path.as_str()));
            rendered.push_str("committed     no\n");
        }
        Outcome::Refused => rendered.push_str("refused: nothing written\n"),
    }
    if let Some(recovery) = result.recovery() {
        rendered.push_str(&format!("recover by    {}\n", recovered(&recovery)));
    }
    rendered
}

/// The one instruction that undoes what landed.
fn recovered(recovery: &Recovery) -> String {
    match recovery {
        Recovery::Revert { commit, .. } => {
            format!("reverting {}", one_line(commit))
        }
        Recovery::Delete { path } => format!("deleting {}", path.as_str()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::write::fixture::{CLOSED, Thought, Vault};

    /// Every rendering carries the note's path, which is the one fact a caller
    /// needs in order to read what it just wrote.
    #[test]
    fn a_capture_that_landed_names_the_note_and_how_to_undo_it() {
        let vault = Vault::new("capture-text-created");
        let rendered = capture_text(&vault.capture(Thought("a loose thought")));
        assert!(rendered.contains("captured      captures/"), "{rendered}");
        assert!(rendered.contains("committed     no\n"), "{rendered}");
        assert!(
            rendered.contains("recover by    deleting captures/"),
            "{rendered}"
        );
    }

    /// A preview says what it *would* create, and offers no recovery, because
    /// nothing happened to recover from.
    #[test]
    fn a_preview_says_what_it_would_have_created_and_offers_no_recovery() {
        let vault = Vault::new("capture-text-preview");
        let rendered = capture_text(&vault.preview(Thought("a loose thought")));
        assert!(
            rendered.starts_with("preview: nothing written\n"),
            "{rendered}"
        );
        assert!(rendered.contains("would create   captures/"), "{rendered}");
        assert!(!rendered.contains("recover by"), "{rendered}");
    }

    #[test]
    fn a_refusal_says_so_and_names_nothing() {
        let vault = Vault::holding("capture-text-refused", CLOSED);
        let rendered = capture_text(&vault.capture(Thought("a loose thought")));
        assert_eq!(rendered, "refused: nothing written\n");
    }

    /// The committed rendering names the commit twice — once as what happened
    /// and once as what to revert — because those are two different questions a
    /// reader asks.
    #[test]
    fn a_committed_capture_names_the_commit_and_reverting_it() {
        let vault = Vault::repository("capture-text-committed");
        let rendered = capture_text(&vault.capture(Thought("a loose thought")));
        assert!(rendered.contains("captured      captures/"), "{rendered}");
        assert!(rendered.contains("committed     "), "{rendered}");
        assert!(rendered.contains("recover by    reverting "), "{rendered}");
        assert!(!rendered.contains("committed     no"), "{rendered}");
    }

    /// Identical input renders identically, which is what a caller diffing two
    /// runs relies on.
    #[test]
    fn the_rendering_is_a_function_of_the_result_alone() {
        let vault = Vault::new("capture-text-deterministic");
        let result = vault.capture(Thought("twice"));
        assert_eq!(capture_text(&result), capture_text(&result));
    }
}
