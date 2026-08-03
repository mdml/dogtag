//! Tests for the contract transformations themselves.
//!
//! Derived negative cases make the harness a program rather than a data set:
//! it gains transformation logic that must itself be correct, and a bug there
//! could make a conformance case vacuously pass. So each transformation is
//! tested twice — once where it finds its target and changes the bytes, and
//! once where the target is absent and it reports failure rather than silently
//! returning the input.
//!
//! The found-target inputs are **every built profile's own contract**, not an
//! authored sample, for the same reason the conformance cases derive from
//! them: a transformation that only works against one corpus's spelling is
//! exactly the fault these tests exist to catch. The absent-target inputs are
//! mostly derived too — running a transformation twice removes its own target
//! — and the one that cannot be is a three-line snippet that is not a contract
//! at all.

use std::fs;

use dogtag_conformance::transform::{
    DERIVED_CATCH_ALL_TYPE, TargetNotFound, Transformed, delete_lifecycle_table, drop_catch_all,
    duplicate_catch_all, flip_property_required, replace_lifecycle_with_none, set_contract_version,
};
use dogtag_conformance::{CorpusStatus, load_profiles, profiles_dir};

/// A text that is well-formed TOML and is not a contract: it declares no
/// version, no lifecycle table and no type, so it is every transformation's
/// absent target at once.
const NOT_A_CONTRACT: &str = "[dialect]\nlinks = \"wikilink\"\n";

/// Two property declarations carrying no `required` key: one followed by
/// another table, one at the end of the document. Neither has a requirement to
/// flip, and a transformation that shrugged at either would leave its derived
/// case asserting a contradiction nobody wrote.
const PROPERTIES_WITHOUT_REQUIRED: &str = concat!(
    "[[type.property]]\nname = \"followed\"\nkind = \"string\"\n",
    "\n[[type.property]]\nname = \"last\"\nkind = \"string\"\n",
);

/// Every built profile's committed contract, by profile name.
fn built_contracts() -> Vec<(String, String)> {
    let profiles = load_profiles().expect("profiles load");
    let built: Vec<(String, String)> = profiles
        .iter()
        .filter(|profile| profile.corpus == CorpusStatus::Built)
        .map(|profile| {
            let path = profiles_dir()
                .join(&profile.name)
                .join("corpus/.dogtag/contract.toml");
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
            (profile.name.clone(), text)
        })
        .collect();
    assert!(
        !built.is_empty(),
        "the transformation tests need at least one built corpus to derive from"
    );
    built
}

/// Applies `transform` to every built profile's contract and asserts it found
/// its target, changed the bytes, and left something recognizable behind.
fn changes_every_built_contract(
    label: &str,
    transform: impl Fn(&str) -> Transformed,
    check: impl Fn(&str, &str),
) {
    for (profile, original) in built_contracts() {
        let transformed = transform(&original)
            .unwrap_or_else(|failure| panic!("`{label}` against `{profile}`: {failure}"));
        assert_ne!(
            transformed, original,
            "`{label}` against `{profile}` changed no bytes"
        );
        check(&original, &transformed);
    }
}

/// Asserts a transformation reported a missing target rather than returning
/// the input unchanged.
fn reports_a_missing_target(label: &str, result: Transformed) -> TargetNotFound {
    let failure = result.expect_err(&format!("`{label}` must refuse an absent target"));
    let message = failure.to_string();
    assert!(
        message.contains(failure.transform()) && message.contains(failure.target()),
        "the refusal names the transformation and its target: {message}"
    );
    assert!(
        message.contains("would test nothing"),
        "the refusal says why it matters: {message}"
    );
    failure
}

#[test]
fn drop_catch_all_removes_the_declaration_in_every_built_profile() {
    changes_every_built_contract("drop_catch_all", drop_catch_all, |original, transformed| {
        assert_eq!(
            transformed.matches("\"catch-all\"").count() + 1,
            original.matches("\"catch-all\"").count(),
            "exactly one catch-all declaration is removed"
        );
    });
}

#[test]
fn drop_catch_all_reports_an_absent_declaration() {
    for (_, original) in built_contracts() {
        let dropped = drop_catch_all(&original).expect("the first drop finds its target");
        reports_a_missing_target("drop_catch_all", drop_catch_all(&dropped));
    }
    reports_a_missing_target("drop_catch_all", drop_catch_all(NOT_A_CONTRACT));
}

#[test]
fn duplicate_catch_all_appends_a_second_declaration() {
    changes_every_built_contract(
        "duplicate_catch_all",
        duplicate_catch_all,
        |original, transformed| {
            assert_eq!(
                transformed.matches("\"catch-all\"").count(),
                original.matches("\"catch-all\"").count() + 1,
                "exactly one catch-all declaration is added"
            );
            assert!(
                transformed.contains(DERIVED_CATCH_ALL_TYPE),
                "the appended type is named so a diagnostic reads as the harness's doing"
            );
            assert!(
                !original.contains(DERIVED_CATCH_ALL_TYPE),
                "the appended type's name must not collide with a declared one"
            );
        },
    );
}

/// A contract whose last line carries no terminator: the appended declaration
/// has to start on a line of its own rather than running onto the last one,
/// which would change a line the transformation was not asked to touch.
#[test]
fn duplicate_catch_all_appends_below_a_contract_with_no_final_newline() {
    for (profile, original) in built_contracts() {
        let unterminated = original.trim_end().to_owned();
        let transformed = duplicate_catch_all(&unterminated).unwrap_or_else(|failure| {
            panic!("`duplicate_catch_all` against `{profile}`: {failure}")
        });
        assert!(
            transformed.starts_with(&format!("{unterminated}\n")),
            "the contract keeps its last line in `{profile}`"
        );
        assert!(
            transformed.contains(DERIVED_CATCH_ALL_TYPE),
            "the second declaration is appended in `{profile}`"
        );
    }
}

#[test]
fn duplicate_catch_all_reports_an_absent_declaration() {
    for (_, original) in built_contracts() {
        let dropped = drop_catch_all(&original).expect("the drop finds its target");
        reports_a_missing_target("duplicate_catch_all", duplicate_catch_all(&dropped));
    }
}

#[test]
fn set_contract_version_rewrites_the_declared_version() {
    // Neither version may be one a built contract already declares, or the
    // transformation leaves the bytes identical and the derived case tests
    // nothing. Both committed contracts declare version 2.
    for version in [0, 1, 3] {
        changes_every_built_contract(
            "set_contract_version",
            |text| set_contract_version(text, version),
            |_, transformed| {
                assert!(
                    transformed.contains(&format!("contract_version = {version}")),
                    "the declared version is rewritten"
                );
            },
        );
    }
}

#[test]
fn set_contract_version_reports_an_absent_version_key() {
    reports_a_missing_target(
        "set_contract_version",
        set_contract_version(NOT_A_CONTRACT, 2),
    );
}

#[test]
fn delete_lifecycle_table_removes_the_whole_table() {
    changes_every_built_contract(
        "delete_lifecycle_table",
        delete_lifecycle_table,
        |_, transformed| {
            assert!(
                !transformed.contains("[lifecycle]"),
                "the lifecycle table is gone"
            );
        },
    );
}

#[test]
fn delete_lifecycle_table_reports_an_absent_table() {
    for (_, original) in built_contracts() {
        let deleted = delete_lifecycle_table(&original).expect("the first delete finds its target");
        reports_a_missing_target("delete_lifecycle_table", delete_lifecycle_table(&deleted));
    }
    reports_a_missing_target(
        "delete_lifecycle_table",
        delete_lifecycle_table(NOT_A_CONTRACT),
    );
}

#[test]
fn replace_lifecycle_with_none_declares_the_absence_of_an_axis() {
    changes_every_built_contract(
        "replace_lifecycle_with_none",
        replace_lifecycle_with_none,
        |_, transformed| {
            assert!(
                transformed.contains("[lifecycle]\nnone = true\n"),
                "the corpus declares it has no life axis"
            );
            assert!(
                !transformed.contains("axis = "),
                "the axis declaration is replaced rather than kept beside the new one"
            );
        },
    );
}

#[test]
fn replace_lifecycle_with_none_reports_an_absent_table() {
    for (_, original) in built_contracts() {
        let deleted = delete_lifecycle_table(&original).expect("the delete finds its target");
        reports_a_missing_target(
            "replace_lifecycle_with_none",
            replace_lifecycle_with_none(&deleted),
        );
    }
}

#[test]
fn flip_property_required_negates_the_declared_requirement() {
    for (profile, original) in built_contracts() {
        let axis = axis_of(&original);
        let flipped = flip_property_required(&original, &axis)
            .unwrap_or_else(|failure| panic!("flipping `{axis}` in `{profile}`: {failure}"));
        assert_ne!(
            flipped, original,
            "flipping changed no bytes in `{profile}`"
        );
        assert_eq!(
            flipped.matches("required = true").count()
                + flipped.matches("required = false").count(),
            original.matches("required = true").count()
                + original.matches("required = false").count(),
            "flipping rewrites one requirement rather than adding or dropping one"
        );
        assert_ne!(
            flipped.matches("required = true").count(),
            original.matches("required = true").count(),
            "one requirement changed direction"
        );
    }
}

#[test]
fn flip_property_required_reports_a_property_it_cannot_find() {
    for (_, original) in built_contracts() {
        reports_a_missing_target(
            "flip_property_required",
            flip_property_required(&original, "a-property-no-corpus-declares"),
        );
    }
    reports_a_missing_target(
        "flip_property_required",
        flip_property_required(NOT_A_CONTRACT, "anything"),
    );
}

/// A declaration carrying no `required` key has no requirement to flip, and
/// the transformation says so rather than reaching into the declaration after
/// it — whether the next thing is another table or the end of the document.
#[test]
fn flip_property_required_reports_a_declaration_with_no_requirement() {
    for property in ["followed", "last"] {
        let failure = reports_a_missing_target(
            "flip_property_required",
            flip_property_required(PROPERTIES_WITHOUT_REQUIRED, property),
        );
        assert!(
            failure.target().contains(property),
            "the refusal names the declaration it was flipping: {}",
            failure.target()
        );
    }
}

/// The lifecycle axis a contract declares, read from its text.
///
/// The tests need a property name that exists in whichever profile they are
/// running against, and the axis is the one property every contract declaring
/// an axis is guaranteed to have.
fn axis_of(contract: &str) -> String {
    let line = contract
        .lines()
        .find(|line| line.trim_start().starts_with("axis = "))
        .expect("a built contract declares a lifecycle axis");
    line.split('"')
        .nth(1)
        .expect("the axis is a quoted string")
        .to_owned()
}
