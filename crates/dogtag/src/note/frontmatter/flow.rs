//! Flow collections: `[one, two]` and `{ x: one }`.
//!
//! Flow style is admitted because a corpus already writes it — `tags: [a, b]`
//! is what a note looks like in the wild — and because refusing it would make
//! the subset refuse ordinary files for no reason a record gives.
//!
//! One narrowing is this module's own and is stated here rather than
//! discovered: **a flow collection closes on the line it opens on.** A
//! multi-line flow collection is refused as unclosed. Nothing in frontmatter
//! needs one, and reading across lines would mean the line-oriented half of the
//! subset and this one disagree about where a value ends.
//!
//! Everything the subset refuses is refused here as well, on the same terms
//! [`super::scalar`] states: an anchor, an alias, a tag, a merge key, a
//! non-string key and a repeated key are constructs of the language rather than
//! of a style, and `[one, *base]` writes the alias `*base` writes. Two kinds of
//! fault come out of this module and they behave differently on purpose. A
//! **grammar** fault ends the read, because a flow collection that stops making
//! sense mid-way has no further structure to report against; a **refusal** does
//! not, because the bytes still parse and a note carrying two aliases deserves
//! to be told about both.

use core::ops::Range;

use super::scalar::{self, Head};
use super::{Entry, Fault, FaultKind, MAX_NESTING, Shape, Value};

/// What ends a plain scalar inside a flow collection.
const DELIMITERS: [char; 3] = [',', ']', '}'];

/// Reads `text` as one flow collection at `depth`, and nothing after it.
///
/// Answers the value the bytes hold, and every construct the subset refused
/// inside them. There is no value when a grammar fault ended the read, and the
/// faults are then never empty.
pub(super) fn read(text: &str, at: usize, depth: usize) -> (Option<Value>, Vec<Fault>) {
    let mut flow = Flow {
        text,
        at,
        index: 0,
        faults: Vec::new(),
    };
    match flow.document(depth) {
        Ok(value) => (Some(value), flow.faults),
        Err(fault) => {
            flow.faults.push(fault);
            (None, flow.faults)
        }
    }
}

/// Which flow collection is being read.
#[derive(Clone, Copy)]
enum Bracket {
    Sequence,
    Mapping,
}

impl Bracket {
    fn closing(self) -> char {
        match self {
            Self::Sequence => ']',
            Self::Mapping => '}',
        }
    }

    fn empty(self) -> Members {
        match self {
            Self::Sequence => Members::Items(Vec::new()),
            Self::Mapping => Members::Entries(Vec::new()),
        }
    }
}

/// A flow collection's members, as it is being read.
enum Members {
    Items(Vec<Value>),
    Entries(Vec<Entry>),
}

impl Members {
    fn shape(self) -> Shape {
        match self {
            Self::Items(items) => Shape::Sequence(items),
            Self::Entries(entries) => Shape::Mapping(entries),
        }
    }
}

/// A cursor over one line's flow text, and what it has refused so far.
struct Flow<'a> {
    text: &'a str,
    at: usize,
    index: usize,
    faults: Vec<Fault>,
}

impl<'a> Flow<'a> {
    /// The whole collection, with nothing after it but a comment.
    fn document(&mut self, depth: usize) -> Result<Value, Fault> {
        let value = self.value(depth)?;
        self.skip_spaces();
        if self.rest().is_empty() || self.rest().starts_with('#') {
            Ok(value)
        } else {
            Err(self.invalid("the flow collection is followed by more than a comment".to_owned()))
        }
    }

    fn rest(&self) -> &'a str {
        &self.text[self.index..]
    }

    fn offset(&self) -> usize {
        self.at + self.index
    }

    fn skip_spaces(&mut self) {
        self.index += self.rest().len() - self.rest().trim_start_matches(' ').len();
    }

    /// Consumes `character` when it is next.
    fn take(&mut self, character: char) -> bool {
        match self.rest().starts_with(character) {
            true => {
                self.index += character.len_utf8();
                true
            }
            false => false,
        }
    }

    fn invalid(&self, message: String) -> Fault {
        Fault::new(FaultKind::Invalid, message, self.offset()..self.end())
    }

    /// Records a construct the subset refuses, and reads on.
    fn refuse(&mut self, message: String, span: Range<usize>) {
        self.faults
            .push(Fault::new(FaultKind::Unsupported, message, span));
    }

    fn end(&self) -> usize {
        self.at + self.text.len()
    }

    /// Refuses a collection that would sit deeper than the subset reads.
    fn deeper(&self, depth: usize) -> Result<(), Fault> {
        if depth > MAX_NESTING {
            return Err(Fault::new(
                FaultKind::Unsupported,
                "the frontmatter nests deeper than the subset reads".to_owned(),
                self.offset()..self.end(),
            ));
        }
        Ok(())
    }

    fn value(&mut self, depth: usize) -> Result<Value, Fault> {
        self.skip_spaces();
        match self.rest().chars().next() {
            Some('[') => self.collection(depth, Bracket::Sequence),
            Some('{') => self.collection(depth, Bracket::Mapping),
            _ => self.scalar(),
        }
    }

    /// One flow collection: its members, up to the bracket that closes it.
    fn collection(&mut self, depth: usize, bracket: Bracket) -> Result<Value, Fault> {
        let start = self.offset();
        self.deeper(depth)?;
        self.index += 1;
        let mut members = bracket.empty();
        loop {
            self.skip_spaces();
            if self.take(bracket.closing()) {
                break;
            }
            self.member(depth, &mut members)?;
            if !self.separated(bracket.closing())? {
                break;
            }
        }
        Ok(Value {
            span: start..self.offset(),
            shape: members.shape(),
        })
    }

    /// One member: an item of a sequence, or an entry of a mapping.
    fn member(&mut self, depth: usize, members: &mut Members) -> Result<(), Fault> {
        match members {
            Members::Items(items) => items.push(self.value(depth + 1)?),
            Members::Entries(entries) => {
                let entry = self.entry(depth)?;
                self.repeated(&entry, entries);
                entries.push(entry);
            }
        }
        Ok(())
    }

    /// Refuses a key the mapping already carries, as the block half does.
    ///
    /// A duplicate key is refused rather than resolved by last-wins, which is
    /// what a loader would do silently — and `{given: A, given: B}` says two
    /// different things about one field of one record.
    fn repeated(&mut self, entry: &Entry, entries: &[Entry]) {
        if entries.iter().any(|seen| seen.key == entry.key) {
            let message = format!("the key `{}` is written twice", entry.key);
            self.refuse(message, entry.key_span.clone());
        }
    }

    /// Whether another member follows: `true` after a comma, `false` after the
    /// closing bracket.
    ///
    /// # Errors
    ///
    /// Returns a fault when neither follows, which is what an unclosed flow
    /// collection ends as.
    fn separated(&mut self, closing: char) -> Result<bool, Fault> {
        self.skip_spaces();
        if self.take(',') {
            return Ok(true);
        }
        if self.take(closing) {
            return Ok(false);
        }
        Err(self.invalid(format!(
            "expected `,` or `{closing}` in the flow collection"
        )))
    }

    fn entry(&mut self, depth: usize) -> Result<Entry, Fault> {
        let start = self.offset();
        let key = self.key()?;
        let key_span = start..self.offset();
        self.skip_spaces();
        if !self.take(':') {
            return Err(self.invalid("expected `:` after a flow mapping's key".to_owned()));
        }
        Ok(Entry {
            key,
            key_span,
            value: self.value(depth + 1)?,
        })
    }

    /// A flow mapping's key: quoted, or the bytes up to its `:`.
    ///
    /// A quoted key needs no refusal test, because quoting is spelling: `'*a'`
    /// names a field called `*a` rather than writing an alias.
    fn key(&mut self) -> Result<String, Fault> {
        if Head(self.rest()).opens_quote() {
            return Ok(self.take_quoted()?.0);
        }
        let start = self.offset();
        let end = self.rest().find(':').unwrap_or(self.rest().len());
        let key = self.rest()[..end].trim_end().to_owned();
        if key.is_empty() {
            return Err(self.invalid("a flow mapping's key is empty".to_owned()));
        }
        self.index += end;
        if let Some(message) = scalar::refused_key(&key) {
            self.refuse(message, start..self.offset());
        }
        Ok(key)
    }

    fn scalar(&mut self) -> Result<Value, Fault> {
        let start = self.offset();
        if Head(self.rest()).opens_quote() {
            let (text, _) = self.take_quoted()?;
            return Ok(Value {
                span: start..self.offset(),
                shape: Shape::Scalar(text),
            });
        }
        let end = self.rest().find(DELIMITERS).unwrap_or(self.rest().len());
        let text = self.rest()[..end].trim_end().to_owned();
        if text.is_empty() {
            return Err(self.invalid("a flow collection holds an empty value".to_owned()));
        }
        self.index += end;
        let span = start..start + text.len();
        if let Some(message) = scalar::refused_value(&text) {
            self.refuse(message, span.clone());
        }
        Ok(Value {
            span,
            shape: Shape::Scalar(text),
        })
    }

    /// Reads the quoted scalar at the cursor, advancing past it.
    fn take_quoted(&mut self) -> Result<(String, Range<usize>), Fault> {
        let start = self.offset();
        let read = Head(self.rest())
            .quoted()
            .map_err(|message| self.invalid(message))?;
        self.index += read.length;
        Ok((read.text, start..self.offset()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(text: &str) -> Value {
        let (value, faults) = read(text, 0, 1);
        assert!(faults.is_empty(), "{text}: {faults:?}");
        value.expect("a well-formed flow collection")
    }

    fn scalars(value: &Value) -> Vec<String> {
        value
            .sequence()
            .expect("a sequence")
            .iter()
            .map(|item| item.scalar().expect("a scalar").to_owned())
            .collect()
    }

    fn entries(value: &Value) -> &[Entry] {
        value.mapping().expect("a mapping")
    }

    /// The one fault `text` is refused for.
    fn refused(text: &str) -> Fault {
        let (_, faults) = read(text, 0, 1);
        assert_eq!(faults.len(), 1, "{text}: {faults:?}");
        faults.into_iter().next().expect("one fault")
    }

    #[test]
    fn a_flow_sequence_reads_its_items_in_order() {
        let read = value("[one, two, three]");
        assert_eq!(scalars(&read), ["one", "two", "three"]);
        assert_eq!(read.span, 0..17);
    }

    #[test]
    fn a_flow_sequence_may_be_empty_and_may_end_with_a_comma() {
        assert_eq!(scalars(&value("[]")), Vec::<String>::new());
        assert_eq!(scalars(&value("[ one , ]")), ["one"]);
        assert_eq!(scalars(&value("[one] # a comment")), ["one"]);
    }

    #[test]
    fn a_flow_sequence_keeps_a_quoted_items_bytes_as_written() {
        assert_eq!(
            scalars(&value("['[[Ada]]', \"a, b\"]")),
            ["[[Ada]]", "a, b"]
        );
    }

    #[test]
    fn a_flow_mapping_reads_its_entries_in_order() {
        let read = value("{caption: A, reached_on: 2026-01-01}");
        let entries = entries(&read);
        let pairs: Vec<(&str, Option<&str>)> = entries
            .iter()
            .map(|entry| (entry.key.as_str(), entry.value.scalar()))
            .collect();
        assert_eq!(
            pairs,
            [("caption", Some("A")), ("reached_on", Some("2026-01-01"))]
        );
        assert_eq!(entries[0].key_span, 1..8);
    }

    #[test]
    fn a_flow_mapping_may_be_empty_and_may_carry_a_quoted_key() {
        let read = value("{'on': yes}");
        assert_eq!(
            entries(&read)[0].key,
            "on",
            "a key is its bytes, never a boolean"
        );
        assert!(entries(&value("{}")).is_empty());
    }

    #[test]
    fn a_sequence_of_mappings_is_read_because_a_list_of_records_is_written_that_way() {
        let read = value("[{caption: A}, {caption: B}]");
        let items = read.sequence().expect("a sequence");
        assert_eq!(items.len(), 2);
        assert_eq!(entries(&items[1])[0].key, "caption");
    }

    #[test]
    fn a_collection_deeper_than_the_subset_reads_is_refused_as_unsupported() {
        let fault = refused("[[[[one]]]]");
        assert_eq!(fault.kind, FaultKind::Unsupported);
        assert!(fault.message.contains("nests deeper"));
    }

    #[test]
    fn every_way_a_flow_collection_stops_making_sense_is_refused_with_its_reason() {
        let cases = [
            ("[one", "expected `,` or `]`"),
            ("{a: one", "expected `,` or `}`"),
            ("[one,,]", "empty value"),
            ("{: one}", "key is empty"),
            ("{a one}", "expected `:`"),
            ("['unclosed]", "not closed"),
            ("{'unclosed: one}", "not closed"),
            ("[one] two", "more than a comment"),
        ];
        for (source, expected) in cases {
            let fault = refused(source);
            assert_eq!(fault.kind, FaultKind::Invalid, "{source}");
            assert!(fault.message.contains(expected), "{source}: {fault:?}");
        }
    }

    #[test]
    fn every_construct_the_subset_refuses_is_refused_in_flow_style_too() {
        // Brackets are a style, not a language: an alias written inside them is
        // the same alias, and reading it as the literal text `*base` would be
        // the silent reinterpretation the whole subset exists to prevent.
        let cases = [
            ("[one, *base]", "an alias"),
            ("[&anchor one]", "an anchor"),
            ("[!!str one]", "a tag"),
            ("[|one]", "a block scalar"),
            ("{caption: *base}", "an alias"),
            ("{caption: !!str A}", "a tag"),
            ("{&anchor: one}", "an anchor"),
            ("{<<: base}", "a merge key"),
            ("{? a: one}", "an explicit key"),
            ("{[a, b]: one}", "cannot be a key"),
            ("{given: A, given: B}", "written twice"),
            ("[{given: A, given: B}]", "written twice"),
        ];
        for (source, expected) in cases {
            let fault = refused(source);
            assert_eq!(fault.kind, FaultKind::Unsupported, "{source}");
            assert!(fault.message.contains(expected), "{source}: {fault:?}");
        }
    }

    #[test]
    fn every_refusal_inside_one_collection_is_collected_rather_than_only_the_first() {
        // A refusal is not a grammar fault: the bytes still parse, so the read
        // goes on and the note is told about all of them at once.
        let (value, faults) = read("[*one, *two]", 0, 1);
        let messages: Vec<&str> = faults.iter().map(|fault| fault.message.as_str()).collect();
        assert_eq!(messages.len(), 2, "{messages:?}");
        assert!(value.is_some());
    }

    #[test]
    fn a_quoted_member_that_looks_like_a_refused_construct_is_ordinary_bytes() {
        assert_eq!(
            scalars(&value("['*base', \"&anchor\"]")),
            ["*base", "&anchor"]
        );
        assert_eq!(entries(&value("{'*a': one}"))[0].key, "*a");
    }

    #[test]
    fn a_refusal_points_at_the_construct_that_was_written() {
        let alias = refused("[one, *base]");
        assert_eq!(alias.span, 6..11);
        let repeat = refused("{a: one, a: two}");
        assert_eq!(repeat.span, 9..10);
    }

    #[test]
    fn a_span_is_measured_over_the_files_own_bytes() {
        let (read, faults) = read("[one]", 40, 1);
        let read = read.expect("well-formed");
        assert!(faults.is_empty());
        assert_eq!(read.span, 40..45);
        assert_eq!(read.sequence().expect("a sequence")[0].span, 41..44);
    }
}
