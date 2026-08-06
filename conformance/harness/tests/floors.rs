//! The M3 fixture floors, harness-enforced beside the M2 set: the version-2
//! constructs a real corpus asked for, the notes that exercise them, and the
//! irregularity the profile's own claim of realism demands.
//!
//! The M4 floors for `docs` sit here too, for the same reason the M3 ones do:
//! a profile's `PROFILE.md` states what its corpus is *for*, and until
//! something reads that statement the corpus can drift away from it while every
//! scenario stays green. `docs` exists to stress five axes, and each one below
//! is one of them made mechanical.

use std::collections::{BTreeMap, BTreeSet};

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

/// The named profile's corpus, read through the SDK against its own contract.
fn notes_of(profile: &str) -> dogtag::note::Corpus {
    let root = dogtag_conformance::profiles_dir()
        .join(profile)
        .join("corpus");
    let root = dogtag::vault::root_at(&root)
        .map(dogtag::vault::Resolved::into_root)
        .unwrap_or_else(|diagnostic| {
            panic!("the {profile} corpus is a vault root: {diagnostic:?}")
        });
    dogtag::note::read_corpus(&root, &contract_of(profile))
}

/// The dense corpus read through the SDK, for the note-level floors.
fn dense_notes() -> dogtag::note::Corpus {
    notes_of("dense")
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

// ---------------------------------------------------------------------------
// The M4 floors: `docs`.
//
// Its shape is derived from real repository documentation trees as proportions
// rather than as content, so the floors are stated as proportions too. Each one
// is a band rather than a number: a fixture whose shape drifts out of the band
// has stopped standing for what the profile says it stands for, and a fixture
// pinned to an exact count could not be edited at all.
// ---------------------------------------------------------------------------

/// Every note path in the docs corpus, and the directories they occupy.
fn docs_paths() -> Vec<String> {
    notes_of("docs")
        .notes()
        .iter()
        .map(|note| note.path().as_str().to_owned())
        .collect()
}

/// The directory each path sits in, the vault root spelled as the empty string.
fn directory_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(directory, _)| directory)
}

/// **Repository documentation shapes.** A tree of the right size and depth: a
/// corpus of a dozen notes in three folders would satisfy every other floor
/// here and stress nothing.
#[test]
fn docs_is_a_documentation_tree_of_the_size_its_record_states() {
    let paths = docs_paths();
    assert!(
        (35..=50).contains(&paths.len()),
        "docs holds 35-50 notes; it holds {}",
        paths.len()
    );
    let directories: BTreeSet<&str> = paths.iter().map(|path| directory_of(path)).collect();
    assert!(
        (12..=18).contains(&directories.len()),
        "docs spreads over 12-18 directories; it spreads over {}",
        directories.len()
    );
    let deepest = paths
        .iter()
        .map(|path| path.matches('/').count())
        .max()
        .unwrap_or(0);
    assert!(
        deepest >= 4,
        "docs reaches at least four directories deep; it reaches {deepest}"
    );
}

/// **Repeated basenames.** `README.md` recurs in a quarter to a third of the
/// directories, and it is not the only name that recurs — repeated names are
/// the normal case here rather than an edge case, which is the whole point of
/// the axis.
#[test]
fn docs_repeats_basenames_as_the_normal_case() {
    let paths = docs_paths();
    let directories: BTreeSet<&str> = paths.iter().map(|path| directory_of(path)).collect();
    let mut bearers: BTreeMap<&str, usize> = BTreeMap::new();
    for path in &paths {
        let name = path.rsplit('/').next().unwrap_or(path);
        *bearers.entry(name).or_default() += 1;
    }
    let readmes = bearers.get("README.md").copied().unwrap_or(0);
    let share = readmes as f64 / directories.len() as f64;
    assert!(
        (0.25..=0.35).contains(&share),
        "README.md recurs in a quarter to a third of directories; it is in {readmes} of {}",
        directories.len()
    );
    let repeated: Vec<&&str> = bearers
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(name, _)| name)
        .collect();
    assert!(
        repeated.len() >= 3,
        "at least three basenames recur, so no single name carries the axis; {repeated:?} do"
    );
}

/// **Most files carrying no frontmatter.** The majority of notes bind through
/// the declared catch-all rather than through anything they say, and the
/// lifecycle filter reaches them anyway — which it can only do because the
/// catch-all declares the axis.
#[test]
fn docs_binds_most_of_its_notes_through_the_declared_catch_all() {
    let contract = contract_of("docs");
    let corpus = notes_of("docs");
    let by_catch_all = corpus
        .notes()
        .iter()
        .filter(|note| note.binding().bound_by() == "catch-all")
        .count();
    assert!(
        by_catch_all * 2 > corpus.notes().len(),
        "most docs notes carry no discriminator; {by_catch_all} of {} do not",
        corpus.notes().len()
    );
    let axis = contract
        .lifecycle()
        .axis()
        .expect("docs declares a lifecycle axis");
    let catch_all = contract.catch_all().expect("docs declares a catch-all");
    assert!(
        catch_all.property(axis).is_some(),
        "the catch-all declares `{axis}`, or no frontmatter-less note could be filtered by it"
    );
    assert!(
        catch_all
            .properties()
            .iter()
            .all(|property| !property.required()),
        "a version-2 catch-all requires nothing, and every note with no frontmatter arrives here"
    );
}

/// **Markdown-link dialect.** Declared, written, and resolving: every reference
/// the corpus commits — typed link and prose alike — names a note, and both
/// halves of the resolution rule are exercised by references the corpus
/// actually wrote.
#[test]
fn docs_writes_markdown_references_that_all_resolve() {
    let contract = contract_of("docs");
    assert_eq!(
        contract.dialect().links().as_str(),
        "markdown",
        "docs is the markdown-link side of the dialect axis"
    );
    let corpus = notes_of("docs");
    let written: Vec<(&str, Option<bool>)> = corpus
        .notes()
        .iter()
        .flat_map(|note| {
            let edges = note
                .relationships()
                .iter()
                .flat_map(|relationship| relationship.edges())
                .map(|edge| (edge.written(), edge.target().is_some()));
            let references = note
                .body_references()
                .iter()
                .map(|reference| (reference.written(), reference.target().is_some()));
            edges
                .chain(references)
                .map(|(written, resolved)| (written, Some(resolved)))
        })
        .collect();
    assert!(
        written.len() >= 3 * corpus.notes().len(),
        "several references per note; {} references across {} notes",
        written.len(),
        corpus.notes().len()
    );
    // A reference off the web is one the SDK reads and no vault can hold: it
    // resolves to nothing and is excused for it, exactly as a prose reference
    // to a note that does not exist yet is. Every *internal* one must resolve.
    let external: Vec<&str> = written
        .iter()
        .map(|(reference, _)| *reference)
        .filter(|reference| is_external(target_of(reference)))
        .collect();
    assert!(
        (2..=8).contains(&external.len()),
        "a handful of references leave the vault, as a real docs tree's do; found {external:?}"
    );
    let dangling: Vec<&str> = written
        .iter()
        .filter(|(_, resolved)| *resolved == Some(false))
        .map(|(reference, _)| *reference)
        .filter(|reference| !is_external(target_of(reference)))
        .collect();
    assert!(
        dangling.is_empty(),
        "every committed reference that names a note resolves; these do not: {dangling:?}"
    );
    assert!(
        written
            .iter()
            .any(|(reference, _)| target_of(reference).contains('/')),
        "some references are path-qualified"
    );
    assert!(
        written.iter().any(|(reference, _)| {
            let target = target_of(reference);
            !target.contains('/') && !target.ends_with(".md")
        }),
        "and some are bare names, so both halves of the resolution rule are written down"
    );
    assert!(
        !corpus.notes().iter().any(|note| note.body().contains("[[")),
        "the corpus writes no wikilinks: a corpus declares one dialect and the other is bytes"
    );
}

/// Whether a target names something outside the vault altogether.
fn is_external(target: &str) -> bool {
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| target.starts_with(scheme))
}

/// The target a written reference names, with any fragment split off.
fn target_of(written: &str) -> &str {
    let inside = written
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(')'))
        .and_then(|rest| rest.rsplit_once("](").map(|(_, target)| target))
        .unwrap_or(written);
    inside.split_once('#').map_or(inside, |(target, _)| target)
}

/// **Folder organization, with no folder name reaching the core.** The tree
/// carries the meaning; the contract carries the declarations. Nothing the
/// contract declares is spelled the way a directory is, so no reader of the
/// contract alone could recover the folder structure — which is the property
/// the profile exists to hold, and the one a well-meaning edit would break
/// first.
#[test]
fn docs_lets_no_folder_name_reach_the_contract() {
    let contract = contract_of("docs");
    let directories: BTreeSet<String> = docs_paths()
        .iter()
        .flat_map(|path| {
            let directory = directory_of(path).to_owned();
            directory
                .split('/')
                .map(str::to_owned)
                .collect::<Vec<String>>()
        })
        .filter(|segment| !segment.is_empty())
        .collect();
    let mut declared: BTreeSet<String> = BTreeSet::new();
    for kind in contract.types() {
        declared.insert(kind.name().to_owned());
        declared.extend(kind.properties().iter().map(|p| p.name().to_owned()));
        declared.extend(
            kind.relationships()
                .iter()
                .map(|r| r.predicate().to_owned()),
        );
        declared.extend(
            kind.tag_namespaces()
                .iter()
                .map(|n| n.prefix().trim_end_matches('/').to_owned()),
        );
    }
    let shared: Vec<&String> = directories.intersection(&declared).collect();
    assert!(
        shared.is_empty(),
        "no declaration is spelled the way a directory is, or the folder-borne meaning would have \
         reached the core through the vocabulary: {shared:?}"
    );
    assert!(
        directories.len() >= 8,
        "and there is a real folder structure for that to be true of; found {directories:?}"
    );
}
