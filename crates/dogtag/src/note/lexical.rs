//! What a value has to look like to satisfy the kind its declaration names.
//!
//! **A kind's lexical form is the whole of its meaning**, so it is checked
//! against the bytes the note wrote and nothing is coerced on the way. `1`
//! satisfies `integer` and not `float`; `1.0` the reverse; `NO` is a string,
//! because the declared kind — never the parser's guess — decides what a value
//! means. A `string` accepts any scalar's bytes as they were written, which is
//! why it is the one kind with nothing to check.
//!
//! Three forms are the vault-contract record's rather than this module's, and
//! are transcribed rather than invented: `date` is an RFC 3339 `full-date`,
//! `datetime` is an RFC 3339 `date-time` **with a mandatory offset**, and a
//! `boolean` is `true` or `false`.
//!
//! Two forms the packet leaves to the implementation, stated here because they
//! are what a corpus is held to. An `integer` is an optional sign and one or
//! more ASCII digits — no separators, no radix prefix, because a radix is a
//! spelling and the kind is a value. A `float` is the same with a fractional
//! part, an exponent, or both; a bare `1` is deliberately not one, which is the
//! whole point of the two kinds being distinct on the wire.

use crate::contract::ScalarKind;

/// The length of each month, in a year that is not a leap year.
const MONTH_LENGTHS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Whether `text` satisfies `kind`'s lexical form.
pub(crate) fn scalar(kind: ScalarKind, text: &str) -> bool {
    let written = Form(text);
    match kind {
        ScalarKind::String => true,
        ScalarKind::Integer => written.integer(),
        ScalarKind::Float => written.float(),
        ScalarKind::Boolean => written.boolean(),
        ScalarKind::Date => written.date(),
        ScalarKind::DateTime => written.datetime(),
    }
}

/// Whether `text` is one of an `enum`'s declared members.
pub(crate) fn member(values: &[String], text: &str) -> bool {
    values.iter().any(|value| value == text)
}

/// Bytes a value was written as, asked whether they take a form.
#[derive(Clone, Copy)]
struct Form<'a>(&'a str);

impl Form<'_> {
    /// An optional sign and one or more digits.
    fn integer(self) -> bool {
        self.unsigned().digits()
    }

    /// The same, with a fractional part, an exponent, or both.
    fn float(self) -> bool {
        let body = self.unsigned();
        match body.0.split_once(['e', 'E']) {
            Some((mantissa, exponent)) => {
                let mantissa = Form(mantissa);
                (mantissa.fractional() || mantissa.digits()) && Form(exponent).integer()
            }
            None => body.fractional(),
        }
    }

    fn boolean(self) -> bool {
        self.0 == "true" || self.0 == "false"
    }

    /// An RFC 3339 `full-date`: `YYYY-MM-DD`, and a day the month really has.
    fn date(self) -> bool {
        let Some((year, rest)) = self.0.split_once('-') else {
            return false;
        };
        let Some((month, day)) = rest.split_once('-') else {
            return false;
        };
        let (year, month, day) = (Form(year), Form(month), Form(day));
        year.fixed(4)
            && month.fixed(2)
            && day.fixed(2)
            && calendar(year.number(), month.number(), day.number())
    }

    /// An RFC 3339 `date-time`: a full date, a separator, and a zoned time.
    fn datetime(self) -> bool {
        let Some((day, time)) = self.0.split_once(['T', 't']) else {
            return false;
        };
        Form(day).date() && Form(time).zoned()
    }

    /// A `full-time`: a clock, and an offset that is never optional.
    fn zoned(self) -> bool {
        if let Some(clock) = self.0.strip_suffix(['Z', 'z']) {
            return Form(clock).clock();
        }
        let Some(index) = self.0.rfind(['+', '-']) else {
            return false;
        };
        Form(&self.0[..index]).clock() && Form(&self.0[index + 1..]).offset()
    }

    fn offset(self) -> bool {
        let Some((hour, minute)) = self.0.split_once(':') else {
            return false;
        };
        Form(hour).within(2, 23) && Form(minute).within(2, 59)
    }

    /// `HH:MM:SS`, with an optional fractional part on the seconds.
    fn clock(self) -> bool {
        match self.0.split_once('.') {
            Some((clock, fraction)) => Form(fraction).digits() && Form(clock).hms(),
            None => self.hms(),
        }
    }

    fn hms(self) -> bool {
        let Some((hour, rest)) = self.0.split_once(':') else {
            return false;
        };
        let Some((minute, second)) = rest.split_once(':') else {
            return false;
        };
        // RFC 3339 admits `60` for the seconds, which is what a leap second is.
        Form(hour).within(2, 23) && Form(minute).within(2, 59) && Form(second).within(2, 60)
    }

    /// The same bytes without a leading sign.
    fn unsigned(self) -> Self {
        Self(self.0.strip_prefix(['-', '+']).unwrap_or(self.0))
    }

    /// Digits, a `.`, and digits.
    fn fractional(self) -> bool {
        match self.0.split_once('.') {
            Some((whole, part)) => Form(whole).digits() && Form(part).digits(),
            None => false,
        }
    }

    /// Exactly `width` digits, reading no higher than `maximum`.
    fn within(self, width: usize, maximum: u32) -> bool {
        self.fixed(width) && self.number() <= maximum
    }

    fn fixed(self, width: usize) -> bool {
        self.0.len() == width && self.digits()
    }

    fn digits(self) -> bool {
        !self.0.is_empty() && self.0.bytes().all(|byte| byte.is_ascii_digit())
    }

    /// A run of digits as a number. Never called before [`Form::fixed`] has
    /// answered for it.
    fn number(self) -> u32 {
        self.0.parse().unwrap_or(0)
    }
}

fn calendar(year: u32, month: u32, day: u32) -> bool {
    (1..=12).contains(&month) && (1..=days_in(year, month)).contains(&day)
}

fn days_in(year: u32, month: u32) -> u32 {
    if month == 2 && leap(year) {
        return 29;
    }
    MONTH_LENGTHS[month as usize - 1]
}

fn leap(year: u32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn accepted(kind: ScalarKind, values: &[&str]) {
        for value in values {
            assert!(scalar(kind, value), "{kind} must accept `{value}`");
        }
    }

    fn refused(kind: ScalarKind, values: &[&str]) {
        for value in values {
            assert!(!scalar(kind, value), "{kind} must refuse `{value}`");
        }
    }

    #[test]
    fn a_string_accepts_any_scalars_bytes_as_they_were_written() {
        accepted(
            ScalarKind::String,
            &["", "NO", "true", "1", "  spaced  ", "[[a link]]"],
        );
    }

    #[test]
    fn an_integer_is_digits_and_a_float_is_never_one() {
        accepted(ScalarKind::Integer, &["0", "1", "-1", "+1", "0042"]);
        refused(
            ScalarKind::Integer,
            &["", "1.0", "1e3", "1_000", "0x10", " 1", "1 ", "one", "-"],
        );
    }

    #[test]
    fn a_float_carries_a_fractional_part_or_an_exponent_and_an_integer_is_neither() {
        accepted(
            ScalarKind::Float,
            &["1.0", "-1.5", "+0.5", "1e3", "1.5e-3", "1E+3"],
        );
        refused(
            ScalarKind::Float,
            &["1", "", ".5", "1.", "1.0.0", "1e", "1e1.5", "e3", "one"],
        );
    }

    #[test]
    fn a_boolean_is_written_out_and_yamls_own_spellings_are_not_it() {
        accepted(ScalarKind::Boolean, &["true", "false"]);
        refused(
            ScalarKind::Boolean,
            &["NO", "no", "yes", "True", "TRUE", "1", "0", ""],
        );
    }

    #[test]
    fn a_date_is_an_rfc_3339_full_date_and_nothing_that_merely_looks_like_one() {
        accepted(
            ScalarKind::Date,
            &["2026-08-03", "2000-02-29", "2024-02-29"],
        );
        refused(
            ScalarKind::Date,
            &[
                "2026-8-3",
                "2026-08-03T00:00:00Z",
                "26-08-03",
                "2026-13-01",
                "2026-00-01",
                "2026-02-30",
                "2023-02-29",
                "1900-02-29",
                "2026-04-31",
                "2026-01-00",
                "2026-01",
                "2026",
                "",
            ],
        );
    }

    #[test]
    fn a_datetime_carries_a_mandatory_offset_and_a_local_time_is_not_one() {
        accepted(
            ScalarKind::DateTime,
            &[
                "2026-07-31T09:15:00-04:00",
                "2026-07-31T09:15:00Z",
                "2026-07-31t09:15:00z",
                "2026-07-31T09:15:00.123456Z",
                "2026-12-31T23:59:60Z",
                "2026-07-31T09:15:00+00:00",
            ],
        );
        refused(
            ScalarKind::DateTime,
            &[
                "2026-07-31T09:15:00",
                "2026-07-31 09:15:00Z",
                "2026-07-31T09:15Z",
                "2026-07-31T24:00:00Z",
                "2026-07-31T09:60:00Z",
                "2026-07-31T09:15:61Z",
                "2026-07-31T09:15:00.Z",
                "2026-07-31T09:15:00-4:00",
                "2026-07-31T09:15:00-24:00",
                "2026-07-31T09:15:00-04:60",
                "2026-07-31T09:15:00-0400",
                "2026-07-31T0915Z",
                "2026-07-31T0915-04:00",
                "2026-07-31T9:15:00Z",
                "2026-07-31T09:1:00Z",
                "2026-07-31T09:15:0Z",
                "2026-07-31T09:15:00-04:0",
                "2026-02-30T09:15:00Z",
                "2026-07-31",
            ],
        );
    }

    #[test]
    fn an_enum_value_is_a_member_and_membership_is_exact() {
        let values = ["draft".to_owned(), "archived".to_owned()];
        let held = (member(&values, "draft"), member(&values, "archived"));
        assert_eq!(held, (true, true));
        let outside = (
            member(&values, "Draft"),
            member(&values, "published"),
            member(&[], "draft"),
        );
        assert_eq!(outside, (false, false, false));
    }

    #[test]
    fn a_leap_year_is_the_gregorian_rule_and_not_merely_every_fourth_year() {
        let leaps = (leap(2024), leap(2000), leap(2023), leap(1900));
        assert_eq!(leaps, (true, true, false, false));
        let february = (days_in(2024, 2), days_in(2023, 2), days_in(2023, 12));
        assert_eq!(february, (29, 28, 31));
    }
}
