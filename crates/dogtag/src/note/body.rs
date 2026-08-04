//! The one thing a note's body is read for: its title.
//!
//! **The title is the first H1, carried as display metadata and never as
//! identity.** Renaming a title is an edit; changing an identity is a
//! link-integrity operation, and keeping the two apart is what lets a note be
//! retitled without breaking a single link.
//!
//! This is the whole of the body grammar M3 commits to. There is no outline, no
//! section model, no task list — "body content is untouched beyond link
//! extraction" reads as *no structure*, not as *no title*, because the shared
//! document-model shape names the title and `show` returns it.
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

/// How far a line is indented before it is a code block rather than prose.
const CODE_INDENT: usize = 4;

/// The two fences CommonMark opens a code block with.
const FENCES: [&str; 2] = ["```", "~~~"];

/// The note's title: its first H1, when it has one.
pub(crate) fn title(body: &str) -> Option<String> {
    let mut scan = Scan {
        fence: None,
        previous: None,
    };
    body.lines().find_map(|raw| scan.read(Line::of(raw)))
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

/// A walk down the body, holding only what a heading depends on.
struct Scan<'a> {
    /// The fence that opened the code block being stepped over.
    fence: Option<&'static str>,
    /// The line above, when it could be a Setext heading's text.
    previous: Option<&'a str>,
}

impl<'a> Scan<'a> {
    /// Reads one line, answering with the title when that line is one.
    fn read(&mut self, line: Line<'a>) -> Option<String> {
        if let Some(fence) = self.fence {
            self.close(line, fence);
            return None;
        }
        self.previous = self.previous.filter(|_| !line.is_blank());
        if line.indent >= CODE_INDENT && self.previous.is_none() {
            return None;
        }
        if let Some(fence) = line.opens() {
            self.fence = Some(fence);
            self.previous = None;
            return None;
        }
        self.heading(line)
    }

    /// Leaves the code block when this line closes it.
    fn close(&mut self, line: Line<'_>, fence: &'static str) {
        self.previous = None;
        if line.content.starts_with(fence) {
            self.fence = None;
        }
    }

    /// The heading this line is, or the paragraph text it becomes.
    fn heading(&mut self, line: Line<'a>) -> Option<String> {
        if let Some(text) = line.atx() {
            return Some(text);
        }
        if let Some(text) = self.previous.filter(|_| line.underlines()) {
            self.previous = None;
            return Some(text.to_owned());
        }
        self.previous = (!line.is_blank()).then_some(line.content);
        None
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

    fn read(body: &str) -> Option<String> {
        title(body)
    }

    fn titled(body: &str) -> String {
        read(body).expect("a title")
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
        let found = (titled("   # Ada\n"), read("    # Ada\n"));
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
        let found = (read("## Ada\n"), read("#Ada\n"), titled("## Ada\n# Real\n"));
        assert_eq!(found, (None, None, "Real".to_owned()));
    }

    #[test]
    fn a_setext_heading_is_a_title_too_because_a_corpus_writes_them() {
        let written = (titled("Ada Lovelace\n============\n"), titled("  Ada\n=\n"));
        assert_eq!(written, ("Ada Lovelace".to_owned(), "Ada".to_owned()));
        // An underline with nothing above it underlines nothing, and a blank
        // line ends the paragraph the underline would have titled.
        let nothing = (read("\n=====\n"), read("Ada\n\n=====\n"));
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
            read("```rust\n# still code\nfn main() {}\n"),
            read("```\nAda\n===\n```\n"),
        );
        assert_eq!(inside, (None, None));
    }

    #[test]
    fn a_note_with_no_heading_at_all_has_no_title() {
        let found = (
            read(""),
            read("just prose\nand more\n"),
            read("    indented\n"),
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
