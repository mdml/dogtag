//! The committed vault contract, `.dogtag/contract.toml`.
//!
//! One file carries a corpus's whole declaration — its types, properties,
//! relationships, capabilities, lifecycle, tag vocabulary, dialect, and the
//! format version they are written in — and its presence is also the vault-root
//! sentinel. Reading
//! it is what lets the kernel enforce a corpus's shape without knowing a single
//! one of its vocabulary words.
//!
//! # What reading it guarantees
//!
//! - **Two passes.** `contract_version` is extracted first and classified
//!   before any structural check runs. A contract outside the supported range
//!   yields exactly *one* compatibility diagnostic and no structural
//!   complaints: the refusal is the version's, not the parser's.
//! - **Unknown keys are fatal, everywhere, at every nesting level**, and key
//!   legality is scoped to the version the contract *declares* rather than to
//!   what this build happens to know. A typo'd `requred` silently demoting a
//!   required property is this format's worst failure mode.
//! - **Every diagnostic is collected.** A contract with three separate faults
//!   reports three. Nothing stops at the first.
//! - **Provenance per leaf value.** Every resolved value records whether it was
//!   written in the contract or supplied by the declared version's format
//!   default — and a default is attributed to *the contract version that
//!   defines it*, never to the SDK version, so an unchanged vault cannot
//!   acquire new semantics by upgrading the tool.
//!
//! # Reading one
//!
//! ```
//! use dogtag::contract::parse_contract;
//!
//! let load = parse_contract(concat!(
//!     "contract_version = 2\n",
//!     "\n[dialect]\nlinks = \"wikilink\"\n",
//!     "\n[lifecycle]\nnone = true\n",
//!     "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
//! ));
//! let contract = load.contract.expect("a conforming contract");
//! assert!(load.diagnostics.is_empty());
//! assert_eq!(contract.catch_all().map(|declared| declared.name()), Some("capture"));
//! ```

mod declarations;
mod fields;
mod kinds;
mod lifecycle;
mod model;
mod parse;
mod schema;
mod sink;
mod tags;
mod validate;
mod vocabulary;

use std::fs;
use std::io;
use std::path::Path;

use crate::compat::{self, SUPPORTED_CONTRACT_VERSIONS, VersionClass};
use crate::diagnostic::{Diagnostic, KernelDiagnostic, Location};
use crate::document;
use crate::encoding::{self, EncodingFault, Text};

pub use kinds::{FieldDecl, FieldKind, PropertyKind, ScalarKind};
pub use model::{
    Capability, Contract, Dialect, FlagDecl, LifecycleDecl, LinkDialect, Ordinary, PropertyDecl,
    RelationshipDecl, TypeDecl,
};
pub use vocabulary::{NamespaceMembership, TagNamespaceDecl, TagsDecl};

use sink::{Sink, contract_file};

/// Where the committed contract lives, relative to the vault root.
///
/// Diagnostics and provenance always name this path, whatever path the bytes
/// were actually read from, so the same fault in the same corpus renders
/// identically on every machine.
pub const CONTRACT_PATH: &str = ".dogtag/contract.toml";

/// Why a contract did not resolve.
///
/// Semantic operations take a resolved [`Contract`] and cannot be reached
/// without one, so this is what a surface reports in place of the answers it
/// would otherwise have given.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnresolvedReason {
    /// There is no file at the path.
    Missing,
    /// The file exists and could not be read.
    Unreadable,
    /// The bytes are not the encoding the format requires.
    Encoding,
    /// The bytes are not well-formed TOML.
    Malformed,
    /// The declared version is outside the supported range.
    VersionUnusable(VersionClass),
    /// The contract parsed and does not satisfy the format's rules.
    Invalid,
}

impl UnresolvedReason {
    /// Why a contract-dependent answer was not evaluated, as a reader sees it.
    pub fn describe(&self) -> String {
        match self {
            Self::Missing => "the vault holds no contract file".to_owned(),
            Self::Unreadable => "the contract file could not be read".to_owned(),
            Self::Encoding => {
                "the contract file's bytes are not the encoding the format requires".to_owned()
            }
            Self::Malformed => "the contract file is not well-formed TOML".to_owned(),
            Self::VersionUnusable(class) => unusable(*class),
            Self::Invalid => "the contract does not satisfy the format's validity rules".to_owned(),
        }
    }
}

fn unusable(class: VersionClass) -> String {
    let (start, end) = (
        SUPPORTED_CONTRACT_VERSIONS.start(),
        SUPPORTED_CONTRACT_VERSIONS.end(),
    );
    let side = match class {
        VersionClass::BelowFloor => "below",
        VersionClass::TooNew => "above",
        VersionClass::Supported | VersionClass::Current => "outside",
    };
    format!("the contract declares a version {side} the supported range {start}..={end}")
}

/// A contract that did not resolve, and why.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContractUnresolved {
    /// Why it did not resolve.
    pub reason: UnresolvedReason,
    /// The version it declared, when it got far enough to declare one.
    pub version: Option<u32>,
}

/// The outcome of reading one contract.
///
/// The diagnostics stand whether or not the contract resolved: a contract that
/// loads may still carry an `info`, and one that does not still says exactly
/// why, in the deterministic total order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractLoad {
    /// The resolved contract, or why there is none.
    pub contract: Result<Contract, ContractUnresolved>,
    /// Everything reading it reported, in the deterministic total order.
    pub diagnostics: Vec<Diagnostic>,
}

/// Reads the contract at an explicit path.
///
/// This is a pure function of its argument: it consults no environment
/// variable, no current directory, and no process-global state. The path it is
/// given never reaches a diagnostic — those name [`CONTRACT_PATH`].
pub fn load_contract(path: &Path) -> ContractLoad {
    match fs::read(path) {
        Ok(bytes) => from_bytes(&bytes),
        Err(error) => unreadable(&error),
    }
}

/// Reads a contract from text already in memory.
///
/// The same pipeline as [`load_contract`], minus the read. Invalid UTF-8 cannot
/// arise here because a `&str` is already valid, but a byte order mark and a
/// carriage return still can, and are still refused.
pub fn parse_contract(text: &str) -> ContractLoad {
    from_bytes(text.as_bytes())
}

fn from_bytes(bytes: &[u8]) -> ContractLoad {
    match encoding::inspect(bytes) {
        Ok(text) => from_text(&text),
        Err(fault) => refuse(encoding_fault(fault), UnresolvedReason::Encoding),
    }
}

fn from_text(text: &Text) -> ContractLoad {
    match document::parse(text.as_str()) {
        Ok(document) => resolve(text, document.get_ref()),
        Err(error) => refuse(malformed(text, &error), UnresolvedReason::Malformed),
    }
}

/// The two passes: the version, its classification, then the body.
///
/// The schema lookup is what gates the second pass, rather than the
/// classification alone: reading a version *is* carrying that version's key set
/// and default table, so a build that classified a version as readable and then
/// had no rules for it would resolve it against some other version's — the
/// lying provenance the vault-contract record names as worse than plain
/// inheritance. The classification still decides what a reader is told, because
/// below the floor and above the ceiling call for different actions.
fn resolve(text: &Text, root: &toml::de::DeTable<'_>) -> ContractLoad {
    let declared = match parse::version(text, root) {
        parse::DeclaredVersion::Found(declared) => declared,
        parse::DeclaredVersion::Beyond(literal) => {
            // Above every range this SDK can declare, whatever the range is.
            return unusable_version(VersionClass::TooNew, &literal, None);
        }
        parse::DeclaredVersion::Refused(diagnostic) => {
            return refuse(diagnostic, UnresolvedReason::Invalid);
        }
    };
    let class = compat::classify(declared.version, SUPPORTED_CONTRACT_VERSIONS);
    let Some(schema) = schema::of(declared.version) else {
        let found = declared.version;
        return unusable_version(class, &found.to_string(), Some(found));
    };
    let mut sink = Sink::new(text, schema);
    sink.record(newer_format_available(class, declared.version));
    sink.written(Some("contract_version".to_owned()), declared.at);
    let parts = parse::body(&mut sink, root);
    let (diagnostics, provenance) = sink.finish();
    let contract = parts
        .assemble(declared.version, provenance)
        .filter(|_| diagnostics.counts().error == 0)
        .ok_or(ContractUnresolved {
            reason: UnresolvedReason::Invalid,
            version: Some(declared.version),
        });
    ContractLoad {
        contract,
        diagnostics: diagnostics.sorted(),
    }
}

/// The refusal a version outside the supported range is, whether or not a
/// `u32` holds the version it declared.
fn unusable_version(class: VersionClass, found: &str, version: Option<u32>) -> ContractLoad {
    ContractLoad {
        contract: Err(ContractUnresolved {
            reason: UnresolvedReason::VersionUnusable(class),
            version,
        }),
        diagnostics: vec![incompatible(class, found)],
    }
}

fn refuse(diagnostic: Diagnostic, reason: UnresolvedReason) -> ContractLoad {
    ContractLoad {
        contract: Err(ContractUnresolved {
            reason,
            version: None,
        }),
        diagnostics: vec![diagnostic],
    }
}

fn unreadable(error: &io::Error) -> ContractLoad {
    let reason = match error.kind() {
        io::ErrorKind::NotFound => UnresolvedReason::Missing,
        _ => UnresolvedReason::Unreadable,
    };
    let diagnostic = Diagnostic::kernel(
        KernelDiagnostic::ContractUnreadable,
        format!("`{CONTRACT_PATH}` could not be read: {error}"),
    )
    .at(Location::whole_file(contract_file()));
    ContractLoad {
        contract: Err(ContractUnresolved {
            reason,
            version: None,
        }),
        diagnostics: vec![diagnostic],
    }
}

/// Each of the three read faults maps to the contract's own identifier.
fn encoding_fault(fault: EncodingFault) -> Diagnostic {
    let kind = match fault {
        EncodingFault::InvalidUtf8 { .. } => KernelDiagnostic::ContractInvalidUtf8,
        EncodingFault::ByteOrderMark => KernelDiagnostic::ContractByteOrderMark,
        EncodingFault::CarriageReturn { .. } => KernelDiagnostic::ContractCarriageReturnLineEnding,
    };
    Diagnostic::kernel(kind, fault.describe())
        .at(Location::whole_file(contract_file()))
        .with_help("a contract is UTF-8 without a byte order mark, with LF line endings")
}

fn malformed(text: &Text, error: &toml::de::Error) -> Diagnostic {
    let at = error.span().map_or_else(
        || Location::whole_file(contract_file()),
        |span| Location::in_file(contract_file(), text.span(span)),
    );
    Diagnostic::kernel(
        KernelDiagnostic::ContractMalformedToml,
        format!("the contract is not well-formed TOML: {}", error.message()),
    )
    .at(at)
}

fn incompatible(class: VersionClass, found: &str) -> Diagnostic {
    let (start, end) = (
        SUPPORTED_CONTRACT_VERSIONS.start(),
        SUPPORTED_CONTRACT_VERSIONS.end(),
    );
    let message =
        format!("the contract declares version {found}; this release reads {start}..={end}");
    match class {
        VersionClass::BelowFloor => Diagnostic::kernel(
            KernelDiagnostic::CompatContractBelowSupportedFloor,
            message,
        )
        .with_help(
            "migration tooling arrives in a later release; until then, pin the release that read \
             this version with `DOGTAG_VERSION`",
        ),
        _ => Diagnostic::kernel(KernelDiagnostic::CompatContractTooNew, message).with_help(
            "upgrade the tool, or migrate the contract down to a version this release reads",
        ),
    }
    .at(Location::whole_file(contract_file()))
}

/// A contract in range but below the newest version this release reads loads
/// fully, and says that a newer format exists. At the maximum it says nothing.
fn newer_format_available(class: VersionClass, found: u32) -> Option<Diagnostic> {
    if class != VersionClass::Supported {
        return None;
    }
    let newest = SUPPORTED_CONTRACT_VERSIONS.end();
    Some(
        Diagnostic::kernel(
            KernelDiagnostic::CompatNewerFormatAvailable,
            format!("the contract declares version {found}; version {newest} is available"),
        )
        .at(Location::whole_file(contract_file())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::Severity;
    use crate::provenance::Source;
    use std::io::Write;

    const CONFORMING: &str = concat!(
        "contract_version = 2\n",
        "\n",
        "[dialect]\n",
        "links = \"wikilink\"\n",
        "\n",
        "[lifecycle]\n",
        "axis = \"status\"\n",
        "ordinary = { absent = true }\n",
        "\n",
        "[[flag]]\n",
        "property = \"leaned_on\"\n",
        "\n",
        "[[type]]\n",
        "name = \"person\"\n",
        "capabilities = [\"identity-bearing\"]\n",
        "\n",
        "  [[type.property]]\n",
        "  name = \"full_name\"\n",
        "  kind = \"string\"\n",
        "  required = true\n",
        "\n",
        "  [[type.property]]\n",
        "  name = \"status\"\n",
        "  kind = \"enum\"\n",
        "  values = [\"draft\", \"archived\", \"superseded\"]\n",
        "  required = false\n",
        "\n",
        "  [[type.property]]\n",
        "  name = \"leaned_on\"\n",
        "  kind = \"boolean\"\n",
        "  required = false\n",
        "\n",
        "  [[type.relationship]]\n",
        "  predicate = \"works-at\"\n",
        "  required = false\n",
        "\n",
        "[[type]]\n",
        "name = \"capture\"\n",
        "capabilities = [\"catch-all\"]\n",
    );

    /// The same contract, declaring the version below the current one.
    fn version_1(source: &str) -> String {
        source.replace("contract_version = 2", "contract_version = 1")
    }

    fn loaded(source: &str) -> Contract {
        parse_contract(source)
            .contract
            .expect("a conforming contract")
    }

    fn ids(load: &ContractLoad) -> Vec<&str> {
        load.diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    /// Writes `bytes` to a file under the system temp directory and reads it
    /// back through [`load_contract`], so the read path is exercised for real.
    fn from_file(name: &str, bytes: &[u8]) -> ContractLoad {
        let path = std::env::temp_dir().join(format!("dogtag-contract-{name}.toml"));
        let mut file = fs::File::create(&path).expect("a writable temporary file");
        file.write_all(bytes).expect("written");
        drop(file);
        let load = load_contract(&path);
        fs::remove_file(&path).expect("removed");
        load
    }

    #[test]
    fn the_worked_example_loads_with_zero_diagnostics() {
        let load = parse_contract(CONFORMING);
        assert_eq!(ids(&load), Vec::<&str>::new());
        let contract = load.contract.expect("resolved");
        assert_eq!(contract.contract_version(), 2);
        assert_eq!(contract.dialect().links(), LinkDialect::Wikilink);
        assert_eq!(contract.types().len(), 2);
    }

    #[test]
    fn declaration_order_survives_the_whole_pipeline() {
        let contract = loaded(CONFORMING);
        let types: Vec<&str> = contract.types().iter().map(TypeDecl::name).collect();
        assert_eq!(types, ["person", "capture"]);
        let properties: Vec<&str> = contract.types()[0]
            .properties()
            .iter()
            .map(PropertyDecl::name)
            .collect();
        assert_eq!(properties, ["full_name", "status", "leaned_on"]);
    }

    #[test]
    fn the_lifecycle_and_flags_read_exactly_as_declared() {
        let contract = loaded(CONFORMING);
        assert_eq!(
            contract.lifecycle(),
            &LifecycleDecl::Axis {
                axis: "status".to_owned(),
                ordinary: Ordinary::Absent,
            }
        );
        assert_eq!(contract.flags()[0].property(), "leaned_on");
        assert_eq!(
            contract
                .property_kind("status")
                .and_then(PropertyKind::values),
            Some(
                &[
                    "draft".to_owned(),
                    "archived".to_owned(),
                    "superseded".to_owned()
                ][..]
            )
        );
    }

    #[test]
    fn every_declared_leaf_is_attributed_to_the_contract() {
        let contract = loaded(CONFORMING);
        let links = contract
            .provenance()
            .get("dialect.links")
            .expect("recorded");
        assert_eq!(links.source, Source::Contract);
        let span = links
            .location
            .as_ref()
            .and_then(|at| at.span)
            .expect("a span");
        assert_eq!((span.start.line, span.start.column), (4, 9));
        assert_eq!(
            links.location.as_ref().map(|at| at.file.display_path()),
            Some(CONTRACT_PATH)
        );
    }

    #[test]
    fn the_version_itself_records_where_it_is_written() {
        let contract = loaded(CONFORMING);
        let entry = contract
            .provenance()
            .get("contract_version")
            .expect("recorded");
        assert_eq!(entry.source, Source::Contract);
        let span = entry
            .location
            .as_ref()
            .and_then(|at| at.span)
            .expect("a span");
        assert_eq!(span.start.offset, 19);
    }

    #[test]
    fn every_omitted_default_is_attributed_to_the_contract_version() {
        // The worked example writes every optional key, so this drops two of
        // them: the omission is what makes the `default` source reachable.
        let source = CONFORMING
            .replace("capabilities = [\"identity-bearing\"]\n", "")
            .replace(
                "  predicate = \"works-at\"\n  required = false\n",
                "  predicate = \"works-at\"\n",
            );
        let contract = loaded(&source);
        let defaults = [
            "type.person.capabilities",
            "type.person.relationship.works-at.required",
        ];
        for key in defaults {
            let entry = contract.provenance().get(key).expect("recorded");
            assert_eq!(
                entry.source,
                Source::Default {
                    contract_version: 2
                },
                "{key}"
            );
            assert!(entry.location.is_none(), "{key}");
        }
    }

    /// The span of the one diagnostic carrying `id`.
    fn span_of(load: &ContractLoad, id: &str) -> crate::diagnostic::Span {
        let found = load
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id.as_str() == id)
            .expect("the diagnostic the test asked about");
        found
            .location
            .as_ref()
            .and_then(|at| at.span)
            .expect("a span")
    }

    #[test]
    fn a_span_names_the_line_and_the_column_the_fault_is_written_at() {
        let source = CONFORMING.replace(
            "[dialect]\nlinks = \"wikilink\"\n",
            "[dialect]\nlinks = \"wikilink\"\nflavour = \"plain\"\n",
        );
        let load = parse_contract(&source);
        assert_eq!(ids(&load), ["contract.unknown-key"]);
        let span = span_of(&load, "contract.unknown-key");
        assert_eq!((span.start.line, span.start.column), (5, 1));
        assert_eq!(span.start.offset, 51);
    }

    #[test]
    fn a_column_on_a_multi_byte_line_counts_scalars_and_not_bytes() {
        // `café`, `·`, `déjà` and `—` are all wider than one byte, so a column
        // counted in bytes would land past the key this points at.
        let source = CONFORMING.replace(
            "links = \"wikilink\"\n",
            "links = \"wikilink\"  # café · déjà — vu\nflavour = \"plain\"\n",
        );
        let load = parse_contract(&source);
        let span = span_of(&load, "contract.unknown-key");
        assert_eq!((span.start.line, span.start.column), (5, 1));
        assert!(
            span.start.offset > 65,
            "the byte offset still counts bytes: {}",
            span.start.offset
        );
    }

    #[test]
    fn a_column_after_a_multi_byte_run_on_one_line_counts_scalars() {
        // `é` is two bytes, so the second value's column counted in bytes would
        // be 26 rather than the 25 scalar values actually precede it.
        let source = CONFORMING.replace(
            "capabilities = [\"identity-bearing\"]\n",
            "capabilities = [\"café\", \"writeable\"]\n",
        );
        let load = parse_contract(&source);
        let columns: Vec<(u32, u32)> = load
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id.as_str() == "contract.unknown-capability")
            .filter_map(|diagnostic| diagnostic.location.as_ref().and_then(|at| at.span))
            .map(|span| (span.start.line, span.start.column))
            .collect();
        assert_eq!(columns, [(15, 17), (15, 25)]);
    }

    #[test]
    fn an_unreadable_path_is_a_diagnostic_and_never_a_bare_failure() {
        let path = std::env::temp_dir().join("dogtag-contract-absent-on-purpose.toml");
        let load = load_contract(&path);
        assert_eq!(ids(&load), ["contract.unreadable"]);
        assert_eq!(
            load.contract.expect_err("unresolved").reason,
            UnresolvedReason::Missing
        );
    }

    #[test]
    fn a_directory_read_as_a_contract_is_unreadable_rather_than_missing() {
        let load = load_contract(&std::env::temp_dir());
        assert_eq!(ids(&load), ["contract.unreadable"]);
        assert_eq!(
            load.contract.expect_err("unresolved").reason,
            UnresolvedReason::Unreadable
        );
    }

    #[test]
    fn a_readable_file_takes_the_same_path_as_text_in_memory() {
        let load = from_file("conforming", CONFORMING.as_bytes());
        assert!(load.diagnostics.is_empty());
        assert!(load.contract.is_ok());
    }

    #[test]
    fn invalid_utf8_is_refused_with_the_contracts_own_identifier() {
        let load = from_file("invalid-utf8", b"contract_version = 1\n\xff\n");
        assert_eq!(ids(&load), ["contract.invalid-utf8"]);
        assert_eq!(
            load.contract.expect_err("unresolved").reason,
            UnresolvedReason::Encoding
        );
    }

    #[test]
    fn a_byte_order_mark_is_refused_rather_than_stripped() {
        let load = parse_contract("\u{feff}contract_version = 1\n");
        assert_eq!(ids(&load), ["contract.byte-order-mark"]);
        assert!(load.diagnostics[0].help.is_some());
    }

    #[test]
    fn carriage_return_line_endings_are_refused_rather_than_normalized() {
        let load = parse_contract("contract_version = 1\r\n");
        assert_eq!(ids(&load), ["contract.carriage-return-line-ending"]);
    }

    #[test]
    fn a_syntax_error_is_reported_with_the_parsers_span() {
        let load = parse_contract("contract_version = 1\n[dialect\n");
        assert_eq!(ids(&load), ["contract.malformed-toml"]);
        assert!(
            load.diagnostics[0]
                .location
                .as_ref()
                .is_some_and(|at| at.span.is_some())
        );
        assert_eq!(
            load.contract.expect_err("unresolved").reason,
            UnresolvedReason::Malformed
        );
    }

    #[test]
    fn a_syntax_error_with_no_span_still_names_the_file() {
        let text = encoding::inspect(b"a = 1\n").expect("valid");
        let error = <toml::de::Error as serde::de::Error>::custom("a spanless fault");
        let diagnostic = malformed(&text, &error);
        assert_eq!(diagnostic.id.as_str(), "contract.malformed-toml");
        let at = diagnostic.location.expect("the file");
        assert!(at.span.is_none());
    }

    #[test]
    fn a_missing_version_is_reported_before_any_structural_check() {
        let load = parse_contract("[dialect]\nlinks = \"org\"\n");
        assert_eq!(ids(&load), ["contract.version-missing"]);
    }

    #[test]
    fn an_invalid_version_is_reported_before_any_structural_check() {
        let load = parse_contract("contract_version = \"one\"\nflavour = 1\n");
        assert_eq!(ids(&load), ["contract.version-invalid"]);
        let unresolved = load.contract.expect_err("unresolved");
        assert_eq!(unresolved.reason, UnresolvedReason::Invalid);
        assert_eq!(unresolved.version, None);
    }

    #[test]
    fn a_version_below_the_floor_refuses_with_exactly_one_diagnostic() {
        let load =
            parse_contract(&CONFORMING.replace("contract_version = 2", "contract_version = 0"));
        assert_eq!(ids(&load), ["compat.contract-below-supported-floor"]);
        assert!(
            load.diagnostics[0]
                .help
                .as_deref()
                .is_some_and(|help| help.contains("DOGTAG_VERSION"))
        );
    }

    #[test]
    fn a_version_above_the_range_refuses_with_exactly_one_diagnostic() {
        let load =
            parse_contract(&CONFORMING.replace("contract_version = 2", "contract_version = 3"));
        assert_eq!(ids(&load), ["compat.contract-too-new"]);
        let unresolved = load.contract.expect_err("unresolved");
        assert_eq!(
            unresolved.reason,
            UnresolvedReason::VersionUnusable(VersionClass::TooNew)
        );
        assert_eq!(unresolved.version, Some(3));
    }

    #[test]
    fn a_version_beyond_a_u32_is_classified_rather_than_refused_by_the_parser() {
        // The format's domain is every whole number 0 or above, and
        // classification is total over it: a version no `u32` holds is above
        // every supported range, so it is the version gate that refuses it.
        let load = parse_contract(
            &CONFORMING.replace("contract_version = 2", "contract_version = 4294967296"),
        );
        assert_eq!(ids(&load), ["compat.contract-too-new"]);
        assert!(load.diagnostics[0].message.contains("version 4294967296"));
        let unresolved = load.contract.expect_err("unresolved");
        assert_eq!(
            unresolved.reason,
            UnresolvedReason::VersionUnusable(VersionClass::TooNew)
        );
        assert_eq!(unresolved.version, None, "no `u32` holds it");
    }

    #[test]
    fn the_refusal_is_the_versions_and_never_the_parsers() {
        let source = concat!(
            "contract_version = 9\n",
            "flavour = \"plain\"\n",
            "[dialect]\n",
            "links = \"org\"\n",
            "requred = true\n",
        );
        let load = parse_contract(source);
        assert_eq!(
            load.diagnostics.len(),
            1,
            "an out-of-range version suppresses every structural complaint"
        );
        assert_eq!(load.diagnostics[0].id.as_str(), "compat.contract-too-new");
    }

    #[test]
    fn a_supported_version_below_the_newest_loads_and_says_so() {
        // Reachable from a real contract for the first time: the range holds
        // two versions, so a version-1 vault loads fully and is told that a
        // newer format exists. `info` for exactly the reason the severity was
        // created — it recurs on every run of a legitimate setup.
        let load = parse_contract(&version_1(CONFORMING));
        assert_eq!(ids(&load), ["compat.newer-format-available"]);
        let reported = &load.diagnostics[0];
        assert_eq!(reported.severity, Severity::Info);
        assert!(reported.message.contains("version 2 is available"));
        assert!(load.contract.is_ok(), "a supported version loads fully");
        assert!(newer_format_available(VersionClass::Current, 2).is_none());
    }

    #[test]
    fn version_1_resolves_exactly_as_it_did_before_version_2_existed() {
        // The claim slice one owes: the same model, the same provenance keys,
        // and the same sources as when `1..=1` was the whole range — the only
        // difference being the info a supported-but-not-current version now
        // earns. Held by comparing the two versions of one text, so a version-2
        // key set or default table leaking into version 1 fails here.
        let at_1 = loaded(&version_1(CONFORMING));
        let at_2 = loaded(CONFORMING);
        assert_eq!(at_1.contract_version(), 1);
        assert_eq!(at_1.types(), at_2.types());
        assert_eq!(at_1.flags(), at_2.flags());
        assert_eq!(at_1.lifecycle(), at_2.lifecycle());
        assert_eq!(at_1.dialect(), at_2.dialect());
        let keys = |contract: &Contract| -> Vec<String> {
            contract
                .provenance()
                .entries()
                .map(|entry| entry.key.clone())
                .collect()
        };
        assert_eq!(keys(&at_1), keys(&at_2));
    }

    #[test]
    fn an_omission_in_a_version_1_contract_resolves_against_version_1s_table() {
        // The other half: not merely attributed to version 1, but *resolved*
        // against version 1's table. A build that judged the omission against
        // the newest table it knew would record a source that lies.
        let source = version_1(&CONFORMING.replace("capabilities = [\"identity-bearing\"]\n", ""));
        let contract = loaded(&source);
        assert_eq!(contract.types()[0].capabilities(), []);
        assert_eq!(
            contract
                .provenance()
                .get("type.person.capabilities")
                .map(|entry| entry.source),
            Some(Source::Default {
                contract_version: 1
            })
        );
    }

    #[test]
    fn three_separate_faults_are_reported_three_times() {
        let source = concat!(
            "contract_version = 2\n",
            "\n[dialect]\nlinks = \"org\"\n",
            "\n[lifecycle]\nnone = true\n",
            "\n[[type]]\nname = \"a\"\ncapabilities = [\"catch-all\", \"writeable\"]\n",
            "colour = \"red\"\n",
        );
        let load = parse_contract(source);
        assert_eq!(
            ids(&load),
            [
                "contract.unknown-link-dialect",
                "contract.unknown-capability",
                "contract.unknown-key",
            ]
        );
    }

    #[test]
    fn diagnostics_come_out_in_the_deterministic_total_order() {
        let source = concat!(
            "contract_version = 2\n",
            "\n[lifecycle]\nnone = true\n",
            "\n[[type]]\nname = \"a\"\ncapabilities = [\"nope\"]\n",
        );
        let load = parse_contract(source);
        assert_eq!(
            ids(&load),
            [
                "contract.missing-catch-all",
                "contract.missing-dialect",
                "contract.unknown-capability",
            ],
            "a location with no span sorts before any span, then by identifier"
        );
    }

    #[test]
    fn a_contract_that_parses_but_breaks_a_rule_does_not_resolve() {
        let source = CONFORMING.replace("capabilities = [\"catch-all\"]", "capabilities = []");
        let load = parse_contract(&source);
        assert_eq!(ids(&load), ["contract.missing-catch-all"]);
        let unresolved = load.contract.expect_err("unresolved");
        assert_eq!(unresolved.reason, UnresolvedReason::Invalid);
        assert_eq!(unresolved.version, Some(2));
    }

    #[test]
    fn every_reason_says_why_an_answer_was_not_evaluated() {
        let unusable = UnresolvedReason::VersionUnusable;
        let described = [
            (UnresolvedReason::Missing, "holds no contract file"),
            (UnresolvedReason::Unreadable, "could not be read"),
            (
                UnresolvedReason::Encoding,
                "not the encoding the format requires",
            ),
            (UnresolvedReason::Malformed, "not well-formed TOML"),
            (UnresolvedReason::Invalid, "validity rules"),
            (
                unusable(VersionClass::BelowFloor),
                "below the supported range 1..=2",
            ),
            (
                unusable(VersionClass::TooNew),
                "above the supported range 1..=2",
            ),
            (
                unusable(VersionClass::Supported),
                "outside the supported range 1..=2",
            ),
            (
                unusable(VersionClass::Current),
                "outside the supported range 1..=2",
            ),
        ];
        for (reason, expected) in described {
            let text = reason.describe();
            assert!(text.contains(expected), "{reason:?} described as {text}");
        }
    }

    #[test]
    fn the_load_outcome_clones_compares_and_formats() {
        let load = parse_contract(CONFORMING);
        assert_eq!(load.clone(), load);
        assert!(format!("{load:?}").contains("Wikilink"));
        let unresolved = ContractUnresolved {
            reason: UnresolvedReason::Missing,
            version: None,
        };
        assert_eq!(unresolved.clone(), unresolved);
        assert_ne!(
            unresolved,
            ContractUnresolved {
                reason: UnresolvedReason::Invalid,
                version: Some(1),
            }
        );
        assert!(format!("{unresolved:?}").contains("Missing"));
    }
}
