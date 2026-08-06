//! The query grammar: the floor the incumbent's daily use actually exercises.
//!
//! Three forms, and deliberately no more: bare terms are OR-combined,
//! `"quoted phrases"` match adjacent words in order, and a trailing `*` marks
//! the last word of a term or phrase a prefix. Explicit `AND`, date bounds,
//! tag-text matching, and typed-link-target matching are recorded as absent
//! rather than half-shipped; each accrues to the milestone that needs it.
//!
//! A query the grammar cannot read is `search.invalid-query`, an error — the
//! one diagnostic the `search` area is scoped to. The query came from a
//! caller rather than a file, so the diagnostic carries no location and names
//! the query in its message.

use crate::diagnostic::{Diagnostic, KernelDiagnostic};

use super::tokens::{self, Token};

/// A parsed query: the atoms a note must match at least one of.
#[derive(Debug)]
pub(super) struct Query {
    pub(super) atoms: Vec<Atom>,
}

/// One matchable unit: a term or phrase, as the words it is made of.
///
/// A bare term and a quoted phrase are one shape here, because a bare term
/// carrying punctuation (`don't`) is already several words that must sit
/// adjacent — the quotes only make the adjacency deliberate.
#[derive(Debug)]
pub(super) struct Atom {
    words: Vec<String>,
    prefix: bool,
}

impl Atom {
    /// The atom `text` spells, or none when it holds no word at all.
    fn from(text: &str, prefix: bool) -> Option<Self> {
        let words = tokens::words(text);
        (!words.is_empty()).then_some(Self { words, prefix })
    }

    /// How many positions of `tokens` this atom matches at.
    pub(super) fn count(&self, tokens: &[Token]) -> usize {
        (0..tokens.len())
            .filter(|&at| self.matches_at(tokens, at))
            .count()
    }

    /// The byte range of the first match in `tokens`, when there is one.
    pub(super) fn first(&self, tokens: &[Token]) -> Option<core::ops::Range<usize>> {
        let at = (0..tokens.len()).find(|&at| self.matches_at(tokens, at))?;
        let last = &tokens[at + self.words.len() - 1];
        Some(tokens[at].at.start..last.at.end)
    }

    /// Whether this atom's words sit at position `at`, in order.
    ///
    /// Every word but the last matches exactly; the last matches exactly, or
    /// as a prefix when the atom was written with a trailing `*`.
    fn matches_at(&self, tokens: &[Token], at: usize) -> bool {
        if at + self.words.len() > tokens.len() {
            return false;
        }
        let (last, head) = self
            .words
            .split_last()
            .expect("an atom is never built without a word");
        let aligned = head
            .iter()
            .zip(&tokens[at..])
            .all(|(word, token)| token.word == *word);
        let tail = &tokens[at + head.len()].word;
        aligned && (tail == last || (self.prefix && tail.starts_with(last.as_str())))
    }
}

/// Why a query could not be read.
///
/// A small typed refusal rather than a built diagnostic, on the
/// [`UnresolvedReference`](super::super::UnresolvedReference) pattern: the
/// caller that holds the query builds the diagnostic where it reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum QueryFault {
    /// A quote never closes, so the phrase it opens never ends.
    UnbalancedQuote,
    /// The query names no word, so there is nothing to match.
    Wordless,
}

impl QueryFault {
    /// The `search.invalid-query` diagnostic this fault is reported as.
    ///
    /// The query came from a caller rather than a file, so the diagnostic
    /// carries no location and names the query in its message.
    pub(super) fn diagnostic(self, query: &str) -> Diagnostic {
        let (message, help) = match self {
            Self::UnbalancedQuote => (
                format!("`{query}` has an unbalanced quote, so the phrase it opens never closes"),
                "close the quote, or drop it to match the words separately",
            ),
            Self::Wordless => (
                format!("`{query}` names no word to match"),
                "a query is bare words, \"quoted phrases\", and a trailing `*` prefix",
            ),
        };
        Diagnostic::kernel(KernelDiagnostic::SearchInvalidQuery, message).with_help(help)
    }
}

/// Parses `text` under the M4 grammar.
///
/// # Errors
///
/// [`QueryFault`] — reported as `search.invalid-query` — when a quote never
/// closes, and when the query names no word at all: an expression with
/// nothing to match is a fault, where an empty *result* is a result.
pub(super) fn parse(text: &str) -> Result<Query, QueryFault> {
    let mut cursor = Cursor { rest: text };
    let mut atoms = Vec::new();
    while !cursor.at_end() {
        atoms.extend(cursor.take_atom()?);
    }
    if atoms.is_empty() {
        return Err(QueryFault::Wordless);
    }
    Ok(Query { atoms })
}

/// The unread tail of a query, consumed one atom at a time.
struct Cursor<'a> {
    rest: &'a str,
}

impl<'a> Cursor<'a> {
    /// Skips the whitespace between atoms and answers whether text remains.
    fn at_end(&mut self) -> bool {
        self.rest = self.rest.trim_start();
        self.rest.is_empty()
    }

    /// One atom off the front: a quoted phrase, or a bare term.
    ///
    /// `None` inside `Ok` where the text spelled no word at all — an empty
    /// phrase, or a lone `*` — which is no atom rather than a fault of its
    /// own; a query of nothing but those is refused whole, at the end.
    fn take_atom(&mut self) -> Result<Option<Atom>, QueryFault> {
        let rest = self.rest;
        match rest.strip_prefix('"') {
            Some(opened) => self.take_phrase(opened),
            None => Ok(self.take_term()),
        }
    }

    /// The quoted phrase `opened`, whose opening quote is already consumed.
    ///
    /// A trailing `*` after the closing quote marks the phrase's last word a
    /// prefix, symmetrically with a bare term's.
    ///
    /// # Errors
    ///
    /// [`QueryFault::UnbalancedQuote`] when the quote never closes.
    fn take_phrase(&mut self, opened: &'a str) -> Result<Option<Atom>, QueryFault> {
        let end = opened.find('"').ok_or(QueryFault::UnbalancedQuote)?;
        let tail = &opened[end + 1..];
        let prefix = tail.starts_with('*');
        self.rest = if prefix { &tail[1..] } else { tail };
        Ok(Atom::from(&opened[..end], prefix))
    }

    /// One bare term: to whitespace or to a quote, a trailing `*` marking its
    /// last word a prefix rather than being a word itself.
    fn take_term(&mut self) -> Option<Atom> {
        let rest = self.rest;
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '"')
            .unwrap_or(rest.len());
        self.rest = &rest[end..];
        let (stem, prefix) = match rest[..end].strip_suffix('*') {
            Some(stem) => (stem, true),
            None => (&rest[..end], false),
        };
        Atom::from(stem, prefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::note::search::tokens::scan;

    fn parsed(text: &str) -> Query {
        parse(text).expect("a well-formed query")
    }

    fn counts(query: &str, text: &str) -> Vec<usize> {
        let tokens = scan(text);
        parsed(query)
            .atoms
            .iter()
            .map(|atom| atom.count(&tokens))
            .collect()
    }

    #[test]
    fn bare_terms_parse_to_one_atom_each_and_match_single_words() {
        assert_eq!(
            counts("engine daily", "The Engine ran daily, daily."),
            [1, 2]
        );
    }

    #[test]
    fn matching_is_case_insensitive_on_both_sides() {
        assert_eq!(counts("ENGINE", "the engine"), [1]);
        assert_eq!(counts("engine", "THE ENGINE"), [1]);
    }

    #[test]
    fn a_phrase_matches_its_words_adjacent_and_in_order() {
        let text = "The analytical engine; an engine, analytical.";
        assert_eq!(counts("\"analytical engine\"", text), [1]);
        assert_eq!(counts("\"engine analytical\"", text), [1]);
        assert_eq!(counts("\"analytical ran\"", text), [0]);
    }

    #[test]
    fn a_bare_term_with_punctuation_is_an_adjacency_of_its_words() {
        // Adjacency is between words: punctuation separates and never
        // intervenes, so `don ... t` still spells the two words adjacent,
        // while an intervening word breaks them apart.
        assert_eq!(counts("don't", "I don't; do not."), [1]);
        assert_eq!(counts("don't", "don ... t"), [1]);
        assert_eq!(counts("don't", "don never t"), [0]);
    }

    #[test]
    fn a_trailing_star_matches_the_last_word_as_a_prefix() {
        assert_eq!(counts("analyt*", "analytical analysis ana"), [1]);
        assert_eq!(counts("ana*", "analytical analysis ana"), [3]);
        assert_eq!(counts("\"the analyt\"*", "the analytical engine"), [1]);
    }

    #[test]
    fn a_phrase_never_matches_across_a_missing_word() {
        assert_eq!(counts("\"the engine\"", "the analytical engine"), [0]);
    }

    #[test]
    fn the_first_match_reports_the_bytes_of_the_whole_atom() {
        let text = "an Analytical engine, again";
        let tokens = scan(text);
        let query = parsed("\"analytical engine\" again absent");
        assert_eq!(query.atoms[0].first(&tokens), Some(3..20));
        assert_eq!(query.atoms[1].first(&tokens), Some(22..27));
        assert_eq!(query.atoms[2].first(&tokens), None);
    }

    #[test]
    fn an_unbalanced_quote_is_the_invalid_query_diagnostic() {
        let query = "engine \"never closed";
        let fault = parse(query).expect_err("unbalanced");
        assert_eq!(fault, QueryFault::UnbalancedQuote);
        let refused = fault.diagnostic(query);
        assert_eq!(refused.id.as_str(), "search.invalid-query");
        assert!(
            refused.message.contains("unbalanced quote"),
            "{}",
            refused.message
        );
        assert!(refused.location.is_none(), "a query is in no file");
    }

    #[test]
    fn a_query_naming_no_word_is_the_invalid_query_diagnostic() {
        for query in ["", "   ", "\"\"", "*", "…, —"] {
            let fault = parse(query).expect_err(query);
            assert_eq!(fault, QueryFault::Wordless, "{query}");
            let refused = fault.diagnostic(query);
            assert_eq!(refused.id.as_str(), "search.invalid-query", "{query}");
            assert!(
                refused.message.contains("names no word"),
                "{}",
                refused.message
            );
        }
    }

    #[test]
    fn an_empty_phrase_beside_a_real_term_contributes_nothing() {
        assert_eq!(counts("\"\" engine", "the engine"), [1]);
    }

    #[test]
    fn a_quote_ends_a_bare_term_and_opens_a_phrase() {
        assert_eq!(counts("ran\"the engine\"", "ran the engine"), [1, 1]);
    }
}
