//! The two things a note's body is read for: its title, and the untyped
//! references it writes.
//!
//! **The title is the first H1, carried as display metadata and never as
//! identity.** Renaming a title is an edit; changing an identity is a
//! link-integrity operation, and keeping the two apart is what lets a note be
//! retitled without breaking a single link.
//!
//! The references are the *other* half of "body content is untouched beyond
//! link extraction": the dialect's delimited form is found in the **prose**
//! and resolved, and nothing else about the prose is interpreted. A code block
//! is not prose — a note quoting example `[[links]]` in one is quoting, not
//! referencing — so link extraction steps over fenced and indented code blocks
//! with exactly the block structure the title scan already has, and no more.
//!
//! Beyond those two, there is no outline, no section model, no task list —
//! "body content is untouched beyond link extraction" reads as *no structure*,
//! not as *no title*, because the shared document-model shape names the title
//! and `show` returns it.
//!
//! Finding the first H1 still needs the smallest amount of block structure that
//! gets there, and the base grammar is CommonMark:
//!
//! - both spellings are headings — ATX (`# Title`) and Setext (`Title` over
//!   `===`) — because a corpus that writes the second has a title;
//! - fenced and indented code blocks are stepped over, because a `# ` line
//!   inside a fence is a line of code and lifting a title out of one would be
//!   visible in daily reading.
//!
//! One narrowing is this module's own: a Setext heading's text is the line
//! immediately above the underline, where CommonMark folds the whole paragraph.
//! A title written across two lines is not a title anyone writes.

use core::ops::Range;

use crate::contract::LinkDialect;

use super::links;

/// How far a line is indented before it is a code block rather than prose.
const CODE_INDENT: usize = 4;

/// The two fences CommonMark opens a code block with.
const FENCES: [&str; 2] = ["```", "~~~"];

/// One untyped reference the prose writes, and where the note wrote it.
pub(crate) struct Written {
    /// The reference as the note wrote it, delimiters included.
    pub(crate) text: String,
    /// Its byte range **in the note**, not in the body: an ambiguous prose
    /// reference is reported against the reference itself, and a reader is
    /// looking at the file rather than at the body alone.
    pub(crate) at: Range<usize>,
}

/// A note's body, and everything read out of it.
pub(crate) struct Body {
    /// The body as uninterpreted text.
    pub(crate) text: String,
    /// The first H1, when the note has one.
    pub(crate) title: Option<String>,
    /// The untyped references the prose writes, unresolved.
    pub(crate) references: Vec<Written>,
}

/// Reads a note's body, in the dialect its contract declares.
///
/// `offset` is where the body begins in the note, and is carried straight into
/// the references so that nothing downstream has to remember to add it.
pub(crate) fn read(text: String, dialect: LinkDialect, offset: usize) -> Body {
    Body {
        title: title(&text),
        references: references(&text, dialect, offset),
        text,
    }
}

/// The note's title: its first H1, when it has one.
fn title(body: &str) -> Option<String> {
    let mut scan = Scan::new();
    body.lines().find_map(|raw| {
        let line = Line::of(raw);
        let above = scan.previous;
        scan.prose(line).then(|| heading(line, above)).flatten()
    })
}

/// The untyped references the prose writes, each located in the note.
///
/// Only the prose is scanned: a reference inside a code block is a line of
/// code, stepped over by the same block structure the title scan reads.
fn references(body: &str, dialect: LinkDialect, offset: usize) -> Vec<Written> {
    prose(body)
        .into_iter()
        .flat_map(|region| {
            links::scan(dialect, &body[region.clone()])
                .into_iter()
                .map(move |at| region.start + at.start..region.start + at.end)
        })
        .map(|at| Written {
            text: body[at.clone()].to_owned(),
            at: offset + at.start..offset + at.end,
        })
        .collect()
}

/// The byte ranges of the body that are prose rather than code.
///
/// Contiguous prose lines merge into one range, so a reference is found
/// wherever its paragraph puts it; a code line splits the prose either side
/// of it, so no scan ever reads across a code block.
fn prose(body: &str) -> Vec<Range<usize>> {
    let mut scan = Scan::new();
    let mut regions: Vec<Range<usize>> = Vec::new();
    let mut at = 0;
    for raw in body.split_inclusive('\n') {
        let end = at + raw.len();
        if scan.prose(Line::of(raw)) {
            match regions.last_mut() {
                Some(last) if last.end == at => last.end = end,
                _ => regions.push(at..end),
            }
        }
        at = end;
    }
    regions
}

/// The H1 `line` is: an ATX heading of its own, or the underline of the
/// paragraph line `above` it.
fn heading(line: Line<'_>, above: Option<&str>) -> Option<String> {
    line.atx()
        .or_else(|| above.filter(|_| line.underlines()).map(str::to_owned))
}

/// One line of a body, with its indentation measured off.
#[derive(Clone, Copy)]
struct Line<'a> {
    /// The line without its indentation or its trailing whitespace.
    content: &'a str,
    /// How many spaces preceded the content.
    indent: usize,
}

impl<'a> Line<'a> {
    fn of(raw: &'a str) -> Self {
        let trimmed = raw.trim_end();
        let content = trimmed.trim_start_matches(' ');
        Self {
            content,
            indent: trimmed.len() - content.len(),
        }
    }

    fn is_blank(self) -> bool {
        self.content.is_empty()
    }

    /// The fence this line opens a code block with, when it opens one.
    fn opens(self) -> Option<&'static str> {
        FENCES
            .into_iter()
            .find(|fence| self.content.starts_with(fence))
    }

    /// An ATX H1's text: one `#`, a space, and the rest without its closing run.
    fn atx(self) -> Option<String> {
        let rest = self.content.strip_prefix('#')?;
        if !rest.is_empty() && !rest.starts_with(' ') {
            return None;
        }
        Some(closed(rest.trim()).trim_end().to_owned())
    }

    /// Whether this line underlines a Setext H1.
    fn underlines(self) -> bool {
        !self.is_blank() && self.content.chars().all(|character| character == '=')
    }
}

/// A walk down the body, holding the block structure a line's reading needs.
///
/// The one walk serves both readers: the title scan asks it which lines are
/// prose so a `#` inside a code block stays code, and link extraction asks it
/// the same question so a `[[link]]` inside one stays code too.
struct Scan<'a> {
    /// The fence that opened the code block being stepped over.
    fence: Option<&'static str>,
    /// The line above, when it could be a Setext heading's text.
    previous: Option<&'a str>,
}

impl<'a> Scan<'a> {
    fn new() -> Self {
        Self {
            fence: None,
            previous: None,
        }
    }

    /// Reads one line, answering whether it is prose rather than code.
    ///
    /// Code is a fence, a line inside a fenced block, or an indented code
    /// block; an indented line continuing a paragraph is still prose, because
    /// CommonMark says an indented code block cannot interrupt one.
    fn prose(&mut self, line: Line<'a>) -> bool {
        if let Some(fence) = self.fence {
            self.close(line, fence);
            return false;
        }
        self.previous = self.previous.filter(|_| !line.is_blank());
        if line.indent >= CODE_INDENT && self.previous.is_none() {
            return false;
        }
        if let Some(fence) = line.opens() {
            self.fence = Some(fence);
            self.previous = None;
            return false;
        }
        self.previous = (!line.is_blank()).then_some(line.content);
        true
    }

    /// Leaves the code block when this line closes it.
    fn close(&mut self, line: Line<'_>, fence: &'static str) {
        self.previous = None;
        if line.content.starts_with(fence) {
            self.fence = None;
        }
    }
}

/// A heading's text without CommonMark's optional closing run of `#`.
fn closed(text: &str) -> &str {
    let stripped = text.trim_end_matches('#');
    match stripped {
        _ if stripped.len() == text.len() => text,
        _ if stripped.is_empty() => stripped,
        _ if stripped.ends_with(' ') => stripped,
        _ => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading(body: &str) -> Option<String> {
        title(body)
    }

    fn titled(body: &str) -> String {
        heading(body).expect("a title")
    }

    #[test]
    fn the_title_is_the_first_atx_h1() {
        let found = [
            titled("# Ada Lovelace\n\nprose\n"),
            titled("prose\n\n# Later\n"),
            titled("# First\n# Second\n"),
        ];
        assert_eq!(found, ["Ada Lovelace", "Later", "First"]);
    }

    #[test]
    fn an_atx_heading_may_be_indented_up_to_the_code_block_margin() {
        // Four spaces is a code block, and a heading in one is a line of code.
        let found = (titled("   # Ada\n"), heading("    # Ada\n"));
        assert_eq!(found, ("Ada".to_owned(), None));
    }

    #[test]
    fn an_atx_heading_loses_its_closing_run_and_keeps_what_looks_like_one() {
        let found = [
            titled("# Ada ###\n"),
            titled("# ###\n"),
            titled("# C#\n"),
            titled("#\n"),
        ];
        assert_eq!(found, ["Ada", "", "C#", ""]);
    }

    #[test]
    fn only_a_first_level_heading_is_a_title() {
        let found = (
            heading("## Ada\n"),
            heading("#Ada\n"),
            titled("## Ada\n# Real\n"),
        );
        assert_eq!(found, (None, None, "Real".to_owned()));
    }

    #[test]
    fn a_setext_heading_is_a_title_too_because_a_corpus_writes_them() {
        let written = (titled("Ada Lovelace\n============\n"), titled("  Ada\n=\n"));
        assert_eq!(written, ("Ada Lovelace".to_owned(), "Ada".to_owned()));
        // An underline with nothing above it underlines nothing, and a blank
        // line ends the paragraph the underline would have titled.
        let nothing = (heading("\n=====\n"), heading("Ada\n\n=====\n"));
        assert_eq!(nothing, (None, None));
    }

    #[test]
    fn a_heading_inside_a_fenced_code_block_is_a_line_of_code() {
        let after = (
            titled("```\n# not a title\n```\n# Real\n"),
            titled("~~~\n# not a title\n~~~\n# Real\n"),
        );
        assert_eq!(after, ("Real".to_owned(), "Real".to_owned()));
        // A fence that is never closed takes the rest of the note with it, and
        // a Setext underline inside one underlines nothing.
        let inside = (
            heading("```rust\n# still code\nfn main() {}\n"),
            heading("```\nAda\n===\n```\n"),
        );
        assert_eq!(inside, (None, None));
    }

    #[test]
    fn a_note_with_no_heading_at_all_has_no_title() {
        let found = (
            heading(""),
            heading("just prose\nand more\n"),
            heading("    indented\n"),
        );
        assert_eq!(found, (None, None, None));
    }

    #[test]
    fn an_indented_line_continuing_a_paragraph_is_not_a_code_block() {
        // CommonMark: an indented code block cannot interrupt a paragraph, so
        // the underline below still finds the text above it.
        assert_eq!(titled("Ada\n    continued\n===\n"), "continued");
    }

    #[test]
    fn a_body_answers_its_title_and_the_references_its_prose_writes() {
        let prose = "# Ada Lovelace\n\nWorked with [[Charles Babbage]] on [[Analytical Engine]].\n";
        let body = read(prose.to_owned(), LinkDialect::Wikilink, 0);
        assert_eq!(body.text, prose, "the body is carried uninterpreted");
        assert_eq!(body.title.as_deref(), Some("Ada Lovelace"));
        let written: Vec<&str> = body
            .references
            .iter()
            .map(|reference| reference.text.as_str())
            .collect();
        assert_eq!(written, ["[[Charles Babbage]]", "[[Analytical Engine]]"]);
    }

    #[test]
    fn a_reference_is_located_in_the_note_rather_than_in_the_body() {
        // The offset is the body's own start, so a span measured from the range
        // points where a reader of the file would look.
        let prose = "See [[Ada]] and [[Babbage]].\n".to_owned();
        let body = read(prose, LinkDialect::Wikilink, 40);
        let located: Vec<Range<usize>> = body
            .references
            .iter()
            .map(|reference| reference.at.clone())
            .collect();
        assert_eq!(located, [44..51, 56..67]);
    }

    #[test]
    fn a_reference_inside_a_code_block_is_a_line_of_code() {
        // Link extraction steps over exactly the code blocks the title scan
        // does: both fences, an unclosed fence, and an indented code block. A
        // note quoting example `[[links]]` in one is quoting, not referencing.
        let quoted = [
            read("```\n[[Ada]]\n```\n".to_owned(), LinkDialect::Wikilink, 0),
            read("~~~\n[[Ada]]\n~~~\n".to_owned(), LinkDialect::Wikilink, 0),
            read(
                "```\n[[Ada]] unclosed\n".to_owned(),
                LinkDialect::Wikilink,
                0,
            ),
            read(
                "intro\n\n    [[Ada]]\n".to_owned(),
                LinkDialect::Wikilink,
                0,
            ),
            read(
                "```\n[Ada](people/ada.md)\n```\n".to_owned(),
                LinkDialect::Markdown,
                0,
            ),
        ];
        for body in &quoted {
            assert!(body.references.is_empty(), "{:?}", body.text);
            assert_eq!(body.title, None);
        }
    }

    #[test]
    fn prose_around_a_code_block_still_writes_its_references() {
        // The block splits the prose rather than swallowing it, and a span
        // after the block still points where the note wrote the reference.
        let prose = "See [[Ada]].\n```\n[[code]]\n```\nAnd [[Babbage]].\n";
        let body = read(prose.to_owned(), LinkDialect::Wikilink, 0);
        let written: Vec<(&str, Range<usize>)> = body
            .references
            .iter()
            .map(|reference| (reference.text.as_str(), reference.at.clone()))
            .collect();
        assert_eq!(written, [("[[Ada]]", 4..11), ("[[Babbage]]", 34..45)]);
        assert_eq!(&prose[34..45], "[[Babbage]]");
    }

    #[test]
    fn an_indented_line_continuing_a_paragraph_still_writes_references() {
        // The same CommonMark rule the title reads by: an indented code block
        // cannot interrupt a paragraph, so the reference is prose.
        let body = read("See\n    [[Ada]]\n".to_owned(), LinkDialect::Wikilink, 0);
        assert_eq!(body.references.len(), 1);
        assert_eq!(body.references[0].text, "[[Ada]]");
    }

    #[test]
    fn a_line_answers_for_the_shape_it_takes() {
        let fences = (Line::of("```rust").opens(), Line::of("~~~").opens());
        assert_eq!(fences, (Some("```"), Some("~~~")));
        let prose = (Line::of("text").opens(), Line::of("  ").is_blank());
        assert_eq!(prose, (None, true));
        let underlining = (
            Line::of("===").underlines(),
            Line::of("").underlines(),
            Line::of("=-=").underlines(),
        );
        assert_eq!(underlining, (true, false, false));
        let runs = [
            closed("Ada"),
            closed("###"),
            closed("Ada ###"),
            closed("C#"),
        ];
        assert_eq!(runs, ["Ada", "", "Ada ", "C#"]);
    }
}
