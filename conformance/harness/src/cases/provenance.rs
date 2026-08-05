//! `contract-loads-with-provenance`: every resolved leaf reports which source
//! supplied it.

use dogtag::contract::{Contract, LifecycleDecl, Ordinary, TypeDecl};
use dogtag::contract::{NamespaceMembership, PropertyKind, TagNamespaceDecl};
use dogtag::installation::{InstallationRecord, load_installation};
use dogtag::provenance::{Provenance, Source};
use dogtag::vault::open;

use super::corpus::{Corpus, require_record_loaded, valid_record};
use super::expect::{Checked, did_not_resolve, require, require_clean, require_same_names};

/// The three sources a resolved vault can attribute a value to.
///
/// All three must be reachable, which is why every fixture is required to omit
/// `required` on at least one property and `capabilities` on at least one type:
/// without an omission there is no `default` to attribute anything to, and the
/// distinction the whole feature exists for — *the author chose this* against
/// *nobody said* — would never be exercised.
const SOURCES: &[&str] = &["contract", "installation", "default"];

/// `contract-loads-with-provenance`.
pub fn contract_loads_with_provenance(corpus: &Corpus) -> Checked {
    let root = corpus.vault_root()?;
    let record = corpus.write_record("installation.toml", &valid_record(root.path()))?;
    let installation = load_installation(&record);
    require_record_loaded(&installation)?;

    let opened = open(root, installation);
    require_clean(opened.diagnostics(), "opening the vault with a record")?;
    let contract = opened
        .contract()
        .map_err(|why| did_not_resolve(why, "the contract"))?;

    every_declared_setting_is_exposed(contract, opened.installation().record())?;
    let merged = opened.provenance();
    covers_every_declared_leaf(contract, &merged)?;
    every_source_is_reachable(&merged)?;
    a_default_names_the_contract_version(contract, &merged)
}

/// The resolved configuration exposes every declared setting: each type by
/// name, each of its properties and relationships, the declared catch-all, the
/// lifecycle axis where one is declared, and the record's own settings beside
/// them.
fn every_declared_setting_is_exposed(
    contract: &Contract,
    record: Option<&InstallationRecord>,
) -> Checked {
    types_are_exposed(contract)?;
    lifecycle_axis_is_exposed(contract)?;
    record_is_exposed(record)
}

/// Each declared type is reachable by name, with everything it declares, and
/// the catch-all the cardinality rule guarantees is reachable as such.
fn types_are_exposed(contract: &Contract) -> Checked {
    require(!contract.types().is_empty(), || {
        "the resolved contract must expose the declared types, and exposes none".to_owned()
    })?;
    for declared in contract.types() {
        require(contract.type_named(declared.name()).is_some(), || {
            format!("the type `{}` is not reachable by name", declared.name())
        })?;
        declarations_are_reachable(declared)?;
    }
    require(contract.catch_all().is_some(), || {
        format!(
            "the resolved contract must expose the declared catch-all type, and exposes none of \
             its {} types as one",
            contract.types().len()
        )
    })
}

/// A declared axis names a property the resolved model can answer about.
fn lifecycle_axis_is_exposed(contract: &Contract) -> Checked {
    let Some(axis) = contract.lifecycle().axis() else {
        return Ok(());
    };
    require(contract.property_kind(axis).is_some(), || {
        format!("the declared lifecycle axis `{axis}` is not reachable")
    })
}

/// The record's own settings are exposed beside the contract's.
fn record_is_exposed(record: Option<&InstallationRecord>) -> Checked {
    let record = record.ok_or_else(|| "the installation record must be exposed".to_owned())?;
    require(
        record.actor().is_some() && !record.vaults().is_empty(),
        || {
            format!(
                "the resolved record must expose the actor and the registry entry it declares, \
                 and exposes {} actor with {} registry entries",
                record.actor().iter().count(),
                record.vaults().len()
            )
        },
    )
}

/// One type's properties and relationships are each reachable by the name the
/// contract declared them under.
fn declarations_are_reachable(declared: &TypeDecl) -> Checked {
    for property in declared.properties() {
        require(declared.property(property.name()).is_some(), || {
            format!("the property `{}` is not reachable", property.name())
        })?;
    }
    for relationship in declared.relationships() {
        require(
            declared.relationship(relationship.predicate()).is_some(),
            || {
                format!(
                    "the relationship `{}` is not reachable",
                    relationship.predicate()
                )
            },
        )?;
    }
    Ok(())
}

/// The provenance covers exactly the leaves the two assets declare: every
/// declaration is attributed, and nothing is attributed that was not declared.
fn covers_every_declared_leaf(contract: &Contract, merged: &Provenance) -> Checked {
    let mut expected = contract_keys(contract);
    expected.extend(record_keys());
    let recorded: Vec<String> = merged.entries().map(|entry| entry.key.clone()).collect();
    require_same_names(&expected, &recorded, "the resolved vault's provenance")
}

/// All three sources appear, so `default` is genuinely reachable rather than a
/// variant nothing produces.
fn every_source_is_reachable(merged: &Provenance) -> Checked {
    for source in SOURCES {
        require(
            merged
                .entries()
                .any(|entry| entry.source.as_str() == *source),
            || format!("no resolved value is attributed to the `{source}` source"),
        )?;
    }
    Ok(())
}

/// A format default is attributed to **the contract version that defines it**,
/// never to the SDK's own version: a default is a property of the version the
/// asset declares, so an unchanged vault cannot acquire new semantics by
/// upgrading the tool.
fn a_default_names_the_contract_version(contract: &Contract, merged: &Provenance) -> Checked {
    let declared = contract.contract_version();
    for entry in merged.entries() {
        if let Source::Default { contract_version } = entry.source {
            require(contract_version == declared, || {
                format!(
                    "`{}` is defaulted from version {contract_version}, but the contract declares \
                     version {declared}",
                    entry.key
                )
            })?;
            require(entry.location.is_none(), || {
                format!("`{}` is a default, so it points at no file", entry.key)
            })?;
        }
    }
    Ok(())
}

/// Every leaf key the contract declares, per the version-1 key space.
///
/// Derived from the **resolved model**, never from the provenance map, which is
/// what lets an assertion prove the map covers the contract rather than merely
/// that a renderer copied it. Shared with the explain case for that reason.
pub(super) fn contract_keys(contract: &Contract) -> Vec<String> {
    let mut keys = vec!["contract_version".to_owned(), "dialect.links".to_owned()];
    keys.extend(lifecycle_keys(contract.lifecycle()));
    if let Some(tags) = contract.tags() {
        let _ = tags;
        keys.push("tags.property".to_owned());
    }
    keys.extend(
        contract
            .flags()
            .iter()
            .map(|flag| format!("flag.{}.property", flag.property())),
    );
    for declared in contract.types() {
        keys.extend(type_keys(declared));
    }
    keys
}

/// The lifecycle leaves, which differ by which declaration was made.
fn lifecycle_keys(lifecycle: &LifecycleDecl) -> Vec<String> {
    match (lifecycle.axis(), lifecycle.ordinary()) {
        (Some(_), Some(Ordinary::Absent)) => vec![
            "lifecycle.axis".to_owned(),
            "lifecycle.ordinary.absent".to_owned(),
        ],
        (Some(_), Some(Ordinary::Value(_))) => vec![
            "lifecycle.axis".to_owned(),
            "lifecycle.ordinary.value".to_owned(),
        ],
        _ => vec!["lifecycle.none".to_owned()],
    }
}

/// One type's leaves: its own, then its properties' and relationships'.
fn type_keys(declared: &TypeDecl) -> Vec<String> {
    let name = declared.name();
    let mut keys = vec![
        format!("type.{name}.name"),
        format!("type.{name}.capabilities"),
    ];
    for property in declared.properties() {
        let at = format!("type.{name}.property.{}", property.name());
        keys.extend([
            format!("{at}.name"),
            format!("{at}.kind"),
            format!("{at}.required"),
        ]);
        keys.extend(property.kind().values().map(|_| format!("{at}.values")));
        keys.extend(property.kind().element().map(|_| format!("{at}.of")));
        if matches!(property.kind(), PropertyKind::ListOfRecord { .. }) {
            keys.push(format!("{at}.of"));
        }
        for field in property.kind().fields().into_iter().flatten() {
            let field_at = format!("{at}.field.{}", field.name());
            keys.extend([
                format!("{field_at}.name"),
                format!("{field_at}.kind"),
                format!("{field_at}.required"),
            ]);
            keys.extend(field.kind().values().map(|_| format!("{field_at}.values")));
        }
    }
    for relationship in declared.relationships() {
        let at = format!("type.{name}.relationship.{}", relationship.predicate());
        keys.extend([format!("{at}.predicate"), format!("{at}.required")]);
    }
    for namespace in declared.tag_namespaces() {
        keys.extend(namespace_keys(name, namespace));
    }
    keys
}

/// One tag namespace's leaves: every namespace has a prefix and a
/// requiredness; a closed one has its vocabulary, an open one its openness.
fn namespace_keys(type_name: &str, namespace: &TagNamespaceDecl) -> Vec<String> {
    let at = format!("type.{type_name}.tag-namespace.{}", namespace.prefix());
    let mut keys = vec![format!("{at}.prefix"), format!("{at}.required")];
    match namespace.membership() {
        NamespaceMembership::Closed { .. } => keys.push(format!("{at}.values")),
        NamespaceMembership::Open => keys.push(format!("{at}.open")),
    }
    keys
}

/// The installation record's leaves, for the record [`valid_record`] writes.
///
/// The record is written by the harness rather than derived from a profile, so
/// its key set is known exactly. That is honest rather than convenient: one
/// machine-local record serves every profile, and there is no per-profile
/// source to derive one from.
fn record_keys() -> Vec<String> {
    [
        "installation_version",
        "actor.name",
        "vault.fixture.name",
        "vault.fixture.path",
    ]
    .iter()
    .map(|key| (*key).to_owned())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use dogtag::contract::parse_contract;
    use dogtag::diagnostic::{FileRef, Location};
    use dogtag::provenance::ProvenanceEntry;

    use super::super::corpus::NO_AXIS;

    /// A record that declares an actor and registers no vault. It loads: the
    /// registry is not mandatory, which is why the assertion has to look.
    const ACTOR_ONLY: &str = "installation_version = 1\n\n[actor]\nname = \"A Maintainer\"\n";

    /// One leaf key, so a synthetic provenance is about a real key path.
    const LEAF: &str = "type.capture.capabilities";

    /// A contract that loads clean and declares no life axis.
    fn no_axis() -> Contract {
        parse_contract(NO_AXIS)
            .contract
            .expect("a contract declaring no life axis loads")
    }

    /// A corpus that declares no axis has no axis property to expose, and that
    /// is an answer rather than an omission.
    #[test]
    fn a_corpus_with_no_axis_exposes_no_axis_property() {
        assert_eq!(lifecycle_axis_is_exposed(&no_axis()), Ok(()));
    }

    /// The leaves recorded for it are the declaration it actually made.
    #[test]
    fn a_corpus_with_no_axis_records_the_declaration_it_made() {
        let contract = no_axis();
        assert_eq!(
            lifecycle_keys(contract.lifecycle()),
            vec!["lifecycle.none".to_owned()]
        );
    }

    /// A record that is not exposed at all is refused.
    #[test]
    fn a_record_that_is_not_exposed_is_refused() {
        let detail =
            record_is_exposed(None).expect_err("a record that is not exposed cannot be inspected");
        assert!(
            detail.contains("the installation record must be exposed"),
            "the failure says what is missing: {detail}"
        );
    }

    /// A record exposing only half of what it owns is refused too: the
    /// registry entry is as much the record's business as the actor is.
    #[test]
    fn a_record_declaring_half_of_what_it_owns_is_refused() {
        let corpus = Corpus::holding("provenance-half-a-record", NO_AXIS);
        for (name, text) in [
            ("actor-only.toml", ACTOR_ONLY.to_owned()),
            ("registry-only.toml", registry_only(&corpus)),
        ] {
            let path = corpus
                .write_record(name, &text)
                .expect("a record is written beside the vault");
            let installation = load_installation(&path);
            let detail = record_is_exposed(installation.record())
                .expect_err("half a record exposes half of what the record owns");
            assert!(
                detail.contains("the actor and the registry entry"),
                "the failure says what was wanted: {detail}"
            );
        }
    }

    /// A record that registers this vault and names no actor. It loads too:
    /// the record owns two things, and declaring one of them is legal.
    fn registry_only(corpus: &Corpus) -> String {
        format!(
            "installation_version = 1\n\n[[vault]]\nname = \"fixture\"\npath = \"{}\"\n",
            corpus.root().display()
        )
    }

    /// All three sources must be reachable, and the one nothing was attributed
    /// to is named — `default` unreached is the whole reason the fixtures are
    /// required to omit something.
    #[test]
    fn a_source_nothing_is_attributed_to_is_named() {
        let mut provenance = Provenance::new();
        provenance.insert(ProvenanceEntry::defaulted(LEAF, 1));
        let detail = every_source_is_reachable(&provenance)
            .expect_err("a provenance of one default reaches one source");
        assert!(
            detail.contains("attributed to the `contract` source"),
            "the failure names the source: {detail}"
        );
    }

    /// A default is attributed to the contract version that defines it, never
    /// to the SDK's: an unchanged vault cannot acquire new semantics by
    /// upgrading the tool.
    #[test]
    fn a_default_from_another_version_is_refused() {
        let mut provenance = Provenance::new();
        provenance.insert(ProvenanceEntry::defaulted(LEAF, 99));
        let detail = a_default_names_the_contract_version(&no_axis(), &provenance)
            .expect_err("a default from version 99 is not this contract's default");
        assert!(
            detail.contains("defaulted from version 99"),
            "the failure names the version recorded: {detail}"
        );
        assert!(
            detail.contains("declares version 2"),
            "the failure names the version declared: {detail}"
        );
    }

    /// ...and it points at no file, because a value nobody wrote is written
    /// nowhere.
    ///
    /// *Which* file it points at is not what is wrong, so the reference here is
    /// the one an external crate can name without a vault: an in-vault path is
    /// spelled by [`dogtag::vault::VaultRoot::relative`] and there is no root in
    /// scope, and inventing one would say something this assertion never reads.
    #[test]
    fn a_default_pointing_at_a_file_is_refused() {
        let mut provenance = Provenance::new();
        provenance.insert(ProvenanceEntry::written(
            LEAF,
            Source::Default {
                contract_version: 2,
            },
            Location::whole_file(FileRef::InstallationRecord),
        ));
        let detail = a_default_names_the_contract_version(&no_axis(), &provenance)
            .expect_err("a default carrying a location points at a file");
        assert!(
            detail.contains("points at no file"),
            "the failure says what is wrong: {detail}"
        );
    }
}
