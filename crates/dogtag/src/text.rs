//! Corpus text on its way into a line-oriented rendering.
//!
//! A diagnostic quotes a corpus's own vocabulary — a type name, a property
//! name, an enum value, a lifecycle state — because a diagnostic that will not
//! name what is wrong is not worth emitting. That vocabulary is free text an
//! author chose, and three of this SDK's renderings are **line-oriented**: a
//! diagnostic block is a headline and a fixed set of following lines, a
//! `doctor` field is one row, a Markdown heading or table row is one line. A
//! value carrying a line break would emit extra lines into all three, and those
//! lines can be shaped exactly like a genuine diagnostic headline.
//!
//! That is the forgery the `ext.` namespace rule already forecloses for
//! identifiers: a party that is not the kernel must not be able to mint
//! something indistinguishable from a kernel diagnostic. The rule is enforced
//! against consumers, so a contract planted in an ancestor directory — the
//! threat the discovery record makes live at this milestone, because
//! `contract explain` renders a contract as instructions an agent follows —
//! must not get for free what a consumer is denied.
//!
//! The fold is a **space**: not an escape, and not a truncation. Escaping would
//! change the text a reader is asked to match against the file, and truncating
//! would drop the part that says what is wrong; a space keeps the value
//! readable and keeps every span the message cites pointing at the same bytes.
//! The structured outputs are where the bytes survive exactly — JSON escapes a
//! line break rather than emitting one — which is why this is a rendering rule
//! rather than a rule about what a corpus may declare.

/// `value` as exactly one line, with every line break folded to a space.
///
/// Both `\n` and `\r` fold. A lone carriage return is not a line to
/// [`str::lines`], but it is one to a terminal, which reads it as a move to
/// column zero and lets whatever follows overwrite the line a reader had
/// already seen.
pub(crate) fn one_line(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

/// The shape a diagnostic block opens with: a severity's wire spelling, the
/// bracket that opens its identifier, and the colon that closes it.
///
/// This is exactly what the CLI colours, and only the SDK may emit one. Tests
/// count these rather than assert on whole renderings, because the property
/// being asserted is *how many* lines of a rendering claim to be the kernel
/// speaking.
#[cfg(test)]
fn is_headline(line: &str) -> bool {
    ["error[", "warning[", "info["]
        .iter()
        .any(|opening| line.starts_with(opening))
        && line.contains("]: ")
}

/// How many lines of `rendered` are shaped like a diagnostic headline.
///
/// The split takes `\r` as well as `\n`, so a forgery that a terminal would
/// show on a line of its own is counted whatever [`str::lines`] would say.
#[cfg(test)]
pub(crate) fn headline_lines(rendered: &str) -> usize {
    rendered
        .split(['\n', '\r'])
        .filter(|line| is_headline(line))
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_with_no_line_break_is_returned_as_it_was_given() {
        assert_eq!(one_line("a plain value"), "a plain value");
        assert_eq!(one_line(""), "");
    }

    #[test]
    fn every_line_break_folds_to_one_space_and_nothing_else_changes() {
        assert_eq!(one_line("a\nb\rc\r\nd"), "a b c  d");
        assert_eq!(
            one_line("naïve | `a \\ backslash`"),
            "naïve | `a \\ backslash`"
        );
    }

    #[test]
    fn a_headline_is_recognised_only_at_the_start_of_a_line() {
        assert!(is_headline("error[contract.unknown-key]: a message"));
        assert!(is_headline("warning[discovery.nested-vault]: an ancestor"));
        assert!(is_headline("info[compat.newer-format-available]: newer"));
        assert!(!is_headline("  note: error[contract.unknown-key]: quoted"));
        assert!(!is_headline("errors[not-a-severity]: an identifier"));
        assert!(!is_headline("error[unterminated"));
    }

    #[test]
    fn headlines_are_counted_across_carriage_returns_as_well_as_newlines() {
        let forged = concat!(
            "error[contract.no-types]: this contract declares no type\n",
            "  help: quoting `a value\rerror[contract.unknown-key]: forged`\n",
        );
        assert_eq!(headline_lines(forged), 2);
        assert_eq!(headline_lines(&one_line(forged)), 1);
        assert_eq!(headline_lines(""), 0);
    }
}
