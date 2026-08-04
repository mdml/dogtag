//! Collecting one note's diagnostics while it is read.
//!
//! Every finding about a note points at that note. Two shapes of location come
//! out of here and both are deliberate:
//!
//! - a **span**, wherever the note wrote the thing that is wrong;
//! - the **whole file**, for a finding whose subject is an absence. A missing
//!   required property has no bytes to point at, and inventing a span — the
//!   frontmatter block, the `type` line — would name a location the record does
//!   not sanction and that the reader would then have to disbelieve.
//!
//! Absence-shaped findings therefore share a location and an identifier, which
//! the standing diagnostic order leaves tied. The tie-break is **emission
//! order**, and emission order is the contract's declaration order: a note
//! missing three required properties reports them in the order the type
//! declares them, never in the order a hash map happened to hold them.

use core::ops::Range;

use crate::contract::Contract;
use crate::diagnostic::{
    Diagnostic, DiagnosticList, FileRef, KernelDiagnostic, Location, Related, VaultPath,
};
use crate::encoding::Text;

/// Evidence pointing at where the contract writes the requirement.
///
/// A requirement is always written rather than defaulted — `required` defaults
/// to `false` — so the provenance the contract already recorded is the location,
/// and evidence with no location is what a contract assembled some other way
/// gets. It sits beside [`Findings`] because it is one rule with two callers: a
/// missing property's evidence and a missing record field's point at their
/// declarations identically.
pub(crate) fn declaration(contract: &Contract, key: &str, message: String) -> Related {
    let mut related = Related::new(message);
    related.location = contract
        .provenance()
        .get(key)
        .and_then(|entry| entry.location.clone());
    related
}

/// One note's diagnostics, and what they point at.
pub(crate) struct Findings<'a> {
    text: &'a Text,
    file: FileRef,
    list: DiagnosticList,
}

impl<'a> Findings<'a> {
    /// A collector for the note at `path`, whose bytes are `text`.
    pub(crate) fn new(path: &VaultPath, text: &'a Text) -> Self {
        Self {
            text,
            file: FileRef::InVault(path.clone()),
            list: DiagnosticList::new(),
        }
    }

    /// The location of a byte range within the note.
    pub(crate) fn at(&self, span: Range<usize>) -> Location {
        Location::in_file(self.file.clone(), self.text.span(span))
    }

    /// The note itself, for a finding about something that is not there.
    pub(crate) fn note(&self) -> Location {
        Location::whole_file(self.file.clone())
    }

    /// Reports a fault at a byte range.
    pub(crate) fn spanned(&mut self, kind: KernelDiagnostic, message: String, span: Range<usize>) {
        let at = self.at(span);
        self.list.push(Diagnostic::kernel(kind, message).at(at));
    }

    /// Reports a fault about something the note does not carry, naming where
    /// the requirement is declared.
    pub(crate) fn absent(&mut self, kind: KernelDiagnostic, message: String, declared: Related) {
        let at = self.note();
        self.list.push(
            Diagnostic::kernel(kind, message)
                .at(at)
                .with_related(declared),
        );
    }

    /// Everything found, in the deterministic total order.
    pub(crate) fn finish(self) -> Vec<Diagnostic> {
        self.list.sorted()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Position, Severity, Span};
    use crate::encoding::inspect;

    fn text() -> Text {
        inspect(b"---\ntype: person\n---\n").expect("well-formed")
    }

    fn findings(text: &Text) -> Findings<'_> {
        Findings::new(&VaultPath::kernel("people/ada.md"), text)
    }

    #[test]
    fn a_span_becomes_a_line_and_a_column_in_the_note_itself() {
        let text = text();
        let findings = findings(&text);
        assert_eq!(
            findings.at(4..8),
            Location::in_file(
                FileRef::InVault(VaultPath::kernel("people/ada.md")),
                Span::between(Position::new(2, 1, 4), Position::new(2, 5, 8))
            )
        );
    }

    #[test]
    fn a_finding_about_an_absence_points_at_the_note_and_carries_the_declaration() {
        let text = text();
        let mut findings = findings(&text);
        findings.absent(
            KernelDiagnostic::NoteMissingRequiredProperty,
            "the type `person` requires `full_name`".to_owned(),
            Related::new("declared here"),
        );
        let reported = findings.finish();
        assert_eq!(reported[0].severity, Severity::Error);
        assert_eq!(
            reported[0].location,
            Some(Location::whole_file(FileRef::InVault(VaultPath::kernel(
                "people/ada.md"
            ))))
        );
        assert_eq!(reported[0].related[0].message, "declared here");
    }

    #[test]
    fn findings_come_out_in_the_total_order_with_absences_before_spans() {
        let text = text();
        let mut findings = findings(&text);
        findings.spanned(
            KernelDiagnostic::NoteUndeclaredProperty,
            "spanned".to_owned(),
            4..8,
        );
        findings.absent(
            KernelDiagnostic::NoteMissingRequiredProperty,
            "absent".to_owned(),
            Related::new("declared here"),
        );
        let reported = findings.finish();
        let messages: Vec<&str> = reported
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect();
        assert_eq!(messages, ["absent", "spanned"]);
    }
}
