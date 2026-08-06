//! Plain-text rendering for search hits.

use crate::note::SearchResult;
use crate::text::one_line;

/// Renders one tab-separated line per hit: path, type, matched context.
///
/// The snippet column appears only where a hit has context beyond its path,
/// exactly as `list`'s lifecycle column appears only where an axis answers.
pub fn search_text(result: &SearchResult) -> String {
    let mut rendered = String::new();
    for hit in result.hits() {
        rendered.push_str(hit.path().as_str());
        rendered.push('\t');
        rendered.push_str(&one_line(hit.type_name().unwrap_or("")));
        if let Some(snippet) = hit.snippet() {
            rendered.push('\t');
            rendered.push_str(&one_line(snippet));
        }
        rendered.push('\n');
    }
    rendered
}
