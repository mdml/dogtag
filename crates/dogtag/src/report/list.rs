//! Plain-text rendering for corpus summaries.

use crate::note::ListResult;
use crate::text::one_line;

/// Renders one tab-separated line per matching note.
pub fn list_text(result: &ListResult) -> String {
    let mut rendered = String::new();
    for note in result.notes() {
        rendered.push_str(note.path().as_str());
        rendered.push('\t');
        rendered.push_str(&one_line(note.type_name().unwrap_or("")));
        if let Some(value) = note.lifecycle() {
            rendered.push('\t');
            rendered.push_str(&one_line(value));
        }
        rendered.push('\n');
    }
    rendered
}
