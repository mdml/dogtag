//! Collecting a contract's diagnostics and provenance while its body is walked.
//!
//! One [`Sink`] carries three things every step of the walk needs: the text, so
//! a byte range becomes a line and a column; the declared version, so a message
//! can say which version's key set refused a key; and the two accumulators. It
//! exists so that no walking function has to take five arguments to say one
//! thing.
//!
//! Every leaf read through a `Sink` records its provenance as it is read, which
//! is what keeps the recorded key set and the resolved model from drifting.

use core::ops::Range;

use toml::Spanned;
use toml::de::{DeArray, DeTable, DeValue};

use crate::diagnostic::{Diagnostic, DiagnosticList, FileRef, KernelDiagnostic, Location, Related};
use crate::document::{self, TypeMismatch};
use crate::encoding::Text;
use crate::provenance::{Provenance, ProvenanceEntry, Source};

use super::CONTRACT_PATH;

/// The file every contract diagnostic and every contract provenance entry
/// names, whatever path the bytes were read from. A machine path never reaches
/// structured output.
pub(crate) fn contract_file() -> FileRef {
    FileRef::InVault(CONTRACT_PATH.to_owned())
}

/// The dotted provenance path of a declaration.
///
/// A declaration the contract does not name — a `[[type]]` whose `name` is
/// missing — has no addressable path. Every leaf under it then records no
/// provenance at all, rather than inventing a key that section 9's exhaustive
/// list does not contain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyPath(Option<String>);

impl KeyPath {
    /// The contract root, under which `contract_version` is a direct leaf.
    pub(crate) fn root() -> Self {
        Self(Some(String::new()))
    }

    /// A path for a declaration that cannot be addressed.
    pub(crate) fn nameless() -> Self {
        Self(None)
    }

    /// This path extended by one segment.
    pub(crate) fn child(&self, segment: &str) -> Self {
        Self(self.0.as_ref().map(|prefix| join(prefix, segment)))
    }

    /// This path extended by a segment the contract may not have written.
    pub(crate) fn child_opt(&self, segment: Option<&str>) -> Self {
        segment.map_or_else(Self::nameless, |segment| self.child(segment))
    }

    /// A leaf under this path: the key name a message quotes, and the
    /// provenance key, when the declaration is addressable.
    pub(crate) fn leaf(&self, name: &'static str) -> Leaf {
        Leaf {
            name,
            key: self.child(name).0,
        }
    }
}

fn join(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_owned()
    } else {
        format!("{prefix}.{segment}")
    }
}

/// A leaf value's key name, and its provenance key when it has one.
pub(crate) struct Leaf {
    pub(crate) name: &'static str,
    pub(crate) key: Option<String>,
}

/// One table being walked, with everything a diagnostic about it needs.
pub(crate) struct Section<'a, 'i> {
    /// The table itself.
    pub(crate) table: &'a DeTable<'i>,
    /// The byte range of the whole table, which is where a missing key points.
    pub(crate) span: Range<usize>,
    /// How a message names this table, for example ``[dialect]``.
    pub(crate) label: String,
    /// The provenance path of the declaration the table carries.
    pub(crate) path: KeyPath,
}

impl<'a, 'i> Section<'a, 'i> {
    /// The value this table declares for `key`.
    pub(crate) fn get(&self, key: &str) -> Option<&'a Spanned<DeValue<'i>>> {
        document::get(self.table, key)
    }

    /// A leaf of this table.
    pub(crate) fn leaf(&self, name: &'static str) -> Leaf {
        self.path.leaf(name)
    }
}

/// A required string that names its own declaration.
///
/// It is read before the declaration's provenance path can exist, so it is the
/// one leaf whose provenance the caller records rather than the [`Sink`].
pub(crate) struct Named<'a> {
    pub(crate) text: &'a str,
    pub(crate) span: Range<usize>,
}

/// What one diagnostic says, before it is given somewhere to point.
pub(crate) struct Report {
    message: String,
    help: Option<String>,
}

impl Report {
    /// A report stating what is wrong.
    pub(crate) fn new(message: String) -> Self {
        Self {
            message,
            help: None,
        }
    }

    /// The same report, saying what to do about it.
    pub(crate) fn with_help(mut self, help: String) -> Self {
        self.help = Some(help);
        self
    }

    fn into_diagnostic(self, kind: KernelDiagnostic, at: Location) -> Diagnostic {
        let diagnostic = Diagnostic::kernel(kind, self.message).at(at);
        match self.help {
            Some(help) => diagnostic.with_help(help),
            None => diagnostic,
        }
    }
}

/// A declaration that repeats one already made.
pub(crate) struct Repeat {
    /// What is repeated.
    pub(crate) message: String,
    /// Where the repeat is.
    pub(crate) at: Range<usize>,
    /// Where the first declaration is.
    pub(crate) first: Range<usize>,
}

/// Names already claimed in one scope, with where each was first claimed.
///
/// Repeats are caught while the walk runs rather than afterwards, because
/// naming both declarations needs two spans and the resolved model keeps only
/// one declaration per name.
pub(crate) struct Seen {
    entries: Vec<(String, Range<usize>)>,
}

impl Seen {
    /// A scope in which nothing is claimed yet.
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Claims `name`, or reports where it was first claimed.
    pub(crate) fn claim(&mut self, name: &str, at: Range<usize>) -> Option<Range<usize>> {
        let first = self
            .entries
            .iter()
            .find(|(seen, _)| seen == name)
            .map(|(_, first)| first.clone());
        if first.is_none() {
            self.entries.push((name.to_owned(), at));
        }
        first
    }
}

/// A name being claimed, and what to say when it is already taken.
pub(crate) struct Claim<'a> {
    pub(crate) named: Named<'a>,
    pub(crate) message: String,
    pub(crate) kind: KernelDiagnostic,
}

/// The diagnostics and provenance one contract parse produces.
pub(crate) struct Sink<'t> {
    text: &'t Text,
    version: u32,
    diagnostics: DiagnosticList,
    provenance: Provenance,
    dropped: bool,
}

impl<'t> Sink<'t> {
    /// A sink over `text`, reporting against the version the contract declares.
    pub(crate) fn new(text: &'t Text, version: u32) -> Self {
        Self {
            text,
            version,
            diagnostics: DiagnosticList::new(),
            provenance: Provenance::new(),
            dropped: false,
        }
    }

    /// Records that a declaration was parsed far enough to be named and then
    /// discarded, so the resolved model is missing something the file declares.
    pub(crate) fn drop_declaration(&mut self) {
        self.dropped = true;
    }

    /// Whether the resolved model holds every declaration the file makes.
    ///
    /// The cross-reference rules — a flag naming a property, a lifecycle axis
    /// naming one — conclude "no type declares it" from the model's silence.
    /// When a declaration was dropped that silence is the parser's, not the
    /// file's, and the conclusion contradicts the contract in front of the
    /// reader. The narrower fault is already reported; inventing a second,
    /// false one on top of it is what this guards.
    pub(crate) fn complete(&self) -> bool {
        !self.dropped
    }

    /// What has been recorded so far, so a validity rule can point at where a
    /// value is written.
    pub(crate) fn provenance(&self) -> &Provenance {
        &self.provenance
    }

    /// A byte range as a location in the contract.
    pub(crate) fn location(&self, span: Range<usize>) -> Location {
        Location::in_file(contract_file(), self.text.span(span))
    }

    /// The contract as a whole, where a fault with no narrower extent points.
    pub(crate) fn whole_file(&self) -> Location {
        Location::whole_file(contract_file())
    }

    /// Evidence pointing somewhere else in the contract.
    pub(crate) fn related(&self, message: &str, span: Range<usize>) -> Related {
        Related::new(message).at(self.location(span))
    }

    /// Records a diagnostic built by the caller.
    pub(crate) fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Records however many diagnostics the caller produced, including none.
    ///
    /// A caller that decides *whether* a diagnostic exists returns an `Option`
    /// and passes it here, so the decision lives in one testable place rather
    /// than as a branch at the call site.
    pub(crate) fn record(&mut self, diagnostics: impl IntoIterator<Item = Diagnostic>) {
        self.diagnostics.extend(diagnostics);
    }

    /// Records a report, pointing it at a location.
    pub(crate) fn report(&mut self, kind: KernelDiagnostic, report: Report, at: Location) {
        self.push(report.into_diagnostic(kind, at));
    }

    /// Records a diagnostic about one byte range.
    pub(crate) fn raise_at(&mut self, kind: KernelDiagnostic, message: String, at: Range<usize>) {
        let at = self.location(at);
        self.report(kind, Report::new(message), at);
    }

    /// Records a diagnostic about a declaration that repeats an earlier one.
    pub(crate) fn repeated(&mut self, kind: KernelDiagnostic, repeat: Repeat) {
        let related = self.related("first declared here", repeat.first);
        let diagnostic = Diagnostic::kernel(kind, repeat.message)
            .at(self.location(repeat.at))
            .with_related(related);
        self.push(diagnostic);
    }

    /// Records that a value's TOML type is not the one the format requires.
    pub(crate) fn wrong_type(&mut self, name: &str, mismatch: &TypeMismatch) {
        self.raise_at(
            KernelDiagnostic::ContractValueWrongType,
            mismatch.message(name),
            mismatch.span.clone(),
        );
    }

    /// Records that a table omits a key the declared version requires.
    pub(crate) fn missing(&mut self, section: &Section<'_, '_>, key: &str) {
        let message = format!(
            "{} does not declare `{key}`, which contract version {} requires",
            section.label, self.version
        );
        self.raise_at(
            KernelDiagnostic::ContractMissingKey,
            message,
            section.span.clone(),
        );
    }

    /// Records every key in a table the declared version does not define.
    ///
    /// Unknown keys are fatal at every nesting level: a typo'd `requred`
    /// silently demoting a required property is this format's worst failure
    /// mode, and no convenience is worth making it survivable.
    pub(crate) fn sweep(&mut self, section: &Section<'_, '_>, allowed: &[&str]) {
        for unknown in document::unknown_keys(section.table, allowed) {
            let message = format!(
                "{} declares `{}`, which contract version {} does not define",
                section.label, unknown.key, self.version
            );
            self.raise_at(KernelDiagnostic::ContractUnknownKey, message, unknown.span);
        }
    }

    /// Records that a leaf was written in the contract at `span`.
    pub(crate) fn written(&mut self, key: Option<String>, span: Range<usize>) {
        if let Some(key) = key {
            let location = self.location(span);
            self.provenance
                .insert(ProvenanceEntry::written(key, Source::Contract, location));
        }
    }

    /// Records that a leaf was omitted and the declared version's format
    /// default supplied it.
    pub(crate) fn defaulted(&mut self, key: Option<String>) {
        if let Some(key) = key {
            self.provenance
                .insert(ProvenanceEntry::defaulted(key, self.version));
        }
    }

    /// The value a table declares for a required key.
    pub(crate) fn required<'a, 'i>(
        &mut self,
        section: &Section<'a, 'i>,
        key: &'static str,
    ) -> Option<&'a Spanned<DeValue<'i>>> {
        let value = section.get(key);
        if value.is_none() {
            self.missing(section, key);
        }
        value
    }

    /// A required string that names its own declaration.
    ///
    /// This is the one place a declaration's name is read — a type's `name`, a
    /// property's `name`, a relationship's `predicate`, a flag's `property` —
    /// so [`Sink::nameable`] covers all four with one rule and one identifier.
    pub(crate) fn name_of<'a, 'i>(
        &mut self,
        section: &Section<'a, 'i>,
        key: &'static str,
    ) -> Option<Named<'a>> {
        let value = self.required(section, key)?;
        let text = document::expect_string(value)
            .map_err(|mismatch| self.wrong_type(key, &mismatch))
            .ok()?;
        self.nameable(text, value.span())?;
        Some(Named {
            text,
            span: value.span(),
        })
    }

    /// Whether `text` can name a declaration. `None` means it cannot, and the
    /// refusal is already recorded.
    ///
    /// A name is never empty and never holds a `.`, because a declaration is
    /// addressed by a dotted key path built by joining the names above it: a
    /// type named `t.property.p` would address exactly what type `t`'s property
    /// `p` addresses, and the second recorded would silently replace the first
    /// in the map whose whole job is saying where a value came from. An empty
    /// name addresses nothing, and renders as an empty heading.
    fn nameable(&mut self, text: &str, span: Range<usize>) -> Option<()> {
        if !text.is_empty() && !text.contains('.') {
            return Some(());
        }
        let at = self.location(span);
        let report = Report::new(format!("`{text}` cannot name a declaration")).with_help(
            "a declaration's name is never empty and never holds a `.`, because provenance \
             addresses a declaration by joining the names above it with one"
                .to_owned(),
        );
        self.report(KernelDiagnostic::ContractDeclarationNameInvalid, report, at);
        None
    }

    /// A value as a table.
    pub(crate) fn table<'a, 'i>(
        &mut self,
        value: &'a Spanned<DeValue<'i>>,
        name: &str,
    ) -> Option<&'a DeTable<'i>> {
        document::expect_table(value)
            .map_err(|mismatch| self.wrong_type(name, &mismatch))
            .ok()
    }

    /// A value as an array.
    pub(crate) fn array<'a, 'i>(
        &mut self,
        value: &'a Spanned<DeValue<'i>>,
        name: &str,
    ) -> Option<&'a DeArray<'i>> {
        document::expect_array(value)
            .map_err(|mismatch| self.wrong_type(name, &mismatch))
            .ok()
    }

    /// A leaf value as a string, recording its provenance.
    pub(crate) fn string<'a>(
        &mut self,
        value: &'a Spanned<DeValue<'_>>,
        leaf: Leaf,
    ) -> Option<&'a str> {
        let text = document::expect_string(value)
            .map_err(|mismatch| self.wrong_type(leaf.name, &mismatch))
            .ok()?;
        self.written(leaf.key, value.span());
        Some(text)
    }

    /// A leaf value as a boolean, recording its provenance.
    pub(crate) fn boolean(&mut self, value: &Spanned<DeValue<'_>>, leaf: Leaf) -> Option<bool> {
        let declared = document::expect_bool(value)
            .map_err(|mismatch| self.wrong_type(leaf.name, &mismatch))
            .ok()?;
        self.written(leaf.key, value.span());
        Some(declared)
    }

    /// An optional boolean leaf, defaulting to `false` — the value version 1
    /// gives both `required` keys — and recording which of the two happened.
    pub(crate) fn optional_flag(&mut self, section: &Section<'_, '_>, key: &'static str) -> bool {
        let leaf = section.leaf(key);
        match section.get(key) {
            Some(value) => self.boolean(value, leaf).unwrap_or(false),
            None => {
                self.defaulted(leaf.key);
                false
            }
        }
    }

    /// Claims a name for a declaration, reporting a repeat rather than keeping
    /// it. `None` means the declaration repeats one already made.
    pub(crate) fn keep(&mut self, seen: &mut Seen, claim: Claim<'_>) -> Option<()> {
        let Some(first) = seen.claim(claim.named.text, claim.named.span.clone()) else {
            return Some(());
        };
        self.repeated(
            claim.kind,
            Repeat {
                message: claim.message,
                at: claim.named.span,
                first,
            },
        );
        None
    }

    /// The diagnostics and the provenance, once the walk is over.
    pub(crate) fn finish(self) -> (DiagnosticList, Provenance) {
        (self.diagnostics, self.provenance)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::encoding;

    pub(crate) fn text_of(source: &str) -> Text {
        encoding::inspect(source.as_bytes()).expect("well-formed")
    }

    /// The document root of `source`, kept alive by the caller's `Text`.
    pub(crate) fn root_of(text: &Text) -> Spanned<DeTable<'_>> {
        document::parse(text.as_str()).expect("well-formed TOML")
    }

    fn section<'a, 'i>(table: &'a DeTable<'i>, label: &str) -> Section<'a, 'i> {
        Section {
            table,
            span: 0..table.len(),
            label: label.to_owned(),
            path: KeyPath::root(),
        }
    }

    #[test]
    fn a_root_path_names_its_leaves_without_a_leading_dot() {
        let root = KeyPath::root();
        assert_eq!(
            root.leaf("contract_version").key.as_deref(),
            Some("contract_version")
        );
        assert_eq!(
            root.child("dialect").leaf("links").key.as_deref(),
            Some("dialect.links")
        );
        assert_eq!(root.leaf("contract_version").name, "contract_version");
    }

    #[test]
    fn a_named_declaration_extends_the_path() {
        let path = KeyPath::root().child("type").child_opt(Some("person"));
        assert_eq!(path.leaf("name").key.as_deref(), Some("type.person.name"));
        let property = path.child("property").child_opt(Some("full_name"));
        assert_eq!(
            property.leaf("kind").key.as_deref(),
            Some("type.person.property.full_name.kind")
        );
    }

    #[test]
    fn a_nameless_declaration_addresses_nothing_under_it() {
        let path = KeyPath::root().child("type").child_opt(None);
        assert!(path.leaf("name").key.is_none());
        assert!(
            path.child("property")
                .child_opt(Some("x"))
                .leaf("kind")
                .key
                .is_none()
        );
        assert_eq!(path.clone(), KeyPath::nameless());
        assert!(format!("{path:?}").contains("KeyPath"));
    }

    #[test]
    fn a_span_becomes_a_line_and_a_column() {
        let text = text_of("a = 1\nb = 2\n");
        let sink = Sink::new(&text, 1);
        let location = sink.location(10..11);
        let span = location.span.expect("a span");
        assert_eq!((span.start.line, span.start.column), (2, 5));
        assert_eq!(location.file, contract_file());
    }

    #[test]
    fn a_whole_file_diagnostic_carries_no_span() {
        let text = text_of("a = 1\n");
        let mut sink = Sink::new(&text, 1);
        let at = sink.whole_file();
        sink.report(
            KernelDiagnostic::ContractNoTypes,
            Report::new("no types".to_owned()),
            at,
        );
        let (diagnostics, provenance) = sink.finish();
        let location = diagnostics.as_slice()[0].location.as_ref().expect("a file");
        assert!(location.span.is_none());
        assert!(provenance.is_empty());
    }

    #[test]
    fn recording_nothing_records_nothing() {
        let text = text_of("a = 1\n");
        let mut sink = Sink::new(&text, 1);
        sink.record(None);
        sink.record(Some(Diagnostic::kernel(
            KernelDiagnostic::CompatNewerFormatAvailable,
            "newer",
        )));
        let (diagnostics, _) = sink.finish();
        assert_eq!(diagnostics.len(), 1);
    }

    #[test]
    fn a_report_carries_advice_only_when_it_was_given_some() {
        let text = text_of("a = 1\n");
        let mut sink = Sink::new(&text, 1);
        let at = sink.whole_file();
        let advice = "declare exactly one".to_owned();
        sink.report(
            KernelDiagnostic::ContractMissingCatchAll,
            Report::new("no catch-all".to_owned()).with_help(advice.clone()),
            at.clone(),
        );
        sink.report(
            KernelDiagnostic::ContractNoTypes,
            Report::new("no types".to_owned()),
            at,
        );
        let (diagnostics, _) = sink.finish();
        assert_eq!(diagnostics.as_slice()[0].help, Some(advice));
        assert!(diagnostics.as_slice()[1].help.is_none());
    }

    #[test]
    fn a_sweep_names_the_table_the_key_and_the_version() {
        let text = text_of("[dialect]\nlinks = \"wikilink\"\nflavour = \"plain\"\n");
        let root = root_of(&text);
        let dialect = document::get(root.get_ref(), "dialect").expect("declared");
        let mut sink = Sink::new(&text, 1);
        let table = sink.table(dialect, "dialect").expect("a table");
        sink.sweep(&section(table, "`[dialect]`"), &["links"]);
        let (diagnostics, _) = sink.finish();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics.as_slice()[0].message,
            "`[dialect]` declares `flavour`, which contract version 1 does not define"
        );
    }

    #[test]
    fn a_missing_key_names_the_table_and_the_version() {
        let text = text_of("[dialect]\n");
        let root = root_of(&text);
        let dialect = document::get(root.get_ref(), "dialect").expect("declared");
        let mut sink = Sink::new(&text, 1);
        let table = sink.table(dialect, "dialect").expect("a table");
        let dialect = section(table, "`[dialect]`");
        assert!(sink.required(&dialect, "links").is_none());
        let (diagnostics, _) = sink.finish();
        assert_eq!(
            diagnostics.as_slice()[0].message,
            "`[dialect]` does not declare `links`, which contract version 1 requires"
        );
    }

    #[test]
    fn a_wrong_type_names_what_was_required_and_what_was_found() {
        let text = text_of("links = 1\n");
        let root = root_of(&text);
        let mut sink = Sink::new(&text, 1);
        let links = document::get(root.get_ref(), "links").expect("declared");
        assert!(sink.table(links, "links").is_none());
        assert!(sink.array(links, "links").is_none());
        let (diagnostics, _) = sink.finish();
        assert_eq!(diagnostics.len(), 2);
        assert!(
            diagnostics.as_slice()[0]
                .message
                .contains("must be a TOML table")
        );
    }

    #[test]
    fn reading_a_leaf_records_where_it_is_written() {
        let text = text_of("links = \"wikilink\"\nkeep = true\n");
        let root = root_of(&text);
        let mut sink = Sink::new(&text, 1);
        let path = KeyPath::root();
        let links = document::get(root.get_ref(), "links").expect("declared");
        let keep = document::get(root.get_ref(), "keep").expect("declared");
        assert_eq!(sink.string(links, path.leaf("links")), Some("wikilink"));
        assert_eq!(sink.boolean(keep, path.leaf("keep")), Some(true));
        let (_, provenance) = sink.finish();
        assert_eq!(
            provenance.get("links").map(|e| e.source),
            Some(Source::Contract)
        );
        assert_eq!(provenance.len(), 2);
    }

    #[test]
    fn a_leaf_of_the_wrong_type_records_no_provenance() {
        let text = text_of("links = 1\nkeep = \"yes\"\n");
        let root = root_of(&text);
        let mut sink = Sink::new(&text, 1);
        let path = KeyPath::root();
        let links = document::get(root.get_ref(), "links").expect("declared");
        let keep = document::get(root.get_ref(), "keep").expect("declared");
        assert!(sink.string(links, path.leaf("links")).is_none());
        assert!(sink.boolean(keep, path.leaf("keep")).is_none());
        let (diagnostics, provenance) = sink.finish();
        assert_eq!(diagnostics.len(), 2);
        assert!(provenance.is_empty());
    }

    #[test]
    fn a_nameless_leaf_records_nothing_and_still_reads() {
        let text = text_of("links = \"wikilink\"\n");
        let root = root_of(&text);
        let mut sink = Sink::new(&text, 1);
        let links = document::get(root.get_ref(), "links").expect("declared");
        assert_eq!(
            sink.string(links, KeyPath::nameless().leaf("links")),
            Some("wikilink")
        );
        sink.defaulted(None);
        let (_, provenance) = sink.finish();
        assert!(provenance.is_empty());
    }

    #[test]
    fn an_omitted_flag_takes_the_declaring_versions_default() {
        let text = text_of("required = true\n");
        let root = root_of(&text);
        let mut sink = Sink::new(&text, 1);
        let table = section(root.get_ref(), "`[[type.property]]`");
        assert!(sink.optional_flag(&table, "required"));
        let (_, provenance) = sink.finish();
        assert_eq!(
            provenance.get("required").map(|e| e.source),
            Some(Source::Contract)
        );
    }

    #[test]
    fn an_absent_flag_is_attributed_to_the_contract_version_that_defines_it() {
        let text = text_of("name = \"x\"\n");
        let root = root_of(&text);
        let mut sink = Sink::new(&text, 1);
        let table = section(root.get_ref(), "`[[type.property]]`");
        assert!(!sink.optional_flag(&table, "required"));
        let (diagnostics, provenance) = sink.finish();
        assert!(diagnostics.is_empty());
        assert_eq!(
            provenance.get("required").map(|entry| entry.source),
            Some(Source::Default {
                contract_version: 1
            })
        );
    }

    #[test]
    fn a_flag_of_the_wrong_type_reads_as_false_and_is_reported() {
        let text = text_of("required = \"yes\"\n");
        let root = root_of(&text);
        let mut sink = Sink::new(&text, 1);
        let table = section(root.get_ref(), "`[[type.property]]`");
        assert!(!sink.optional_flag(&table, "required"));
        let (diagnostics, provenance) = sink.finish();
        assert_eq!(diagnostics.len(), 1);
        assert!(provenance.is_empty());
    }

    #[test]
    fn a_repeat_points_at_both_declarations() {
        let text = text_of("a = 1\nb = 2\n");
        let mut sink = Sink::new(&text, 1);
        sink.repeated(
            KernelDiagnostic::ContractDuplicateType,
            Repeat {
                message: "two types share the name `person`".to_owned(),
                at: 6..7,
                first: 0..1,
            },
        );
        let (diagnostics, _) = sink.finish();
        let diagnostic = &diagnostics.as_slice()[0];
        assert_eq!(diagnostic.related.len(), 1);
        assert_eq!(diagnostic.related[0].message, "first declared here");
        assert!(diagnostic.related[0].location.is_some());
    }

    #[test]
    fn a_name_is_read_before_its_path_exists() {
        let text = text_of("name = \"person\"\n");
        let root = root_of(&text);
        let mut sink = Sink::new(&text, 1);
        let entry = section(root.get_ref(), "`[[type]]`");
        let named = sink.name_of(&entry, "name").expect("a name");
        assert_eq!(named.text, "person");
        assert_eq!(named.span, 7..15);
        let (_, provenance) = sink.finish();
        assert!(provenance.is_empty(), "the caller records it, not the sink");
    }

    #[test]
    fn a_name_of_the_wrong_type_is_reported_and_yields_nothing() {
        let text = text_of("name = 4\n");
        let root = root_of(&text);
        let mut sink = Sink::new(&text, 1);
        let entry = section(root.get_ref(), "`[[type]]`");
        assert!(sink.name_of(&entry, "name").is_none());
        assert!(sink.name_of(&entry, "predicate").is_none());
        let (diagnostics, _) = sink.finish();
        assert_eq!(diagnostics.len(), 2);
    }

    #[test]
    fn a_section_reads_its_own_keys_and_names_its_own_leaves() {
        let text = text_of("links = \"wikilink\"\n");
        let root = root_of(&text);
        let table = section(root.get_ref(), "`[dialect]`");
        assert!(table.get("links").is_some());
        assert!(table.get("absent").is_none());
        assert_eq!(table.leaf("links").name, "links");
    }
}
