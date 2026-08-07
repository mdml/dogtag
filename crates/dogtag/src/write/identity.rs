//! What a capture is called, and where it lands.
//!
//! Identity is the path, as everywhere, so this module is what decides a
//! capture's identity. It derives a file name from the instant the capture was
//! made and a slug of its first line, and it resolves a collision by appending
//! a suffix rather than by refusing: a name coincidence is not a reason to lose
//! a thought.
//!
//! The calendar arithmetic is done here rather than by a dependency. It is
//! `UTC`, it is thirty lines, and the alternative is a crate in the shipped
//! closure of an SDK whose whole dependency list is three parsers.

use super::actor::CapturedAt;

/// The extension every note carries, which is what makes it a note.
const EXTENSION: &str = ".md";

/// How many characters of the first line reach the name.
///
/// Long enough that a name is recognizable in a listing, short enough that it
/// stays a name rather than a copy of the note. Measured in characters rather
/// than bytes so the bound does not depend on the alphabet a thought is
/// written in.
const SLUG_LIMIT: usize = 48;

/// Seconds in a day, which is where the civil arithmetic splits.
const DAY: u64 = 86_400;

/// The file name a capture takes, without its directory.
///
/// `<date>-<time>-<slug>.md`, or `<date>-<time>.md` where the first line slugs
/// to nothing at all — which a capture of punctuation, or of a line in a script
/// this slug rule does not transliterate, legitimately does. The time is part
/// of the name rather than only the date because two captures a day apart and
/// two a second apart are the same case.
pub(super) fn file_name(at: CapturedAt, text: &str) -> String {
    let stamp = stamp(at);
    let slug = slug(text);
    if slug.is_empty() {
        return format!("{stamp}{EXTENSION}");
    }
    format!("{stamp}-{slug}{EXTENSION}")
}

/// The same name with a collision suffix, counting from the second bearer.
///
/// `<stem>-2.md`, `<stem>-3.md`, and so on. The first bearer wears the bare
/// name, so a vault that never collides never sees a suffix.
pub(super) fn nth(name: &str, nth: usize) -> String {
    let stem = name.strip_suffix(EXTENSION).unwrap_or(name);
    format!("{stem}-{nth}{EXTENSION}")
}

/// `<YYYY-MM-DD>-<HHMMSS>` in UTC.
///
/// UTC rather than the machine's zone, because the name is identity: the same
/// capture must be called the same thing wherever it is read, and a zone that
/// travels with the machine would make a vault synced between two of them
/// disagree with itself.
fn stamp(at: CapturedAt) -> String {
    let seconds = at.unix_seconds();
    let (year, month, day) = civil(seconds / DAY);
    let clock = seconds % DAY;
    let (hour, minute, second) = (clock / 3600, (clock / 60) % 60, clock % 60);
    format!("{year:04}-{month:02}-{day:02}-{hour:02}{minute:02}{second:02}")
}

/// The civil date `days` after 1970-01-01, by the proleptic Gregorian calendar.
///
/// Howard Hinnant's `civil_from_days`, which is exact over the whole range a
/// `u64` of seconds can name and needs no table. The shifted era begins on
/// 0000-03-01 so that a leap day falls at the end of a year rather than in the
/// middle of one, which is what removes every special case but the two lines
/// that shift March back to January.
fn civil(days: u64) -> (u64, u64, u64) {
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// A file-name slug of the captured text's first line.
///
/// Lowercase ASCII letters and digits survive; every other run of characters
/// becomes one `-`; leading and trailing separators go. What that guarantees is
/// what the name has to guarantee and no more: the result never begins with a
/// `.` (so the traversal cannot skip the directory it lands in for looking
/// hidden, and no note is hidden by its own name), never holds a `/` (so it
/// cannot address a directory), and never ends in `.md` before the extension is
/// appended (so a note's bare name is not itself path-shaped).
///
/// Deliberately not transliteration. A thought written in a script this rule
/// drops keeps every byte of itself in the body — the name is a convenience,
/// and the body is the capture.
fn slug(text: &str) -> String {
    let first = text.lines().next().unwrap_or_default();
    let mut slug = String::new();
    let mut written = 0;
    for character in first.chars() {
        if written == SLUG_LIMIT {
            break;
        }
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if slug.is_empty() || slug.ends_with('-') {
            continue;
        } else {
            slug.push('-');
        }
        written += 1;
    }
    slug.trim_end_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: u64) -> CapturedAt {
        CapturedAt::from_unix_seconds(seconds)
    }

    #[test]
    fn the_epoch_is_the_first_day_of_1970() {
        assert_eq!(stamp(at(0)), "1970-01-01-000000");
    }

    /// Each boundary the arithmetic could get wrong, named: the last second of
    /// a day, a leap day, the day after one, the end of a century that is not a
    /// leap year, and the end of one that is.
    #[test]
    fn the_calendar_is_right_at_every_boundary_it_could_be_wrong_at() {
        let cases: &[(u64, &str)] = &[
            (86_399, "1970-01-01-235959"),
            (86_400, "1970-01-02-000000"),
            (951_782_400, "2000-02-29-000000"),
            (951_868_800, "2000-03-01-000000"),
            (4_107_456_000, "2100-02-28-000000"),
            (4_107_542_400, "2100-03-01-000000"),
            (1_786_000_000, "2026-08-06-070640"),
        ];
        for (seconds, expected) in cases {
            assert_eq!(stamp(at(*seconds)), *expected, "{seconds}");
        }
    }

    #[test]
    fn a_first_line_becomes_a_lowercase_hyphenated_slug() {
        assert_eq!(slug("A Loose Thought"), "a-loose-thought");
        assert_eq!(slug("Order 66, revisited"), "order-66-revisited");
        assert_eq!(slug("  leading space"), "leading-space");
    }

    /// Only the first line, and only up to the bound: a name is a name.
    #[test]
    fn a_slug_stops_at_the_first_line_and_at_the_bound() {
        assert_eq!(slug("first line\nsecond line"), "first-line");
        let long = "a".repeat(SLUG_LIMIT + 10);
        assert_eq!(slug(&long).chars().count(), SLUG_LIMIT);
    }

    /// The four things the name must never be, each from the input that would
    /// have produced it: a hidden file, a path, a doubled extension, and a bare
    /// separator.
    #[test]
    fn a_slug_is_never_hidden_never_a_path_and_never_an_extension() {
        assert_eq!(slug(".hidden"), "hidden");
        assert_eq!(slug("../escape"), "escape");
        assert_eq!(slug("a/b"), "a-b");
        assert_eq!(slug("notes.md"), "notes-md");
        assert_eq!(slug("---"), "");
        assert_eq!(slug("  "), "");
    }

    /// A thought in a script this rule drops keeps its instant for a name and
    /// loses nothing else — the body is the capture.
    #[test]
    fn a_first_line_that_slugs_to_nothing_leaves_the_instant_as_the_whole_name() {
        assert_eq!(slug("……"), "");
        assert_eq!(file_name(at(0), "……"), "1970-01-01-000000.md");
        assert_eq!(file_name(at(0), ""), "1970-01-01-000000.md");
    }

    #[test]
    fn a_name_is_the_instant_then_the_slug() {
        assert_eq!(
            file_name(at(1_786_000_000), "A loose thought"),
            "2026-08-06-070640-a-loose-thought.md"
        );
    }

    /// The first bearer wears the bare name; a second and a third count from
    /// two, so a vault that never collides never sees a suffix.
    #[test]
    fn a_collision_appends_a_counting_suffix_before_the_extension() {
        let name = file_name(at(0), "twice");
        assert_eq!(name, "1970-01-01-000000-twice.md");
        assert_eq!(nth(&name, 2), "1970-01-01-000000-twice-2.md");
        assert_eq!(nth(&name, 3), "1970-01-01-000000-twice-3.md");
    }

    /// A name with no extension to strip still takes its suffix in one piece,
    /// which is the arm the `unwrap_or` covers.
    #[test]
    fn a_name_without_the_extension_still_takes_a_suffix() {
        assert_eq!(nth("bare", 2), "bare-2.md");
    }
}
