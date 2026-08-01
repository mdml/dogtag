//! The diagnostic envelope: what every failure in this SDK is reported as.
//!
//! Every foreseeable failure — an unreadable file, a permission denial,
//! malformed TOML, an unsupported format version — is a [`Diagnostic`] with an
//! identifier, never a bare error. The envelope is the same for the kernel's
//! own diagnostics and for a consumer's, which is what lets a corpus-specific
//! linter written over this API render and exit exactly as the kernel does.
//!
//! - [`id`] holds the exhaustive identifier set and the `ext.` namespace rule.
//! - [`location`] holds the two file references and spans.
//! - [`order`] holds the deterministic total order.
//! - [`render`] holds the colourless plain-text rendering.

pub mod id;
pub mod location;
pub mod order;
pub mod render;

use core::fmt;

pub use id::{DiagnosticId, InvalidExternalId, KernelDiagnostic};
pub use location::{FileRef, Location, Position, Span};
pub use render::render_plain;

/// How much a diagnostic matters.
///
/// The ordering is the rendering order — errors first — and is deliberately
/// *not* an importance ordering to compare with `max`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The operation cannot produce a trustworthy result.
    Error,
    /// The result stands, but something needs attention.
    Warning,
    /// Context the caller did not ask for and should see.
    Info,
}

impl Severity {
    /// The lowercase wire spelling, used by every structured format.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A second place worth looking, attached to a diagnostic.
///
/// "This contract declares two catch-all types" is unusable without pointing
/// at both, which is why evidence is part of the envelope rather than prose
/// folded into the message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Related {
    /// Where the evidence is, when it is somewhere in particular.
    pub location: Option<Location>,
    /// What the evidence says.
    pub message: String,
}

impl Related {
    /// Evidence with no location.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            location: None,
            message: message.into(),
        }
    }

    /// The same evidence, pointing at a location.
    pub fn at(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }
}

/// One reported fault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// The stable identifier consumers match on.
    pub id: DiagnosticId,
    /// How much it matters.
    pub severity: Severity,
    /// What is wrong, in the reader's terms.
    pub message: String,
    /// Where it is, when a structured location exists.
    ///
    /// Diagnostics about a *directory* deliberately carry `None` and name the
    /// directory in their message: a machine path is never a structured
    /// location, which is what keeps conformance goldens machine-independent.
    pub location: Option<Location>,
    /// Other places worth looking.
    pub related: Vec<Related>,
    /// What to do about it.
    pub help: Option<String>,
}

impl Diagnostic {
    /// A diagnostic under an explicit identifier and severity.
    ///
    /// This is the constructor a consumer uses, with an identifier from
    /// [`DiagnosticId::external`].
    pub fn new(id: DiagnosticId, severity: Severity, message: impl Into<String>) -> Self {
        Self {
            id,
            severity,
            message: message.into(),
            location: None,
            related: Vec::new(),
            help: None,
        }
    }

    /// A kernel diagnostic, at the severity its identifier declares.
    pub fn kernel(kind: KernelDiagnostic, message: impl Into<String>) -> Self {
        Self::new(DiagnosticId::kernel(kind), kind.severity(), message)
    }

    /// The same diagnostic, pointing at a location.
    pub fn at(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    /// The same diagnostic, carrying one more piece of related evidence.
    pub fn with_related(mut self, related: Related) -> Self {
        self.related.push(related);
        self
    }

    /// The same diagnostic, carrying a help line.
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// How many diagnostics of each severity a run produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeverityCounts {
    /// Error-severity diagnostics.
    pub error: usize,
    /// Warning-severity diagnostics.
    pub warning: usize,
    /// Info-severity diagnostics.
    pub info: usize,
}

impl SeverityCounts {
    /// No diagnostics at all.
    pub fn zero() -> Self {
        Self {
            error: 0,
            warning: 0,
            info: 0,
        }
    }

    fn tally(&mut self, severity: Severity) {
        match severity {
            Severity::Error => self.error += 1,
            Severity::Warning => self.warning += 1,
            Severity::Info => self.info += 1,
        }
    }
}

/// Diagnostics as they accumulate, before they are ordered.
///
/// Collection is deliberately separate from ordering: a parse collects every
/// diagnostic it finds rather than stopping at the first, and the order is
/// imposed once, at the end, by [`DiagnosticList::sorted`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DiagnosticList {
    items: Vec<Diagnostic>,
}

impl DiagnosticList {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one diagnostic.
    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    /// The diagnostics in the order they were emitted.
    pub fn as_slice(&self) -> &[Diagnostic] {
        &self.items
    }

    /// How many diagnostics the list holds.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the list is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// How many diagnostics of each severity the list holds.
    pub fn counts(&self) -> SeverityCounts {
        let mut counts = SeverityCounts::zero();
        for item in &self.items {
            counts.tally(item.severity);
        }
        counts
    }

    /// The diagnostics in the deterministic total order.
    ///
    /// The sort is stable, so exact ties keep emission order — which is itself
    /// deterministic, because nothing in this SDK collects in filesystem order.
    pub fn sorted(self) -> Vec<Diagnostic> {
        let mut items = self.items;
        items.sort_by(order::compare);
        items
    }
}

impl Extend<Diagnostic> for DiagnosticList {
    fn extend<T: IntoIterator<Item = Diagnostic>>(&mut self, diagnostics: T) {
        self.items.extend(diagnostics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::cmp::Ordering;

    fn contract_location() -> Location {
        Location::in_file(
            FileRef::InVault(".dogtag/contract.toml".to_owned()),
            Span::at(Position::new(4, 3, 31)),
        )
    }

    fn multiple_catch_all() -> Diagnostic {
        Diagnostic::kernel(KernelDiagnostic::ContractMultipleCatchAll, "two")
    }

    #[test]
    fn severities_render_for_structured_output() {
        assert_eq!(Severity::Error.as_str(), "error");
        assert_eq!(Severity::Warning.to_string(), "warning");
        assert_eq!(Severity::Info.as_str(), "info");
    }

    #[test]
    fn severities_order_errors_first() {
        assert!(Severity::Error < Severity::Warning);
        assert!(Severity::Warning < Severity::Info);
        assert_eq!(Severity::Info.cmp(&Severity::Info), Ordering::Equal);
    }

    #[test]
    fn severities_format_for_debugging() {
        let levels = vec![Severity::Error, Severity::Warning, Severity::Info];
        assert_eq!(levels.clone(), levels);
        assert!(format!("{levels:?}").contains("Warning"));
    }

    #[test]
    fn a_kernel_diagnostic_takes_the_severity_its_identifier_declares() {
        let diagnostic = Diagnostic::kernel(KernelDiagnostic::DiscoveryNestedVault, "an ancestor");
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(diagnostic.id.as_str(), "discovery.nested-vault");
        assert_eq!(diagnostic.message, "an ancestor");
    }

    #[test]
    fn a_new_diagnostic_carries_nothing_it_was_not_given() {
        let diagnostic = Diagnostic::kernel(KernelDiagnostic::ContractNoTypes, "no types");
        assert!(diagnostic.location.is_none());
        assert!(diagnostic.related.is_empty());
        assert!(diagnostic.help.is_none());
    }

    #[test]
    fn a_consumer_diagnostic_carries_its_own_identifier_and_severity() {
        let id = DiagnosticId::external("ext.acme.dangling-link").expect("well-formed");
        let diagnostic = Diagnostic::new(id, Severity::Warning, "a link resolves to nothing");
        assert_eq!(diagnostic.id.as_str(), "ext.acme.dangling-link");
        assert_eq!(diagnostic.severity, Severity::Warning);
    }

    #[test]
    fn the_builder_attaches_a_location_and_a_help_line() {
        let diagnostic = multiple_catch_all()
            .at(contract_location())
            .with_help("exactly one type may declare catch-all");
        assert_eq!(diagnostic.location, Some(contract_location()));
        let help = diagnostic.help.as_deref();
        assert_eq!(help, Some("exactly one type may declare catch-all"));
    }

    #[test]
    fn the_builder_appends_evidence_in_the_order_it_was_given() {
        let diagnostic = multiple_catch_all()
            .with_related(Related::new("also declared here").at(contract_location()))
            .with_related(Related::new("no location here"));
        assert_eq!(diagnostic.related.len(), 2);
        assert_eq!(diagnostic.related[0].message, "also declared here");
        assert_eq!(diagnostic.related[0].location, Some(contract_location()));
        assert!(diagnostic.related[1].location.is_none());
    }

    #[test]
    fn diagnostics_clone_and_format() {
        let diagnostic = multiple_catch_all()
            .at(contract_location())
            .with_related(Related::new("here").at(contract_location()));
        let copy = diagnostic.clone();
        assert_eq!(copy, diagnostic);
        assert!(format!("{diagnostic:?}").contains("ContractMultipleCatchAll"));
        assert!(format!("{:?}", diagnostic.related[0]).contains("here"));
    }

    #[test]
    fn a_list_starts_empty() {
        let list = DiagnosticList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert_eq!(list.counts(), SeverityCounts::zero());
    }

    #[test]
    fn a_list_takes_pushes_and_extensions() {
        let mut list = DiagnosticList::new();
        list.push(multiple_catch_all());
        list.extend([Diagnostic::kernel(
            KernelDiagnostic::DiscoveryNestedVault,
            "nested",
        )]);
        assert_eq!(list.len(), 2);
        assert!(!list.is_empty());
        assert_eq!(list.as_slice()[0].message, "two");
    }

    #[test]
    fn a_list_counts_each_severity_separately() {
        let mut list = DiagnosticList::new();
        list.extend([
            multiple_catch_all(),
            Diagnostic::kernel(KernelDiagnostic::DiscoveryNestedVault, "nested"),
            Diagnostic::kernel(KernelDiagnostic::CompatNewerFormatAvailable, "newer"),
        ]);
        let counts = list.counts();
        assert_eq!(counts.error, 1);
        assert_eq!(counts.warning, 1);
        assert_eq!(counts.info, 1);
    }

    #[test]
    fn severity_counts_clone_and_format() {
        let counts = DiagnosticList::default().counts();
        assert_eq!(counts, SeverityCounts::zero());
        let boxed = vec![counts];
        assert_eq!(boxed.clone(), boxed);
        assert!(format!("{counts:?}").contains("error"));
    }

    #[test]
    fn sorting_a_list_applies_the_total_order() {
        let mut list = DiagnosticList::new();
        list.push(multiple_catch_all().at(contract_location()));
        list.push(Diagnostic::kernel(
            KernelDiagnostic::DiscoveryNoVaultFound,
            "unlocated",
        ));
        let sorted = list.sorted();
        assert_eq!(sorted[0].message, "unlocated");
        assert_eq!(sorted[1].message, "two");
    }

    #[test]
    fn a_list_clones_and_formats() {
        let mut list = DiagnosticList::new();
        list.push(multiple_catch_all());
        let copy = list.clone();
        assert_eq!(copy, list);
        assert!(format!("{list:?}").contains("ContractMultipleCatchAll"));
    }
}
