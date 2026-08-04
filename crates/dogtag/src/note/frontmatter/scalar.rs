//! Reading one scalar, telling a key from a value, and the constructs the
//! subset refuses outright.
//!
//! Nothing here decides what a scalar *means*. Quoting is undone because it is
//! spelling — `"true"` and `true` carry the same four bytes to the declared
//! kind — and a trailing comment is removed because YAML says it is not part of
//! the value. Beyond that the bytes are handed on exactly as they were written,
//! which is what lets the declared kind, rather than the parser's guess, decide
//! what the value is.
//!
//! The refusals live here rather than in either half of the grammar because
//! both halves need them and the subset has **one** answer, not one per style.
//! `[one, *base]` writes the same alias `*base` does, and a reader who is told
//! that one spelling is refused while the other is quietly reinterpreted as
//! text has met exactly the silent reinterpretation the hand-written subset
//! exists to prevent.

use core::str::CharIndices;

/// The escapes a double-quoted scalar admits, beside `\uXXXX`.
const ESCAPES: &[(char, char)] = &[
    ('\\', '\\'),
    ('"', '"'),
    ('/', '/'),
    ('n', '\n'),
    ('t', '\t'),
    ('r', '\r'),
    ('b', '\u{8}'),
    ('f', '\u{c}'),
    ('0', '\0'),
];

/// How many hexadecimal digits a `\u` escape takes.
const UNICODE_DIGITS: usize = 4;

/// The characters that open something the subset refuses outright.
///
/// `&` and `*` are an anchor and an alias, `!` is a tag, and `|` and `>` open a
/// block scalar. Each is named where it is refused, so a corpus using one is
/// told which construct stopped its note rather than that its note is wrong.
const REFUSED_OPENERS: &[(char, &str)] = &[
    ('&', "an anchor"),
    ('*', "an alias"),
    ('!', "a tag"),
    ('|', "a block scalar"),
    ('>', "a folded block scalar"),
];

/// Whether text opens a flow collection.
pub(super) fn opens_flow(text: &str) -> bool {
    text.starts_with('[') || text.starts_with('{')
}

/// Why a value cannot be read, when the subset refuses the construct outright.
pub(super) fn refused_value(text: &str) -> Option<String> {
    REFUSED_OPENERS
        .iter()
        .find(|(opener, _)| text.starts_with(*opener))
        .map(|(opener, what)| format!("`{opener}` writes {what}, which the subset refuses"))
}

/// Why what is written as a key cannot be one, when it cannot.
///
/// A key is refused for everything a value is, and for three more: an explicit
/// key, a merge key — which is an alias wearing a key's clothes — and a flow
/// collection, which is the non-string key the subset names.
pub(super) fn refused_key(text: &str) -> Option<String> {
    if text == "?" || text.starts_with("? ") {
        return Some("`?` writes an explicit key, which the subset refuses".to_owned());
    }
    if text.starts_with("<<") {
        return Some("`<<` writes a merge key, which resolves an alias".to_owned());
    }
    if opens_flow(text) {
        return Some("a flow collection cannot be a key: keys are strings".to_owned());
    }
    refused_value(text)
}

/// A scalar read from the front of some text.
#[derive(Debug)]
pub(super) struct Read {
    /// The bytes, with quoting undone.
    pub(super) text: String,
    /// How many bytes of the input the scalar occupied.
    pub(super) length: usize,
}

/// A key read from the front of a line, and where its value begins.
pub(super) struct Key {
    /// The key's bytes, with quoting undone.
    pub(super) text: String,
    /// How many bytes of the line the key occupied, quotes included.
    pub(super) length: usize,
    /// The offset within the line where the value begins.
    pub(super) value_at: usize,
}

/// Text read from its front.
#[derive(Clone, Copy)]
pub(super) struct Head<'a>(pub(super) &'a str);

impl<'a> Head<'a> {
    /// Whether the text opens a quoted scalar.
    pub(super) fn opens_quote(self) -> bool {
        self.0.starts_with('\'') || self.0.starts_with('"')
    }

    /// Reads the quoted scalar at the front, which [`Head::opens_quote`]
    /// answers for.
    ///
    /// # Errors
    ///
    /// Returns what is wrong, for the caller to turn into a fault with a span.
    pub(super) fn quoted(self) -> Result<Read, String> {
        if let Some(rest) = self.0.strip_prefix('\'') {
            return Self(rest).single();
        }
        let rest = self.0.strip_prefix('"').ok_or("not a quoted scalar")?;
        Self(rest).double()
    }

    /// A single-quoted scalar: no escapes at all, and `''` for one quote.
    fn single(self) -> Result<Read, String> {
        let mut read = String::new();
        let mut rest = self.0.char_indices();
        while let Some((index, character)) = rest.next() {
            if character != '\'' {
                read.push(character);
                continue;
            }
            if rest.as_str().starts_with('\'') {
                read.push('\'');
                rest.next();
                continue;
            }
            return Ok(Read {
                text: read,
                length: index + 2,
            });
        }
        Err("the single-quoted scalar is not closed on this line".to_owned())
    }

    /// A double-quoted scalar, with the escapes the subset admits.
    fn double(self) -> Result<Read, String> {
        let mut read = String::new();
        let mut rest = self.0.char_indices();
        while let Some((index, character)) = rest.next() {
            match character {
                '"' => {
                    return Ok(Read {
                        text: read,
                        length: index + 2,
                    });
                }
                '\\' => read.push(escape(&mut rest)?),
                other => read.push(other),
            }
        }
        Err("the double-quoted scalar is not closed on this line".to_owned())
    }

    /// A plain scalar's bytes: the text up to a trailing comment, trimmed.
    ///
    /// A `#` opens a comment at the start of the scalar or after a space, which
    /// is YAML's own rule and the one that keeps `https://example.com/#anchor`
    /// a value rather than a fragment of one.
    pub(super) fn plain(self) -> &'a str {
        let end = if self.0.starts_with('#') {
            0
        } else {
            self.0.find(" #").unwrap_or(self.0.len())
        };
        self.0[..end].trim_end()
    }

    /// Reads the text as `key: value`, when that is what it is.
    ///
    /// Answers `None` where the text is not a mapping entry at all, which is
    /// how a sequence item tells `- caption: A` from `- a plain scalar`.
    pub(super) fn key(self) -> Option<Key> {
        if self.opens_quote() {
            return self.quoted_key();
        }
        self.plain_key()
    }

    /// A quoted key tolerates spaces before its colon, exactly as a plain key
    /// and a flow mapping's do: `'on' : one` reads as `on : one` does.
    fn quoted_key(self) -> Option<Key> {
        let read = self.quoted().ok()?;
        let after = &self.0[read.length..];
        let spaces = after.len() - after.trim_start_matches(' ').len();
        let rest = after[spaces..].strip_prefix(':')?;
        (rest.is_empty() || rest.starts_with(' ')).then(|| Key {
            value_at: Self(rest).value_at(read.length + spaces + 1),
            text: read.text,
            length: read.length,
        })
    }

    fn plain_key(self) -> Option<Key> {
        let colon = self.colon()?;
        let key = self.0[..colon].trim_end();
        (!key.is_empty()).then(|| Key {
            text: key.to_owned(),
            length: key.len(),
            value_at: Self(&self.0[colon + 1..]).value_at(colon + 1),
        })
    }

    /// The offset of the first `:` that ends a key: one followed by a space, or
    /// one ending the line.
    fn colon(self) -> Option<usize> {
        self.0
            .char_indices()
            .filter(|(_, character)| *character == ':')
            .map(|(index, _)| index)
            .find(|index| Self(&self.0[index + 1..]).ends_key())
    }

    fn ends_key(self) -> bool {
        self.0.is_empty() || self.0.starts_with(' ')
    }

    /// Where a value begins: past the colon at `after_colon`, and any spaces.
    fn value_at(self, after_colon: usize) -> usize {
        after_colon + self.0.len() - self.0.trim_start_matches(' ').len()
    }
}

/// The character one escape sequence stands for.
fn escape(rest: &mut CharIndices<'_>) -> Result<char, String> {
    let (_, marker) = rest.next().ok_or("a `\\` ends the scalar")?;
    if marker == 'u' {
        return unicode(rest);
    }
    ESCAPES
        .iter()
        .find(|(spelling, _)| *spelling == marker)
        .map(|(_, character)| *character)
        .ok_or(format!("`\\{marker}` is not an escape the subset admits"))
}

/// The character a `\uXXXX` escape names.
fn unicode(rest: &mut CharIndices<'_>) -> Result<char, String> {
    let digits: String = rest.take(UNICODE_DIGITS).map(|(_, digit)| digit).collect();
    let code = u32::from_str_radix(&digits, 16)
        .map_err(|_| format!("`\\u{digits}` is not four hexadecimal digits"))?;
    char::from_u32(code).ok_or(format!("`\\u{digits}` is not a Unicode scalar value"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_quoted(text: &str) -> (String, usize) {
        let read = Head(text).quoted().expect("a closed scalar");
        (read.text, read.length)
    }

    fn refuse(text: &str) -> String {
        Head(text).quoted().expect_err("refused")
    }

    /// A key as `(bytes, length, where the value begins)`.
    fn key_of(text: &str) -> Option<(String, usize, usize)> {
        Head(text)
            .key()
            .map(|key| (key.text, key.length, key.value_at))
    }

    #[test]
    fn a_single_quoted_scalar_takes_its_bytes_as_written() {
        let read = [
            read_quoted("'[[Some Note]]'"),
            read_quoted("'true' # a comment"),
            read_quoted("''"),
        ];
        assert_eq!(
            read,
            [
                ("[[Some Note]]".to_owned(), 15),
                ("true".to_owned(), 6),
                (String::new(), 2)
            ]
        );
    }

    #[test]
    fn a_single_quoted_scalar_writes_one_quote_as_two() {
        let read = (read_quoted("'it''s'"), read_quoted("''''"));
        assert_eq!(read, (("it's".to_owned(), 7), ("'".to_owned(), 4)));
    }

    #[test]
    fn a_double_quoted_scalar_undoes_every_escape_the_subset_admits() {
        for (spelling, character) in ESCAPES {
            let source = format!("\"a\\{spelling}b\"");
            assert_eq!(read_quoted(&source).0, format!("a{character}b"));
        }
        assert_eq!(read_quoted("\"\\u00e9t\\u00e9\"").0, "été");
    }

    #[test]
    fn a_double_quoted_scalar_refuses_an_escape_it_does_not_admit() {
        let refused = [
            ("\"a\\qb\"", "not an escape"),
            ("\"a\\", "ends the scalar"),
            ("\"\\uzzzz\"", "hexadecimal"),
            ("\"\\ud800\"", "not a Unicode scalar value"),
        ];
        for (source, expected) in refused {
            let message = refuse(source);
            assert!(message.contains(expected), "{source}: {message}");
        }
    }

    #[test]
    fn an_unclosed_quoted_scalar_says_which_quote_opened_it() {
        // The message names the quote the reader has to close.
        let messages = (refuse("'one"), refuse("\"one"));
        assert!(messages.0.contains("single-quoted"), "{messages:?}");
        assert!(messages.1.contains("double-quoted"), "{messages:?}");
        let opens = (
            Head("'a'").opens_quote(),
            Head("\"a\"").opens_quote(),
            Head("a").opens_quote(),
        );
        assert_eq!(opens, (true, true, false));
    }

    #[test]
    fn a_plain_scalar_loses_a_trailing_comment_and_nothing_else() {
        let read = [
            Head("person  # the type").plain(),
            Head("https://example.com/#anchor").plain(),
            Head("a value with spaces").plain(),
            Head("#not-a-value").plain(),
        ];
        assert_eq!(
            read,
            [
                "person",
                "https://example.com/#anchor",
                "a value with spaces",
                ""
            ]
        );
    }

    #[test]
    fn a_line_that_is_a_mapping_entry_reports_its_key_and_where_the_value_is() {
        let read = key_of("type: person");
        assert_eq!(read, Some(("type".to_owned(), 4, 6)));
        let bare = key_of("type:");
        assert_eq!(bare, Some(("type".to_owned(), 4, 5)));
        let spaced = key_of("type   :    person");
        assert_eq!(spaced, Some(("type".to_owned(), 4, 12)));
    }

    #[test]
    fn a_quoted_key_is_a_key_and_keeps_the_bytes_it_quoted() {
        let read = key_of("\"a: b\": one");
        assert_eq!(read, Some(("a: b".to_owned(), 6, 8)));
        let bare = key_of("'on':");
        assert_eq!(bare, Some(("on".to_owned(), 4, 5)));
    }

    #[test]
    fn a_quoted_key_tolerates_spaces_before_its_colon_as_a_plain_key_does() {
        // `'on' : one` parses like `on : one`: the same key and the same
        // value, with the quotes counted only in the key's own extent.
        let spaced = key_of("'on' : one");
        assert_eq!(spaced, Some(("on".to_owned(), 4, 7)));
        let wide = key_of("\"on\"   :   one");
        assert_eq!(wide, Some(("on".to_owned(), 4, 11)));
        let bare = key_of("'on' :");
        assert_eq!(bare, Some(("on".to_owned(), 4, 6)));
        // Spaces end the tolerance at the colon: quoted text followed by
        // anything else is still a value rather than a key.
        assert_eq!(key_of("'on' x: y"), None);
    }

    #[test]
    fn text_that_is_not_a_mapping_entry_is_not_read_as_one() {
        // A URL's colon is followed by a slash, an empty key addresses nothing,
        // and a quoted scalar with nothing after it is a value.
        let text = [
            "https://example.com",
            ": one",
            "'quoted'",
            "\"unclosed",
            "x",
        ];
        let read: Vec<bool> = text.iter().map(|text| key_of(text).is_some()).collect();
        assert_eq!(read, [false, false, false, false, false]);
    }
}
