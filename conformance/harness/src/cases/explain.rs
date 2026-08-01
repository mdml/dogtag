//! `contract-explain-renders-every-declaration`: the two renderings carry
//! every declaration, carry nothing else, and agree with each other.

use dogtag::contract::{
    Contract, LifecycleDecl, Ordinary, PropertyDecl, RelationshipDecl, TypeDecl,
};
use dogtag::report::{contract_json, contract_markdown};
use dogtag::vault::VaultRoot;

use crate::transform::replace_lifecycle_with_none;

use super::corpus::Corpus;
use super::expect::{Checked, require, require_contains, require_same_names};
use super::scan;

/// The two renderings of one contract, kept together because every assertion
/// below is about both of them.
struct Rendered {
    markdown: String,
    json: String,
}

impl Rendered {
    /// Render both formats. Provenance is off in the Markdown, which is the
    /// default and the mode an agent reads; the JSON always carries it.
    fn of(root: &VaultRoot, contract: &Contract) -> Self {
        Rendered {
            markdown: contract_markdown(root, contract, false),
            json: contract_json(root, contract),
        }
    }
}

/// `contract-explain-renders-every-declaration`.
pub fn contract_explain(corpus: &Corpus) -> Checked {
    let root = corpus.vault_root()?;
    let contract = corpus.clean_contract()?;
    let rendered = Rendered::of(&root, &contract);

    names_agree(&contract, &rendered)?;
    capabilities_agree(&contract, &rendered)?;
    declarations_carry_their_detail(&contract, &rendered)?;
    lifecycle_and_dialect_appear(&contract, &rendered)?;
    provenance_covers_every_leaf(&contract, &rendered)?;
    a_corpus_with_no_axis_says_so(corpus)
}

/// Type names, property names and predicates: every declaration appears in
/// both renderings, and neither carries one the contract does not make.
fn names_agree(contract: &Contract, rendered: &Rendered) -> Checked {
    let types = scan::unique(contract.types().iter().map(|t| t.name().to_owned()));
    let properties = scan::unique(
        contract
            .types()
            .iter()
            .flat_map(|t| t.properties().iter().map(|p| p.name().to_owned())),
    );
    let predicates = scan::unique(
        contract
            .types()
            .iter()
            .flat_map(|t| t.relationships().iter().map(|r| r.predicate().to_owned())),
    );

    require_same_names(
        &types,
        &scan::backticked_after(&rendered.markdown, "### "),
        "the Markdown's type headings",
    )?;
    let mut labels = properties.clone();
    labels.extend(predicates.clone());
    require_same_names(
        &scan::unique(labels),
        &scan::unique(scan::row_labels(&rendered.markdown)),
        "the Markdown's table rows",
    )?;

    let mut named = types;
    named.extend(properties);
    require_same_names(
        &scan::unique(named),
        &scan::unique(scan::json_strings(&rendered.json, "name")),
        "the JSON's names",
    )?;
    require_same_names(
        &predicates,
        &scan::unique(scan::json_strings(&rendered.json, "predicate")),
        "the JSON's predicates",
    )?;
    flags_agree(contract, rendered)
}

/// Flags render as their own remark in the Markdown and their own array in the
/// JSON; a contract with none says so rather than omitting the section.
fn flags_agree(contract: &Contract, rendered: &Rendered) -> Checked {
    let flags = scan::unique(
        contract
            .flags()
            .iter()
            .map(|flag| flag.property().to_owned()),
    );
    require_same_names(
        &flags,
        &scan::unique(scan::json_strings(&rendered.json, "property")),
        "the JSON's flags",
    )?;
    let section = scan::section(&rendered.markdown, "Flags")
        .ok_or_else(|| "the Markdown omits the Flags section".to_owned())?;
    if flags.is_empty() {
        return require_contains(section, "no flags", "the Markdown's Flags section");
    }
    require_same_names(
        &flags,
        &scan::unique(scan::backticked_after(section, "`")),
        "the Markdown's flags",
    )
}

/// Every type's capabilities reach both renderings, and neither invents one.
///
/// The Markdown puts them after the type name in its heading, which the name
/// scanner deliberately stops short of — so without this the whole capability
/// vocabulary was asserted by nothing. A rendering that dropped `closed-write`
/// from every type that declares it, or granted `catch-all` to a second type,
/// passed both renderings green.
fn capabilities_agree(contract: &Contract, rendered: &Rendered) -> Checked {
    for declared in contract.types() {
        let expected = capability_clause(declared);
        let heading = format!("### `{}` — {expected}", declared.name());
        require_contains(&rendered.markdown, &heading, "the Markdown's type headings")?;
    }
    // Both directions: the set the Markdown carries is the set the contract
    // declares, so a heading for a type that declares none cannot quietly
    // acquire one.
    let declared = scan::unique(contract.types().iter().map(capability_clause));
    require_same_names(
        &declared,
        &scan::unique(clauses_after(&rendered.markdown, "### `")),
        "the Markdown's capability clauses",
    )?;
    let named = scan::unique(
        contract
            .types()
            .iter()
            .flat_map(|t| t.capabilities().iter().map(|c| c.as_str().to_owned())),
    );
    require_same_names(
        &named,
        &scan::unique(json_array_strings(&rendered.json, "capabilities")),
        "the JSON's capabilities",
    )
}

/// What each heading says after its name.
///
/// `scan::backticked_after` deliberately stops at the backticked name, so the
/// clause carrying the capabilities is read by nothing else.
fn clauses_after(text: &str, prefix: &str) -> Vec<String> {
    text.lines()
        .filter(|line| line.starts_with(prefix))
        .filter_map(|line| Some(line.split_once(" — ")?.1.trim().to_owned()))
        .collect()
}

/// Every string inside each `"<key>": [ ... ]` array.
///
/// `scan::json_strings` reads scalars, so an array-valued key read through it
/// yields nothing at all — which is indistinguishable from a rendering that
/// carries the key and declares it empty.
fn json_array_strings(json: &str, key: &str) -> Vec<String> {
    let opening = format!("\"{key}\": [");
    let mut found = Vec::new();
    let mut rest = json;
    while let Some(at) = rest.find(&opening) {
        rest = &rest[at + opening.len()..];
        let Some(end) = rest.find(']') else { break };
        found.extend(rest[..end].split('"').skip(1).step_by(2).map(str::to_owned));
        rest = &rest[end..];
    }
    found
}

/// How a type's capabilities are spelled after its name in a heading.
fn capability_clause(declared: &TypeDecl) -> String {
    if declared.capabilities().is_empty() {
        return "no capabilities".to_owned();
    }
    declared
        .capabilities()
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<&str>>()
        .join(", ")
}

/// Each property's kind and required flag, and each relationship's required
/// flag, reach the Markdown table; the JSON's kinds are exactly the declared
/// ones.
fn declarations_carry_their_detail(contract: &Contract, rendered: &Rendered) -> Checked {
    for declared in contract.types() {
        for property in declared.properties() {
            require(property_row(&rendered.markdown, property), || {
                format!(
                    "the Markdown carries no row for `{}` as {} ({})",
                    property.name(),
                    property.kind().as_str(),
                    yes_no(property.required())
                )
            })?;
        }
        for relationship in declared.relationships() {
            require(relationship_row(&rendered.markdown, relationship), || {
                format!(
                    "the Markdown carries no row for `{}` ({})",
                    relationship.predicate(),
                    yes_no(relationship.required())
                )
            })?;
        }
    }
    let kinds = scan::unique(contract.types().iter().flat_map(|declared| {
        declared
            .properties()
            .iter()
            .map(|property| property.kind().as_str().to_owned())
    }));
    require_same_names(
        &kinds,
        &scan::unique(scan::json_strings(&rendered.json, "kind")),
        "the JSON's property kinds",
    )
}

/// A property row: the name, a kind cell naming the kind, and the required
/// answer.
fn property_row(markdown: &str, property: &PropertyDecl) -> bool {
    let name = format!("`{}`", property.name());
    markdown.lines().any(|line| {
        let cells = scan::cells(line);
        cells.len() >= 5
            && cells[1] == name
            && cells[2].contains(property.kind().as_str())
            && cells[3] == yes_no(property.required())
    })
}

/// A relationship row: the predicate and the required answer.
fn relationship_row(markdown: &str, relationship: &RelationshipDecl) -> bool {
    let predicate = format!("`{}`", relationship.predicate());
    markdown.lines().any(|line| {
        let cells = scan::cells(line);
        cells.len() == 4 && cells[1] == predicate && cells[2] == yes_no(relationship.required())
    })
}

/// The lifecycle declaration and the dialect appear in both renderings.
fn lifecycle_and_dialect_appear(contract: &Contract, rendered: &Rendered) -> Checked {
    let lifecycle = scan::section(&rendered.markdown, "Lifecycle")
        .ok_or_else(|| "the Markdown omits the Lifecycle section".to_owned())?;
    require_contains(
        &rendered.json,
        &format!("\"declared\": \"{}\"", contract.lifecycle().declared()),
        "the JSON's lifecycle",
    )?;
    lifecycle_detail(contract.lifecycle(), lifecycle, &rendered.json)?;

    let links = contract.dialect().links().as_str();
    require_contains(
        &rendered.json,
        &format!("\"links\": \"{links}\""),
        "the JSON's dialect",
    )?;
    let dialect = scan::section(&rendered.markdown, "Dialect")
        .ok_or_else(|| "the Markdown omits the Dialect section".to_owned())?;
    require(dialect.to_lowercase().contains(links), || {
        format!("the Markdown's Dialect section does not name `{links}`: {dialect}")
    })
}

/// The axis and the encoding of its ordinary state, or the statement that
/// there is no axis — never an omission.
fn lifecycle_detail(lifecycle: &LifecycleDecl, markdown: &str, json: &str) -> Checked {
    let Some(axis) = lifecycle.axis() else {
        return require_contains(
            &markdown.to_lowercase(),
            "no lifecycle axis",
            "the Markdown's Lifecycle section",
        );
    };
    require_contains(markdown, axis, "the Markdown's Lifecycle section")?;
    require_contains(
        json,
        &format!("\"axis\": \"{axis}\""),
        "the JSON's lifecycle",
    )?;
    match lifecycle.ordinary() {
        Some(Ordinary::Value(value)) => {
            require_contains(markdown, value, "the Markdown's Lifecycle section")?;
            require_contains(
                json,
                &format!("\"value\": \"{value}\""),
                "the JSON's lifecycle",
            )
        }
        _ => require_contains(json, "\"absent\": true", "the JSON's lifecycle"),
    }
}

/// The JSON carries provenance for every recorded leaf, and for nothing else.
fn provenance_covers_every_leaf(contract: &Contract, rendered: &Rendered) -> Checked {
    let recorded: Vec<String> = contract
        .provenance()
        .entries()
        .map(|entry| entry.key.clone())
        .collect();
    require_same_names(
        &recorded,
        &scan::json_strings(&rendered.json, "key"),
        "the JSON's provenance",
    )
}

/// A corpus that declares it has no life axis renders **the statement it is**,
/// derived from this profile's own contract rather than authored.
fn a_corpus_with_no_axis_says_so(corpus: &Corpus) -> Checked {
    let derived = corpus.derived("explain-lifecycle-none", replace_lifecycle_with_none)?;
    let root = derived.vault_root()?;
    let contract = derived.clean_contract()?;
    let rendered = Rendered::of(&root, &contract);
    let section = scan::section(&rendered.markdown, "Lifecycle")
        .ok_or_else(|| "a no-axis contract must still render a Lifecycle section".to_owned())?;
    require_contains(
        &section.to_lowercase(),
        "no lifecycle axis",
        "the Markdown's Lifecycle section for a corpus with no axis",
    )?;
    require_contains(
        &rendered.json,
        "\"declared\": \"none\"",
        "the JSON for a corpus with no axis",
    )
}

/// A boolean as the renderings write it.
fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::corpus::{NO_AXIS, WITH_AXIS};

    /// A contract and the two renderings of it, which each test below spoils
    /// in exactly one way.
    fn subject(label: &str, text: &str) -> (Contract, Rendered) {
        let corpus = Corpus::holding(label, text);
        let root = corpus.vault_root().expect("a vault root");
        let contract = corpus
            .clean_contract()
            .expect("a contract that loads clean, or the spoiling proves nothing");
        let rendered = Rendered::of(&root, &contract);
        (contract, rendered)
    }

    /// The same renderings with one substring of the Markdown rewritten.
    fn markdown_edited(rendered: &Rendered, from: &str, to: &str) -> Rendered {
        Rendered {
            markdown: rendered.markdown.replace(from, to),
            json: rendered.json.clone(),
        }
    }

    /// The same renderings with one substring of the JSON rewritten.
    fn json_edited(rendered: &Rendered, from: &str, to: &str) -> Rendered {
        Rendered {
            markdown: rendered.markdown.clone(),
            json: rendered.json.clone().replace(from, to),
        }
    }

    /// The detail a spoiled rendering earned, or a panic naming what the
    /// assertion accepted instead.
    fn refusal(checked: Checked) -> String {
        checked.expect_err("a rendering that disagrees with the contract must not pass")
    }

    /// Every name is asserted in both renderings and in both directions, so a
    /// rendering that drops a declaration and one that invents a declaration
    /// the contract never made are each named — whichever rendering it is in.
    #[test]
    fn a_rendering_that_drops_or_invents_a_name_is_named() {
        let (contract, rendered) = subject("explain-names", WITH_AXIS);
        let spoiled = [
            (
                markdown_edited(&rendered, "`capture`", "`ghost`"),
                "capture",
            ),
            (
                markdown_edited(&rendered, "| `title` |", "| `ghost` |"),
                "title",
            ),
            (
                json_edited(&rendered, "\"name\": \"capture\"", "\"name\": \"ghost\""),
                "capture",
            ),
            (
                json_edited(
                    &rendered,
                    "\"predicate\": \"mentions\"",
                    "\"predicate\": \"ghost\"",
                ),
                "mentions",
            ),
            (
                json_edited(
                    &rendered,
                    "\"property\": \"leaned_on\"",
                    "\"property\": \"ghost\"",
                ),
                "leaned_on",
            ),
        ];
        for (rendering, dropped) in spoiled {
            let detail = refusal(names_agree(&contract, &rendering));
            assert!(
                detail.contains(&format!("omits the declared `{dropped}`")),
                "the failure names the declaration that went missing: {detail}"
            );
            assert!(
                detail.contains("ghost"),
                "the failure carries what the rendering does say: {detail}"
            );
        }
    }

    /// A row that contradicts its declaration is as wrong as a missing one,
    /// and the failure repeats the declaration it was looking for.
    #[test]
    fn a_row_that_contradicts_its_declaration_is_named() {
        let (contract, rendered) = subject("explain-rows", WITH_AXIS);
        let renamed = markdown_edited(&rendered, "`title`", "`ghost`");
        let missing = refusal(declarations_carry_their_detail(&contract, &renamed));
        assert!(
            missing.contains("no row for `title` as string (yes)"),
            "the failure names the property, its kind and its requirement: {missing}"
        );
        let rekinded = markdown_edited(&rendered, "| `title` | string |", "| `title` | integer |");
        let wrong_kind = refusal(declarations_carry_their_detail(&contract, &rekinded));
        assert!(
            wrong_kind.contains("no row for `title` as string (yes)"),
            "a row under the wrong kind is not the declaration's row: {wrong_kind}"
        );
        let flipped = markdown_edited(&rendered, "| `mentions` | no |", "| `mentions` | yes |");
        let contradicted = refusal(declarations_carry_their_detail(&contract, &flipped));
        assert!(
            contradicted.contains("no row for `mentions` (no)"),
            "the failure names the relationship and its requirement: {contradicted}"
        );
    }

    /// The lifecycle declaration and the dialect are asserted in both
    /// renderings too, so one rendering cannot quietly say something the other
    /// does not.
    #[test]
    fn a_lifecycle_or_dialect_the_renderings_disagree_on_is_named() {
        let (contract, rendered) = subject("explain-lifecycle", WITH_AXIS);
        let spoiled = [
            (
                json_edited(
                    &rendered,
                    "\"declared\": \"axis\"",
                    "\"declared\": \"none\"",
                ),
                "the JSON's lifecycle",
            ),
            (
                json_edited(
                    &rendered,
                    "\"links\": \"wikilink\"",
                    "\"links\": \"markdown\"",
                ),
                "the JSON's dialect",
            ),
            (
                markdown_edited(&rendered, "wikilinks", "a private convention"),
                "Dialect section does not name `wikilink`",
            ),
        ];
        for (rendering, expected) in spoiled {
            let detail = refusal(lifecycle_and_dialect_appear(&contract, &rendering));
            assert!(
                detail.contains(expected),
                "the failure names what disagreed: {detail}"
            );
        }
    }

    /// The axis reaches the JSON under its own key: a rendering naming another
    /// property is not the contract's rendering.
    #[test]
    fn a_json_naming_another_axis_is_named() {
        let (contract, rendered) = subject("explain-axis", WITH_AXIS);
        let json = rendered
            .json
            .replace("\"axis\": \"status\"", "\"axis\": \"standing\"");
        let section = scan::section(&rendered.markdown, "Lifecycle").expect("a Lifecycle section");
        let detail = refusal(lifecycle_detail(contract.lifecycle(), section, &json));
        assert!(
            detail.contains("the JSON's lifecycle"),
            "the failure names the rendering: {detail}"
        );
        assert!(
            detail.contains("`\"axis\": \"status\"`"),
            "the failure names what it looked for: {detail}"
        );
    }

    /// A corpus with no life axis renders **the statement it is**. An omitted
    /// section is the failure this assertion exists to catch, so a section
    /// that says nothing is refused rather than passing for want of a
    /// contradiction.
    #[test]
    fn a_lifecycle_section_that_omits_the_absence_of_an_axis_is_named() {
        let (contract, _) = subject("explain-no-axis", NO_AXIS);
        let detail = refusal(lifecycle_detail(
            contract.lifecycle(),
            "a section that says nothing",
            "",
        ));
        assert!(
            detail.contains("the Markdown's Lifecycle section"),
            "the failure names the section: {detail}"
        );
        assert!(
            detail.contains("no lifecycle axis"),
            "the failure names the statement it wanted: {detail}"
        );
    }
}
