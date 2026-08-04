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
//! # The alias and the anchor
//!
//! Inside a wikilink's inner text, the first `|` splits off a **display
//! alias**: everything after it is cosmetic — never validated, never part of
//! resolution. A markdown link's label is native to its syntax and comes off
//! with the delimiters, and a value carrying no delimiters is the reference
//! part itself, so the alias split runs only between a wikilink's brackets.
//!
//! Within the reference part — whichever dialect and spelling produced it —
//! the first `#` splits the **target** from a **fragment**. The fragment is
//! unvalidated at M3 and never part of resolution, with one shape it alone
//! expresses: an empty target with a nonempty fragment (`[[#heading]]`) names
//! the note it is written in, so it is never dangling. Bare-name against
//! path-qualified is classified on the target after both splits.
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

/// One link's value, split into the parts resolution reads.
///
/// The display alias is not carried: it is cosmetic and nothing downstream may
/// consult it, so the parse discards it where a field would invite a reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Reference<'a> {
    /// What resolution reads: the reference part before any `#`.
    pub(crate) target: &'a str,
    /// What follows the first `#` of the reference part, when one is written.
    /// Unvalidated at M3, and never part of resolution.
    pub(crate) fragment: Option<&'a str>,
}

impl Reference<'_> {
    /// Whether this is `[[#heading]]`: an empty target with a nonempty
    /// fragment, which names the note it is written in and never dangles.
    pub(crate) fn names_own_note(&self) -> bool {
        self.target.is_empty() && self.fragment.is_some_and(|fragment| !fragment.is_empty())
    }
}

/// The reference a link's value names.
///
/// The whole value is one link or it is none: a scalar that opens with the
/// dialect's opening delimiter and ends with its closing one names what sits
/// between them — its alias split off under `wikilink` — and every other
/// scalar is the reference part it spells. Either way, the first `#` of the
/// reference part then splits the target from its fragment.
pub(crate) fn reference(dialect: LinkDialect, written: &str) -> Reference<'_> {
    let part = Grammar::of(dialect).unwrap(written).unwrap_or(written);
    let (target, fragment) = match part.split_once('#') {
        Some((target, fragment)) => (target, Some(fragment)),
        None => (part, None),
    };
    Reference { target, fragment }
}

/// Every link a body writes, as byte ranges into `text`, in the order it wrote
/// them.
///
/// Ranges rather than slices, because a `link.*` finding is addressed to the
/// reference that carries it and a range is what a span is measured from. The
/// delimiters are inside the range, so one answer serves both a reader who
/// wants the bytes and [`reference`], which takes them off again.
pub(crate) fn scan(dialect: LinkDialect, text: &str) -> Vec<Range<usize>> {
    let grammar = Grammar::of(dialect);
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(span) = grammar.next(text, cursor) {
        cursor = span.end;
        if grammar.unwrap(&text[span.clone()]).is_some() {
            found.push(span);
        }
    }
    found
}

/// One dialect's delimiters.
///
/// `split` is what a dialect writing a label alongside its target needs: a
/// markdown link is `[label](target)`, so the target is what follows the last
/// `](` inside the outer pair. A wikilink instead writes its label *after* the
/// reference — `alias` is the `|` that introduces it, and the reference is
/// what sits before the first one.
struct Grammar {
    open: &'static str,
    close: &'static str,
    split: Option<&'static str>,
    alias: Option<char>,
}

impl Grammar {
    fn of(dialect: LinkDialect) -> Self {
        match dialect {
            LinkDialect::Wikilink => Self {
                open: "[[",
                close: "]]",
                split: None,
                alias: Some('|'),
            },
            LinkDialect::Markdown => Self {
                open: "[",
                close: ")",
                split: Some("]("),
                alias: None,
            },
        }
    }

    /// The reference `text` names, when the whole of it is one link.
    fn unwrap<'a>(&self, text: &'a str) -> Option<&'a str> {
        let inside = text.strip_prefix(self.open)?.strip_suffix(self.close)?;
        let labelled = match self.split {
            Some(split) => inside.rsplit_once(split)?.1,
            None => inside,
        };
        Some(match self.alias {
            Some(alias) => labelled
                .split_once(alias)
                .map_or(labelled, |(reference, _)| reference),
            None => labelled,
        })
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

    fn target(dialect: LinkDialect, written: &str) -> &str {
        reference(dialect, written).target
    }

    fn found(dialect: LinkDialect, text: &str) -> Vec<(&str, &str)> {
        scan(dialect, text)
            .into_iter()
            .map(|at| &text[at])
            .map(|written| (written, target(dialect, written)))
            .collect()
    }

    #[test]
    fn a_wikilink_value_names_what_its_brackets_hold() {
        let read = [
            target(LinkDialect::Wikilink, "[[Analytical Engine]]"),
            target(LinkDialect::Wikilink, "[[people/ada]]"),
            target(LinkDialect::Wikilink, "[[]]"),
        ];
        assert_eq!(read, ["Analytical Engine", "people/ada", ""]);
    }

    #[test]
    fn a_markdown_value_names_its_target_rather_than_its_label() {
        let read = [
            target(LinkDialect::Markdown, "[Ada Lovelace](people/ada.md)"),
            target(LinkDialect::Markdown, "[](ada.md)"),
        ];
        assert_eq!(read, ["people/ada.md", "ada.md"]);
    }

    #[test]
    fn an_alias_splits_at_the_first_pipe_and_only_between_a_wikilinks_brackets() {
        let read = [
            target(LinkDialect::Wikilink, "[[Analytical Engine|the Engine]]"),
            target(LinkDialect::Wikilink, "[[Analytical Engine|]]"),
            target(LinkDialect::Wikilink, "[[a|b|c]]"),
            // No delimiters means no inner text: the scalar is the reference
            // part itself, and a `|` in it is bytes rather than an alias.
            target(LinkDialect::Wikilink, "a|b"),
        ];
        assert_eq!(read, ["Analytical Engine", "Analytical Engine", "a", "a|b"]);
    }

    #[test]
    fn a_fragment_splits_at_the_first_hash_of_the_reference_part() {
        let read = [
            reference(LinkDialect::Wikilink, "[[engine#history]]"),
            reference(LinkDialect::Wikilink, "[[engine#a#b]]"),
            reference(LinkDialect::Wikilink, "[[engine#]]"),
            reference(LinkDialect::Wikilink, "[[engine]]"),
            // An undelimited scalar is the reference part, so it splits too.
            reference(LinkDialect::Wikilink, "engine#history"),
        ];
        let parts: Vec<(&str, Option<&str>)> = read
            .iter()
            .map(|parsed| (parsed.target, parsed.fragment))
            .collect();
        assert_eq!(
            parts,
            [
                ("engine", Some("history")),
                ("engine", Some("a#b")),
                ("engine", Some("")),
                ("engine", None),
                ("engine", Some("history")),
            ]
        );
    }

    #[test]
    fn the_alias_comes_off_before_the_fragment_is_looked_for() {
        // The `|` splits the inner text first; the `#` then splits only the
        // reference part, so a `#` in the display text is display text.
        let read = [
            reference(LinkDialect::Wikilink, "[[engine#history|the Engine]]"),
            reference(LinkDialect::Wikilink, "[[engine|see #3]]"),
        ];
        let parts: Vec<(&str, Option<&str>)> = read
            .iter()
            .map(|parsed| (parsed.target, parsed.fragment))
            .collect();
        assert_eq!(parts, [("engine", Some("history")), ("engine", None)]);
    }

    #[test]
    fn a_markdown_target_strips_its_fragment_the_same_way() {
        let read = [
            reference(LinkDialect::Markdown, "[Ada](people/ada.md#early-life)"),
            reference(LinkDialect::Markdown, "people/ada.md#early-life"),
        ];
        let parts: Vec<(&str, Option<&str>)> = read
            .iter()
            .map(|parsed| (parsed.target, parsed.fragment))
            .collect();
        let expected = ("people/ada.md", Some("early-life"));
        assert_eq!(parts, [expected, expected]);
    }

    #[test]
    fn an_empty_target_with_a_nonempty_fragment_names_the_containing_note() {
        let own = [
            reference(LinkDialect::Wikilink, "[[#heading]]"),
            reference(LinkDialect::Markdown, "[above](#heading)"),
        ];
        assert!(own.iter().all(Reference::names_own_note), "{own:?}");
        // A wholly empty reference is not a self-reference — with no fragment
        // or an empty one, there is nothing said to be in the containing note.
        let elsewhere = [
            reference(LinkDialect::Wikilink, "[[]]"),
            reference(LinkDialect::Wikilink, "[[|alias]]"),
            reference(LinkDialect::Wikilink, "[[#]]"),
            reference(LinkDialect::Wikilink, "[[engine#history]]"),
        ];
        assert!(
            !elsewhere.iter().any(Reference::names_own_note),
            "{elsewhere:?}"
        );
    }

    #[test]
    fn a_parsed_reference_clones_compares_and_formats() {
        let parsed = reference(LinkDialect::Wikilink, "[[engine#history|the Engine]]");
        assert_eq!(parsed.clone(), parsed);
        assert_ne!(parsed, reference(LinkDialect::Wikilink, "[[engine]]"));
        assert!(format!("{parsed:?}").contains("history"));
    }

    #[test]
    fn a_value_that_is_not_one_delimited_link_is_the_reference_it_spells() {
        // The reading this module states: no lexical-fault identifier, because
        // the value either names a note or it does not.
        let plain = [
            target(LinkDialect::Wikilink, "Acme Corp"),
            target(LinkDialect::Wikilink, "[[unclosed"),
            target(LinkDialect::Wikilink, "[[closed]] and more"),
            target(LinkDialect::Markdown, "[no target]"),
            target(LinkDialect::Markdown, "people/ada.md"),
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
            target(LinkDialect::Wikilink, "[[a]] and [[b]]"),
            "a]] and [[b"
        );
    }

    #[test]
    fn the_dialect_the_contract_declares_is_the_only_one_read() {
        // A corpus declares one spelling; the other is bytes.
        let crossed = (
            target(LinkDialect::Wikilink, "[Ada](ada.md)"),
            target(LinkDialect::Markdown, "[[Ada]]"),
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
    fn a_scan_answers_where_in_the_prose_each_reference_is_written() {
        // The range is what a span is measured from, so it holds the whole
        // delimited run rather than the reference inside it.
        let prose = "See [[Ada]] and [[Babbage]].\n";
        assert_eq!(scan(LinkDialect::Wikilink, prose), [4..11, 16..27]);
        assert_eq!(&prose[4..11], "[[Ada]]");
    }

    #[test]
    fn a_reference_holding_the_other_dialects_delimiters_is_still_one_reference() {
        assert_eq!(
            found(LinkDialect::Wikilink, "[[a (b)]] then [[c]]\n"),
            [("[[a (b)]]", "a (b)"), ("[[c]]", "c")]
        );
    }
}
