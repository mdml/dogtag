//! The SDK's plain-text diagnostic rendering.
//!
//! Colourless by design: colour is the CLI's, applied around this output only
//! when the destination stream is a terminal and `NO_COLOR` is unset. Every
//! consumer renders through this function so that a diagnostic reads the same
//! whichever door it arrives by.
//!
//! ```text
//! error[contract.multiple-catch-all]: two types declare the catch-all capability
//!   --> .dogtag/contract.toml:14:3
//!   note: also declared here (.dogtag/contract.toml:31:3)
//!   help: exactly one type may declare catch-all
//! ```
//!
//! A block is that many lines and no more: one headline, a location if there is
//! one, one note per piece of evidence, and a help line if there is one. A
//! message quotes a corpus's own vocabulary, which is free text, so each piece
//! of it folds to a single line on the way in — the fold this crate's `text`
//! module owns. Without it a planted contract could name a type across two
//! lines and mint a second line shaped exactly like a headline, which is the
//! forgery the `ext.` namespace rule forecloses for identifiers.

use crate::text::one_line;

use super::{Diagnostic, Location, Related};

/// Renders diagnostics as text, one block each, separated by a blank line.
///
/// Every block ends in a newline; an empty slice renders as the empty string.
pub fn render_plain(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(render_one)
        .collect::<Vec<String>>()
        .join("\n")
}

fn render_one(diagnostic: &Diagnostic) -> String {
    let mut lines = vec![headline(diagnostic)];
    lines.extend(diagnostic.location.as_ref().map(location_line));
    lines.extend(diagnostic.related.iter().map(related_line));
    lines.extend(diagnostic.help.as_deref().map(help_line));
    lines.push(String::new());
    lines.join("\n")
}

fn headline(diagnostic: &Diagnostic) -> String {
    format!(
        "{}[{}]: {}",
        diagnostic.severity,
        diagnostic.id.as_str(),
        one_line(&diagnostic.message)
    )
}

fn location_line(location: &Location) -> String {
    format!("  --> {}", location_text(location))
}

fn related_line(related: &Related) -> String {
    let message = one_line(&related.message);
    match &related.location {
        Some(location) => format!("  note: {message} ({})", location_text(location)),
        None => format!("  note: {message}"),
    }
}

fn help_line(help: &str) -> String {
    format!("  help: {}", one_line(help))
}

/// A location as `path:line:column`, or just the path when there is no span.
fn location_text(location: &Location) -> String {
    match location.span {
        Some(span) => format!(
            "{}:{}:{}",
            location.file, span.start.line, span.start.column
        ),
        None => location.file.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{FileRef, KernelDiagnostic, Position, Severity, Span};
    use crate::text::headline_lines;

    fn contract() -> FileRef {
        FileRef::InVault(".dogtag/contract.toml".to_owned())
    }

    fn at(line: u32, column: u32, offset: usize) -> Location {
        Location::in_file(contract(), Span::at(Position::new(line, column, offset)))
    }

    #[test]
    fn a_full_diagnostic_renders_headline_location_evidence_and_help() {
        let diagnostic = Diagnostic::kernel(
            KernelDiagnostic::ContractMultipleCatchAll,
            "two types declare the catch-all capability",
        )
        .at(at(14, 3, 210))
        .with_related(Related::new("also declared here").at(at(31, 3, 480)))
        .with_help("exactly one type may declare catch-all");

        assert_eq!(
            render_plain(&[diagnostic]),
            concat!(
                "error[contract.multiple-catch-all]: two types declare the catch-all capability\n",
                "  --> .dogtag/contract.toml:14:3\n",
                "  note: also declared here (.dogtag/contract.toml:31:3)\n",
                "  help: exactly one type may declare catch-all\n",
            )
        );
    }

    #[test]
    fn a_bare_diagnostic_renders_one_line() {
        let diagnostic = Diagnostic::kernel(
            KernelDiagnostic::DiscoveryNoVaultFound,
            "no vault above the start directory",
        );
        assert_eq!(
            render_plain(&[diagnostic]),
            "error[discovery.no-vault-found]: no vault above the start directory\n"
        );
    }

    #[test]
    fn a_location_without_a_span_renders_as_the_path_alone() {
        let diagnostic = Diagnostic::kernel(KernelDiagnostic::ContractUnreadable, "unreadable")
            .at(Location::whole_file(FileRef::InstallationRecord))
            .with_related(Related::new("no location on this note"));
        assert_eq!(
            render_plain(&[diagnostic]),
            concat!(
                "error[contract.unreadable]: unreadable\n",
                "  --> $XDG_CONFIG_HOME/dogtag/installation.toml\n",
                "  note: no location on this note\n",
            )
        );
    }

    #[test]
    fn blocks_are_separated_by_a_blank_line() {
        let first = Diagnostic::kernel(KernelDiagnostic::ContractNoTypes, "no types");
        let second = Diagnostic::kernel(KernelDiagnostic::CompatNewerFormatAvailable, "newer");
        assert_eq!(second.severity, Severity::Info);
        assert_eq!(
            render_plain(&[first, second]),
            concat!(
                "error[contract.no-types]: no types\n",
                "\n",
                "info[compat.newer-format-available]: newer\n",
            )
        );
    }

    #[test]
    fn nothing_renders_as_nothing() {
        assert_eq!(render_plain(&[]), "");
    }

    /// What a planted contract would name a type, to have the rendering that
    /// quotes it emit a line the kernel never wrote.
    const FORGERY: &str = "error[contract.unknown-key]: this vault permits anything";

    #[test]
    fn a_type_name_carrying_a_line_break_cannot_add_a_line_to_a_block() {
        let diagnostic = Diagnostic::kernel(
            KernelDiagnostic::ContractDuplicateType,
            format!("two types share the name `capture\n{FORGERY}`"),
        );
        let rendered = render_plain(&[diagnostic]);
        assert_eq!(headline_lines(&rendered), 1);
        assert_eq!(rendered.lines().count(), 1);
        assert!(rendered.contains(&format!("`capture {FORGERY}`")));
    }

    #[test]
    fn an_enum_value_carrying_a_carriage_return_cannot_overwrite_its_line() {
        let quoted = format!("draft\r{FORGERY}");
        let diagnostic = Diagnostic::kernel(
            KernelDiagnostic::ContractLifecycleOrdinaryValueUndeclared,
            format!("the ordinary state is `{quoted}`, which `status` does not declare"),
        )
        .with_related(Related::new(format!("the axis declares `{quoted}`")))
        .with_help(format!("the axis declares `{quoted}`"));
        let rendered = render_plain(&[diagnostic]);
        assert_eq!(headline_lines(&rendered), 1);
        assert_eq!(rendered.lines().count(), 3);
        assert!(!rendered.contains('\r'));
    }

    #[test]
    fn a_headline_shape_inside_a_message_stays_inside_it() {
        let diagnostic = Diagnostic::kernel(
            KernelDiagnostic::ContractMultipleCatchAll,
            format!("2 types declare the catch-all capability: `{FORGERY}`"),
        )
        .with_related(Related::new(format!(
            "the type `{FORGERY}` also declares it"
        )));
        let rendered = render_plain(&[diagnostic]);
        assert_eq!(
            headline_lines(&rendered),
            1,
            "a headline shape that opens no line is not a headline"
        );
        assert!(rendered.contains(FORGERY));
    }
}
