//! Which stream a rendering goes to, and the colour applied on the way.
//!
//! Every rendering is the SDK's; nothing here re-derives one. What this module
//! decides is where it goes and whether escape sequences are wrapped around
//! it.
//!
//! # Streams
//!
//! A report goes to standard output, and diagnostics about a run that produced
//! no report go to standard error. That line matters most for `--format json`:
//! a consumer piping standard output receives valid JSON or nothing at all,
//! never a human-readable refusal it cannot parse.
//!
//! # Colour
//!
//! Colour is applied only when the destination stream **is a terminal** and
//! `NO_COLOR` is unset, and only around the diagnostic headline the SDK's
//! renderer documents. It never changes the text: a piped run and a terminal
//! run differ by escape sequences and by nothing else. Structured output — the
//! JSON reports and the generated agent contract — takes no colour at all,
//! because it is parsed and diffed rather than read.

use std::borrow::Cow;
use std::io::{self, IsTerminal, Write};

use crate::environment::Environment;

/// The escape that returns the terminal to its own colours.
const RESET: &str = "\u{1b}[0m";

/// Each severity as a diagnostic block opens with it, and the colour it takes.
///
/// The prefixes are the severities' own wire spellings followed by the bracket
/// that opens the identifier — the shape `render_plain` documents. Matching a
/// line's opening is enough to find a headline because the SDK folds every line
/// break out of the corpus text that reaches a line-oriented rendering: a
/// message, a note, a help line and a report row are each exactly one line, so
/// nothing inside one can start a line here and be coloured as a headline.
const SEVERITIES: &[(&str, &str)] = &[
    ("error[", "\u{1b}[31m"),
    ("warning[", "\u{1b}[33m"),
    ("info[", "\u{1b}[36m"),
];

/// Text the SDK rendered, on its way to a stream.
///
/// A type rather than a bare string because the constraint is the whole point:
/// these bytes are the SDK's, and a consumer may wrap escape sequences around
/// them but may never change one of them. Which rendering it is decides
/// whether colour may reach it at all.
#[derive(Clone, Copy)]
pub struct Rendering<'a> {
    text: &'a str,
    colourable: bool,
}

impl<'a> Rendering<'a> {
    /// A rendering carrying diagnostics, whose headlines colour may reach.
    pub fn diagnostics(text: &'a str) -> Self {
        Self {
            text,
            colourable: true,
        }
    }

    /// A rendering colour never touches: a structured report, the generated
    /// agent contract, a version string, a note pointing at another command.
    pub fn verbatim(text: &'a str) -> Self {
        Self {
            text,
            colourable: false,
        }
    }

    /// The bytes to write, with escapes around each headline when this
    /// rendering takes colour and the stream does.
    fn resolved(self, terminal: bool) -> Cow<'a, str> {
        if self.colourable && terminal {
            Cow::Owned(paint(self.text))
        } else {
            Cow::Borrowed(self.text)
        }
    }
}

/// Writes a rendering to standard output.
pub fn to_stdout(environment: &Environment, rendering: Rendering<'_>) {
    let colour = environment.colour() && io::stdout().is_terminal();
    emit(&mut io::stdout().lock(), &rendering.resolved(colour));
}

/// Writes a rendering to standard error.
///
/// An empty rendering writes nothing at all, so a clean run stays silent on
/// this stream rather than emitting a blank line.
pub fn to_stderr(environment: &Environment, rendering: Rendering<'_>) {
    if rendering.text.is_empty() {
        return;
    }
    let colour = environment.colour() && io::stderr().is_terminal();
    emit(&mut io::stderr().lock(), &rendering.resolved(colour));
}

/// Writes `text`, and gives up quietly if the stream will not take it.
///
/// A closed pipe is the ordinary case — `| head` closes one — and the only
/// place to report a failure to write would be the stream that just refused
/// the bytes, so this reports nothing rather than panicking.
fn emit(stream: &mut dyn Write, text: &str) {
    if stream.write_all(text.as_bytes()).is_ok() {
        stream.flush().ok();
    }
}

/// Every diagnostic headline in `text`, coloured.
///
/// Line endings are preserved exactly, and every byte that is not an escape
/// sequence survives unchanged.
fn paint(text: &str) -> String {
    text.split_inclusive('\n').map(paint_line).collect()
}

/// One line, coloured around `<severity>[<identifier>]` when it opens a
/// diagnostic block, and left alone when it does not.
fn paint_line(line: &str) -> Cow<'_, str> {
    let Some((_, colour)) = SEVERITIES
        .iter()
        .find(|(opening, _)| line.starts_with(opening))
    else {
        return Cow::Borrowed(line);
    };
    let Some((headline, rest)) = line.split_once("]: ") else {
        return Cow::Borrowed(line);
    };
    Cow::Owned(format!("{colour}{headline}]{RESET}: {rest}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPORT: &str = concat!(
        "vault\n",
        "  root          /vaults/work\n",
        "\n",
        "error[contract.no-types]: this contract declares no type\n",
        "  --> .dogtag/contract.toml:1:1\n",
        "warning[discovery.nested-vault]: an ancestor also holds a contract\n",
        "info[compat.newer-format-available]: a newer format exists\n",
    );

    /// A writer that refuses every byte, standing in for a closed pipe.
    struct Closed;

    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }
    }

    /// The text with every escape sequence this module can emit removed.
    fn stripped(painted: &str) -> String {
        let mut text = painted.replace(RESET, "");
        for (_, colour) in SEVERITIES {
            text = text.replace(colour, "");
        }
        text
    }

    #[test]
    fn colour_adds_escapes_and_changes_no_other_byte() {
        let painted = paint(REPORT);
        assert_ne!(painted, REPORT);
        assert_eq!(stripped(&painted), REPORT);
    }

    #[test]
    fn every_severity_takes_its_own_colour() {
        let painted = paint(REPORT);
        for (opening, colour) in SEVERITIES {
            assert!(
                painted.contains(&format!("{colour}{opening}")),
                "{opening} was not coloured: {painted}"
            );
        }
    }

    #[test]
    fn a_line_that_does_not_open_a_block_is_left_alone() {
        let left_alone = [
            "  --> .dogtag/contract.toml:1:1\n",
            "errors[are-not-a-severity]: an identifier is not a headline\n",
            "error[unterminated\n",
        ];
        for line in left_alone {
            assert_eq!(paint_line(line), line);
        }
    }

    #[test]
    fn a_rendering_that_takes_no_colour_reaches_a_terminal_unchanged() {
        let structured = Rendering::verbatim(REPORT);
        assert!(matches!(structured.resolved(true), Cow::Borrowed(_)));
        let diagnostics = Rendering::diagnostics(REPORT);
        assert!(matches!(diagnostics.resolved(false), Cow::Borrowed(_)));
        assert!(matches!(diagnostics.resolved(true), Cow::Owned(_)));
    }

    #[test]
    fn a_stream_that_refuses_the_bytes_is_not_a_failure_here() {
        let mut closed = Closed;
        emit(&mut closed, REPORT);
        assert!(
            closed.flush().is_err(),
            "the stand-in refuses a flush as well as a write"
        );
        let mut taken = Vec::new();
        emit(&mut taken, REPORT);
        assert_eq!(String::from_utf8(taken).expect("what was written"), REPORT);
    }
}
