//! The bytes a capture writes.
//!
//! A capture creates one note: frontmatter that **omits `type` entirely** —
//! absence is what binds a note to the catch-all, so stamping the name would
//! hand-write what the model derives and would orphan every old capture the day
//! the catch-all is renamed — plus the flags the contract says a note of that
//! type is born carrying, and the captured text as the body, byte for byte.
//! Nothing else.
//!
//! **Byte for byte is meant literally.** The captured text is not trimmed, not
//! re-wrapped, not newline-normalized, and not given a trailing newline it did
//! not arrive with. A writing surface preserves every byte it did not
//! semantically touch, and none of the body is a byte this surface touched. A
//! capture whose text carries carriage returns or a byte order mark therefore
//! produces a note that earns the warning any other note with those bytes earns
//! — reported by the post-write read, never repaired behind the author's back.
//!
//! The one thing this module writes that the author did not is a frontmatter
//! block, and it writes one in exactly two cases: when there is a birth flag to
//! stamp, and when the text would otherwise be misread as its own frontmatter.
//! See [`document`].
//!
//! Whether it *would* be misread is asked of the reader's own grammar —
//! [`crate::note::opens_a_block`] — and never re-derived here. A writer with its
//! own copy of what a fence is would be a second definition, and the narrow copy
//! this module first carried missed three shapes the reader accepts: a fence
//! followed by a space, one followed by a tab, and one terminated by a lone
//! carriage return.

use crate::contract::{Contract, TypeDecl};
use crate::note::opens_a_block;

/// The fence a frontmatter block opens and closes with.
const FENCE: &str = "---";

/// The bytes of the note a capture creates.
///
/// The empty block is not decoration. Frontmatter is recognized only at line
/// zero, so a capture whose first line is `---` would otherwise have that line
/// read as an opening fence and part of its own body read as declarations —
/// silently, and differently depending on what the second line happened to say.
/// An empty block in front of it is a true statement about the note (it
/// declares nothing) and it makes the body unambiguous, which is the whole of
/// what the round-trip contract asks of a surface that must write *something*.
pub(super) fn document(born_flagged: &[String], text: &str) -> String {
    if born_flagged.is_empty() && !opens_a_block(text) {
        return text.to_owned();
    }
    let mut rendered = String::from(FENCE);
    rendered.push('\n');
    for flag in born_flagged {
        rendered.push_str(flag);
        rendered.push_str(": true\n");
    }
    rendered.push_str(FENCE);
    rendered.push('\n');
    rendered.push_str(text);
    rendered
}

/// The flags a note bound to `declared` is born carrying.
///
/// Read from the contract's declaration and never from a vocabulary this SDK
/// knows: the corpus says which properties are flags and which types are born
/// carrying them, and a version whose format has no seat to say it in says
/// nothing, which is *stamp nothing*.
pub(super) fn birth_flags(contract: &Contract) -> &[String] {
    contract.catch_all().map_or(&[], TypeDecl::born_flagged)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::contract::parse_contract;

    /// A contract at the version that defines the seats, with `body` spliced
    /// into its catch-all.
    fn contract(source: &str) -> Contract {
        parse_contract(source)
            .contract
            .expect("a contract this test expects to resolve")
    }

    /// A version-3 contract whose catch-all declares no birth state.
    const PLAIN: &str = concat!(
        "contract_version = 3\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// The same, whose catch-all is born carrying the triage flag it declares.
    const BORN: &str = concat!(
        "contract_version = 3\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[flag]]\nproperty = \"needs_triage\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
        "born-flagged = [\"needs_triage\"]\n",
        "\n  [[type.property]]\n  name = \"needs_triage\"\n  kind = \"boolean\"\n",
    );

    /// The same corpus at the version below the seats.
    const OLDER: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    #[test]
    fn a_contract_declaring_no_birth_state_stamps_nothing() {
        assert!(birth_flags(&contract(PLAIN)).is_empty());
    }

    #[test]
    fn a_contract_declaring_one_stamps_exactly_it() {
        assert_eq!(birth_flags(&contract(BORN)), ["needs_triage"]);
    }

    /// A version with no seat, and a contract with no catch-all at all: two
    /// different absences, one answer.
    #[test]
    fn a_version_without_the_seat_stamps_nothing_either() {
        assert!(birth_flags(&contract(OLDER)).is_empty());
    }

    /// Nothing to stamp and nothing to disambiguate: the note *is* the text,
    /// with no block in front of it and no newline appended to it.
    #[test]
    fn an_unflagged_capture_is_the_captured_bytes_and_nothing_else() {
        assert_eq!(document(&[], "a loose thought"), "a loose thought");
        assert_eq!(document(&[], "trailing\n"), "trailing\n");
        assert_eq!(document(&[], ""), "");
    }

    #[test]
    fn a_flagged_capture_carries_the_flag_and_then_the_bytes() {
        let flags = vec!["needs_triage".to_owned()];
        assert_eq!(
            document(&flags, "a loose thought"),
            "---\nneeds_triage: true\n---\na loose thought"
        );
    }

    #[test]
    fn every_declared_birth_flag_is_stamped_in_declaration_order() {
        let flags = vec!["needs_triage".to_owned(), "leaned_on".to_owned()];
        assert_eq!(
            document(&flags, "text"),
            "---\nneeds_triage: true\nleaned_on: true\n---\ntext"
        );
    }

    /// Text that opens with a fence gets an empty block in front of it, so the
    /// body it round-trips to is the body that was captured.
    #[test]
    fn text_that_would_be_read_as_frontmatter_gets_a_block_of_its_own() {
        assert_eq!(
            document(&[], "---\nnot: mine\n---\n"),
            "---\n---\n---\nnot: mine\n---\n"
        );
    }

    /// Every shape the reader accepts as a fence, including the three a
    /// narrower rule misses: trailing whitespace is trimmed before the
    /// comparison, and a lone carriage return is a line terminator.
    ///
    /// Asked of the reader's grammar rather than restated, so this test is
    /// about the *writer* protecting each shape rather than about the
    /// grammar's own definition, which `frontmatter` tests where it lives.
    #[test]
    fn every_shape_the_reader_calls_a_fence_is_protected() {
        for text in [
            "---",
            "---\nmore",
            "---\r\nmore",
            "\u{feff}---",
            "--- ",
            "---\t",
            "--- \nmore",
            "---\rmore",
        ] {
            assert!(opens_a_block(text), "the reader calls `{text:?}` a fence");
            let rendered = document(&[], text);
            assert!(
                rendered.starts_with("---\n---\n"),
                "`{text:?}` must be protected: {rendered:?}"
            );
            assert!(rendered.ends_with(text), "the body is kept: {rendered:?}");
        }
    }

    /// Only line zero, only at indent zero, and only the fence: every near miss
    /// is body, and is written without a block it does not need.
    #[test]
    fn text_that_merely_resembles_a_fence_is_left_alone() {
        for text in ["----", " ---", "--", "text\n---\n", "---text", "", "\t---"] {
            assert!(!opens_a_block(text), "`{text:?}`");
            assert_eq!(document(&[], text), text);
        }
    }
}
