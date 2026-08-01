//! The two structured reports.
//!
//! Both are one JSON document carrying its own [`SCHEMA_VERSION`], pretty
//! printed and newline-terminated. `serde`'s derive is used here and **only**
//! here — for output. Nothing in this SDK parses an input with it.
//!
//! # The shapes are settled here
//!
//! `doctor`'s section names and its representation of *not evaluated* are what a
//! parallel run's triage will diff, so they are fixed by this first
//! implementation rather than accreted. A contract-dependent section is an
//! object carrying `"evaluated": false` and a `"reason"` — **never a `null` and
//! never an omission**, because a consumer cannot tell an omitted section from a
//! section its own reader forgot.
//!
//! The two documents take deliberately different positions on absence, and the
//! difference is the point:
//!
//! - **`doctor` emits a fixed key set**, with `null` for a fact that does not
//!   apply. It is a report with a schema, read by diffing one run against
//!   another, and a key that comes and goes makes every diff say something
//!   happened.
//! - **`contract` omits a key whose declaration the contract does not make.** It
//!   is a rendering of declarations, where emitting `"values": null` on a
//!   `string` property would assert a declaration that was never written — the
//!   same line the Markdown draws, and what keeps the two semantically equal.
//!
//! Object keys are emitted in a fixed order, arrays of declarations in
//! declaration order, and provenance in key order, so identical input produces
//! byte-identical output.

use serde::Serialize;

use super::{
    ContractFacts, DoctorReport, Evaluated, InstallationFacts, SCHEMA_VERSION, Sections,
    VersionFacts,
};
use crate::contract::{
    CONTRACT_PATH, Capability, Contract, LifecycleDecl, Ordinary, PropertyDecl, PropertyKind,
    RelationshipDecl, ScalarKind, TypeDecl,
};
use crate::diagnostic::{Diagnostic, FileRef, Location, Position, Related, SeverityCounts, Span};
use crate::provenance::{ProvenanceEntry, Source};
use crate::vault::VaultRoot;

/// Renders the `doctor` report as JSON.
pub fn doctor_json(report: &DoctorReport) -> String {
    document(&DoctorWire {
        schema_version: SCHEMA_VERSION,
        report: "doctor",
        vault: vault_wire(report),
        contract: contract_facts_wire(&report.contract),
        installation: installation_wire(&report.installation),
        sections: sections_wire(&report.sections),
        diagnostics: report.diagnostics().iter().map(diagnostic_wire).collect(),
        summary: summary_wire(report.counts()),
    })
}

/// Renders a resolved contract as JSON, with per-leaf provenance.
///
/// Every declaration the contract makes appears, and nothing else does. The
/// provenance is not optional here as it is in the Markdown: a caller that asked
/// for the structured model asked for the whole model, and a value's origin is
/// part of it.
pub fn contract_json(root: &VaultRoot, contract: &Contract) -> String {
    let shown = root.display();
    document(&ExplainWire {
        schema_version: SCHEMA_VERSION,
        report: "contract",
        vault: ExplainVaultWire {
            root: shown.as_ref(),
        },
        contract: contract_body_wire(contract),
        provenance: contract
            .provenance()
            .entries()
            .map(provenance_wire)
            .collect(),
    })
}

/// One JSON document, pretty printed and newline-terminated.
///
/// Pretty rather than compact because the audience is a person diffing two runs
/// as often as it is a parser, and a one-line document makes every change look
/// like every other change.
fn document<T: Serialize>(value: &T) -> String {
    let mut rendered = serde_json::to_string_pretty(value)
        .expect("a report holds no float and no map with a non-string key, so it serializes");
    rendered.push('\n');
    rendered
}

/// A fact that exists only when the contract resolved.
///
/// The `evaluated` discriminator is a field rather than a tag so that a consumer
/// reads one boolean in the same place in every section.
#[derive(Serialize)]
#[serde(untagged)]
enum Evaluation<'a, T> {
    Evaluated(T),
    NotEvaluated(NotEvaluated<'a>),
}

/// A section with no answer, and the reason there is none.
#[derive(Serialize)]
struct NotEvaluated<'a> {
    evaluated: bool,
    reason: &'a str,
}

/// Builds one section from the contract-dependent state.
fn section<'a, T>(
    sections: &'a Sections,
    build: impl FnOnce(&'a Evaluated) -> T,
) -> Evaluation<'a, T> {
    match sections {
        Sections::Evaluated(evaluated) => Evaluation::Evaluated(build(evaluated)),
        Sections::NotEvaluated(reason) => Evaluation::NotEvaluated(NotEvaluated {
            evaluated: false,
            reason,
        }),
    }
}

#[derive(Serialize)]
struct DoctorWire<'a> {
    schema_version: u32,
    report: &'static str,
    vault: VaultWire<'a>,
    contract: DoctorContractWire<'a>,
    installation: InstallationWire<'a>,
    sections: SectionsWire<'a>,
    diagnostics: Vec<DiagnosticWire<'a>>,
    summary: SummaryWire,
}

#[derive(Serialize)]
struct VaultWire<'a> {
    root: &'a str,
    selection: SelectionWire<'a>,
}

#[derive(Serialize)]
struct SelectionWire<'a> {
    how: &'static str,
    requested: Option<&'a str>,
}

#[derive(Serialize)]
struct DoctorContractWire<'a> {
    path: &'static str,
    present: bool,
    state: &'static str,
    unresolved_reason: Option<&'a str>,
    version: VersionWire,
}

#[derive(Serialize)]
struct VersionWire {
    found: Option<u32>,
    supported: SupportedWire,
    classification: Option<&'static str>,
}

#[derive(Serialize)]
struct SupportedWire {
    min: u32,
    max: u32,
}

#[derive(Serialize)]
struct InstallationWire<'a> {
    path: &'static str,
    state: &'static str,
    version: Option<VersionWire>,
    actor: Option<ActorWire<'a>>,
    registry_entry: Option<EntryWire<'a>>,
}

#[derive(Serialize)]
struct ActorWire<'a> {
    name: &'a str,
}

#[derive(Serialize)]
struct EntryWire<'a> {
    name: &'a str,
    path: &'a str,
}

#[derive(Serialize)]
struct SectionsWire<'a> {
    capabilities: Evaluation<'a, CapabilitiesWire<'a>>,
    lifecycle: Evaluation<'a, LifecycleWire<'a>>,
    dialect: Evaluation<'a, DialectWire>,
}

#[derive(Serialize)]
struct CapabilitiesWire<'a> {
    evaluated: bool,
    types_declared: usize,
    identity_bearing: &'a [String],
    catch_all: Option<&'a str>,
    closed_write: &'a [String],
}

#[derive(Serialize)]
struct LifecycleWire<'a> {
    evaluated: bool,
    declared: &'static str,
    axis: Option<&'a str>,
    ordinary: Option<OrdinaryWire<'a>>,
}

#[derive(Serialize)]
struct DialectWire {
    evaluated: bool,
    links: &'static str,
}

/// How the ordinary state is encoded, spelled the way the contract spells it.
#[derive(Serialize)]
#[serde(untagged)]
enum OrdinaryWire<'a> {
    Absent { absent: bool },
    Value { value: &'a str },
}

#[derive(Serialize)]
struct DiagnosticWire<'a> {
    id: &'a str,
    severity: &'static str,
    message: &'a str,
    location: Option<LocationWire<'a>>,
    related: Vec<RelatedWire<'a>>,
    help: Option<&'a str>,
}

#[derive(Serialize)]
struct RelatedWire<'a> {
    location: Option<LocationWire<'a>>,
    message: &'a str,
}

#[derive(Serialize)]
struct LocationWire<'a> {
    file: &'a str,
    span: Option<SpanWire>,
}

#[derive(Serialize)]
struct SpanWire {
    start: PositionWire,
    end: Option<PositionWire>,
}

#[derive(Serialize)]
struct PositionWire {
    line: u32,
    column: u32,
    offset: usize,
}

#[derive(Serialize)]
struct SummaryWire {
    error: usize,
    warning: usize,
    info: usize,
}

#[derive(Serialize)]
struct ExplainWire<'a> {
    schema_version: u32,
    report: &'static str,
    vault: ExplainVaultWire<'a>,
    contract: ContractBodyWire<'a>,
    provenance: Vec<ProvenanceWire<'a>>,
}

#[derive(Serialize)]
struct ExplainVaultWire<'a> {
    root: &'a str,
}

#[derive(Serialize)]
struct ContractBodyWire<'a> {
    contract_version: u32,
    dialect: DialectBodyWire,
    lifecycle: LifecycleBodyWire<'a>,
    flags: Vec<FlagWire<'a>>,
    types: Vec<TypeWire<'a>>,
}

#[derive(Serialize)]
struct DialectBodyWire {
    links: &'static str,
}

#[derive(Serialize)]
struct LifecycleBodyWire<'a> {
    declared: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    axis: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ordinary: Option<OrdinaryWire<'a>>,
}

#[derive(Serialize)]
struct FlagWire<'a> {
    property: &'a str,
}

#[derive(Serialize)]
struct TypeWire<'a> {
    name: &'a str,
    capabilities: Vec<&'static str>,
    properties: Vec<PropertyWire<'a>>,
    relationships: Vec<RelationshipWire<'a>>,
}

#[derive(Serialize)]
struct PropertyWire<'a> {
    name: &'a str,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    of: Option<&'static str>,
    required: bool,
}

#[derive(Serialize)]
struct RelationshipWire<'a> {
    predicate: &'a str,
    required: bool,
}

#[derive(Serialize)]
struct ProvenanceWire<'a> {
    key: &'a str,
    source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    contract_version: Option<u32>,
    location: Option<LocationWire<'a>>,
}

fn vault_wire(report: &DoctorReport) -> VaultWire<'_> {
    VaultWire {
        root: &report.root,
        selection: SelectionWire {
            how: report.selection.how().as_str(),
            requested: report.selection.requested(),
        },
    }
}

fn contract_facts_wire(facts: &ContractFacts) -> DoctorContractWire<'_> {
    DoctorContractWire {
        path: CONTRACT_PATH,
        present: facts.present,
        state: facts.state,
        unresolved_reason: facts.reason.as_deref(),
        version: version_wire(&facts.version),
    }
}

fn version_wire(facts: &VersionFacts) -> VersionWire {
    VersionWire {
        found: facts.found,
        supported: SupportedWire {
            min: facts.min,
            max: facts.max,
        },
        classification: facts.classification.map(|class| class.as_str()),
    }
}

fn installation_wire(facts: &InstallationFacts) -> InstallationWire<'_> {
    InstallationWire {
        path: FileRef::INSTALLATION_RECORD_PATH,
        state: facts.state,
        version: facts.version.as_ref().map(version_wire),
        actor: facts.actor.as_deref().map(|name| ActorWire { name }),
        registry_entry: facts.entry.as_ref().map(|entry| EntryWire {
            name: &entry.name,
            path: &entry.path,
        }),
    }
}

fn sections_wire(sections: &Sections) -> SectionsWire<'_> {
    SectionsWire {
        capabilities: section(sections, capabilities_wire),
        lifecycle: section(sections, lifecycle_wire),
        dialect: section(sections, dialect_wire),
    }
}

fn capabilities_wire(evaluated: &Evaluated) -> CapabilitiesWire<'_> {
    CapabilitiesWire {
        evaluated: true,
        types_declared: evaluated.types_declared,
        identity_bearing: &evaluated.identity_bearing,
        catch_all: evaluated.catch_all.as_deref(),
        closed_write: &evaluated.closed_write,
    }
}

fn lifecycle_wire(evaluated: &Evaluated) -> LifecycleWire<'_> {
    LifecycleWire {
        evaluated: true,
        declared: evaluated.lifecycle.declared(),
        axis: evaluated.lifecycle.axis(),
        ordinary: evaluated.lifecycle.ordinary().map(ordinary_wire),
    }
}

fn dialect_wire(evaluated: &Evaluated) -> DialectWire {
    DialectWire {
        evaluated: true,
        links: evaluated.links.as_str(),
    }
}

fn ordinary_wire(ordinary: &Ordinary) -> OrdinaryWire<'_> {
    match ordinary.value() {
        Some(value) => OrdinaryWire::Value { value },
        None => OrdinaryWire::Absent { absent: true },
    }
}

fn diagnostic_wire(diagnostic: &Diagnostic) -> DiagnosticWire<'_> {
    DiagnosticWire {
        id: diagnostic.id.as_str(),
        severity: diagnostic.severity.as_str(),
        message: &diagnostic.message,
        location: diagnostic.location.as_ref().map(location_wire),
        related: diagnostic.related.iter().map(related_wire).collect(),
        help: diagnostic.help.as_deref(),
    }
}

fn related_wire(related: &Related) -> RelatedWire<'_> {
    RelatedWire {
        location: related.location.as_ref().map(location_wire),
        message: &related.message,
    }
}

fn location_wire(location: &Location) -> LocationWire<'_> {
    LocationWire {
        file: location.file.display_path(),
        span: location.span.map(span_wire),
    }
}

fn span_wire(span: Span) -> SpanWire {
    SpanWire {
        start: position_wire(span.start),
        end: span.end.map(position_wire),
    }
}

fn position_wire(position: Position) -> PositionWire {
    PositionWire {
        line: position.line,
        column: position.column,
        offset: position.offset,
    }
}

fn summary_wire(counts: SeverityCounts) -> SummaryWire {
    SummaryWire {
        error: counts.error,
        warning: counts.warning,
        info: counts.info,
    }
}

fn contract_body_wire(contract: &Contract) -> ContractBodyWire<'_> {
    ContractBodyWire {
        contract_version: contract.contract_version(),
        dialect: DialectBodyWire {
            links: contract.dialect().links().as_str(),
        },
        lifecycle: lifecycle_body_wire(contract.lifecycle()),
        flags: contract
            .flags()
            .iter()
            .map(|flag| FlagWire {
                property: flag.property(),
            })
            .collect(),
        types: contract.types().iter().map(type_wire).collect(),
    }
}

fn lifecycle_body_wire(lifecycle: &LifecycleDecl) -> LifecycleBodyWire<'_> {
    LifecycleBodyWire {
        declared: lifecycle.declared(),
        axis: lifecycle.axis(),
        ordinary: lifecycle.ordinary().map(ordinary_wire),
    }
}

fn type_wire(declared: &TypeDecl) -> TypeWire<'_> {
    TypeWire {
        name: declared.name(),
        capabilities: declared
            .capabilities()
            .iter()
            .copied()
            .map(Capability::as_str)
            .collect(),
        properties: declared.properties().iter().map(property_wire).collect(),
        relationships: declared
            .relationships()
            .iter()
            .map(relationship_wire)
            .collect(),
    }
}

fn property_wire(declared: &PropertyDecl) -> PropertyWire<'_> {
    let kind: &PropertyKind = declared.kind();
    PropertyWire {
        name: declared.name(),
        kind: kind.as_str(),
        values: kind.values(),
        of: kind.element().map(ScalarKind::as_str),
        required: declared.required(),
    }
}

fn relationship_wire(declared: &RelationshipDecl) -> RelationshipWire<'_> {
    RelationshipWire {
        predicate: declared.predicate(),
        required: declared.required(),
    }
}

fn provenance_wire(entry: &ProvenanceEntry) -> ProvenanceWire<'_> {
    ProvenanceWire {
        key: &entry.key,
        source: entry.source.as_str(),
        contract_version: defining_version(entry.source),
        location: entry.location.as_ref().map(location_wire),
    }
}

/// The version whose format table supplied a value, for a value nobody wrote.
fn defining_version(source: Source) -> Option<u32> {
    match source {
        Source::Default { contract_version } => Some(contract_version),
        Source::Contract | Source::Installation => None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture::{
        ABSENT_ORDINARY, AWKWARD, Body, CLEAN, FIXTURES, NAMED_ORDINARY, RECORD, Tree, contract,
        no_record, opened, registering, rendered, shown,
    };
    use super::super::{Selection, SelectionRoute, doctor_report};
    use super::*;
    use crate::contract::UnresolvedReason;
    use crate::diagnostic::{KernelDiagnostic, Position, Span};
    use crate::installation::parse_installation;
    use crate::vault::open;
    use serde_json::Value;
    use std::fs;

    fn discovery() -> Selection {
        Selection::new(SelectionRoute::Discovery, None)
    }

    fn doctor_of(tree: &Tree, body: Body<'_>, record: Body<'_>) -> Value {
        parsed(&doctor_json(&doctor_report(
            &opened(tree, body, record),
            discovery(),
            &[],
        )))
    }

    fn parsed(rendered: &str) -> Value {
        serde_json::from_str(rendered).expect("this module's own output is JSON")
    }

    /// Where each top-level key of a pretty document begins.
    fn top_level_positions(rendered: &str, keys: &[&str]) -> Vec<usize> {
        keys.iter()
            .map(|key| {
                let missing = format!("no top-level `{key}` in {rendered}");
                rendered.find(&format!("\n  \"{key}\":")).expect(&missing)
            })
            .collect()
    }

    fn assert_key_order(rendered: &str, keys: &[&str]) {
        let positions = top_level_positions(rendered, keys);
        let mut sorted = positions.clone();
        sorted.sort_unstable();
        assert_eq!(positions, sorted, "keys are out of order: {keys:?}");
    }

    #[test]
    fn the_doctor_document_carries_its_keys_in_a_fixed_order() {
        let tree = Tree::new("json-order");
        let report = doctor_report(&opened(&tree, NAMED_ORDINARY, RECORD), discovery(), &[]);
        let json = doctor_json(&report);
        assert!(json.starts_with("{\n  \"schema_version\": 1,\n  \"report\": \"doctor\",\n"));
        assert_key_order(
            &json,
            &[
                "schema_version",
                "report",
                "vault",
                "contract",
                "installation",
                "sections",
                "diagnostics",
                "summary",
            ],
        );
        assert!(json.ends_with("}\n"));
    }

    #[test]
    fn a_healthy_vault_reports_every_fact_it_established() {
        let tree = Tree::new("json-healthy");
        let root = tree.vault(NAMED_ORDINARY);
        let expected = shown(root.path());
        let record = registering(&root);
        let report = doctor_report(
            &open(root, parse_installation(&record)),
            Selection::new(SelectionRoute::FlagName, Some("work".to_owned())),
            &[],
        );
        let json = parsed(&doctor_json(&report));
        let vault = &json["vault"];
        assert_eq!(
            (
                &vault["root"],
                &vault["selection"]["how"],
                &vault["selection"]["requested"]
            ),
            (
                &Value::from(expected),
                &Value::from("flag-name"),
                &Value::from("work")
            )
        );
        let contract = &json["contract"];
        assert_eq!(
            (
                &contract["path"],
                &contract["state"],
                &contract["unresolved_reason"]
            ),
            (
                &Value::from(".dogtag/contract.toml"),
                &Value::from("loaded"),
                &Value::Null
            )
        );
        let version = &contract["version"];
        assert_eq!(
            (
                &version["found"],
                &version["classification"],
                &version["supported"]["max"]
            ),
            (&Value::from(1), &Value::from("current"), &Value::from(1))
        );
        let installation = &json["installation"];
        assert_eq!(
            (
                &installation["path"],
                &installation["actor"]["name"],
                &installation["registry_entry"]["name"],
                &json["summary"]["error"]
            ),
            (
                &Value::from("$XDG_CONFIG_HOME/dogtag/installation.toml"),
                &Value::from("A Maintainer"),
                &Value::from("work"),
                &Value::from(0)
            )
        );
    }

    #[test]
    fn the_registry_entry_is_the_resolved_vaults_and_nothing_around_it() {
        let tree = Tree::new("json-registry");
        let root = tree.vault(CLEAN);
        let record = registering(&root);
        let report = doctor_report(&open(root, parse_installation(&record)), discovery(), &[]);
        let json = doctor_json(&report);
        assert!(
            !json.contains("elsewhere"),
            "the inventory must not reach the report: {json}"
        );
        let entry = &parsed(&json)["installation"]["registry_entry"];
        assert_eq!(entry.as_object().expect("an object").len(), 2);
    }

    #[test]
    fn an_absent_record_nulls_every_fact_that_would_have_come_from_it() {
        let tree = Tree::new("json-absent");
        let report = doctor_report(&open(tree.vault(CLEAN), no_record(&tree)), discovery(), &[]);
        let json = parsed(&doctor_json(&report));
        let installation = &json["installation"];
        assert_eq!(installation["state"], Value::from("absent"));
        let nulled = (
            &installation["version"],
            &installation["actor"],
            &installation["registry_entry"],
            &json["vault"]["selection"]["requested"],
        );
        assert_eq!(
            nulled,
            (&Value::Null, &Value::Null, &Value::Null, &Value::Null)
        );
    }

    #[test]
    fn an_evaluated_section_carries_its_answers_beside_the_discriminator() {
        let tree = Tree::new("json-evaluated");
        let json = doctor_of(&tree, ABSENT_ORDINARY, RECORD);
        let capabilities = &json["sections"]["capabilities"];
        assert_eq!(
            (
                &capabilities["evaluated"],
                &capabilities["types_declared"],
                &capabilities["catch_all"]
            ),
            (&Value::from(true), &Value::from(5), &Value::from("unfiled"))
        );
        let bearing = capabilities["identity_bearing"]
            .as_array()
            .expect("an array");
        assert_eq!(bearing[0], Value::from("person"));
        let lifecycle = &json["sections"]["lifecycle"];
        assert_eq!(
            (
                &lifecycle["ordinary"]["absent"],
                &lifecycle["axis"],
                &json["sections"]["dialect"]["links"]
            ),
            (
                &Value::from(true),
                &Value::from("standing"),
                &Value::from("wikilink")
            )
        );
    }

    #[test]
    fn a_corpus_with_no_life_axis_states_it_rather_than_nulling_the_section() {
        let tree = Tree::new("json-no-axis");
        let lifecycle = doctor_of(&tree, CLEAN, RECORD)["sections"]["lifecycle"].clone();
        let stated = (
            &lifecycle["evaluated"],
            &lifecycle["declared"],
            &lifecycle["axis"],
            &lifecycle["ordinary"],
        );
        assert_eq!(
            stated,
            (
                &Value::from(true),
                &Value::from("none"),
                &Value::Null,
                &Value::Null
            )
        );
    }

    /// A vault whose contract fails to resolve for each reason in turn.
    fn unresolved(tree: &Tree, reason: UnresolvedReason) -> Value {
        match reason {
            UnresolvedReason::Missing => {
                let root = tree.vault(CLEAN);
                fs::remove_file(root.contract_path()).expect("a contract this test owns");
                parsed(&doctor_json(&doctor_report(
                    &open(root, parse_installation(RECORD.as_str())),
                    discovery(),
                    &[],
                )))
            }
            UnresolvedReason::Unreadable => {
                let root = tree.vault(CLEAN);
                let contract = root.contract_path();
                fs::remove_file(&contract).expect("a contract this test owns");
                fs::create_dir(&contract).expect("a directory where the contract was");
                parsed(&doctor_json(&doctor_report(
                    &open(root, parse_installation(RECORD.as_str())),
                    discovery(),
                    &[],
                )))
            }
            UnresolvedReason::Encoding => {
                doctor_of(tree, Body::new("contract_version = 1\r\n"), RECORD)
            }
            UnresolvedReason::Malformed => {
                doctor_of(tree, Body::new("contract_version = = 1\n"), RECORD)
            }
            UnresolvedReason::VersionUnusable(_) => {
                doctor_of(tree, Body::new("contract_version = 2\n"), RECORD)
            }
            UnresolvedReason::Invalid => {
                doctor_of(tree, Body::new("contract_version = 1\n"), RECORD)
            }
        }
    }

    #[test]
    fn every_way_a_contract_fails_gives_every_section_the_same_shape() {
        let tree = Tree::new("json-not-evaluated");
        for reason in [
            UnresolvedReason::Missing,
            UnresolvedReason::Unreadable,
            UnresolvedReason::Encoding,
            UnresolvedReason::Malformed,
            UnresolvedReason::VersionUnusable(crate::compat::VersionClass::TooNew),
            UnresolvedReason::Invalid,
        ] {
            let json = unresolved(&tree, reason);
            assert_eq!(json["contract"]["state"], Value::from("unresolved"));
            assert_eq!(
                json["contract"]["unresolved_reason"],
                Value::from(reason.describe())
            );
            for name in ["capabilities", "lifecycle", "dialect"] {
                let shape = format!("`{name}` must be an object for {reason:?}");
                let section = json["sections"][name].as_object().expect(&shape);
                assert_eq!(
                    section.keys().collect::<Vec<_>>(),
                    ["evaluated", "reason"],
                    "a section with no answer carries exactly the discriminator and the reason"
                );
                assert_eq!(section["evaluated"], Value::from(false));
                assert_eq!(section["reason"], Value::from(reason.describe()));
            }
        }
    }

    #[test]
    fn a_diagnostic_renders_its_identifier_location_evidence_and_help() {
        let tree = Tree::new("json-diagnostics");
        let file = FileRef::InVault(".dogtag/contract.toml".to_owned());
        let planted = Diagnostic::kernel(KernelDiagnostic::ContractMultipleCatchAll, "two")
            .at(Location::in_file(
                file.clone(),
                Span::between(Position::new(4, 3, 31), Position::new(4, 9, 37)),
            ))
            .with_related(Related::new("also here").at(Location::whole_file(file)))
            .with_help("exactly one");
        let report = doctor_report(
            &opened(&tree, CLEAN, RECORD),
            discovery(),
            std::slice::from_ref(&planted),
        );
        let json = parsed(&doctor_json(&report));
        let diagnostic = &json["diagnostics"][0];
        assert_eq!(diagnostic["id"], Value::from("contract.multiple-catch-all"));
        assert_eq!(diagnostic["severity"], Value::from("error"));
        assert_eq!(diagnostic["help"], Value::from("exactly one"));
        assert_eq!(
            diagnostic["location"]["span"]["start"]["column"],
            Value::from(3)
        );
        assert_eq!(
            diagnostic["location"]["span"]["end"]["line"],
            Value::from(4)
        );
        assert_eq!(diagnostic["related"][0]["location"]["span"], Value::Null);
        assert_eq!(json["summary"]["error"], Value::from(1));
    }

    #[test]
    fn a_diagnostic_with_nothing_attached_nulls_rather_than_omits() {
        let tree = Tree::new("json-bare-diagnostic");
        let planted = Diagnostic::kernel(KernelDiagnostic::DiscoveryNestedVault, "an ancestor");
        let report = doctor_report(
            &opened(&tree, CLEAN, RECORD),
            discovery(),
            std::slice::from_ref(&planted),
        );
        // Asserted against the document rather than a parsed value, because
        // parsing sorts an object's keys and the order is the thing under test.
        let json = doctor_json(&report);
        assert!(
            json.contains(concat!(
                "      \"id\": \"discovery.nested-vault\",\n",
                "      \"severity\": \"warning\",\n",
                "      \"message\": \"an ancestor\",\n",
                "      \"location\": null,\n",
                "      \"related\": [],\n",
                "      \"help\": null\n",
            )),
            "a diagnostic with nothing attached still carries every key: {json}"
        );
        assert_eq!(parsed(&json)["summary"]["warning"], Value::from(1));
    }

    #[test]
    fn the_contract_document_carries_its_keys_in_a_fixed_order() {
        let tree = Tree::new("json-contract-order");
        let (root, contract) = rendered(&tree, NAMED_ORDINARY);
        let json = contract_json(&root, &contract);
        assert!(json.starts_with("{\n  \"schema_version\": 1,\n  \"report\": \"contract\",\n"));
        assert_key_order(
            &json,
            &[
                "schema_version",
                "report",
                "vault",
                "contract",
                "provenance",
            ],
        );
    }

    #[test]
    fn the_contract_document_renders_every_declaration_and_its_provenance() {
        let tree = Tree::new("json-contract");
        let (root, declared) = rendered(&tree, NAMED_ORDINARY);
        let json = parsed(&contract_json(&root, &declared));
        assert_eq!(json["vault"]["root"], Value::from(shown(root.path())));
        assert_eq!(json["contract"]["contract_version"], Value::from(1));
        assert_eq!(
            json["contract"]["dialect"]["links"],
            Value::from("wikilink")
        );
        assert_eq!(
            json["contract"]["lifecycle"]["ordinary"]["value"],
            Value::from("active")
        );
        let types = json["contract"]["types"].as_array().expect("an array");
        assert_eq!(types.len(), 3);
        assert_eq!(types[0]["name"], Value::from("note"));
        assert_eq!(types[0]["capabilities"][0], Value::from("catch-all"));
        assert_eq!(
            types[0]["properties"][0]["values"][0],
            Value::from("active")
        );
        assert_eq!(types[0]["properties"][1]["of"], Value::from("string"));
        assert_eq!(
            types[2]["relationships"][0]["predicate"],
            Value::from("involves")
        );
    }

    #[test]
    fn a_key_the_contract_does_not_declare_is_absent_rather_than_null() {
        let tree = Tree::new("json-contract-absence");
        let (root, declared) = rendered(&tree, NAMED_ORDINARY);
        let json = contract_json(&root, &declared);
        assert!(
            json.contains(concat!(
                "            \"name\": \"full_name\",\n",
                "            \"kind\": \"string\",\n",
                "            \"required\": true\n",
            )),
            "a `string` property declares neither `values` nor `of`: {json}"
        );
        let (bare_root, bare) = rendered(&tree, CLEAN);
        let bare_json = contract_json(&bare_root, &bare);
        assert!(
            bare_json.contains("    \"lifecycle\": {\n      \"declared\": \"none\"\n    },\n"),
            "a corpus with no axis declares no axis and nothing more: {bare_json}"
        );
        assert!(
            parsed(&bare_json)["contract"]["flags"]
                .as_array()
                .expect("an array")
                .is_empty()
        );
    }

    #[test]
    fn provenance_names_a_file_and_a_span_or_the_version_that_defaulted_it() {
        let tree = Tree::new("json-provenance");
        let (root, declared) = rendered(&tree, NAMED_ORDINARY);
        let document = contract_json(&root, &declared);
        assert!(
            document.contains(concat!(
                "      \"key\": \"dialect.links\",\n",
                "      \"source\": \"contract\",\n",
                "      \"location\": {\n",
                "        \"file\": \".dogtag/contract.toml\",\n",
                "        \"span\": {\n",
                "          \"start\": {\n",
                "            \"line\": 4,\n",
                "            \"column\": 9,\n",
                "            \"offset\": 40\n",
                "          },\n",
                "          \"end\": {\n",
                "            \"line\": 4,\n",
                "            \"column\": 19,\n",
                "            \"offset\": 50\n",
            )),
            "a written leaf names its file and its span, and no version: {document}"
        );
        assert!(
            document.contains(concat!(
                "      \"key\": \"type.project.capabilities\",\n",
                "      \"source\": \"default\",\n",
                "      \"contract_version\": 1,\n",
                "      \"location\": null\n",
            )),
            "a defaulted leaf names the version that defines it, and no file: {document}"
        );
    }

    #[test]
    fn provenance_renders_in_key_order() {
        let tree = Tree::new("json-provenance-order");
        let (root, declared) = rendered(&tree, ABSENT_ORDINARY);
        let json = parsed(&contract_json(&root, &declared));
        let keys: Vec<&str> = json["provenance"]
            .as_array()
            .expect("an array")
            .iter()
            .map(|entry| entry["key"].as_str().expect("a string"))
            .collect();
        let mut sorted = keys.clone();
        sorted.sort_unstable();
        assert_eq!(keys, sorted);
    }

    #[test]
    fn types_render_in_declaration_order_and_never_sorted() {
        let tree = Tree::new("json-declaration-order");
        let (root, declared) = rendered(&tree, ABSENT_ORDINARY);
        let json = parsed(&contract_json(&root, &declared));
        let names: Vec<&str> = json["contract"]["types"]
            .as_array()
            .expect("an array")
            .iter()
            .map(|kind| kind["name"].as_str().expect("a string"))
            .collect();
        let declared_order: Vec<&str> = declared
            .types()
            .iter()
            .map(crate::contract::TypeDecl::name)
            .collect();
        assert_eq!(names, declared_order);
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_ne!(names, sorted, "the fixture must not already be sorted");
    }

    #[test]
    fn a_corpus_vocabulary_that_has_to_be_escaped_round_trips() {
        let tree = Tree::new("json-escaping");
        let (root, declared) = rendered(&tree, AWKWARD);
        let rendered_json = contract_json(&root, &declared);
        let escapes = [
            r#"a \" quote"#,
            r#"a \\ backslash"#,
            r#"a \n break"#,
            "naïve",
        ];
        for escape in escapes {
            assert!(
                rendered_json.contains(escape),
                "`{escape}` did not survive: {rendered_json}"
            );
        }
        let json = parsed(&rendered_json);
        let values = json["contract"]["types"][0]["properties"][0]["values"]
            .as_array()
            .expect("an array");
        let declared_values = declared.types()[0].properties()[0]
            .kind()
            .values()
            .expect("an enum");
        assert_eq!(values.len(), declared_values.len());
        for (rendered_value, declared_value) in values.iter().zip(declared_values) {
            assert_eq!(rendered_value, &Value::from(declared_value.as_str()));
        }
        assert_eq!(json["contract"]["lifecycle"]["axis"], Value::from("état"));
    }

    #[test]
    fn the_same_input_renders_the_same_bytes_every_time() {
        let tree = Tree::new("json-deterministic");
        for (name, body) in FIXTURES {
            let (root, declared) = rendered(&tree, body);
            assert_eq!(
                contract_json(&root, &declared),
                contract_json(&root, &declared),
                "`{name}` did not render identically twice"
            );
            let report = doctor_report(&opened(&tree, body, RECORD), discovery(), &[]);
            assert_eq!(doctor_json(&report), doctor_json(&report), "`{name}`");
        }
    }

    #[test]
    fn a_contract_read_twice_from_the_same_bytes_renders_identically() {
        let tree = Tree::new("json-stable");
        let (root, first) = rendered(&tree, ABSENT_ORDINARY);
        let second = contract(ABSENT_ORDINARY);
        assert_eq!(contract_json(&root, &first), contract_json(&root, &second));
    }
}
