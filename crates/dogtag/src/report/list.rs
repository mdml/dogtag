//! Plain-text rendering for corpus summaries.

use crate::note::{ListResult, NoteSummary};
use crate::text::one_line;

/// Renders one tab-separated line per matching note.
pub fn list_text(result: &ListResult) -> String {
    result.notes().iter().map(summary_line).collect()
}

/// One summary as one tab-separated line: path, type, and the axis value
/// where an axis answers.
///
/// Shared with `find`, whose one answer is a summary rendered exactly as
/// `list` renders each of its own.
pub(super) fn summary_line(note: &NoteSummary) -> String {
    let mut rendered = String::new();
    rendered.push_str(note.path().as_str());
    rendered.push('\t');
    rendered.push_str(&one_line(note.type_name().unwrap_or("")));
    if let Some(value) = note.lifecycle() {
        rendered.push('\t');
        rendered.push_str(&one_line(value));
    }
    rendered.push('\n');
    rendered
}
