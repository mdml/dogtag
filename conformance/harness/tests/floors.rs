//! The M3 fixture floors, harness-enforced beside the M2 set: the version-2
//! constructs a real corpus asked for, the notes that exercise them, and the
//! irregularity the profile's own claim of realism demands.

use std::collections::BTreeSet;

use dogtag::contract::{Capability, Contract, load_contract};

/// The named profile's committed contract, resolved.
fn contract_of(profile: &str) -> Contract {
    let path = dogtag_conformance::profiles_dir()
        .join(profile)
        .join("corpus/.dogtag/contract.toml");
    load_contract(&path)
        .contract
        .clone()
        .unwrap_or_else(|_| panic!("the {profile} corpus holds a contract that loads"))
}

/// The dense corpus read through the SDK, for the note-level floors.
fn dense_notes() -> dogtag::note::Corpus {
    let root = dogtag_conformance::profiles_dir().join("dense/corpus");
    let root = dogtag::vault::root_at(&root)
        .map(dogtag::vault::Resolved::into_root)
        .unwrap_or_else(|diagnostic| panic!("the dense corpus is a vault root: {diagnostic:?}"));
    dogtag::note::read_corpus(&root, &contract_of("dense"))
}

/// The M3 construct floors: the version-2 constructs a real corpus asked for
/// are declared, and declared where the record says.
#[test]
fn dense_declares_the_version_two_constructs_the_floor_names() {
    use dogtag::contract::{NamespaceMembership, PropertyKind};
    let dense = contract_of("dense");
    assert!(dense.tags().is_some(), "dense declares a [tags] table");
    let kinds = |predicate: fn(&PropertyKind) -> bool| {
        dense
            .types()
            .iter()
            .flat_map(|t| t.properties())
            .filter(|p| predicate(p.kind()))
            .count()
    };
    assert!(
        kinds(|kind| matches!(kind, PropertyKind::Record { .. })) >= 1,
        "at least one record property"
    );
    assert!(
        dense
            .types_with(Capability::IdentityBearing)
            .flat_map(|t| t.properties())
            .any(|p| matches!(p.kind(), PropertyKind::ListOfRecord { .. })),
        "at least one list of record on an identity-bearing type"
    );
    let namespaces: Vec<_> = dense
        .types()
        .iter()
        .flat_map(|t| t.tag_namespaces())
        .collect();
    assert!(
        namespaces
            .iter()
            .any(|n| matches!(n.membership(), NamespaceMembership::Closed { .. })),
        "at least one closed namespace"
    );
    assert!(
        namespaces
            .iter()
            .any(|n| matches!(n.membership(), NamespaceMembership::Open)),
        "at least one open namespace"
    );
    assert!(
        namespaces.iter().any(|n| n.required()),
        "at least one required namespace"
    );
}

/// The notes exercise what the contract declares: a conforming record value, a
/// list of records, and a tag in every declared namespace.
#[test]
fn dense_notes_exercise_the_declared_constructs() {
    use dogtag::contract::PropertyKind;
    let dense = contract_of("dense");
    let corpus = dense_notes();
    let carried = |predicate: fn(&PropertyKind) -> bool| {
        corpus.notes().iter().any(|note| {
            let Some(declared) = note
                .binding()
                .type_name()
                .and_then(|name| dense.type_named(name))
            else {
                return false;
            };
            declared.properties().iter().any(|property| {
                predicate(property.kind()) && note.property(property.name()).is_some()
            })
        })
    };
    assert!(
        carried(|kind| matches!(kind, PropertyKind::Record { .. })),
        "a note carries a conforming record value"
    );
    assert!(
        carried(|kind| matches!(kind, PropertyKind::ListOfRecord { .. })),
        "a note carries a list of records"
    );
    for namespace in dense.types().iter().flat_map(|t| t.tag_namespaces()) {
        let prefix = namespace.prefix();
        assert!(
            corpus
                .notes()
                .iter()
                .flat_map(|note| note.tags())
                .any(|tag| tag.starts_with(prefix)),
            "a note carries a tag in the declared `{prefix}` namespace"
        );
    }
}

/// The mechanized irregularity floors: a zero-note type, a never-used optional
/// property, and a deliberately non-uniform notes-per-type distribution.
#[test]
fn dense_is_irregular_where_the_floor_demands() {
    let dense = contract_of("dense");
    let corpus = dense_notes();
    let mut per_type: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for note in corpus.notes() {
        if let Some(name) = note.binding().type_name() {
            *per_type.entry(name).or_default() += 1;
        }
    }
    assert!(
        dense
            .types()
            .iter()
            .any(|t| !per_type.contains_key(t.name())),
        "at least one declared type has zero notes"
    );
    let unused_optional = dense.types().iter().any(|declared| {
        declared.properties().iter().any(|property| {
            !property.required()
                && corpus.notes().iter().all(|note| {
                    note.binding().type_name() != Some(declared.name())
                        || note.property(property.name()).is_none()
                })
        })
    });
    assert!(
        unused_optional,
        "at least one optional property is used by no note"
    );
    let distinct: BTreeSet<usize> = per_type.values().copied().collect();
    assert!(
        distinct.len() >= 3,
        "notes-per-type is deliberately non-uniform; found counts {distinct:?}"
    );
}
