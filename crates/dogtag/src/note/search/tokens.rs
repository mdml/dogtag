//! Words, for matching: lowercase alphanumeric runs.
//!
//! Retrieval matches **words**, not bytes: a query term meets a text through
//! the words each of them is made of, so `Engine,` in prose and `engine` in a
//! query are one word. A word is a maximal run of alphanumeric characters,
//! lowercased — punctuation separates and never matches, and case never
//! decides, which is the resolution behavior the incumbent's daily use
//! established and the cutover must not lose.
//!
//! Every token remembers where its bytes are, because a snippet quotes the
//! note's own text around the first match and only a byte range can find it.

use core::ops::Range;

/// One word of a text, and where its bytes are.
pub(super) struct Token {
    /// The word's bytes in the scanned text.
    pub(super) at: Range<usize>,
    /// The word, lowercased.
    pub(super) word: String,
}

/// Every word of `text`, in order.
pub(super) fn scan(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut start = None;
    for (index, character) in text.char_indices() {
        if character.is_alphanumeric() {
            start.get_or_insert(index);
        } else if let Some(from) = start.take() {
            tokens.push(token(text, from..index));
        }
    }
    if let Some(from) = start {
        tokens.push(token(text, from..text.len()));
    }
    tokens
}

/// The words of `text` alone, for a side of the match that needs no spans.
pub(super) fn words(text: &str) -> Vec<String> {
    scan(text).into_iter().map(|token| token.word).collect()
}

fn token(text: &str, at: Range<usize>) -> Token {
    Token {
        word: text[at.clone()].to_lowercase(),
        at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scanned(text: &str) -> Vec<(&str, Range<usize>)> {
        scan(text)
            .into_iter()
            .map(|token| (&text[token.at.clone()], token.at))
            .collect()
    }

    #[test]
    fn words_are_alphanumeric_runs_and_punctuation_separates() {
        assert_eq!(
            words("The Engine, restarted."),
            ["the", "engine", "restarted"]
        );
        assert_eq!(words("don't"), ["don", "t"]);
        assert_eq!(
            words("engines/analytical.md"),
            ["engines", "analytical", "md"]
        );
    }

    #[test]
    fn a_token_remembers_the_bytes_it_was_read_from() {
        assert_eq!(
            scanned("a naïve η-run"),
            [("a", 0..1), ("naïve", 2..8), ("η", 9..11), ("run", 12..15)]
        );
    }

    #[test]
    fn a_text_with_no_word_scans_to_nothing() {
        assert!(words("").is_empty());
        assert!(words(" …, — !").is_empty());
    }

    #[test]
    fn a_trailing_word_is_read_to_the_end_of_the_text() {
        assert_eq!(scanned("to 2026")[1], ("2026", 3..7));
    }
}
