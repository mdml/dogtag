//! Plain-text rendering for entity lookup.

use crate::note::FindResult;

use super::list::summary_line;

/// Renders the one found note as the tab-separated summary line `list` uses.
///
/// A refusal — an ambiguous name, a name nothing bears — has no summary to
/// render and produces an empty string, exactly as `show`'s text does; the
/// candidates live in the diagnostic's related evidence.
pub fn find_text(result: &FindResult) -> String {
    result.note().map(summary_line).unwrap_or_default()
}
