//! Reading an asset's bytes, and turning byte offsets into positions.
//!
//! Both committed and local assets are UTF-8 without a byte order mark, with LF
//! line endings. On read, invalid UTF-8, a BOM, and a carriage return each
//! produce their own fault — **checked in that order** — and the SDK never
//! normalizes: spans are measured in Unicode scalar values and byte offsets, so
//! silently rewriting the bytes would make every span a lie.
//!
//! There are exactly **three** read faults. A missing trailing newline is an
//! emission rule, not a read fault, and adding a fourth here would refuse files
//! the format accepts.

use core::ops::Range;
use core::str;

use crate::diagnostic::{Position, Span};

/// Why an asset's bytes could not be accepted as written.
///
/// Each variant maps to a per-asset diagnostic — the contract's or the
/// installation record's — chosen by the caller, so the two areas keep their
/// own identifiers without this module knowing about either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodingFault {
    /// The bytes are not valid UTF-8; `offset` is the first invalid byte.
    InvalidUtf8 {
        /// Byte offset of the first invalid byte.
        offset: usize,
    },
    /// The text begins with a UTF-8 byte order mark.
    ByteOrderMark,
    /// The text holds a carriage return; `offset` is the first one.
    CarriageReturn {
        /// Byte offset of the first carriage return.
        offset: usize,
    },
}

impl EncodingFault {
    /// The byte offset the fault was found at, when it has one.
    pub fn offset(self) -> Option<usize> {
        match self {
            Self::InvalidUtf8 { offset } | Self::CarriageReturn { offset } => Some(offset),
            Self::ByteOrderMark => None,
        }
    }

    /// A message describing the fault, shared by every asset that reads bytes.
    pub fn describe(self) -> String {
        match self {
            Self::InvalidUtf8 { offset } => {
                format!("the file is not valid UTF-8; the first invalid byte is at offset {offset}")
            }
            Self::ByteOrderMark => "the file begins with a UTF-8 byte order mark".to_owned(),
            Self::CarriageReturn { offset } => format!(
                "the file uses carriage-return line endings; the first carriage return is at \
                 offset {offset}"
            ),
        }
    }
}

/// An asset's text, with the index that turns a byte offset into a position.
#[derive(Clone, Debug)]
pub struct Text {
    text: String,
    line_starts: Vec<usize>,
}

impl Text {
    fn new(text: String) -> Self {
        let line_starts = line_starts(&text);
        Self { text, line_starts }
    }

    /// The text exactly as it was read — never normalized.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// The position of a byte offset.
    ///
    /// The line is 1-based, and the column is 1-based **counted in Unicode
    /// scalar values** rather than bytes or UTF-16 code units. An offset past
    /// the end of the text reports the end of the text.
    pub fn position(&self, offset: usize) -> Position {
        let offset = offset.min(self.text.len());
        let index = self.line_index(offset);
        let start = self.line_starts[index];
        Position::new(
            one_based(index),
            one_based(scalars_before(&self.text[start..], offset - start)),
            offset,
        )
    }

    /// The span of a byte range, as the TOML parser reports one.
    pub fn span(&self, range: Range<usize>) -> Span {
        Span::between(self.position(range.start), self.position(range.end))
    }

    fn line_index(&self, offset: usize) -> usize {
        self.line_starts.partition_point(|start| *start <= offset) - 1
    }
}

/// Reads bytes as an asset's text.
///
/// # Errors
///
/// Returns the first of invalid UTF-8, a byte order mark, and a carriage
/// return, checked in that order.
pub fn inspect(bytes: &[u8]) -> Result<Text, EncodingFault> {
    let text = str::from_utf8(bytes).map_err(|error| EncodingFault::InvalidUtf8 {
        offset: error.valid_up_to(),
    })?;
    check_byte_order_mark(text)?;
    check_carriage_return(text)?;
    Ok(Text::new(text.to_owned()))
}

fn check_byte_order_mark(text: &str) -> Result<(), EncodingFault> {
    if text.starts_with('\u{feff}') {
        Err(EncodingFault::ByteOrderMark)
    } else {
        Ok(())
    }
}

fn check_carriage_return(text: &str) -> Result<(), EncodingFault> {
    match text.find('\r') {
        Some(offset) => Err(EncodingFault::CarriageReturn { offset }),
        None => Ok(()),
    }
}

/// The byte offset each line starts at, line 1 first.
fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(text.match_indices('\n').map(|(index, _)| index + 1));
    starts
}

/// How many Unicode scalar values of `line` precede `byte_offset`.
fn scalars_before(line: &str, byte_offset: usize) -> usize {
    line.char_indices()
        .take_while(|(index, _)| *index < byte_offset)
        .count()
}

/// A 0-based index as a 1-based number, saturating rather than wrapping on a
/// file no machine will produce.
fn one_based(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text_of(source: &str) -> Text {
        inspect(source.as_bytes()).expect("well-formed")
    }

    #[test]
    fn well_formed_bytes_are_kept_exactly() {
        let text = text_of("contract_version = 1\n");
        assert_eq!(text.as_str(), "contract_version = 1\n");
        assert!(format!("{text:?}").contains("contract_version"));
        let copy = text.clone();
        assert_eq!(copy.as_str(), text.as_str());
    }

    #[test]
    fn invalid_utf8_is_reported_before_anything_else() {
        let mut bytes = "\u{feff}".as_bytes().to_vec();
        bytes.extend_from_slice(b"\xff\r\n");
        assert_eq!(
            inspect(&bytes).expect_err("invalid"),
            EncodingFault::InvalidUtf8 { offset: 3 }
        );
    }

    #[test]
    fn a_byte_order_mark_is_reported_before_a_carriage_return() {
        assert_eq!(
            inspect("\u{feff}a = 1\r\n".as_bytes()).expect_err("a mark"),
            EncodingFault::ByteOrderMark
        );
    }

    #[test]
    fn a_carriage_return_is_reported_with_its_offset() {
        assert_eq!(
            inspect(b"a = 1\r\nb = 2\n").expect_err("a carriage return"),
            EncodingFault::CarriageReturn { offset: 5 }
        );
        assert_eq!(
            inspect(b"a = 1\rb = 2").expect_err("a carriage return"),
            EncodingFault::CarriageReturn { offset: 5 }
        );
    }

    #[test]
    fn faults_carry_an_offset_and_a_description() {
        let invalid = EncodingFault::InvalidUtf8 { offset: 3 };
        let mark = EncodingFault::ByteOrderMark;
        let carriage = EncodingFault::CarriageReturn { offset: 5 };
        assert_eq!(invalid.offset(), Some(3));
        assert_eq!(mark.offset(), None);
        assert_eq!(carriage.offset(), Some(5));
        assert!(invalid.describe().contains("offset 3"));
        assert!(mark.describe().contains("byte order mark"));
        assert!(carriage.describe().contains("offset 5"));
        let faults = vec![invalid, mark, carriage];
        assert_eq!(faults.clone(), faults);
        assert!(format!("{faults:?}").contains("ByteOrderMark"));
    }

    #[test]
    fn a_missing_trailing_newline_is_not_a_read_fault() {
        assert_eq!(text_of("a = 1").as_str(), "a = 1");
    }

    #[test]
    fn positions_count_lines_from_one() {
        let text = text_of("one\ntwo\nthree\n");
        assert_eq!(text.position(0), Position::new(1, 1, 0));
        assert_eq!(text.position(4), Position::new(2, 1, 4));
        assert_eq!(text.position(6), Position::new(2, 3, 6));
        assert_eq!(text.position(8), Position::new(3, 1, 8));
    }

    #[test]
    fn columns_count_unicode_scalar_values_not_bytes() {
        // `é` is two bytes, `→` is three: the third scalar is at byte 5.
        let text = text_of("a = \"éa→b\"\n");
        assert_eq!(text.position(5).column, 6);
        assert_eq!(text.position(7).column, 7);
        assert_eq!(text.position(8).column, 8);
    }

    #[test]
    fn columns_count_an_astral_plane_character_as_one() {
        // U+1D11E is four bytes, and one scalar value.
        let text = text_of("x = \"\u{1d11e}!\"\n");
        assert_eq!(text.position(5).column, 6);
        assert_eq!(text.position(9), Position::new(1, 7, 9));
    }

    #[test]
    fn an_offset_past_the_end_reports_the_end() {
        let text = text_of("ab\n");
        assert_eq!(text.position(999), Position::new(2, 1, 3));
    }

    #[test]
    fn a_byte_range_becomes_a_span() {
        let text = text_of("links = \"wikilink\"\n");
        assert_eq!(
            text.span(8..18),
            Span::between(Position::new(1, 9, 8), Position::new(1, 19, 18))
        );
    }
}
