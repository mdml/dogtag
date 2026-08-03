//! The contract-loading cases: a conforming contract, capability cardinality,
//! and the mandatory lifecycle declaration.

use dogtag::contract::{Contract, Ordinary};

use crate::transform::{
    Transformed, delete_lifecycle_table, drop_catch_all, duplicate_catch_all,
    flip_property_required, replace_lifecycle_with_none,
};

use super::corpus::Corpus;
use super::expect::{Checked, did_not_resolve, require_clean, require_id};

/// `conforming-contract-loads-with-zero-diagnostics`.
///
/// The bar is zero diagnostics **at any severity**, not merely no errors: a
/// contract that loads while emitting advice is not a conforming contract, and
/// `starter` is the definition of what a fresh install stamps.
///
/// [`dogtag::vault::inspect_root_trust`] is deliberately not called. Trust is a
/// property of where the vault sits relative to the *user's* home directory,
/// which is not a property of the contract, and this copy lives outside any
/// home directory by construction.
pub fn conforming_contract(corpus: &Corpus) -> Checked {
    let opened = corpus.opened_without_a_record()?;
    require_clean(opened.diagnostics(), "opening the vault")?;
    opened
        .contract()
        .map(|_| ())
        .map_err(|why| did_not_resolve(why, "the contract"))
}

/// `capability-cardinality-enforced`.
///
/// The one-catch-all case is the untransformed contract loading clean; the
/// zero- and two-catch-all cases are derived from it, so each profile's own
/// vocabulary carries the assertion.
pub fn capability_cardinality(corpus: &Corpus) -> Checked {
    corpus.clean_contract()?;
    derived_reports(
        corpus,
        "drop-catch-all",
        drop_catch_all,
        "contract.missing-catch-all",
    )?;
    derived_reports(
        corpus,
        "duplicate-catch-all",
        duplicate_catch_all,
        "contract.multiple-catch-all",
    )
}

/// `lifecycle-declaration-is-mandatory`.
///
/// Three declarations, all derived from the profile's own contract: an axis
/// (untransformed), no axis at all (`none = true`, which must load exactly as
/// well), and no `[lifecycle]` table (which must not).
pub fn lifecycle_declaration(corpus: &Corpus) -> Checked {
    let contract = corpus.clean_contract()?;
    derived_reports(
        corpus,
        "delete-lifecycle",
        delete_lifecycle_table,
        "contract.missing-lifecycle",
    )?;
    let none = corpus.derived("lifecycle-none", replace_lifecycle_with_none)?;
    require_clean(
        &none.load()?.diagnostics,
        "a contract declaring no life axis",
    )?;
    axis_consistency(corpus, &contract)
}

/// The ordinary encoding must agree with whether the axis property is required.
///
/// Which diagnostic that yields is read from the **declaration**, never from a
/// corpus's vocabulary: a corpus whose ordinary state is the absence of a value
/// breaks when the axis becomes required somewhere, and one whose ordinary
/// state is a named value breaks when the axis becomes optional somewhere.
fn axis_consistency(corpus: &Corpus, contract: &Contract) -> Checked {
    let lifecycle = contract.lifecycle();
    let (Some(axis), Some(ordinary)) = (lifecycle.axis(), lifecycle.ordinary()) else {
        return Err("the fixture must declare a lifecycle axis to contradict".to_owned());
    };
    let expected = match ordinary {
        Ordinary::Absent => "contract.lifecycle-ordinary-absent-required",
        Ordinary::Value(_) => "contract.lifecycle-ordinary-value-optional",
    };
    let axis = axis.to_owned();
    derived_reports(
        corpus,
        "axis-required-contradicted",
        move |text| flip_property_required(text, &axis),
        expected,
    )
}

/// Derives a contract, loads it, and requires `id` among what it reported.
///
/// The other two assertions every derived case owes — the untransformed
/// contract loads clean, and the transformed bytes differ — are made by
/// [`Corpus::derived`].
fn derived_reports(
    corpus: &Corpus,
    label: &str,
    transform: impl Fn(&str) -> Transformed,
    id: &str,
) -> Checked {
    let derived = corpus.derived(label, transform)?;
    require_id(
        &derived.load()?.diagnostics,
        id,
        &format!("the `{label}` derived contract"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::corpus::NO_AXIS;

    /// A contract whose catch-all is declared across several lines: valid
    /// TOML, and a spelling the textual transformation cannot see.
    const CATCH_ALL_ACROSS_LINES: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\n  \"catch-all\",\n]\n",
    );

    /// The same, with the lifecycle header spelled with inner spaces: also
    /// valid TOML, and also invisible to a transformation matching the header
    /// literally.
    const SPACED_LIFECYCLE_HEADER: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[ lifecycle ]\nnone = true\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    /// A derived case whose transformation cannot find its target **fails the
    /// pair**. It never passes vacuously — a contract this suite could not
    /// transform is a contract this suite did not test.
    #[test]
    fn a_catch_all_the_transformation_cannot_see_fails_the_capability_case() {
        let corpus = Corpus::holding("contract-catch-all-across-lines", CATCH_ALL_ACROSS_LINES);
        corpus
            .clean_contract()
            .expect("the contract itself loads clean, or this proves nothing");
        let detail = capability_cardinality(&corpus)
            .expect_err("a transformation that finds nothing fails the case");
        assert!(
            detail.contains("drop_catch_all"),
            "the failure names the transformation: {detail}"
        );
        assert!(
            detail.contains("would test nothing"),
            "the failure says why it matters: {detail}"
        );
    }

    /// The same for the lifecycle case, whose transformation matches the table
    /// header rather than parsing the document.
    #[test]
    fn a_lifecycle_table_the_transformation_cannot_see_fails_the_lifecycle_case() {
        let corpus = Corpus::holding("contract-spaced-lifecycle", SPACED_LIFECYCLE_HEADER);
        corpus
            .clean_contract()
            .expect("the contract itself loads clean, or this proves nothing");
        let detail = lifecycle_declaration(&corpus)
            .expect_err("a transformation that finds nothing fails the case");
        assert!(
            detail.contains("delete_lifecycle_table"),
            "the failure names the transformation: {detail}"
        );
        assert!(
            detail.contains("`[lifecycle]` table"),
            "the failure names the target: {detail}"
        );
    }

    /// The axis-consistency assertion is about contradicting a declared axis,
    /// so a corpus that declares none cannot carry it — and says so rather
    /// than reporting a contradiction it never made.
    #[test]
    fn the_axis_consistency_assertion_needs_an_axis_to_contradict() {
        let corpus = Corpus::holding("contract-no-axis", NO_AXIS);
        let contract = corpus.clean_contract().expect("a contract with no axis");
        let detail =
            axis_consistency(&corpus, &contract).expect_err("there is no axis to contradict");
        assert!(
            detail.contains("must declare a lifecycle axis to contradict"),
            "the failure says what is missing: {detail}"
        );
    }
}
