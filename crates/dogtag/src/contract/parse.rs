//! The two-pass parse of a contract body.
//!
//! Pass one extracts `contract_version` and **nothing else**. Pass two
//! validates the body against *that* version's key set. The order is
//! load-bearing rather than tidy: a key is legal exactly when the version the
//! contract declares defines it, not when the reading tool happens to know it,
//! so a file declaring version 1 with a version-2 key is refused identically by
//! every build.
//!
//! Version classification sits between the two passes and can suppress the
//! second entirely. A contract outside the supported range yields exactly one
//! compatibility diagnostic — the refusal is the version's, not the parser's —
//! rather than a pile of unknown-key errors misdiagnosing a newer format as a
//! typo.

use core::ops::Range;

use toml::Spanned;
use toml::de::{DeInteger, DeTable, DeValue};

use crate::compat::SUPPORTED_CONTRACT_VERSIONS;
use crate::diagnostic::{Diagnostic, KernelDiagnostic, Location};
use crate::document;
use crate::encoding::Text;
use crate::provenance::Provenance;

use super::model::{Contract, Dialect, FlagDecl, LifecycleDecl, LinkDialect, TypeDecl};
use super::sink::{Claim, KeyPath, Report, Section, Seen, Sink, contract_file};
use super::vocabulary::TagsDecl;
use super::{declarations, lifecycle, tags, validate};

/// The version a contract declares, and where it declares it.
#[derive(Debug)]
pub(crate) struct Declared {
    pub(crate) version: u32,
    pub(crate) at: Range<usize>,
}

/// What the first pass found where `contract_version` should be.
///
/// Deliberately not a `Result`: a [`Diagnostic`] is a wide value, and this is
/// not an error channel a caller propagates with `?` — it is the first pass's
/// answer, and the second pass runs only for one of the two shapes.
pub(crate) enum DeclaredVersion {
    /// A version the supported range can be asked about.
    Found(Declared),
    /// A whole number 0 or above that no `u32` holds. It is above every range
    /// this SDK can support, so it is the version gate's refusal rather than
    /// the parser's; the literal travels because a message may not restate it
    /// as a number it is not.
    Beyond(String),
    /// `contract.version-missing`, when the key is absent, or
    /// `contract.version-invalid` when its value is not an integer the domain
    /// admits — the domain is every whole number 0 or above, so a negative is
    /// refused here and an over-large one is classified instead.
    Refused(Diagnostic),
}

/// Extracts `contract_version`, and only that.
pub(crate) fn version(text: &Text, root: &DeTable<'_>) -> DeclaredVersion {
    let Some(value) = document::get(root, "contract_version") else {
        return DeclaredVersion::Refused(version_missing());
    };
    let integer = match document::expect_integer(value) {
        Ok(integer) => integer,
        Err(mismatch) => {
            let message = mismatch.message("contract_version");
            return DeclaredVersion::Refused(invalid(text, message, mismatch.span));
        }
    };
    match as_u32(integer) {
        Some(found) => DeclaredVersion::Found(Declared {
            version: found,
            at: value.span(),
        }),
        None if negative(integer) => {
            DeclaredVersion::Refused(invalid(text, out_of_domain(integer), value.span()))
        }
        None => DeclaredVersion::Beyond(text.as_str()[value.span()].to_owned()),
    }
}

/// Whether the declared integer is below the domain rather than above it.
///
/// [`DeInteger::as_str`] keeps the sign, so this is the whole question: a
/// literal that is not negative and does not fit a `u32` is above every
/// supported range, and above is classified rather than refused.
fn negative(integer: &DeInteger<'_>) -> bool {
    integer.as_str().starts_with('-')
}

fn out_of_domain(integer: &DeInteger<'_>) -> String {
    format!("`contract_version` is `{integer}`, which is not a whole number 0 or above")
}

fn as_u32(integer: &DeInteger<'_>) -> Option<u32> {
    u32::from_str_radix(integer.as_str(), integer.radix()).ok()
}

/// The newest version this release reads, which is what a contract declaring
/// none is told to write. Taken from the range rather than restated, so a
/// widening that forgot this advice would leave it naming a stale version.
fn current() -> u32 {
    *SUPPORTED_CONTRACT_VERSIONS.end()
}

fn version_missing() -> Diagnostic {
    Diagnostic::kernel(
        KernelDiagnostic::ContractVersionMissing,
        "the contract declares no `contract_version`",
    )
    .at(Location::whole_file(contract_file()))
    .with_help(format!(
        "every contract declares `contract_version`, and version {} is the current format",
        current()
    ))
}

fn invalid(text: &Text, message: String, at: Range<usize>) -> Diagnostic {
    Diagnostic::kernel(KernelDiagnostic::ContractVersionInvalid, message)
        .at(Location::in_file(contract_file(), text.span(at)))
        .with_help(format!(
            "`contract_version` is a single whole number, `{}` at this release",
            current()
        ))
}

/// Everything the body walk resolved, before the diagnostics decide whether it
/// becomes a [`Contract`].
pub(crate) struct Parts {
    pub(crate) dialect: Option<Dialect>,
    pub(crate) lifecycle: Option<LifecycleDecl>,
    pub(crate) tags: Option<TagsDecl>,
    pub(crate) flags: Vec<FlagDecl>,
    pub(crate) types: Vec<TypeDecl>,
}

impl Parts {
    /// The resolved contract, when every mandatory construct was declared.
    ///
    /// `tags` is not among them: the tag vocabulary is optional at the one
    /// version that defines it, and absent at the one that does not.
    pub(crate) fn assemble(
        self,
        contract_version: u32,
        provenance: Provenance,
    ) -> Option<Contract> {
        Some(Contract {
            contract_version,
            dialect: self.dialect?,
            lifecycle: self.lifecycle?,
            tags: self.tags,
            flags: self.flags,
            types: self.types,
            provenance,
        })
    }
}

/// Walks the body against the declared version's key set, collecting every
/// diagnostic rather than stopping at the first.
pub(crate) fn body(sink: &mut Sink<'_>, root: &DeTable<'_>) -> Parts {
    let section = Section {
        table: root,
        span: 0..0,
        label: "the contract".to_owned(),
        path: KeyPath::root(),
    };
    let allowed = sink.schema().keys.root;
    sink.sweep(&section, allowed);
    let parts = Parts {
        dialect: dialect(sink, root),
        lifecycle: lifecycle::declaration(sink, root),
        tags: tags::table(sink, root),
        flags: flags(sink, root),
        types: declarations::types(sink, root),
    };
    validate::run(sink, &parts);
    parts
}

/// Reads the mandatory `[dialect]` table.
///
/// **`[dialect]` is mandatory at version 1 and at version 2 both**, and its
/// absence is `contract.missing-dialect`. That was an inference the parser made
/// when nothing decided it; the contract-version-2 record ratifies it for both
/// versions, so the identifier and the behavior are now record-backed rather
/// than derived from the shape of the table alone. No version states a default
/// for `links`, so absence stays a load error rather than a value this parser
/// invents — and a later decision to default it needs a version to carry it.
fn dialect(sink: &mut Sink<'_>, root: &DeTable<'_>) -> Option<Dialect> {
    let Some(value) = document::get(root, "dialect") else {
        let at = sink.whole_file();
        let report = Report::new("the contract declares no `[dialect]` table".to_owned())
            .with_help(links_help());
        sink.report(KernelDiagnostic::ContractMissingDialect, report, at);
        return None;
    };
    let table = sink.table(value, "dialect")?;
    let section = Section {
        table,
        span: value.span(),
        label: "`[dialect]`".to_owned(),
        path: KeyPath::root().child("dialect"),
    };
    let allowed = sink.schema().keys.dialect;
    sink.sweep(&section, allowed);
    let declared = sink.required(&section, "links")?;
    let spelled = sink.string(declared, section.leaf("links"))?;
    links(sink, spelled, declared.span()).map(|links| Dialect { links })
}

fn links(sink: &mut Sink<'_>, spelled: &str, at: Range<usize>) -> Option<LinkDialect> {
    let found = LinkDialect::named(spelled);
    if found.is_none() {
        let message = format!("`{spelled}` is not a link dialect this contract version defines");
        let at = sink.location(at);
        let report = Report::new(message).with_help(links_help());
        sink.report(KernelDiagnostic::ContractUnknownLinkDialect, report, at);
    }
    found
}

fn links_help() -> String {
    let quoted: Vec<String> = LinkDialect::ALL
        .iter()
        .map(|dialect| format!("`{dialect}`"))
        .collect();
    format!(
        "`[dialect]` declares `links`, one of {}",
        quoted.join(" or ")
    )
}

/// Reads every `[[flag]]`, in declaration order.
fn flags(sink: &mut Sink<'_>, root: &DeTable<'_>) -> Vec<FlagDecl> {
    let Some(value) = document::get(root, "flag") else {
        return Vec::new();
    };
    let Some(array) = sink.array(value, "flag") else {
        return Vec::new();
    };
    let mut seen = Seen::new();
    array
        .iter()
        .filter_map(|entry| flag(sink, entry, &mut seen))
        .collect()
}

fn flag(sink: &mut Sink<'_>, value: &Spanned<DeValue<'_>>, seen: &mut Seen) -> Option<FlagDecl> {
    let table = sink.table(value, "flag")?;
    let entry = Section {
        table,
        span: value.span(),
        label: "`[[flag]]`".to_owned(),
        path: KeyPath::nameless(),
    };
    let named = sink.name_of(&entry, "property");
    let section = Section {
        path: KeyPath::root()
            .child("flag")
            .child_opt(named.as_ref().map(|found| found.text)),
        ..entry
    };
    let allowed = sink.schema().keys.flag;
    sink.sweep(&section, allowed);
    let named = named?;
    sink.written(section.leaf("property").key, named.span.clone());
    let property = named.text.to_owned();
    let claim = Claim {
        message: format!("two flags name the property `{property}`"),
        kind: KernelDiagnostic::ContractDuplicateFlag,
        named,
    };
    sink.keep(seen, claim)?;
    Some(FlagDecl { property })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::schema;
    use crate::contract::sink::tests::{root_of, text_of};
    use crate::diagnostic::DiagnosticList;

    /// How the first pass answered for `source`.
    fn first_pass(source: &str) -> DeclaredVersion {
        let text = text_of(source);
        let document = root_of(&text);
        version(&text, document.get_ref())
    }

    /// The version `source` declares, when it declares one a `u32` holds.
    fn declared(source: &str) -> Option<Declared> {
        match first_pass(source) {
            DeclaredVersion::Found(found) => Some(found),
            _ => None,
        }
    }

    /// The diagnostic that refused `source`, when one did.
    fn refusal(source: &str) -> Option<Diagnostic> {
        match first_pass(source) {
            DeclaredVersion::Refused(diagnostic) => Some(diagnostic),
            _ => None,
        }
    }

    /// The literal `source` declares where no `u32` holds it.
    fn beyond(source: &str) -> Option<String> {
        match first_pass(source) {
            DeclaredVersion::Beyond(literal) => Some(literal),
            _ => None,
        }
    }

    /// The version `source` declares, or the identifier that refused it.
    fn declared_version(source: &str) -> Result<u32, String> {
        match (declared(source), refusal(source)) {
            (Some(found), _) => Ok(found.version),
            (None, Some(diagnostic)) => Err(diagnostic.id.as_str().to_owned()),
            (None, None) => Err("beyond the domain".to_owned()),
        }
    }

    struct Read {
        parts: Parts,
        diagnostics: DiagnosticList,
        provenance: Provenance,
    }

    fn read(source: &str) -> Read {
        let text = text_of(source);
        let document = root_of(&text);
        let mut sink = Sink::new(&text, &schema::VERSION_1);
        let parts = body(&mut sink, document.get_ref());
        let (diagnostics, provenance) = sink.finish();
        Read {
            parts,
            diagnostics,
            provenance,
        }
    }

    fn ids(read: &Read) -> Vec<&str> {
        read.diagnostics
            .as_slice()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect()
    }

    #[test]
    fn a_declared_version_is_read_with_its_span() {
        let declared = declared("contract_version = 1\n").expect("a declared version");
        assert!(refusal("contract_version = 1\n").is_none());
        assert_eq!(declared.version, 1);
        assert_eq!(declared.at, 19..20);
        assert!(format!("{declared:?}").contains("version: 1"));
    }

    #[test]
    fn a_version_written_in_another_radix_is_still_read() {
        assert_eq!(declared_version("contract_version = 0x1\n"), Ok(1));
        assert_eq!(declared_version("contract_version = 0\n"), Ok(0));
    }

    #[test]
    fn an_absent_version_is_reported_before_anything_else() {
        assert_eq!(
            declared_version("[dialect]\nlinks = \"wikilink\"\n"),
            Err("contract.version-missing".to_owned())
        );
    }

    #[test]
    fn a_version_that_is_not_an_integer_is_invalid() {
        assert_eq!(
            declared_version("contract_version = \"1\"\n"),
            Err("contract.version-invalid".to_owned())
        );
    }

    #[test]
    fn a_negative_version_is_below_the_domain_and_invalid() {
        assert_eq!(
            declared_version("contract_version = -1\n"),
            Err("contract.version-invalid".to_owned())
        );
        assert_eq!(
            declared_version("contract_version = -0\n"),
            Err("contract.version-invalid".to_owned())
        );
    }

    #[test]
    fn a_version_beyond_a_u32_is_carried_as_the_file_writes_it() {
        // Not refused here: it is in the domain the format declares, so the
        // version gate classifies it. The literal is the file's own bytes,
        // radix prefix and digit separators included.
        assert_eq!(
            beyond("contract_version = 4294967296\n").as_deref(),
            Some("4294967296")
        );
        assert_eq!(
            beyond("contract_version = 0xFFFF_FFFF_F\n").as_deref(),
            Some("0xFFFF_FFFF_F")
        );
        assert_eq!(beyond("contract_version = 1\n"), None);
        assert_eq!(
            declared_version("contract_version = 4294967296\n"),
            Err("beyond the domain".to_owned())
        );
        assert!(declared("contract_version = 4294967296\n").is_none());
    }

    #[test]
    fn an_invalid_version_diagnostic_points_at_the_value() {
        let diagnostic = refusal("contract_version = -1\n").expect("invalid");
        let span = diagnostic.location.and_then(|at| at.span).expect("a span");
        assert_eq!((span.start.line, span.start.column), (1, 20));
        assert!(diagnostic.help.is_some());
    }

    const DIALECT: &str = "[dialect]\nlinks = \"wikilink\"\n";

    #[test]
    fn a_dialect_reads_and_records_where_it_is_written() {
        let read = read(DIALECT);
        assert_eq!(
            read.parts.dialect.map(Dialect::links),
            Some(LinkDialect::Wikilink)
        );
        let entry = read.provenance.get("dialect.links").expect("recorded");
        let span = entry
            .location
            .as_ref()
            .and_then(|at| at.span)
            .expect("a span");
        assert_eq!((span.start.line, span.start.column), (2, 9));
    }

    #[test]
    fn a_contract_with_no_dialect_is_refused() {
        assert!(ids(&read("[lifecycle]\nnone = true\n")).contains(&"contract.missing-dialect"));
    }

    #[test]
    fn a_dialect_that_is_not_a_table_is_reported_as_a_wrong_type() {
        assert!(ids(&read("dialect = \"wikilink\"\n")).contains(&"contract.value-wrong-type"));
    }

    #[test]
    fn a_dialect_without_links_is_a_missing_key() {
        assert!(ids(&read("[dialect]\n")).contains(&"contract.missing-key"));
    }

    #[test]
    fn an_unknown_link_dialect_names_the_closed_set() {
        let read = read("[dialect]\nlinks = \"org\"\n");
        assert!(ids(&read).contains(&"contract.unknown-link-dialect"));
        let help = read.diagnostics.as_slice()[0].help.as_deref();
        assert_eq!(
            help,
            Some("`[dialect]` declares `links`, one of `wikilink` or `markdown`")
        );
    }

    #[test]
    fn an_unknown_root_key_is_fatal() {
        let read = read("[dialect]\nlinks = \"markdown\"\n\n[schema]\nx = 1\n");
        assert_eq!(ids(&read)[0], "contract.unknown-key");
        assert!(
            read.diagnostics.as_slice()[0]
                .message
                .contains("the contract declares `schema`")
        );
    }

    #[test]
    fn flags_read_in_declaration_order_and_record_their_provenance() {
        let source = "[[flag]]\nproperty = \"b\"\n\n[[flag]]\nproperty = \"a\"\n";
        let read = read(source);
        let properties: Vec<&str> = read.parts.flags.iter().map(FlagDecl::property).collect();
        assert_eq!(properties, ["b", "a"]);
        assert!(read.provenance.get("flag.a.property").is_some());
        assert!(read.provenance.get("flag.b.property").is_some());
    }

    #[test]
    fn a_contract_declaring_no_flag_declares_none() {
        assert!(read(DIALECT).parts.flags.is_empty());
    }

    #[test]
    fn a_flag_array_of_the_wrong_shape_is_reported_as_a_wrong_type() {
        assert!(ids(&read("flag = 1\n")).contains(&"contract.value-wrong-type"));
        assert!(ids(&read("flag = [1]\n")).contains(&"contract.value-wrong-type"));
    }

    #[test]
    fn a_flag_without_a_property_is_a_missing_key() {
        assert!(ids(&read("[[flag]]\n")).contains(&"contract.missing-key"));
    }

    #[test]
    fn an_unknown_key_on_a_flag_is_fatal() {
        let read = read("[[flag]]\nproperty = \"a\"\neligible = true\n");
        assert!(ids(&read).contains(&"contract.unknown-key"));
    }

    #[test]
    fn two_flags_naming_one_property_point_at_both() {
        let read = read("[[flag]]\nproperty = \"a\"\n\n[[flag]]\nproperty = \"a\"\n");
        assert!(ids(&read).contains(&"contract.duplicate-flag"));
        assert_eq!(read.parts.flags.len(), 1);
    }

    #[test]
    fn parts_become_a_contract_only_when_every_mandatory_construct_resolved() {
        let partial = read(DIALECT);
        assert!(partial.parts.assemble(1, Provenance::new()).is_none());
        let whole = read("[dialect]\nlinks = \"markdown\"\n\n[lifecycle]\nnone = true\n");
        let contract = whole
            .parts
            .assemble(1, Provenance::new())
            .expect("resolved");
        assert_eq!(contract.contract_version(), 1);
        assert_eq!(contract.lifecycle(), &LifecycleDecl::None);
    }

    #[test]
    fn a_body_missing_both_mandatory_tables_reports_both() {
        let read = read(
            "contract_version = 1\n\n[[type]]\nname = \"a\"\ncapabilities = [\"catch-all\"]\n",
        );
        assert_eq!(
            ids(&read),
            ["contract.missing-dialect", "contract.missing-lifecycle"]
        );
    }
}
