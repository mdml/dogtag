//! Where each resolved value came from.
//!
//! Provenance is recorded **per leaf value**, addressed by a stable dotted key
//! path, because the question worth answering mechanically is *"is this
//! property optional because the author decided so, or because nobody said?"* —
//! and the omitted field is precisely the one a per-node model cannot describe.
//!
//! There are three sources and no more. A format default is attributed to the
//! **contract version that defines it**, never to the SDK version: a default is
//! a property of the version the asset declares, so an unchanged vault cannot
//! acquire new semantics by upgrading the tool.

use core::fmt;
use std::collections::BTreeMap;

use crate::diagnostic::Location;

/// Where a resolved value came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Written explicitly in the committed vault contract.
    Contract,
    /// Written explicitly in the local installation record.
    Installation,
    /// Omitted, and filled from the declaring version's default table.
    Default {
        /// The contract version whose format defines the default.
        contract_version: u32,
    },
}

impl Source {
    /// The lowercase wire spelling, used by every structured format.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Contract => "contract",
            Self::Installation => "installation",
            Self::Default { .. } => "default",
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One resolved leaf value's provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvenanceEntry {
    /// The stable dotted key path, for example
    /// `type.person.property.full_name.required`.
    pub key: String,
    /// Where the value came from.
    pub source: Source,
    /// Where it is written, when it is written anywhere. A default carries no
    /// location, because there is no file to point at.
    pub location: Option<Location>,
}

impl ProvenanceEntry {
    /// An entry for a value written in a file.
    pub fn written(key: impl Into<String>, source: Source, location: Location) -> Self {
        Self {
            key: key.into(),
            source,
            location: Some(location),
        }
    }

    /// An entry for a value the declaring version's format default supplied.
    pub fn defaulted(key: impl Into<String>, contract_version: u32) -> Self {
        Self {
            key: key.into(),
            source: Source::Default { contract_version },
            location: None,
        }
    }
}

/// Every resolved leaf value's provenance, in key order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Provenance {
    entries: BTreeMap<String, ProvenanceEntry>,
}

impl Provenance {
    /// Empty provenance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records an entry, replacing any entry already held for its key.
    pub fn insert(&mut self, entry: ProvenanceEntry) {
        self.entries.insert(entry.key.clone(), entry);
    }

    /// The entry for a key path.
    pub fn get(&self, key: &str) -> Option<&ProvenanceEntry> {
        self.entries.get(key)
    }

    /// Every entry, in key order.
    pub fn entries(&self) -> impl Iterator<Item = &ProvenanceEntry> {
        self.entries.values()
    }

    /// How many entries are recorded.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether anything is recorded.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Absorbs another provenance, so an opened vault can report the contract's
    /// and the installation record's together, still in key order.
    pub fn merge(&mut self, other: Provenance) {
        self.entries.extend(other.entries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{FileRef, Position, Span};

    fn contract_location() -> Location {
        Location::in_file(
            FileRef::InVault(".dogtag/contract.toml".to_owned()),
            Span::at(Position::new(4, 9, 31)),
        )
    }

    fn record_location() -> Location {
        Location::in_file(
            FileRef::InstallationRecord,
            Span::at(Position::new(2, 8, 30)),
        )
    }

    #[test]
    fn sources_render_for_structured_output() {
        assert_eq!(Source::Contract.as_str(), "contract");
        assert_eq!(Source::Installation.to_string(), "installation");
        assert_eq!(
            Source::Default {
                contract_version: 1
            }
            .as_str(),
            "default"
        );
    }

    #[test]
    fn sources_clone_compare_and_format() {
        let sources = vec![
            Source::Contract,
            Source::Installation,
            Source::Default {
                contract_version: 1,
            },
        ];
        assert_eq!(sources.clone(), sources);
        assert_ne!(
            sources[2],
            Source::Default {
                contract_version: 2
            }
        );
        assert!(format!("{sources:?}").contains("contract_version"));
    }

    #[test]
    fn a_written_value_carries_its_file_and_span() {
        let entry =
            ProvenanceEntry::written("dialect.links", Source::Contract, contract_location());
        assert_eq!(entry.key, "dialect.links");
        assert_eq!(entry.source, Source::Contract);
        assert_eq!(entry.location, Some(contract_location()));
        assert_eq!(entry.clone(), entry);
        assert!(format!("{entry:?}").contains("dialect.links"));
    }

    #[test]
    fn a_default_names_the_contract_version_that_defines_it() {
        let entry = ProvenanceEntry::defaulted("type.person.capabilities", 1);
        assert_eq!(
            entry.source,
            Source::Default {
                contract_version: 1
            }
        );
        assert!(entry.location.is_none());
    }

    #[test]
    fn entries_iterate_in_key_order() {
        let mut provenance = Provenance::new();
        assert!(provenance.is_empty());
        provenance.insert(ProvenanceEntry::defaulted("type.person.capabilities", 1));
        provenance.insert(ProvenanceEntry::written(
            "dialect.links",
            Source::Contract,
            contract_location(),
        ));
        provenance.insert(ProvenanceEntry::written(
            "contract_version",
            Source::Contract,
            contract_location(),
        ));
        let keys: Vec<&str> = provenance.entries().map(|e| e.key.as_str()).collect();
        assert_eq!(
            keys,
            [
                "contract_version",
                "dialect.links",
                "type.person.capabilities"
            ]
        );
        assert_eq!(provenance.len(), 3);
        assert!(!provenance.is_empty());
    }

    #[test]
    fn a_key_is_looked_up_by_its_dotted_path() {
        let mut provenance = Provenance::default();
        provenance.insert(ProvenanceEntry::defaulted("actor.name", 1));
        assert_eq!(
            provenance.get("actor.name").map(|e| e.source),
            Some(Source::Default {
                contract_version: 1
            })
        );
        assert!(provenance.get("actor.absent").is_none());
    }

    #[test]
    fn inserting_a_key_twice_keeps_the_later_entry() {
        let mut provenance = Provenance::new();
        provenance.insert(ProvenanceEntry::defaulted("dialect.links", 1));
        provenance.insert(ProvenanceEntry::written(
            "dialect.links",
            Source::Contract,
            contract_location(),
        ));
        assert_eq!(provenance.len(), 1);
        assert_eq!(
            provenance.get("dialect.links").map(|e| e.source),
            Some(Source::Contract)
        );
    }

    #[test]
    fn merging_keeps_key_order_across_both_assets() {
        let mut contract = Provenance::new();
        contract.insert(ProvenanceEntry::written(
            "dialect.links",
            Source::Contract,
            contract_location(),
        ));
        let mut installation = Provenance::new();
        installation.insert(ProvenanceEntry::written(
            "actor.name",
            Source::Installation,
            record_location(),
        ));
        installation.insert(ProvenanceEntry::written(
            "vault.work.path",
            Source::Installation,
            record_location(),
        ));
        let cloned = contract.clone();
        contract.merge(installation);
        assert_ne!(contract, cloned);
        let keys: Vec<&str> = contract.entries().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["actor.name", "dialect.links", "vault.work.path"]);
        assert!(format!("{contract:?}").contains("vault.work.path"));
    }
}
