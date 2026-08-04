//! The block-style half of the subset: lines, indentation, and their entries.
//!
//! The walk is line-oriented and indentation-driven, exactly as YAML's block
//! style is, with three narrowings this module owns and states here:
//!
//! - **A plain scalar is one line.** YAML folds a plain scalar across lines;
//!   the subset does not, because a value that continues onto the next line and
//!   a key that failed to parse look identical to a reader.
//! - **Indentation is spaces.** A tab in a line's indentation is refused rather
//!   than counted, which is YAML's own rule.
//! - **A sequence item's mapping starts on the item's own line.** `- name: A`
//!   opens a mapping whose later keys line up under `name`, which is what a
//!   list of records is written as.

use core::ops::Range;
use std::collections::BTreeSet;

use super::scalar::{Head, REFUSED_OPENERS};
use super::{DOCUMENT_END, Entry, Fault, FaultKind, Line, MAX_NESTING, Shape, Value, flow};

/// Reads a frontmatter block's lines as a mapping of keys to values.
pub(super) fn read<'a>(lines: &[Line<'a>]) -> (Vec<Entry>, Vec<Fault>) {
    let mut block = Block {
        lines: lines.to_vec(),
        index: 0,
        faults: Vec::new(),
    };
    let entries = block.document();
    (entries, block.faults)
}

/// One block being walked.
struct Block<'a> {
    lines: Vec<Line<'a>>,
    index: usize,
    faults: Vec<Fault>,
}

impl<'a> Block<'a> {
    /// The whole block: one mapping at the left margin, and nothing else.
    fn document(&mut self) -> Vec<Entry> {
        let entries = self.mapping(0, 0);
        if let Some(line) = self.peek() {
            self.refuse(
                FaultKind::Invalid,
                "frontmatter is a mapping of keys to values".to_owned(),
                line.range(),
            );
            self.index = self.lines.len();
        }
        entries
    }

    /// The next line the grammar reads, without consuming it.
    ///
    /// Blank lines and comments are stepped over. A `...` is stepped over too,
    /// after being refused: it is the document-end marker, and the only thing
    /// that writes one is a multi-document stream.
    fn peek(&mut self) -> Option<Line<'a>> {
        while let Some(line) = self.lines.get(self.index).copied() {
            if line.is_ignorable() {
                self.index += 1;
                continue;
            }
            if line.text != DOCUMENT_END {
                return Some(line);
            }
            self.refuse(
                FaultKind::Unsupported,
                format!("`{DOCUMENT_END}` ends a document: this is a multi-document stream"),
                line.range(),
            );
            self.index += 1;
        }
        None
    }

    fn refuse(&mut self, kind: FaultKind, message: String, span: Range<usize>) {
        self.faults.push(Fault::new(kind, message, span));
    }

    /// Steps over every line indented under `indent`.
    fn skip_under(&mut self, indent: usize) {
        while let Some(line) = self.lines.get(self.index) {
            if line.indent <= indent && !line.is_ignorable() {
                return;
            }
            self.index += 1;
        }
    }

    /// A mapping whose keys sit at `indent`.
    fn mapping(&mut self, indent: usize, depth: usize) -> Vec<Entry> {
        let mut entries = Vec::new();
        let mut seen = BTreeSet::new();
        while let Some(line) = self.peek() {
            if line.indent < indent || is_item(line.text) {
                break;
            }
            if line.indent > indent {
                self.stray(line);
                continue;
            }
            let Some(entry) = self.entry(line, indent, depth) else {
                continue;
            };
            if !seen.insert(entry.key.clone()) {
                let message = format!("the key `{}` is written twice", entry.key);
                self.refuse(FaultKind::Unsupported, message, entry.key_span.clone());
            }
            entries.push(entry);
        }
        entries
    }

    fn stray(&mut self, line: Line<'a>) {
        self.refuse(
            FaultKind::Invalid,
            "this line is indented under a key that opens no nested block".to_owned(),
            line.range(),
        );
        self.index += 1;
    }

    /// One `key: value` entry, consuming whatever the value takes.
    fn entry(&mut self, line: Line<'a>, indent: usize, depth: usize) -> Option<Entry> {
        self.index += 1;
        if let Some((kind, message)) = refused_key(line.text) {
            self.refuse(kind, message, line.range());
            self.skip_under(indent);
            return None;
        }
        let Some(key) = Head(line.text).key() else {
            self.refuse(
                FaultKind::Invalid,
                "expected `key: value`".to_owned(),
                line.range(),
            );
            self.skip_under(indent);
            return None;
        };
        let rest = &line.text[key.value_at..];
        let spot = Spot {
            at: line.at + key.value_at,
            indent,
            depth: depth + 1,
        };
        Some(Entry {
            key: key.text,
            key_span: line.at..line.at + key.length,
            value: self.value(rest, line.end(), spot),
        })
    }

    /// The value an entry or an item carries: on the line, or beneath it.
    fn value(&mut self, rest: &'a str, empty_at: usize, spot: Spot) -> Value {
        if rest.is_empty() {
            return self.nested(spot, empty_at);
        }
        self.inline(rest, spot)
    }

    /// A collection written in the lines beneath its key, if there is one.
    ///
    /// A sequence may sit at its key's own indentation, which is YAML's rule
    /// and how a corpus writes one; a mapping must be indented further.
    fn nested(&mut self, spot: Spot, empty_at: usize) -> Value {
        let Some(line) = self.peek() else {
            return empty(empty_at);
        };
        let indent = if line.indent == spot.indent && is_item(line.text) {
            spot.indent
        } else if line.indent > spot.indent {
            line.indent
        } else {
            return empty(empty_at);
        };
        if self.too_deep(line, spot) {
            return empty(line.at);
        }
        self.collection(line, indent, spot.depth)
    }

    /// Refuses, and steps over, a collection deeper than the subset reads.
    fn too_deep(&mut self, line: Line<'a>, spot: Spot) -> bool {
        if spot.depth <= MAX_NESTING {
            return false;
        }
        self.refuse(
            FaultKind::Unsupported,
            "the frontmatter nests deeper than the subset reads".to_owned(),
            line.range(),
        );
        self.skip_under(spot.indent);
        true
    }

    /// The sequence or mapping beginning at `line`.
    fn collection(&mut self, line: Line<'a>, indent: usize, depth: usize) -> Value {
        let shape = if is_item(line.text) {
            Shape::Sequence(self.sequence(indent, depth))
        } else {
            Shape::Mapping(self.mapping(indent, depth))
        };
        Value {
            span: line.range(),
            shape,
        }
    }

    /// Every item of one block sequence.
    fn sequence(&mut self, indent: usize, depth: usize) -> Vec<Value> {
        let mut items = Vec::new();
        while let Some(line) = self.peek() {
            if line.indent != indent || !is_item(line.text) {
                break;
            }
            items.push(self.item(line, indent, depth));
        }
        items
    }

    /// One sequence item.
    ///
    /// The `- ` is stepped over by rewriting the line to start after it, which
    /// is how `- caption: A` opens a mapping whose remaining keys line up under
    /// `caption` without the grammar needing a second notion of where a line
    /// begins.
    fn item(&mut self, line: Line<'a>, indent: usize, depth: usize) -> Value {
        let after = &line.text[1..];
        let lead = after.len() - after.trim_start_matches(' ').len();
        let inner = &after[lead..];
        let spot = Spot {
            at: line.at + 1 + lead,
            indent,
            depth: depth + 1,
        };
        if inner.is_empty() {
            self.index += 1;
            return self.nested(spot, line.end());
        }
        let opened = Line {
            indent: indent + 1 + lead,
            text: inner,
            at: spot.at,
            start: line.start,
        };
        self.lines[self.index] = opened;
        if !opens_mapping(inner) {
            self.index += 1;
            return self.inline(inner, spot);
        }
        if self.too_deep(opened, spot) {
            return empty(opened.at);
        }
        self.collection(opened, opened.indent, spot.depth)
    }

    /// A value written on the line its key is on.
    fn inline(&mut self, text: &'a str, spot: Spot) -> Value {
        let span = spot.at..spot.at + text.len();
        if let Some(message) = refused_value(text) {
            self.refuse(FaultKind::Unsupported, message, span);
            self.skip_under(spot.indent);
            return empty(spot.at);
        }
        if opens_flow(text) {
            return match flow::read(text, spot.at, spot.depth) {
                Ok(value) => value,
                Err(fault) => {
                    self.faults.push(fault);
                    empty(spot.at)
                }
            };
        }
        if Head(text).opens_quote() {
            return self.quoted(text, spot.at);
        }
        let read = Head(text).plain();
        Value {
            span: spot.at..spot.at + read.len(),
            shape: Shape::Scalar(read.to_owned()),
        }
    }

    /// A quoted scalar, and whatever was written after it.
    fn quoted(&mut self, text: &'a str, at: usize) -> Value {
        let span = at..at + text.len();
        let read = match Head(text).quoted() {
            Ok(read) => read,
            Err(message) => {
                self.refuse(FaultKind::Invalid, message, span);
                return empty(at);
            }
        };
        let trailing = text[read.length..].trim_start();
        if !trailing.is_empty() && !trailing.starts_with('#') {
            self.refuse(
                FaultKind::Invalid,
                "the quoted scalar is followed by more than a comment".to_owned(),
                span,
            );
        }
        Value {
            span: at..at + read.length,
            shape: Shape::Scalar(read.text),
        }
    }
}

/// Where a value sits, and how deep a collection written there would be.
///
/// The three travel together because every one of them is needed at the same
/// moment: the offset to span the value, the indentation to step over a block
/// written under a construct the subset refused, and the depth to know whether
/// a collection there is still one the subset reads.
#[derive(Clone, Copy)]
struct Spot {
    at: usize,
    indent: usize,
    depth: usize,
}

/// A key that carries nothing at all.
///
/// YAML would call this `null`; the subset calls it a scalar with no bytes,
/// because the declared kind decides what a value means and an empty string is
/// what the file actually holds.
fn empty(at: usize) -> Value {
    Value {
        span: at..at,
        shape: Shape::Scalar(String::new()),
    }
}

/// Whether a line opens a sequence item.
fn is_item(text: &str) -> bool {
    text == "-" || text.starts_with("- ")
}

/// Whether a sequence item's content opens a mapping rather than a scalar.
fn opens_mapping(text: &str) -> bool {
    !opens_flow(text) && Head(text).key().is_some()
}

fn opens_flow(text: &str) -> bool {
    text.starts_with('[') || text.starts_with('{')
}

/// Why a line cannot open a mapping entry, when it cannot.
fn refused_key(text: &str) -> Option<(FaultKind, String)> {
    if text.starts_with('\t') {
        let message = "a tab indents this line; the subset indents with spaces".to_owned();
        return Some((FaultKind::Invalid, message));
    }
    if text == "?" || text.starts_with("? ") {
        let message = "`?` writes an explicit key, which the subset refuses".to_owned();
        return Some((FaultKind::Unsupported, message));
    }
    if text.starts_with("<<") {
        let message = "`<<` writes a merge key, which resolves an alias".to_owned();
        return Some((FaultKind::Unsupported, message));
    }
    if opens_flow(text) {
        let message = "a flow collection cannot be a key: keys are strings".to_owned();
        return Some((FaultKind::Unsupported, message));
    }
    refused_value(text).map(|message| (FaultKind::Unsupported, message))
}

/// Why a value cannot be read, when the subset refuses the construct outright.
fn refused_value(text: &str) -> Option<String> {
    REFUSED_OPENERS
        .iter()
        .find(|(opener, _)| text.starts_with(*opener))
        .map(|(opener, what)| format!("`{opener}` writes {what}, which the subset refuses"))
}

#[cfg(test)]
mod tests {
    use super::super::{Front, Parsed, read as frontmatter};
    use super::*;

    fn parse(block: &str) -> Parsed {
        frontmatter(&format!("---\n{block}---\nbody\n"))
    }

    fn entries(block: &str) -> Vec<Entry> {
        let parsed = parse(block);
        parsed
            .front
            .entries()
            .expect("the block must load")
            .to_vec()
    }

    fn scalar_of(entry: &Entry) -> &str {
        entry.value.scalar().expect("a scalar")
    }

    fn items(entry: &Entry) -> Vec<&str> {
        sequence(&entry.value)
            .iter()
            .map(|item| item.scalar().expect("a scalar"))
            .collect()
    }

    fn sequence(value: &Value) -> &[Value] {
        value.sequence().expect("a sequence")
    }

    fn fields(value: &Value) -> Vec<(&str, &str)> {
        value
            .mapping()
            .expect("a mapping")
            .iter()
            .map(|entry| (entry.key.as_str(), scalar_of(entry)))
            .collect()
    }

    fn faults(block: &str) -> Vec<Fault> {
        let parsed = parse(block);
        assert_eq!(parsed.front, Front::Refused, "the block must be refused");
        parsed.faults
    }

    /// Asserts the block is refused for `expected`, at `kind`.
    fn refused_for(block: &str, kind: FaultKind, expected: &str) {
        let faults = faults(block);
        let kinds: Vec<FaultKind> = faults
            .iter()
            .filter(|fault| fault.message.contains(expected))
            .map(|fault| fault.kind)
            .collect();
        let context = format!("{block}: {faults:?}");
        assert_eq!(kinds, [kind], "{context}");
    }

    #[test]
    fn a_mapping_of_scalars_keeps_its_keys_in_the_order_the_note_wrote_them() {
        let read = entries("type: person\nname: Ada\n");
        let pairs: Vec<(&str, &str)> = read
            .iter()
            .map(|entry| (entry.key.as_str(), scalar_of(entry)))
            .collect();
        assert_eq!(pairs, [("type", "person"), ("name", "Ada")]);
    }

    #[test]
    fn a_key_with_nothing_after_it_carries_a_scalar_with_no_bytes() {
        let read = entries("type:\n");
        assert_eq!(scalar_of(&read[0]), "");
        assert_eq!(read[0].value.span, 9..9);
    }

    #[test]
    fn blank_lines_and_comments_carry_nothing_the_grammar_reads() {
        let read = entries("# a comment\n\ntype: person # and another\n\n");
        assert_eq!(read.len(), 1);
        assert_eq!(scalar_of(&read[0]), "person");
    }

    #[test]
    fn a_block_sequence_reads_at_its_keys_indentation_or_beneath_it() {
        let flush = entries("tags:\n- one\n- two\n");
        assert_eq!(items(&flush[0]), ["one", "two"]);
        let indented = entries("tags:\n  - one\n  - two\n");
        assert_eq!(items(&indented[0]), ["one", "two"]);
    }

    #[test]
    fn a_flow_sequence_on_the_key_s_own_line_reads_the_same_way() {
        assert_eq!(items(&entries("tags: [one, two]\n")[0]), ["one", "two"]);
    }

    #[test]
    fn a_mapping_beneath_a_key_is_a_record_value() {
        let read = entries("legal_name:\n  given: Ada\n  family: Lovelace\n");
        assert_eq!(
            fields(&read[0].value),
            [("given", "Ada"), ("family", "Lovelace")]
        );
    }

    #[test]
    fn a_sequence_of_mappings_is_a_list_of_records() {
        let read = entries(concat!(
            "waypoints:\n",
            "  - caption: A\n",
            "    reached_on: 2026-01-01\n",
            "  - caption: B\n",
        ));
        let items = sequence(&read[0].value);
        assert_eq!(
            fields(&items[0]),
            [("caption", "A"), ("reached_on", "2026-01-01")]
        );
        assert_eq!(fields(&items[1]), [("caption", "B")]);
    }

    #[test]
    fn a_sequence_item_that_carries_its_value_beneath_the_dash_is_read_too() {
        let read = entries("waypoints:\n  -\n    caption: A\n");
        assert_eq!(fields(&sequence(&read[0].value)[0]), [("caption", "A")]);
    }

    #[test]
    fn a_quoted_scalar_is_spelling_and_never_typing() {
        let read = entries("link: \"[[Some Note]]\"\nflag: 'true' # and a comment\n");
        assert_eq!(scalar_of(&read[0]), "[[Some Note]]");
        assert_eq!(scalar_of(&read[1]), "true");
        assert_eq!(read[0].value.span, 10..25);
    }

    #[test]
    fn a_quoted_key_is_read_as_the_bytes_it_quoted() {
        let read = entries("\"on\": one\n");
        assert_eq!(
            (read[0].key.as_str(), read[0].key_span.clone()),
            ("on", 4..8)
        );
    }

    #[test]
    fn every_construct_the_subset_refuses_is_named_where_it_is_written() {
        let cases = [
            ("a: &anchor one\n", "an anchor"),
            ("a: *alias\n", "an alias"),
            ("a: !!str one\n", "a tag"),
            ("a: |\n  block\n", "a block scalar"),
            ("a: >\n  folded\n", "a folded block scalar"),
            ("&anchor: one\n  b: two\n\n  c: three\n", "an anchor"),
            ("? explicit\n: one\n", "explicit key"),
            ("?\n", "explicit key"),
            ("<<: *base\n", "merge key"),
            ("[a, b]: one\n", "cannot be a key"),
            ("a: one\n...\nb: two\n", "multi-document stream"),
            ("a: one\na: two\n", "written twice"),
        ];
        for (block, expected) in cases {
            refused_for(block, FaultKind::Unsupported, expected);
        }
    }

    #[test]
    fn every_way_the_grammar_itself_breaks_is_refused_as_invalid() {
        let cases = [
            ("\ta: one\n", "a tab indents this line"),
            ("not an entry\n", "expected `key: value`"),
            ("a: one\n  b: two\n", "indented under a key"),
            ("tags:\n  - one\n  name: Ada\n", "indented under a key"),
            ("- one\n", "a mapping of keys to values"),
            ("a: 'unclosed\n", "not closed"),
            ("a: 'one' two\n", "more than a comment"),
            ("a: [one\n", "expected `,` or `]`"),
        ];
        for (block, expected) in cases {
            refused_for(block, FaultKind::Invalid, expected);
        }
    }

    #[test]
    fn a_key_carrying_nothing_at_all_is_not_the_key_below_it() {
        let read = entries("a:\nb: one\n");
        assert_eq!(scalar_of(&read[0]), "");
        assert_eq!(scalar_of(&read[1]), "one");
    }

    #[test]
    fn a_sequence_ends_where_the_next_key_begins() {
        let read = entries("tags:\n  - one\nname: Ada\n");
        assert_eq!(items(&read[0]), ["one"]);
        assert_eq!(scalar_of(&read[1]), "Ada");
    }

    #[test]
    fn a_refused_key_takes_the_block_written_under_it_with_it() {
        // The anchor is one fault, not one fault plus three complaints about
        // the lines that were indented under it.
        let faults = faults("a: &anchor\n  b: one\n  c: two\nd: one\n");
        assert_eq!(faults.len(), 1);
        assert!(faults[0].message.contains("an anchor"));
    }

    #[test]
    fn a_block_nested_deeper_than_the_subset_reads_is_refused_as_unsupported() {
        // One level below the deepest shape is still read, so that the shape
        // rule is what refuses a shape a note could have meant; below that the
        // walk stops rather than recursing on whatever a file asks for.
        refused_for(
            "a:\n  - b:\n      c: one\n",
            FaultKind::Unsupported,
            "the field `b` holds",
        );
        refused_for(
            "a:\n  - b:\n      c:\n        d: one\n",
            FaultKind::Unsupported,
            "nests deeper",
        );
        // The same bound, met by a sequence item's mapping rather than a key's.
        refused_for(
            "a:\n  - b:\n      - c: one\n",
            FaultKind::Unsupported,
            "nests deeper",
        );
    }

    #[test]
    fn every_diagnosable_fault_is_collected_rather_than_only_the_first() {
        let faults = faults("a: *alias\nb: &anchor\nc: one\nc: two\n");
        let messages: Vec<&str> = faults.iter().map(|fault| fault.message.as_str()).collect();
        assert_eq!(messages.len(), 3, "{messages:?}");
    }

    #[test]
    fn the_grammar_helpers_answer_for_the_shapes_a_line_takes() {
        assert!(is_item("-"));
        assert!(is_item("- one"));
        assert!(!is_item("-5"));
        assert!(!is_item("a"));
        assert!(opens_mapping("caption: A"));
        assert!(!opens_mapping("{caption: A}"));
        assert!(!opens_mapping("a plain scalar"));
        assert!(opens_flow("[a]"));
        assert!(opens_flow("{a: b}"));
        assert!(!opens_flow("a"));
        assert!(refused_key("plain: one").is_none());
        assert!(refused_value("plain").is_none());
    }
}
