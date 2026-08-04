//! Frontmatter: a strict YAML subset, read by hand.
//!
//! Frontmatter is where a corpus already lives, so the language is kept and the
//! dangerous half is fenced off. The subset admits scalars, sequences, and
//! mappings; it refuses **anchors, aliases, tags, multi-document streams,
//! non-string keys, and duplicate keys**.
//!
//! # Why this is written by hand
//!
//! For the four reasons `crates/dogtag/Cargo.toml` already records for the
//! contract, every one of which reaches a note: every key *and* value carries a
//! span measured over the file's actual bytes; a malformed note yields every
//! diagnostic rather than the first; a fault is reported as *our* diagnostic
//! with *our* identifier; and — the one that rules out a document-level YAML
//! library outright — **implicit typing never runs**. A loader that answers
//! with typed values has already decided that `NO` is a boolean and that `1.0`
//! is a number, and the declared kind is what decides here. Duplicate keys are
//! *refused* where a loader silently keeps the last, and an alias is refused
//! with a span pointing at the construct rather than quietly resolved.
//!
//! Every refusal is of the **language**, never of a style: `[one, *base]`
//! writes the alias `*base` writes, and `{given: A, given: A}` repeats a key
//! the same way two lines do. [`scalar`] holds the refusals for that reason,
//! and both halves of the grammar — [`block`] and [`flow`] — consult it.
//!
//! # What "nested at most one level below the top" means here
//!
//! It is a **shape** rule, not a depth counter, and [`shape`] holds it. The
//! legal shapes are exactly four: a scalar, a sequence of scalars, a mapping of
//! scalars (a record value), and a sequence of mappings of scalars (a list of
//! records). A literal depth count would refuse the last of those — a list of
//! records puts its mappings two levels below the top — and that is the one
//! shape contract version 2 added the `record` kind in order to write.
//!
//! # Quoting is spelling, never typing
//!
//! A single- or double-quoted scalar carries the same bytes with none of plain
//! style's hazards, so `"[[Some Note]]"` and a quoted `"true"` for a string
//! property are ordinary, and a quoted scalar validates against its declared
//! kind exactly as an unquoted one does. Block scalars (`|`, `>`) stay outside
//! the subset: nothing in frontmatter needs them and their chomping rules are
//! precisely the incidental complexity the subset exists to exclude.

mod block;
mod flow;
mod scalar;
mod shape;

use core::ops::Range;

/// The fence that opens and closes a frontmatter block.
const FENCE: &str = "---";

/// The document-end marker, which only a multi-document stream writes.
const DOCUMENT_END: &str = "...";

/// A UTF-8 byte order mark, which a note may carry and still be read.
const BYTE_ORDER_MARK: &str = "\u{feff}";

/// How deep a collection may sit before the subset stops reading.
///
/// A top-level mapping is depth 0, the value it carries is depth 1, and a
/// sequence's element is depth 2 — which is exactly a list of records, the
/// deepest shape [`shape`] admits. One level below that is read anyway, so that
/// [`shape`] rather than this bound is what refuses a shape a note could
/// plausibly have meant, and says which shape it was. Below *that* the walk
/// simply stops: the bound exists to keep recursion a property of the grammar
/// rather than of a note's author.
const MAX_NESTING: usize = 3;

/// One key and the value it carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Entry {
    /// The key's bytes. A key is **always** a byte string: implicit typing
    /// never runs, so a field named `on`, `no`, or `off` is its own name rather
    /// than a boolean, and the non-string-key refusal covers only a key that is
    /// structurally not a scalar.
    pub(crate) key: String,
    /// The byte range of the key as it is written.
    pub(crate) key_span: Range<usize>,
    /// The value the key carries.
    pub(crate) value: Value,
}

/// A frontmatter value, before any declaration has been consulted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Value {
    /// The byte range the value is written at. A collection points at where it
    /// starts, which is what a reader needs in order to find it.
    pub(crate) span: Range<usize>,
    /// What the value is.
    pub(crate) shape: Shape,
}

/// What a frontmatter value is, structurally.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    /// A scalar's bytes, with quoting undone and nothing else interpreted.
    Scalar(String),
    /// A sequence's items, in order.
    Sequence(Vec<Value>),
    /// A mapping's entries, in order.
    Mapping(Vec<Entry>),
}

impl Value {
    /// The scalar's bytes, when the value is a scalar.
    pub(crate) fn scalar(&self) -> Option<&str> {
        match &self.shape {
            Shape::Scalar(text) => Some(text),
            _ => None,
        }
    }

    /// The items, when the value is a sequence.
    pub(crate) fn sequence(&self) -> Option<&[Value]> {
        match &self.shape {
            Shape::Sequence(items) => Some(items),
            _ => None,
        }
    }

    /// The entries, when the value is a mapping.
    pub(crate) fn mapping(&self) -> Option<&[Entry]> {
        match &self.shape {
            Shape::Mapping(entries) => Some(entries),
            _ => None,
        }
    }

    /// How a message names what was found where a kind was expected.
    pub(crate) fn describe(&self) -> &'static str {
        match &self.shape {
            Shape::Scalar(_) => "a scalar",
            Shape::Sequence(_) => "a sequence",
            Shape::Mapping(_) => "a mapping",
        }
    }
}

/// Which refusal a fault is, which is what picks its identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FaultKind {
    /// The bytes are not the subset's grammar at all.
    Invalid,
    /// The bytes are YAML the subset deliberately refuses.
    Unsupported,
}

/// One reason a frontmatter block did not load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Fault {
    /// Which refusal this is.
    pub(crate) kind: FaultKind,
    /// What is wrong, naming the construct.
    pub(crate) message: String,
    /// The byte range of the construct itself.
    pub(crate) span: Range<usize>,
}

impl Fault {
    fn new(kind: FaultKind, message: String, span: Range<usize>) -> Self {
        Self {
            kind,
            message,
            span,
        }
    }
}

/// What a note's frontmatter block turned out to be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Front {
    /// The note carries no frontmatter block at all.
    Absent,
    /// The block loaded, with these entries in the order it wrote them.
    Read(Vec<Entry>),
    /// The block did not load. The faults say why, and the note has no
    /// frontmatter — not an empty one.
    Refused,
}

impl Front {
    /// The entries the block loaded, when it loaded.
    pub(crate) fn entries(&self) -> Option<&[Entry]> {
        match self {
            Self::Read(entries) => Some(entries),
            _ => None,
        }
    }
}

/// A note split into its frontmatter and its body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Parsed {
    /// What the frontmatter block turned out to be.
    pub(crate) front: Front,
    /// Every reason the block did not load, in the order they were found.
    pub(crate) faults: Vec<Fault>,
    /// The byte range of the body — everything after the block.
    pub(crate) body: Range<usize>,
}

/// Reads a note's frontmatter block, and says where its body starts.
pub(crate) fn read(text: &str) -> Parsed {
    let lines = lines(text);
    let Some(opening) = opening_fence(&lines) else {
        return Parsed {
            front: Front::Absent,
            faults: Vec::new(),
            body: 0..text.len(),
        };
    };
    match closing_fence(&lines, opening) {
        Some(closing) => parse(
            &lines[opening + 1..closing],
            body_after(&lines, closing, text),
        ),
        None => unterminated(&lines[opening], body_after(&lines, opening, text)),
    }
}

/// The block's lines, parsed and then held to the subset's four shapes.
fn parse(block: &[Line<'_>], body: Range<usize>) -> Parsed {
    let (entries, mut faults) = block::read(block);
    shape::check(&entries, &mut faults);
    Parsed {
        front: if faults.is_empty() {
            Front::Read(entries)
        } else {
            Front::Refused
        },
        faults,
        body,
    }
}

/// An opening fence with no closing one: the block has no extent, so there is
/// nothing to parse and the whole rest of the file is body.
fn unterminated(opening: &Line<'_>, body: Range<usize>) -> Parsed {
    Parsed {
        front: Front::Refused,
        faults: vec![Fault::new(
            FaultKind::Invalid,
            format!("the frontmatter block opens with `{FENCE}` and is never closed"),
            opening.range(),
        )],
        body,
    }
}

fn opening_fence(lines: &[Line<'_>]) -> Option<usize> {
    lines.first().filter(|line| line.is_fence()).map(|_| 0)
}

fn closing_fence(lines: &[Line<'_>], opening: usize) -> Option<usize> {
    lines[opening + 1..]
        .iter()
        .position(Line::is_fence)
        .map(|offset| opening + 1 + offset)
}

/// Where the body starts: after the line at `index`, or the end of the file.
fn body_after(lines: &[Line<'_>], index: usize, text: &str) -> Range<usize> {
    let start = lines.get(index + 1).map_or(text.len(), |line| line.start);
    start..text.len()
}

/// One line of a note, as the subset's grammar sees it.
///
/// `text` is the line without its indentation, its trailing spaces, or its
/// terminator; `at` is where `text` begins in the file, so every span this
/// module produces is measured over the file's actual bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Line<'a> {
    indent: usize,
    text: &'a str,
    at: usize,
    start: usize,
}

impl Line<'_> {
    /// The byte range of the line's content.
    fn range(&self) -> Range<usize> {
        self.at..self.at + self.text.len()
    }

    /// The offset just past the line's content, where an absent value sits.
    fn end(&self) -> usize {
        self.at + self.text.len()
    }

    /// Whether this line is a frontmatter fence.
    fn is_fence(&self) -> bool {
        self.indent == 0 && self.text == FENCE
    }

    /// Whether the line carries nothing the grammar reads.
    fn is_ignorable(&self) -> bool {
        self.text.is_empty() || self.text.starts_with('#')
    }
}

/// Every line of `text`, indentation measured and terminators removed.
///
/// A leading byte order mark is stepped over rather than removed: the offsets
/// still count it, so spans stay measured over the bytes the file holds, and a
/// note that carries one keeps its frontmatter instead of silently losing it.
/// The mark is a warning elsewhere; here it is simply not part of the grammar.
fn lines(text: &str) -> Vec<Line<'_>> {
    let (text, base) = match text.strip_prefix(BYTE_ORDER_MARK) {
        Some(rest) => (rest, BYTE_ORDER_MARK.len()),
        None => (text, 0),
    };
    let mut lines = Vec::new();
    let mut start = base;
    for raw in text.split_inclusive('\n') {
        let content = raw.trim_end_matches('\n').trim_end_matches('\r').trim_end();
        let indent = content.len() - content.trim_start_matches(' ').len();
        lines.push(Line {
            indent,
            text: &content[indent..],
            at: start + indent,
            start,
        });
        start += raw.len();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn front_of(text: &str) -> Front {
        read(text).front
    }

    fn entries_of(text: &str) -> Vec<Entry> {
        let parsed = read(text);
        parsed
            .front
            .entries()
            .expect("the block must load")
            .to_vec()
    }

    fn faults_of(text: &str) -> Vec<Fault> {
        let parsed = read(text);
        assert_eq!(parsed.front, Front::Refused, "the block must be refused");
        parsed.faults
    }

    #[test]
    fn a_note_with_no_opening_fence_carries_no_frontmatter_and_is_all_body() {
        let parsed = read("# Title\n\nprose\n");
        assert_eq!(parsed.front, Front::Absent);
        assert!(parsed.faults.is_empty());
        assert_eq!(parsed.body, 0..15);
    }

    #[test]
    fn an_empty_note_carries_no_frontmatter() {
        let parsed = read("");
        assert_eq!(parsed.front, Front::Absent);
        assert_eq!(parsed.body, 0..0);
    }

    #[test]
    fn a_fenced_block_is_read_and_the_body_is_what_follows_it() {
        let text = "---\ntype: person\n---\n# Ada\n";
        let parsed = read(text);
        assert_eq!(
            parsed.front,
            Front::Read(vec![Entry {
                key: "type".to_owned(),
                key_span: 4..8,
                value: Value {
                    span: 10..16,
                    shape: Shape::Scalar("person".to_owned()),
                },
            }])
        );
        assert_eq!(&text[parsed.body], "# Ada\n");
    }

    #[test]
    fn a_block_that_ends_with_the_file_leaves_an_empty_body() {
        let parsed = read("---\ntype: person\n---");
        assert!(parsed.front.entries().is_some());
        assert_eq!(parsed.body, 20..20);
    }

    #[test]
    fn a_block_that_is_never_closed_is_refused_and_names_the_fence() {
        let faults = faults_of("---\ntype: person\n# Ada\n");
        assert_eq!(faults.len(), 1);
        assert_eq!(faults[0].kind, FaultKind::Invalid);
        assert!(faults[0].message.contains("never closed"));
        assert_eq!(faults[0].span, 0..3);
    }

    #[test]
    fn a_byte_order_mark_is_stepped_over_rather_than_hiding_the_block() {
        let entries = entries_of("\u{feff}---\ntype: person\n---\n");
        assert_eq!(entries[0].key, "type");
        assert_eq!(
            entries[0].key_span,
            7..11,
            "the mark's bytes are still counted, so the span points at the file's own bytes"
        );
    }

    #[test]
    fn carriage_return_line_endings_are_read_rather_than_refused() {
        let entries = entries_of("---\r\ntype: person\r\n---\r\n");
        assert_eq!(entries[0].key, "type");
        assert_eq!(entries[0].value.scalar(), Some("person"));
    }

    #[test]
    fn a_block_holding_nothing_reads_as_a_note_with_no_keys() {
        assert_eq!(front_of("---\n---\nbody\n"), Front::Read(Vec::new()));
        assert_eq!(
            front_of("---\n# only a comment\n\n---\n"),
            Front::Read(Vec::new())
        );
    }

    #[test]
    fn a_value_answers_what_it_is_for_a_message_that_has_to_say_so() {
        let entries = entries_of("---\na: one\nb: [one]\nc:\n  d: one\n---\n");
        let described: Vec<&str> = entries.iter().map(|entry| entry.value.describe()).collect();
        assert_eq!(described, ["a scalar", "a sequence", "a mapping"]);
        assert_eq!(entries[1].value.scalar(), None);
    }

    #[test]
    fn the_frontmatter_types_clone_compare_and_format() {
        let parsed = read("---\na: one\n---\n");
        assert_eq!(parsed.clone(), parsed);
        assert!(format!("{parsed:?}").contains("one"));
        let fault = Fault::new(FaultKind::Invalid, "broken".to_owned(), 0..1);
        assert_eq!(fault.clone(), fault);
        assert!(format!("{fault:?}").contains("broken"));
        assert_ne!(FaultKind::Invalid, FaultKind::Unsupported);
    }

    #[test]
    fn a_line_reports_its_own_extent() {
        let lines = lines("---\n  key: one\n");
        assert_eq!(lines[1].indent, 2);
        assert_eq!(lines[1].text, "key: one");
        assert_eq!(lines[1].range(), 6..14);
        assert_eq!(lines[1].end(), 14);
        assert!(lines[0].is_fence());
        assert!(!lines[1].is_fence());
        assert!(!lines[1].is_ignorable());
    }
}
