//! The deterministic total order diagnostics are reported in.
//!
//! Never discovery order, which varies with the filesystem. The tiers are:
//!
//! 1. diagnostics with **no location** first, ordered by identifier;
//! 2. then located ones: [`FileRef::InVault`] before
//!    [`FileRef::InstallationRecord`], and within `InVault` by path, so the two
//!    path kinds never interleave ambiguously;
//! 3. then line, then column, then byte offset, with a location carrying no
//!    span sorting before one that does;
//! 4. then identifier.
//!
//! Tiers 1 to 3 are exactly the derived ordering on `Option<Location>`, which
//! is why [`Location`] and its parts derive `Ord` rather than open-coding a
//! comparison here.
//!
//! [`FileRef::InVault`]: super::FileRef::InVault
//! [`FileRef::InstallationRecord`]: super::FileRef::InstallationRecord
//! [`Location`]: super::Location

use core::cmp::Ordering;

use super::Diagnostic;

/// Compares two diagnostics in the total order.
///
/// Pass this to a **stable** sort — [`DiagnosticList::sorted`] does — so exact
/// ties keep emission order.
///
/// [`DiagnosticList::sorted`]: super::DiagnosticList::sorted
pub fn compare(left: &Diagnostic, right: &Diagnostic) -> Ordering {
    left.location
        .cmp(&right.location)
        .then_with(|| left.id.as_str().cmp(right.id.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{
        DiagnosticList, FileRef, KernelDiagnostic, Location, Position, Span, VaultPath,
    };

    fn vault(path: &'static str) -> FileRef {
        FileRef::InVault(VaultPath::kernel(path))
    }

    fn contract() -> FileRef {
        vault(".dogtag/contract.toml")
    }

    fn at(file: FileRef, line: u32, column: u32, offset: usize) -> Location {
        Location::in_file(file, Span::at(Position::new(line, column, offset)))
    }

    /// A diagnostic under `kind`, whose message repeats the identifier so the
    /// sorted order is readable in an assertion.
    fn raised(kind: KernelDiagnostic, location: Option<Location>) -> Diagnostic {
        let diagnostic = Diagnostic::kernel(kind, kind.id());
        match location {
            Some(location) => diagnostic.at(location),
            None => diagnostic,
        }
    }

    fn sorted_messages(diagnostics: [Diagnostic; 2]) -> Vec<String> {
        let mut list = DiagnosticList::new();
        list.extend(diagnostics);
        list.sorted().iter().map(|d| d.message.clone()).collect()
    }

    /// Emits two diagnostics under one identifier, `later` first, and asserts
    /// the order puts `earlier` first. Only the location can decide.
    fn assert_location_sorts_first(earlier: Location, later: Location) {
        let kind = KernelDiagnostic::ContractUnknownKey;
        let mut list = DiagnosticList::new();
        list.extend([
            Diagnostic::kernel(kind, "later").at(later.clone()),
            Diagnostic::kernel(kind, "earlier").at(earlier.clone()),
        ]);
        let sorted = list.sorted();
        assert_eq!(sorted[0].location, Some(earlier));
        assert_eq!(sorted[1].location, Some(later));
    }

    #[test]
    fn tier_one_puts_unlocated_diagnostics_first_ordered_by_identifier() {
        let messages = sorted_messages([
            raised(KernelDiagnostic::DiscoveryNoVaultFound, None),
            raised(KernelDiagnostic::CompatContractTooNew, None),
        ]);
        assert_eq!(
            messages,
            ["compat.contract-too-new", "discovery.no-vault-found"]
        );
    }

    #[test]
    fn tier_one_puts_every_unlocated_diagnostic_before_a_located_one() {
        let messages = sorted_messages([
            raised(
                KernelDiagnostic::ContractNoTypes,
                Some(at(contract(), 1, 1, 0)),
            ),
            raised(KernelDiagnostic::DiscoveryNoVaultFound, None),
        ]);
        assert_eq!(messages, ["discovery.no-vault-found", "contract.no-types"]);
    }

    #[test]
    fn tiers_two_and_three_order_by_file_then_by_position() {
        let cases = [
            // in-vault paths before the installation record
            (
                at(contract(), 9, 9, 99),
                at(FileRef::InstallationRecord, 1, 1, 0),
            ),
            // within the vault, by path
            (
                at(vault(".dogtag/a.toml"), 9, 9, 99),
                at(vault(".dogtag/z.toml"), 1, 1, 0),
            ),
            // no span before any span
            (Location::whole_file(contract()), at(contract(), 1, 1, 0)),
            // then line, then column, then byte offset
            (at(contract(), 2, 7, 20), at(contract(), 4, 1, 40)),
            (at(contract(), 2, 3, 16), at(contract(), 2, 7, 20)),
            (at(contract(), 2, 3, 16), at(contract(), 2, 3, 17)),
        ];
        for (earlier, later) in cases {
            assert_location_sorts_first(earlier, later);
        }
    }

    #[test]
    fn tier_four_breaks_a_shared_location_by_identifier() {
        let location = at(contract(), 3, 3, 30);
        let messages = sorted_messages([
            raised(KernelDiagnostic::ContractUnknownKey, Some(location.clone())),
            raised(KernelDiagnostic::ContractMissingKey, Some(location)),
        ]);
        assert_eq!(messages, ["contract.missing-key", "contract.unknown-key"]);
    }

    #[test]
    fn an_exact_tie_keeps_emission_order() {
        let location = at(contract(), 5, 5, 50);
        let kind = KernelDiagnostic::ContractUnknownKey;
        let first = Diagnostic::kernel(kind, "emitted first").at(location.clone());
        let second = Diagnostic::kernel(kind, "emitted second").at(location);
        assert_eq!(compare(&first, &second), Ordering::Equal);
        let messages = sorted_messages([first, second]);
        assert_eq!(messages, ["emitted first", "emitted second"]);
    }
}
