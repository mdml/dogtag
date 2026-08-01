//! Reading names back out of a rendering.
//!
//! The explain case has to assert both directions — every declaration appears,
//! and nothing that is not a declaration appears — so it needs the names a
//! rendering actually carries, not only whether a name it was looking for is
//! somewhere in the text. These helpers are deliberately lexical: they read
//! the rendering the way a person would, which is the point of asserting on a
//! rendering at all.

/// Every value of `"<key>": "<value>"` in a JSON document, in document order.
pub fn json_strings(json: &str, key: &str) -> Vec<String> {
    let opening = format!("\"{key}\": \"");
    let mut found = Vec::new();
    let mut rest = json;
    while let Some(at) = rest.find(&opening) {
        rest = &rest[at + opening.len()..];
        let Some(end) = rest.find('"') else { break };
        found.push(rest[..end].to_owned());
        rest = &rest[end..];
    }
    found
}

/// The name in the first pair of backticks on each line starting with `prefix`.
pub fn backticked_after(text: &str, prefix: &str) -> Vec<String> {
    text.lines()
        .filter(|line| line.starts_with(prefix))
        .filter_map(backticked)
        .collect()
}

/// The name in the first pair of backticks in a table row's first cell.
///
/// Header rows (`| property | kind |`) and rules (`| --- |`) carry no
/// backticks and so contribute nothing.
pub fn row_labels(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| line.starts_with("| `"))
        .filter_map(backticked)
        .collect()
}

/// One Markdown section's body: everything under `## <heading>` up to the next
/// second-level heading.
pub fn section<'a>(markdown: &'a str, heading: &str) -> Option<&'a str> {
    let marker = format!("\n## {heading}\n");
    let start = markdown.find(&marker)? + marker.len();
    let rest = &markdown[start..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    Some(&rest[..end])
}

/// A table row's cells, trimmed, including the empty ones at each end.
pub fn cells(line: &str) -> Vec<&str> {
    line.split('|').map(str::trim).collect()
}

/// The text between the first pair of backticks.
fn backticked(line: &str) -> Option<String> {
    let after = line.split_once('`')?.1;
    let (name, _) = after.split_once('`')?;
    Some(name.to_owned())
}

/// Deduplicates, preserving order, so a name declared on several types is one
/// element of an expected set rather than several.
pub fn unique(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for name in names {
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A value the document never closes yields no name at all. A lexical
    /// reader that ran to the end of the text instead would hand the explain
    /// case a "name" made of the rest of the document, and the assertion built
    /// on it would be about nothing.
    #[test]
    fn a_value_that_is_never_closed_yields_no_name() {
        let closed = json_strings("{\n  \"name\": \"capture\"\n}", "name");
        assert_eq!(closed, vec!["capture".to_owned()]);
        let unterminated = json_strings("{\n  \"name\": \"capture", "name");
        assert!(
            unterminated.is_empty(),
            "an unclosed value is not a name: {unterminated:?}"
        );
    }
}
