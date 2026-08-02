//! The installation record's body walk.
//!
//! The walk is **version-first**, exactly as the contract's is: the record's
//! `installation_version` is extracted before anything else, classified against
//! the supported range, and a version outside that range yields exactly one
//! compatibility diagnostic and stops. A record from a newer format is never
//! reported as a pile of misspellings.
//!
//! `installation_version` is mandatory from this, the first release that reads
//! the file. That is not symmetry for its own sake: unknown keys are fatal here,
//! so a version key could never be retrofitted — every already-installed build
//! would reject the very key announcing the new format.
//!
//! Inside the declared version, the walk collects **every** diagnostic rather
//! than stopping at the first, and records the provenance of every leaf it
//! accepts. An error at any point makes the record unusable; the diagnostics are
//! what say why.

use core::ops::{Range, RangeInclusive};

use toml::Spanned;
use toml::de::{DeInteger, DeTable, DeValue};

use super::{Actor, Installation, InstallationRecord, VaultEntry};
use crate::compat::{self, SUPPORTED_INSTALLATION_VERSIONS, VersionClass};
use crate::diagnostic::{Diagnostic, DiagnosticList, FileRef, KernelDiagnostic, Location, Related};
use crate::document::{self, TypeMismatch};
use crate::encoding::{self, EncodingFault, Text};
use crate::provenance::{Provenance, ProvenanceEntry, Source};

/// The key the version-first walk reads before anything else.
const VERSION_KEY: &str = "installation_version";

/// What the record owns, said once so every unknown key can say it.
const PARTITION_HELP: &str = "the installation record owns the vault registry and actor identity; \
                              every other setting is the committed vault contract's";

/// One table's legal key set at version 1, and how the record names it.
struct Section {
    /// How a diagnostic refers to this table in prose.
    label: &'static str,
    /// The dotted prefix this table's keys are reported under.
    prefix: &'static str,
    /// Every key version 1 defines here. Anything else is fatal.
    keys: &'static [&'static str],
}

const ROOT: Section = Section {
    label: "the installation record",
    prefix: "",
    keys: &["actor", "installation_version", "vault"],
};

const ACTOR: Section = Section {
    label: "`[actor]`",
    prefix: "actor.",
    keys: &["name"],
};

const VAULT: Section = Section {
    label: "a `[[vault]]` entry",
    prefix: "vault.",
    keys: &["name", "path"],
};

/// A table being walked, and the span to blame when a required key is absent.
struct Scope<'a, 'i> {
    section: &'static Section,
    table: &'a DeTable<'i>,
    header: Range<usize>,
}

impl<'a, 'i> Scope<'a, 'i> {
    /// Reports every key the declared version does not define here.
    fn sweep(&self, sink: &mut Sink<'_>, version: u32) {
        for unknown in document::unknown_keys(self.table, self.section.keys) {
            let at = sink.at(unknown.span);
            let message = format!(
                "`{}` is not a key {} defines at `{VERSION_KEY} = {version}`",
                unknown.key, self.section.label
            );
            sink.push(
                Diagnostic::kernel(KernelDiagnostic::InstallationUnknownKey, message)
                    .at(at)
                    .with_help(PARTITION_HELP),
            );
        }
    }

    /// The string a required key carries, reporting its absence or its type.
    fn string(&self, sink: &mut Sink<'_>, key: &str) -> Option<Written> {
        let value = self.required(sink, key)?;
        let span = value.span();
        match document::expect_string(value) {
            Ok(text) => Some(Written {
                value: text.to_owned(),
                span,
            }),
            Err(mismatch) => {
                wrong_type(sink, &mismatch, &self.label_of(key));
                None
            }
        }
    }

    /// The value a required key carries, reporting its absence.
    fn required(&self, sink: &mut Sink<'_>, key: &str) -> Option<&'a Spanned<DeValue<'i>>> {
        let declared = document::get(self.table, key);
        if declared.is_none() {
            let at = sink.at(self.header.clone());
            let message = format!(
                "{} does not declare `{key}`, which it requires",
                self.section.label
            );
            sink.push(Diagnostic::kernel(KernelDiagnostic::InstallationMissingKey, message).at(at));
        }
        declared
    }

    /// A key's dotted name, as diagnostics and provenance spell it.
    fn label_of(&self, key: &str) -> String {
        format!("{}{key}", self.section.prefix)
    }
}

/// A string value as the record writes it, with the bytes it occupies.
struct Written {
    value: String,
    span: Range<usize>,
}

/// The version the record declares, and where it is written.
struct Declared {
    version: Version,
    span: Range<usize>,
}

/// A declared version, whether or not a `u32` holds it.
///
/// The domain the record declares is every whole number 0 or above, and
/// classification is total over it, so a literal too large for a `u32` is
/// *above the supported range* rather than outside the domain. It travels as
/// the file's own bytes because a message may not restate it as a number it is
/// not.
enum Version {
    Held(u32),
    Beyond(String),
}

impl Version {
    /// The version as a message names it.
    fn found(&self) -> String {
        match self {
            Self::Held(version) => version.to_string(),
            Self::Beyond(literal) => literal.clone(),
        }
    }
}

/// A registry entry, and the span of the name that identifies it.
struct Registered {
    entry: VaultEntry,
    name_span: Range<usize>,
}

/// Everything a successful walk produced, before it is judged.
struct Parsed {
    version: u32,
    actor: Option<Actor>,
    vaults: Vec<VaultEntry>,
}

/// Diagnostics and provenance as a walk accumulates them.
struct Sink<'t> {
    text: &'t Text,
    diagnostics: DiagnosticList,
    provenance: Provenance,
}

impl<'t> Sink<'t> {
    fn new(text: &'t Text) -> Self {
        Self {
            text,
            diagnostics: DiagnosticList::new(),
            provenance: Provenance::new(),
        }
    }

    /// A byte range as a location, always under the unexpanded record path.
    fn at(&self, span: Range<usize>) -> Location {
        Location::in_file(FileRef::InstallationRecord, self.text.span(span))
    }

    fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    /// Records that a leaf was written in this record, at `span`.
    fn remember(&mut self, key: String, span: Range<usize>) {
        let location = self.at(span);
        self.provenance.insert(ProvenanceEntry::written(
            key,
            Source::Installation,
            location,
        ));
    }

    /// The record could not be used, with whatever was collected.
    fn refused(self) -> Installation {
        Installation::unusable(self.diagnostics.sorted())
    }

    /// The record loaded, unless something error-severity refused it.
    fn finish(self, parsed: Parsed) -> Installation {
        if self.diagnostics.counts().error > 0 {
            return self.refused();
        }
        let record = InstallationRecord {
            installation_version: parsed.version,
            actor: parsed.actor,
            vaults: parsed.vaults,
            provenance: self.provenance,
        };
        Installation::loaded(record, self.diagnostics.sorted())
    }
}

/// Reads a record's bytes against the versions this SDK supports.
pub(super) fn parse_bytes(bytes: &[u8]) -> Installation {
    parse_within(bytes, SUPPORTED_INSTALLATION_VERSIONS)
}

/// Reads a record's bytes against an explicit supported range.
///
/// The range is a parameter for the same reason [`compat::classify`]'s is: at
/// this milestone the real range holds one version, so the in-range-but-not-newest
/// classification is unreachable from any real record. Injecting a wider range
/// reaches it without fabricating an impossible asset. Every caller outside this
/// module's own tests passes the real constant.
fn parse_within(bytes: &[u8], supported: RangeInclusive<u32>) -> Installation {
    match encoding::inspect(bytes) {
        Ok(text) => parse_text(&text, supported),
        Err(fault) => Installation::unusable(vec![encoding_diagnostic(fault)]),
    }
}

fn parse_text(text: &Text, supported: RangeInclusive<u32>) -> Installation {
    match document::parse(text.as_str()) {
        Ok(document) => walk(text, &document, supported),
        Err(error) => Installation::unusable(vec![malformed(text, &error)]),
    }
}

/// The version-first walk: extract, classify, and only then validate.
fn walk(
    text: &Text,
    document: &Spanned<DeTable<'_>>,
    supported: RangeInclusive<u32>,
) -> Installation {
    let mut sink = Sink::new(text);
    let Some(declared) = declared_version(&mut sink, text, document.get_ref()) else {
        return sink.refused();
    };
    let Some(version) = usable_version(&mut sink, &declared, supported) else {
        return sink.refused();
    };
    let root = Scope {
        section: &ROOT,
        table: document.get_ref(),
        header: document.span(),
    };
    let parsed = body(&mut sink, &root, version);
    sink.finish(parsed)
}

/// The version the record declares, or nothing when it declares none usably.
fn declared_version(sink: &mut Sink<'_>, text: &Text, root: &DeTable<'_>) -> Option<Declared> {
    let Some(value) = document::get(root, VERSION_KEY) else {
        sink.push(version_missing());
        return None;
    };
    let span = value.span();
    let integer = match document::expect_integer(value) {
        Ok(integer) => integer,
        Err(mismatch) => return version_invalid(sink, mismatch.message(VERSION_KEY), span),
    };
    match version_from(integer) {
        Some(version) => {
            sink.remember(VERSION_KEY.to_owned(), span.clone());
            Some(Declared {
                version: Version::Held(version),
                span,
            })
        }
        None if negative(integer) => version_invalid(sink, out_of_domain(integer), span),
        None => Some(Declared {
            version: Version::Beyond(text.as_str()[span.clone()].to_owned()),
            span,
        }),
    }
}

/// Whether the declared integer is below the domain rather than above it.
///
/// [`DeInteger::as_str`] keeps the sign, so this is the whole question.
fn negative(integer: &DeInteger<'_>) -> bool {
    integer.as_str().starts_with('-')
}

/// The declared version as a `u32`, or nothing when it is not one.
///
/// [`DeInteger::as_str`] keeps a sign, drops digit separators, and strips a
/// radix prefix that [`DeInteger::radix`] reports separately, so a negative or
/// over-large literal is rejected here without this module re-deriving TOML's
/// integer grammar.
fn version_from(integer: &DeInteger<'_>) -> Option<u32> {
    u32::from_str_radix(integer.as_str(), integer.radix()).ok()
}

fn out_of_domain(integer: &DeInteger<'_>) -> String {
    format!(
        "`{VERSION_KEY}` must be a whole number 0 or above, but is `{}`",
        integer.as_str()
    )
}

fn version_missing() -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::InstallationVersionMissing,
        format!("the installation record declares no `{VERSION_KEY}`, which is mandatory"),
    )
    .at(Location::whole_file(FileRef::InstallationRecord))
    .with_help(format!(
        "add `{VERSION_KEY} = {}` as the record's first line",
        SUPPORTED_INSTALLATION_VERSIONS.end()
    ))
}

fn version_invalid(sink: &mut Sink<'_>, message: String, span: Range<usize>) -> Option<Declared> {
    let at = sink.at(span);
    sink.push(Diagnostic::kernel(KernelDiagnostic::InstallationVersionInvalid, message).at(at));
    None
}

/// The version the body is walked against, or nothing when the record is not
/// read any further, noting why either way.
fn usable_version(
    sink: &mut Sink<'_>,
    declared: &Declared,
    supported: RangeInclusive<u32>,
) -> Option<u32> {
    let class = match declared.version {
        Version::Held(version) => compat::classify(version, supported.clone()),
        // Above every range this SDK can support, whatever the range is.
        Version::Beyond(_) => VersionClass::TooNew,
    };
    if let Some(note) = compat_note(class, &declared.version.found(), &supported) {
        let at = sink.at(declared.span.clone());
        sink.push(note.at(at));
    }
    match declared.version {
        Version::Held(version) if class.is_usable() => Some(version),
        _ => None,
    }
}

/// The compatibility diagnostic a classification calls for, if it calls for one.
fn compat_note(
    class: VersionClass,
    found: &str,
    supported: &RangeInclusive<u32>,
) -> Option<Diagnostic> {
    match class {
        VersionClass::BelowFloor => Some(below_floor(found, supported)),
        VersionClass::TooNew => Some(too_new(found, supported)),
        VersionClass::Supported => Some(newer_available(found, supported)),
        VersionClass::Current => None,
    }
}

fn below_floor(found: &str, supported: &RangeInclusive<u32>) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::CompatInstallationBelowSupportedFloor,
        format!(
            "the installation record declares version {found}, below the supported range {}",
            range_text(supported)
        ),
    )
    .with_help(
        "migration arrives in a later release; until it does, pin an older build with the \
         installer's `DOGTAG_VERSION`",
    )
}

fn too_new(found: &str, supported: &RangeInclusive<u32>) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::CompatInstallationTooNew,
        format!(
            "the installation record declares version {found}, above the supported range {}",
            range_text(supported)
        ),
    )
    .with_help("upgrade dogtag to a release that reads this record's format")
}

fn newer_available(found: &str, supported: &RangeInclusive<u32>) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::CompatNewerInstallationFormatAvailable,
        format!(
            "the installation record declares version {found}; this build reads up to version {}",
            supported.end()
        ),
    )
}

fn range_text(supported: &RangeInclusive<u32>) -> String {
    format!("{}..={}", supported.start(), supported.end())
}

/// Validates the body against the declared version, collecting everything.
fn body(sink: &mut Sink<'_>, root: &Scope<'_, '_>, version: u32) -> Parsed {
    root.sweep(sink, version);
    Parsed {
        version,
        actor: actor(sink, root.table, version),
        vaults: vaults(sink, root.table, version),
    }
}

/// The declared actor, when `[actor]` is present and usable.
fn actor(sink: &mut Sink<'_>, root: &DeTable<'_>, version: u32) -> Option<Actor> {
    let value = document::get(root, "actor")?;
    let table = match document::expect_table(value) {
        Ok(table) => table,
        Err(mismatch) => {
            wrong_type(sink, &mismatch, "actor");
            return None;
        }
    };
    let scope = Scope {
        section: &ACTOR,
        table,
        header: value.span(),
    };
    scope.sweep(sink, version);
    let name = scope.string(sink, "name")?;
    sink.remember(scope.label_of("name"), name.span);
    Some(Actor { name: name.value })
}

/// Every registered vault, with duplicate names refused.
fn vaults(sink: &mut Sink<'_>, root: &DeTable<'_>, version: u32) -> Vec<VaultEntry> {
    let Some(value) = document::get(root, "vault") else {
        return Vec::new();
    };
    let array = match document::expect_array(value) {
        Ok(array) => array,
        Err(mismatch) => {
            wrong_type(sink, &mismatch, "vault");
            return Vec::new();
        }
    };
    let registered: Vec<Registered> = array
        .iter()
        .filter_map(|element| vault_entry(sink, element, version))
        .collect();
    reject_duplicates(sink, &registered);
    registered.into_iter().map(|held| held.entry).collect()
}

/// One `[[vault]]` entry, when its name and its path are both usable.
fn vault_entry(
    sink: &mut Sink<'_>,
    element: &Spanned<DeValue<'_>>,
    version: u32,
) -> Option<Registered> {
    let table = match document::expect_table(element) {
        Ok(table) => table,
        Err(mismatch) => {
            wrong_type(sink, &mismatch, "vault");
            return None;
        }
    };
    let scope = Scope {
        section: &VAULT,
        table,
        header: element.span(),
    };
    scope.sweep(sink, version);
    // Both keys are looked up before either answer is unwrapped, so an entry
    // missing both reports both rather than only the first.
    let (name, path) = (scope.string(sink, "name"), scope.string(sink, "path"));
    accept(sink, name?, path?)
}

/// A named path, once both halves are known to be usable.
fn accept(sink: &mut Sink<'_>, name: Written, path: Written) -> Option<Registered> {
    // Both checks run before either verdict is read, for the same reason.
    let named = check_name(sink, &name);
    let located = check_path(sink, &path);
    if !(named && located) {
        return None;
    }
    sink.remember(format!("vault.{}.name", name.value), name.span.clone());
    sink.remember(format!("vault.{}.path", name.value), path.span);
    Some(Registered {
        entry: VaultEntry {
            name: name.value,
            path: path.value.into(),
        },
        name_span: name.span,
    })
}

/// Refuses a second entry under a name an earlier entry already took.
///
/// This is what forecloses shadowing a registered vault by appending an entry,
/// rather than merely settling which of two entries wins.
fn reject_duplicates(sink: &mut Sink<'_>, registered: &[Registered]) {
    let mut seen: Vec<(&str, Range<usize>)> = Vec::new();
    for held in registered {
        let name = held.entry.name();
        match seen.iter().find(|(taken, _)| *taken == name) {
            Some((_, first)) => {
                let here = sink.at(held.name_span.clone());
                let there = sink.at(first.clone());
                sink.push(duplicate_name(name, here, there));
            }
            None => seen.push((name, held.name_span.clone())),
        }
    }
}

fn duplicate_name(name: &str, here: Location, there: Location) -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::InstallationDuplicateVaultName,
        format!("two registry entries are named `{name}`"),
    )
    .at(here)
    .with_related(Related::new("first registered here").at(there))
    .with_help("registry names are unique, so an appended entry can never shadow a registered one")
}

/// Whether a registry name is usable, reporting the rejection when it is not.
fn check_name(sink: &mut Sink<'_>, name: &Written) -> bool {
    let Some(fault) = name_fault(&name.value) else {
        return true;
    };
    let at = sink.at(name.span.clone());
    sink.push(
        Diagnostic::kernel(
            KernelDiagnostic::InstallationVaultNameInvalid,
            format!("the registry name `{}` {fault}", name.value),
        )
        .at(at)
        .with_help(
            "a registry name is kebab-case and holds no path separator, because an argument \
             holding one is always a path and never a name",
        ),
    );
    false
}

/// Why a registry name is unusable, or nothing when it is fine.
fn name_fault(name: &str) -> Option<&'static str> {
    if name.contains('/') || name.contains('\\') {
        Some("holds a path separator")
    } else if is_kebab(name) {
        None
    } else {
        Some("is not kebab-case")
    }
}

/// Whether `name` is hyphen-separated words of ASCII lowercase and digits.
fn is_kebab(name: &str) -> bool {
    name.split('-').all(is_kebab_word)
}

fn is_kebab_word(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

/// Whether a registry path is usable, reporting the rejection when it is not.
fn check_path(sink: &mut Sink<'_>, path: &Written) -> bool {
    let Some(fault) = path_fault(&path.value) else {
        return true;
    };
    let at = sink.at(path.span.clone());
    sink.push(
        Diagnostic::kernel(
            KernelDiagnostic::InstallationVaultPathNotAbsolute,
            format!("the registry path `{}` {fault}", path.value),
        )
        .at(at)
        .with_help(
            "a registry path is absolute and literal, so that an entry cannot resolve \
             differently from different directories",
        ),
    );
    false
}

/// Why a registry path is unusable, or nothing when it is fine.
///
/// A leading `~` and a `$` are rejections rather than something to expand: a
/// path that depends on the environment reintroduces exactly the ambiguity an
/// absolute registry path exists to remove.
fn path_fault(path: &str) -> Option<&'static str> {
    if path.starts_with('~') {
        Some("begins with `~`, which is never expanded")
    } else if path.contains('$') {
        Some("names an environment variable, which is never expanded")
    } else if std::path::Path::new(path).is_absolute() {
        None
    } else {
        Some("is not absolute")
    }
}

fn wrong_type(sink: &mut Sink<'_>, mismatch: &TypeMismatch, label: &str) {
    let at = sink.at(mismatch.span.clone());
    sink.push(
        Diagnostic::kernel(
            KernelDiagnostic::InstallationValueWrongType,
            mismatch.message(label),
        )
        .at(at),
    );
}

fn encoding_diagnostic(fault: EncodingFault) -> Diagnostic {
    Diagnostic::kernel(encoding_kind(fault), fault.describe())
        .at(Location::whole_file(FileRef::InstallationRecord))
}

fn encoding_kind(fault: EncodingFault) -> KernelDiagnostic {
    match fault {
        EncodingFault::InvalidUtf8 { .. } => KernelDiagnostic::InstallationInvalidUtf8,
        EncodingFault::ByteOrderMark => KernelDiagnostic::InstallationByteOrderMark,
        EncodingFault::CarriageReturn { .. } => {
            KernelDiagnostic::InstallationCarriageReturnLineEnding
        }
    }
}

/// The diagnostic for a syntax fault, carrying the parser's own span.
///
/// A parser error with no span at all falls back to the first byte rather than
/// to no location: the record is short and the reader is looking at the top of
/// it either way.
fn malformed(text: &Text, error: &toml::de::Error) -> Diagnostic {
    let span = text.span(error.span().unwrap_or(0..0));
    let at = Location::in_file(FileRef::InstallationRecord, span);
    Diagnostic::kernel(
        KernelDiagnostic::InstallationMalformedToml,
        format!(
            "the installation record is not well-formed TOML: {}",
            error.message()
        ),
    )
    .at(at)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::render_plain;
    use crate::installation::parse_installation;

    fn ids(installation: &Installation) -> Vec<&str> {
        installation
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    fn refusal(source: &str) -> Installation {
        let installation = parse_installation(source);
        assert!(installation.record().is_none(), "expected a refusal");
        installation
    }

    fn only(source: &str) -> String {
        let installation = refusal(source);
        assert_eq!(installation.diagnostics().len(), 1);
        installation.diagnostics()[0].id.as_str().to_owned()
    }

    fn versioned(body: &str) -> String {
        format!("installation_version = 1\n{body}")
    }

    fn registry(entry: &str) -> String {
        versioned(&format!("\n[[vault]]\n{entry}"))
    }

    fn help_of(installation: &Installation) -> String {
        installation.diagnostics()[0]
            .help
            .clone()
            .expect("a help line")
    }

    #[test]
    fn the_records_worked_example_loads_with_nothing_to_say() {
        let source = versioned(concat!(
            "\n[actor]\nname = \"A Maintainer\"\n",
            "\n[[vault]]\nname = \"work\"\npath = \"/data/vaults/work\"\n",
        ));
        let installation = parse_installation(&source);
        assert!(installation.diagnostics().is_empty());
        assert_eq!(installation.record().expect("loaded").vaults().len(), 1);
    }

    #[test]
    fn a_record_declaring_only_a_version_loads() {
        let installation = parse_installation("installation_version = 1\n");
        assert!(installation.diagnostics().is_empty());
        let record = installation.record().expect("loaded");
        assert!(record.actor().is_none());
        assert!(record.vaults().is_empty());
    }

    #[test]
    fn provenance_names_every_leaf_the_record_writes() {
        let source = versioned(concat!(
            "\n[actor]\nname = \"A Maintainer\"\n",
            "\n[[vault]]\nname = \"work\"\npath = \"/data/vaults/work\"\n",
        ));
        let installation = parse_installation(&source);
        let record = installation.record().expect("loaded");
        let keys: Vec<&str> = record
            .provenance()
            .entries()
            .map(|e| e.key.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "actor.name",
                "installation_version",
                "vault.work.name",
                "vault.work.path"
            ]
        );
    }

    #[test]
    fn every_provenance_entry_points_into_the_unexpanded_record() {
        let source = registry("name = \"work\"\npath = \"/data/vaults/work\"\n");
        let installation = parse_installation(&source);
        let record = installation.record().expect("loaded");
        let entry = record
            .provenance()
            .get("vault.work.path")
            .expect("recorded");
        assert_eq!(entry.source, Source::Installation);
        let location = entry.location.as_ref().expect("a location");
        assert_eq!(location.file, FileRef::InstallationRecord);
    }

    #[test]
    fn a_record_declaring_no_version_is_refused() {
        assert_eq!(only(""), "installation.version-missing");
    }

    #[test]
    fn the_missing_version_diagnostic_teaches_the_line_to_add() {
        let installation = refusal("");
        assert!(help_of(&installation).contains("installation_version = 1"));
    }

    #[test]
    fn a_version_that_is_not_an_integer_is_refused() {
        assert_eq!(
            only("installation_version = \"1\"\n"),
            "installation.version-invalid"
        );
    }

    #[test]
    fn a_negative_version_is_refused() {
        let installation = refusal("installation_version = -1\n");
        assert_eq!(ids(&installation), ["installation.version-invalid"]);
        assert!(installation.diagnostics()[0].message.contains("`-1`"));
    }

    #[test]
    fn a_version_beyond_a_u32_is_classified_rather_than_refused_as_a_non_version() {
        let installation = refusal("installation_version = 4294967296\n");
        assert_eq!(ids(&installation), ["compat.installation-too-new"]);
        assert!(
            installation.diagnostics()[0]
                .message
                .contains("version 4294967296")
        );
    }

    #[test]
    fn a_version_written_in_another_radix_classifies_by_its_value() {
        let installation = parse_installation("installation_version = 0x1\n");
        assert!(installation.diagnostics().is_empty());
        assert_eq!(
            installation
                .record()
                .expect("loaded")
                .installation_version(),
            1
        );
    }

    #[test]
    fn a_version_below_the_floor_refuses_under_its_own_identifier() {
        assert_eq!(
            only("installation_version = 0\n"),
            "compat.installation-below-supported-floor"
        );
    }

    #[test]
    fn the_below_floor_diagnostic_names_the_interim_recourse() {
        let installation = refusal("installation_version = 0\n");
        assert!(help_of(&installation).contains("DOGTAG_VERSION"));
        assert!(installation.diagnostics()[0].message.contains("1..=1"));
    }

    #[test]
    fn a_version_above_the_range_refuses_under_its_own_identifier() {
        assert_eq!(
            only("installation_version = 2\n"),
            "compat.installation-too-new"
        );
    }

    #[test]
    fn a_supported_version_below_the_newest_loads_with_an_info() {
        let installation = parse_within(b"installation_version = 3\n", 2..=4);
        assert_eq!(
            ids(&installation),
            ["compat.newer-installation-format-available"]
        );
        assert_eq!(
            installation
                .record()
                .expect("loaded")
                .installation_version(),
            3
        );
    }

    #[test]
    fn the_newest_supported_version_says_nothing() {
        let installation = parse_within(b"installation_version = 4\n", 2..=4);
        assert!(installation.diagnostics().is_empty());
    }

    #[test]
    fn the_version_is_classified_before_the_body_is_looked_at() {
        let source = concat!(
            "installation_version = 2\n",
            "\n[dialect]\nlinks = \"wikilink\"\n",
            "\n[[vault]]\nname = \"Not Kebab\"\npath = \"relative\"\n",
        );
        assert_eq!(only(source), "compat.installation-too-new");
    }

    #[test]
    fn a_byte_order_mark_is_refused() {
        assert_eq!(
            only("\u{feff}installation_version = 1\n"),
            "installation.byte-order-mark"
        );
    }

    #[test]
    fn carriage_return_line_endings_are_refused() {
        assert_eq!(
            only("installation_version = 1\r\n"),
            "installation.carriage-return-line-ending"
        );
    }

    #[test]
    fn malformed_toml_is_refused_with_the_parsers_own_words() {
        let installation = refusal("installation_version = =\n");
        assert_eq!(ids(&installation), ["installation.malformed-toml"]);
        assert!(installation.diagnostics()[0].location.is_some());
    }

    #[test]
    fn a_contract_owned_type_declaration_is_an_unknown_key() {
        let source = versioned("\n[[type]]\nname = \"person\"\n");
        assert_eq!(only(&source), "installation.unknown-key");
    }

    #[test]
    fn a_contract_owned_dialect_table_is_an_unknown_key() {
        let source = versioned("\n[dialect]\nlinks = \"wikilink\"\n");
        assert_eq!(only(&source), "installation.unknown-key");
    }

    #[test]
    fn a_contract_owned_lifecycle_table_is_an_unknown_key() {
        let source = versioned("\n[lifecycle]\naxis = \"status\"\n");
        assert_eq!(only(&source), "installation.unknown-key");
    }

    #[test]
    fn an_unknown_key_says_what_the_record_actually_owns() {
        let source = versioned("\n[lifecycle]\naxis = \"status\"\n");
        let installation = refusal(&source);
        assert!(help_of(&installation).contains("vault registry and actor identity"));
        assert!(
            installation.diagnostics()[0]
                .message
                .contains("`lifecycle`")
        );
    }

    #[test]
    fn an_unknown_key_inside_the_actor_table_is_fatal() {
        let source = versioned("\n[actor]\nname = \"A\"\nemail = \"a@example.invalid\"\n");
        let installation = refusal(&source);
        assert_eq!(ids(&installation), ["installation.unknown-key"]);
        assert!(installation.diagnostics()[0].message.contains("`[actor]`"));
    }

    #[test]
    fn an_unknown_key_inside_a_registry_entry_is_fatal() {
        let source = registry("name = \"work\"\npath = \"/vaults/work\"\ndefault = true\n");
        let installation = refusal(&source);
        assert_eq!(ids(&installation), ["installation.unknown-key"]);
        assert!(
            installation.diagnostics()[0]
                .message
                .contains("`[[vault]]`")
        );
    }

    #[test]
    fn an_actor_table_must_declare_a_name() {
        let source = versioned("\n[actor]\n");
        assert_eq!(only(&source), "installation.missing-key");
    }

    #[test]
    fn an_actor_that_is_not_a_table_is_the_wrong_type() {
        assert_eq!(
            only(&versioned("actor = 3\n")),
            "installation.value-wrong-type"
        );
    }

    #[test]
    fn an_actor_name_that_is_not_a_string_is_the_wrong_type() {
        let source = versioned("\n[actor]\nname = 3\n");
        let installation = refusal(&source);
        assert_eq!(ids(&installation), ["installation.value-wrong-type"]);
        assert!(
            installation.diagnostics()[0]
                .message
                .contains("`actor.name`")
        );
    }

    #[test]
    fn a_registry_that_is_not_an_array_is_the_wrong_type() {
        assert_eq!(
            only(&versioned("vault = 3\n")),
            "installation.value-wrong-type"
        );
    }

    #[test]
    fn a_registry_entry_that_is_not_a_table_is_the_wrong_type() {
        assert_eq!(
            only(&versioned("vault = [3]\n")),
            "installation.value-wrong-type"
        );
    }

    #[test]
    fn a_registry_entry_declares_both_a_name_and_a_path() {
        let installation = refusal(&registry("\n"));
        assert_eq!(
            ids(&installation),
            ["installation.missing-key", "installation.missing-key"]
        );
    }

    #[test]
    fn a_registry_path_that_is_not_a_string_is_the_wrong_type() {
        let source = registry("name = \"work\"\npath = 3\n");
        let installation = refusal(&source);
        assert!(
            installation.diagnostics()[0]
                .message
                .contains("`vault.path`")
        );
    }

    #[test]
    fn duplicate_registry_names_are_a_load_error() {
        let source = registry(concat!(
            "name = \"work\"\npath = \"/vaults/one\"\n",
            "\n[[vault]]\nname = \"work\"\npath = \"/vaults/two\"\n",
        ));
        assert_eq!(only(&source), "installation.duplicate-vault-name");
    }

    #[test]
    fn a_duplicate_name_cites_the_entry_that_took_it_first() {
        let source = registry(concat!(
            "name = \"work\"\npath = \"/vaults/one\"\n",
            "\n[[vault]]\nname = \"work\"\npath = \"/vaults/two\"\n",
        ));
        let installation = refusal(&source);
        let related = &installation.diagnostics()[0].related;
        assert_eq!(related.len(), 1);
        assert!(related[0].location.is_some());
    }

    #[test]
    fn a_registry_name_must_be_kebab_case() {
        let source = registry("name = \"Work\"\npath = \"/vaults/work\"\n");
        let installation = refusal(&source);
        assert_eq!(ids(&installation), ["installation.vault-name-invalid"]);
        assert!(installation.diagnostics()[0].message.contains("kebab-case"));
    }

    #[test]
    fn a_registry_name_may_not_hold_a_path_separator() {
        // TOML literal strings, so the backslash reaches the walk as a backslash.
        for name in ["work/nested", "work\\nested"] {
            let source = registry(&format!("name = '{name}'\npath = \"/vaults/work\"\n"));
            let installation = refusal(&source);
            assert!(installation.diagnostics()[0].message.contains("separator"));
        }
    }

    #[test]
    fn kebab_names_may_carry_digits_and_hyphens() {
        let source = registry("name = \"work-2\"\npath = \"/vaults/work\"\n");
        assert!(parse_installation(&source).record().is_some());
    }

    #[test]
    fn a_registry_name_may_not_hold_an_empty_word() {
        for name in ["", "-work", "work-", "work--2"] {
            let source = registry(&format!("name = \"{name}\"\npath = \"/vaults/work\"\n"));
            let installation = refusal(&source);
            assert_eq!(ids(&installation), ["installation.vault-name-invalid"]);
        }
    }

    #[test]
    fn a_relative_registry_path_is_refused() {
        let source = registry("name = \"work\"\npath = \"vaults/work\"\n");
        let installation = refusal(&source);
        assert_eq!(ids(&installation), ["installation.vault-path-not-absolute"]);
        assert!(
            installation.diagnostics()[0]
                .message
                .contains("is not absolute")
        );
    }

    #[test]
    fn a_tilde_path_is_refused_rather_than_expanded() {
        let source = registry("name = \"work\"\npath = \"~/vaults/work\"\n");
        let installation = refusal(&source);
        assert_eq!(ids(&installation), ["installation.vault-path-not-absolute"]);
        assert!(
            installation.diagnostics()[0]
                .message
                .contains("never expanded")
        );
    }

    #[test]
    fn an_environment_variable_in_a_path_is_refused_rather_than_expanded() {
        let source = registry("name = \"work\"\npath = \"/home/$USER/vaults\"\n");
        let installation = refusal(&source);
        assert_eq!(ids(&installation), ["installation.vault-path-not-absolute"]);
        assert!(
            installation.diagnostics()[0]
                .message
                .contains("environment")
        );
    }

    #[test]
    fn one_entry_reports_a_bad_name_and_a_bad_path_together() {
        let source = registry("name = \"Work\"\npath = \"vaults/work\"\n");
        let installation = refusal(&source);
        assert_eq!(
            ids(&installation),
            [
                "installation.vault-name-invalid",
                "installation.vault-path-not-absolute"
            ]
        );
    }

    #[test]
    fn a_walk_collects_every_fault_rather_than_stopping_at_the_first() {
        let source = versioned(concat!(
            "surprise = true\n",
            "\n[actor]\n",
            "\n[[vault]]\nname = \"Work\"\npath = \"/vaults/work\"\n",
        ));
        let installation = refusal(&source);
        assert_eq!(installation.diagnostics().len(), 3);
    }

    #[test]
    fn every_diagnostic_points_at_the_unexpanded_record_path() {
        let source = registry("name = \"Work\"\npath = \"vaults/work\"\n");
        let rendered = render_plain(refusal(&source).diagnostics());
        assert_eq!(
            rendered.matches(FileRef::INSTALLATION_RECORD_PATH).count(),
            2
        );
    }

    #[test]
    fn a_registry_entry_missing_its_name_is_blamed_on_its_header() {
        let source = registry("path = \"/vaults/work\"\n");
        let installation = refusal(&source);
        let rendered = render_plain(installation.diagnostics());
        assert!(rendered.contains("installation.toml:3:1"));
    }
}
