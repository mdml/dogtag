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

/// `value` as exactly one line, with every control character folded to a space.
///
/// Both `\n` and `\r` fold. A lone carriage return is not a line to
/// [`str::lines`], but it is one to a terminal, which reads it as a move to
/// column zero and lets whatever follows overwrite the line a reader had
/// already seen.
///
/// Every other control character folds for the same reason, and the reason
/// generalizes further than the two line breaks do: `ESC[2K` erases the line
/// a reader had already seen, and `ESC[1A` moves the cursor up to overwrite
/// one. Every string value in a contract is free text, and a contract planted
/// in an ancestor is attacker-authored text rendered to the reader's terminal,
/// so the two lines the trust warnings depend on are exactly what a repainting
/// sequence would target.
pub(crate) fn one_line(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
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
    fn every_control_character_folds_for_the_same_reason_a_carriage_return_does() {
        // ESC[2K erases the line a reader had already seen and ESC[1A moves
        // the cursor up to overwrite one, which is what a carriage return
        // does and more. A contract planted in an ancestor is attacker-
        // authored text on its way to the reader's terminal.
        assert_eq!(one_line("a\u{1b}[2Kb"), "a [2Kb");
        assert_eq!(one_line("up\u{1b}[1A"), "up [1A");
        assert_eq!(one_line("bell\u{7}tab\ttext"), "bell tab text");
        // C1, which a terminal reads as a control sequence introducer too.
        assert_eq!(one_line("csi\u{9b}2K"), "csi 2K");
        // Nothing else moves: printable punctuation and non-ASCII are text.
        assert_eq!(one_line("naïve — a•b"), "naïve — a•b");
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
