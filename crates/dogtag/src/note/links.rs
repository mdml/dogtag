//! What a link looks like, in the dialect the contract declares.
//!
//! `[dialect] links` is corpus-wide, valued `wikilink` or `markdown`. M2
//! parsed, validated and explained it; **this milestone is its first
//! consumer.** The dialect is never sniffed per link: a corpus declares one
//! spelling, and every reference in it — in frontmatter and in prose alike — is
//! read as that spelling.
//!
//! # What a link is, lexically
//!
//! The packet fixes the two dialects' spellings and never says what a
//! relationship *value* must look like inside a frontmatter scalar, so the
//! reading is taken here, where it binds: **the declared dialect's delimiters
//! are stripped when the scalar carries them, and a scalar carrying none is
//! the reference itself.** `employed-by: Acme Corp` under the `wikilink`
//! dialect therefore names the note `Acme Corp` and either resolves or dangles,
//! rather than needing a lexical-fault identifier the records do not name. It
//! is also the reading that honours the markdown-flavor obligation to read what
//! is there, and it keeps every failure inside the two `link.*` identifiers the
//! record's floor already spells.
//!
//! A wikilink value has to be **quoted** in frontmatter: unquoted, `[[Foo]]` is
//! YAML's nested flow sequences and the subset refuses it long before this
//! module sees it. That is the ecosystem's own convention and it costs a note
//! two characters.
//!
//! # What this module deliberately does not know
//!
//! No markdown beyond the delimiters. An image (`![alt](pic.png)`) reads as a
//! reference under the `markdown` dialect, because telling one from the other
//! is the body grammar the document-model record declines to grow — "body
//! content is untouched at M3 beyond link extraction", and a dangling untyped
//! reference is a finding at no severity, so nothing is reported either way.

use core::ops::Range;

use crate::contract::LinkDialect;

/// The reference a link's value names.
///
/// The whole value is one link or it is none: a scalar that opens with the
/// dialect's opening delimiter and ends with its closing one names what sits
/// between them, and every other scalar is the reference it spells.
pub(crate) fn reference(dialect: LinkDialect, written: &str) -> &str {
    Grammar::of(dialect).unwrap(written).unwrap_or(written)
}

/// Every link a body writes, as the note wrote it, in the order it wrote them.
///
/// Delimiters included, so one answer serves both a reader who wants the bytes
/// and [`reference`], which takes them off again.
pub(crate) fn scan(dialect: LinkDialect, text: &str) -> Vec<&str> {
    let grammar = Grammar::of(dialect);
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(span) = grammar.next(text, cursor) {
        cursor = span.end;
        let written = &text[span];
        if grammar.unwrap(written).is_some() {
            found.push(written);
        }
    }
    found
}

/// One dialect's delimiters.
///
/// `split` is what a dialect writing a label alongside its target needs: a
/// markdown link is `[label](target)`, so the target is what follows the last
/// `](` inside the outer pair. A wikilink carries no label, and the whole
/// inside is the reference.
struct Grammar {
    open: &'static str,
    close: &'static str,
    split: Option<&'static str>,
}

impl Grammar {
    fn of(dialect: LinkDialect) -> Self {
        match dialect {
            LinkDialect::Wikilink => Self {
                open: "[[",
                close: "]]",
                split: None,
            },
            LinkDialect::Markdown => Self {
                open: "[",
                close: ")",
                split: Some("]("),
            },
        }
    }

    /// The reference `text` names, when the whole of it is one link.
    fn unwrap<'a>(&self, text: &'a str) -> Option<&'a str> {
        let inside = text.strip_prefix(self.open)?.strip_suffix(self.close)?;
        match self.split {
            Some(split) => inside.rsplit_once(split).map(|(_, target)| target),
            None => Some(inside),
        }
    }

    /// The next delimited run at or after `cursor`, delimiters included.
    fn next(&self, text: &str, cursor: usize) -> Option<Range<usize>> {
        let start = cursor + text[cursor..].find(self.open)?;
        let inside = start + self.open.len();
        let end = inside + text[inside..].find(self.close)? + self.close.len();
        Some(start..end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(dialect: LinkDialect, text: &str) -> Vec<(&str, &str)> {
        scan(dialect, text)
            .into_iter()
            .map(|written| (written, reference(dialect, written)))
            .collect()
    }

    #[test]
    fn a_wikilink_value_names_what_its_brackets_hold() {
        let read = [
            reference(LinkDialect::Wikilink, "[[Analytical Engine]]"),
            reference(LinkDialect::Wikilink, "[[people/ada]]"),
            reference(LinkDialect::Wikilink, "[[]]"),
        ];
        assert_eq!(read, ["Analytical Engine", "people/ada", ""]);
    }

    #[test]
    fn a_markdown_value_names_its_target_rather_than_its_label() {
        let read = [
            reference(LinkDialect::Markdown, "[Ada Lovelace](people/ada.md)"),
            reference(LinkDialect::Markdown, "[](ada.md)"),
        ];
        assert_eq!(read, ["people/ada.md", "ada.md"]);
    }

    #[test]
    fn a_value_that_is_not_one_delimited_link_is_the_reference_it_spells() {
        // The reading this module states: no lexical-fault identifier, because
        // the value either names a note or it does not.
        let plain = [
            reference(LinkDialect::Wikilink, "Acme Corp"),
            reference(LinkDialect::Wikilink, "[[unclosed"),
            reference(LinkDialect::Wikilink, "[[closed]] and more"),
            reference(LinkDialect::Markdown, "[no target]"),
            reference(LinkDialect::Markdown, "people/ada.md"),
        ];
        assert_eq!(
            plain,
            [
                "Acme Corp",
                "[[unclosed",
                "[[closed]] and more",
                "[no target]",
                "people/ada.md",
            ]
        );
    }

    #[test]
    fn a_value_wearing_both_delimiters_is_the_one_link_they_mark() {
        // Two links under one predicate is a *sequence*, which the format
        // already spells. A scalar carrying two is read as the one reference
        // its outer delimiters mark — and names no note, which is reported.
        assert_eq!(
            reference(LinkDialect::Wikilink, "[[a]] and [[b]]"),
            "a]] and [[b"
        );
    }

    #[test]
    fn the_dialect_the_contract_declares_is_the_only_one_read() {
        // A corpus declares one spelling; the other is bytes.
        let crossed = (
            reference(LinkDialect::Wikilink, "[Ada](ada.md)"),
            reference(LinkDialect::Markdown, "[[Ada]]"),
        );
        assert_eq!(crossed, ("[Ada](ada.md)", "[[Ada]]"));
    }

    #[test]
    fn a_body_answers_every_reference_it_writes_in_the_order_it_writes_them() {
        let prose = "See [[Ada]] and then [[people/babbage]], twice: [[Ada]].\n";
        assert_eq!(
            found(LinkDialect::Wikilink, prose),
            [
                ("[[Ada]]", "Ada"),
                ("[[people/babbage]]", "people/babbage"),
                ("[[Ada]]", "Ada"),
            ]
        );
    }

    #[test]
    fn a_body_in_the_markdown_dialect_answers_targets_rather_than_labels() {
        let prose = "See [Ada](people/ada.md) and ![a portrait](pic.png).\n";
        assert_eq!(
            found(LinkDialect::Markdown, prose),
            [
                ("[Ada](people/ada.md)", "people/ada.md"),
                ("[a portrait](pic.png)", "pic.png"),
            ],
            "an image reads as a reference: telling one from the other is body grammar M3 declines"
        );
    }

    #[test]
    fn prose_that_delimits_nothing_answers_nothing() {
        let nothing = [
            found(LinkDialect::Wikilink, "plain prose, no links at all\n"),
            found(LinkDialect::Wikilink, "an opening [[ and no close\n"),
            found(LinkDialect::Markdown, "a [bracketed] aside\n"),
            // Both of the markdown dialect's outer delimiters, with no `](`
            // between them: a run that is delimited but names no target.
            found(LinkDialect::Markdown, "a (parenthetical [aside) here\n"),
        ];
        assert!(nothing.iter().all(Vec::is_empty), "{nothing:?}");
    }

    #[test]
    fn a_reference_holding_the_other_dialects_delimiters_is_still_one_reference() {
        assert_eq!(
            found(LinkDialect::Wikilink, "[[a (b)]] then [[c]]\n"),
            [("[[a (b)]]", "a (b)"), ("[[c]]", "c")]
        );
    }
}
