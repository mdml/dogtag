//! Retrieval cases: the M4 search surface over any profile's corpus.
//!
//! Every needle is derived, never committed: an invented word planted in
//! known notes is what makes "exactly those notes" assertable against any
//! corpus, whatever its vocabulary. Membership and count are the contract;
//! ordering is deliberately not asserted, because the surfaces record keeps
//! ranking free to improve without an amendment.

use std::collections::BTreeSet;

use dogtag::note::{ListFilter, SearchRequest, SearchResult, list, search};
use dogtag::report::{search_json, search_text};

use super::corpus::Corpus;
use super::derive::{self, NoteDerivation, Planting};
use super::expect::{Checked, Subject, require, require_clean, require_id};
use crate::transform::replace_lifecycle_with_none;

/// `search-membership-by-body-term`.
pub fn membership_by_body_term(corpus: &Corpus) -> Checked {
    let derived = derive::derived_planting(
        corpus,
        &Planting {
            label: "search-membership",
            notes: &[
                ("derived-search/one.md", "# One\n\nA quixarine passage.\n"),
                (
                    "derived-search/two.md",
                    "Plain quixarine, then quixarine.\n",
                ),
                (
                    "derived-search/other.md",
                    "# Other\n\nNothing planted here.\n",
                ),
            ],
        },
    )?;
    let result = searched(&derived, "quixarine")?;
    require_clean(result.diagnostics(), Subject::new("the membership search"))?;
    require_hits(
        &result,
        &["derived-search/one.md", "derived-search/two.md"],
        "the body-term query",
    )?;
    for hit in result.hits() {
        require(hit.type_name().is_some(), || untyped_hit(hit))?;
        let quoted = hit
            .snippet()
            .is_some_and(|snippet| snippet.contains("quixarine"));
        require(quoted, || unquoted_hit(hit))?;
    }
    Ok(())
}

/// The details only a misbehaving SDK can produce, built as functions so the
/// text a failing run would print is itself tested — the closures that raise
/// them can never run under a passing suite.
fn untyped_hit(hit: &dogtag::note::SearchHit) -> String {
    format!(
        "`{}` bound by catch-all and must say which type",
        hit.path()
    )
}

fn unquoted_hit(hit: &dogtag::note::SearchHit) -> String {
    format!("`{}`'s snippet must quote the matched context", hit.path())
}

/// `search-phrase-matches-adjacent-words`.
pub fn phrase_matches_adjacent_words(corpus: &Corpus) -> Checked {
    let derived = derive::derived_planting(
        corpus,
        &Planting {
            label: "search-phrase",
            notes: &[
                (
                    "derived-search/adjacent.md",
                    "The zanthiqor veldrune holds.\n",
                ),
                (
                    "derived-search/reversed.md",
                    "A veldrune zanthiqor instead.\n",
                ),
                (
                    "derived-search/separated.md",
                    "zanthiqor against the veldrune\n",
                ),
            ],
        },
    )?;
    require_hits(
        &searched(&derived, "\"zanthiqor veldrune\"")?,
        &["derived-search/adjacent.md"],
        "the quoted phrase",
    )?;
    require_hits(
        &searched(&derived, "zanthiqor veldrune")?,
        &[
            "derived-search/adjacent.md",
            "derived-search/reversed.md",
            "derived-search/separated.md",
        ],
        "the same words unquoted, OR-combined",
    )
}

/// `search-prefix-wildcard`.
pub fn prefix_wildcard(corpus: &Corpus) -> Checked {
    let derived = derive::derived_planting(
        corpus,
        &Planting {
            label: "search-prefix",
            notes: &[
                ("derived-search/long.md", "About zanthiqory matters.\n"),
                ("derived-search/short.md", "About zanthiqal matters.\n"),
                ("derived-search/other.md", "About zephyrine matters.\n"),
            ],
        },
    )?;
    require_hits(
        &searched(&derived, "zanthiq*")?,
        &["derived-search/long.md", "derived-search/short.md"],
        "the trailing-* prefix",
    )?;
    require_hits(
        &searched(&derived, "zanthiqal")?,
        &["derived-search/short.md"],
        "the same stem without the wildcard, exact",
    )
}

/// `search-composes-with-list-filters`.
///
/// Search is enumeration plus a text predicate, so for every filter the hits
/// must be exactly the unfiltered hits intersected with what `list` answers
/// for the same filter — under whichever ordinary-state encoding this
/// corpus's contract declares, without either vocabulary reaching the core.
///
/// Two arms carry a construction guarantee against the vacuous pass: the
/// type filter names the derived participant's own bound type, and the
/// axis-value filter (when a committed note carries a value at all) names a
/// value a needle-carrying note actually holds — so their expected sets are
/// provably nonempty, and an empty-set equality cannot report green.
pub fn composes_with_list_filters(corpus: &Corpus) -> Checked {
    let seeded = seed_needles(corpus)?;
    let derived = &seeded.derived;
    let everything = hit_paths(&searched(derived, TERM)?);
    require(everything.len() >= 2, || missed_needles(&everything))?;
    let fixed = [
        Arm {
            filter: ListFilter {
                type_name: Some(seeded.participant_type.clone()),
                ..ListFilter::default()
            },
            floor: 1,
        },
        Arm {
            filter: ListFilter {
                tag: Some("derived/topic".to_owned()),
                ..ListFilter::default()
            },
            floor: 0,
        },
        Arm {
            filter: ListFilter {
                ordinary: true,
                ..ListFilter::default()
            },
            floor: 0,
        },
    ];
    let value_arm = seeded.axis_value.iter().map(|value| Arm {
        filter: ListFilter {
            lifecycle: Some(value.clone()),
            ..ListFilter::default()
        },
        floor: 1,
    });
    for arm in fixed.into_iter().chain(value_arm) {
        compose(derived, &everything, &arm)?;
    }
    let none = derived.derived("search-no-axis", replace_lifecycle_with_none)?;
    let refused = searched_with(
        &none,
        TERM,
        ListFilter {
            ordinary: true,
            ..ListFilter::default()
        },
    )?;
    require_id(
        refused.diagnostics(),
        "note.lifecycle-axis-absent",
        Subject::new("a lifecycle filter against a corpus declaring no axis"),
    )
}

/// One filter's composition check: hits narrowed by the filter must equal
/// the unfiltered hits intersected with `list`'s answer for the same filter.
struct Arm {
    filter: ListFilter,
    /// How many notes the seeding guarantees the expected set holds. A floor
    /// of one makes an empty-set equality a failure rather than a vacuous
    /// pass; a floor of zero is an arm whose evidence the corpus decides.
    floor: usize,
}

fn compose(derived: &Corpus, everything: &BTreeSet<String>, arm: &Arm) -> Checked {
    let contract = derived.clean_contract()?;
    let listed: BTreeSet<String> = list(&derived.vault_root()?, &contract, &arm.filter)
        .notes()
        .iter()
        .map(|note| note.path().as_str().to_owned())
        .collect();
    let narrowed = hit_paths(&searched_with(derived, TERM, arm.filter.clone())?);
    let expected: BTreeSet<String> = everything.intersection(&listed).cloned().collect();
    require(expected.len() >= arm.floor, || starved(&arm.filter))?;
    let composed = narrowed == expected;
    require(composed, || uncomposed(&arm.filter, &narrowed, &expected))
}

/// `search-empty-result-is-a-result`.
pub fn empty_result_is_a_result(corpus: &Corpus) -> Checked {
    let result = searched(corpus, "quixarine")?;
    require_clean(
        result.diagnostics(),
        Subject::new("a search matching nothing"),
    )?;
    require(result.hits().is_empty(), || {
        format!(
            "no committed note carries the invented word, yet the hits were {:?}",
            hit_paths(&result)
        )
    })
}

/// `search-repeat-is-deterministic`.
pub fn repeat_is_deterministic(corpus: &Corpus) -> Checked {
    let derived = derive::derived_planting(
        corpus,
        &Planting {
            label: "search-repeat",
            notes: &[
                ("derived-search/one.md", "# One\n\nquixarine\n"),
                ("derived-search/two.md", "quixarine quixarine\n"),
            ],
        },
    )?;
    let first = searched(&derived, "quixarine")?;
    let second = searched(&derived, "quixarine")?;
    require(first == second, || nondeterminism("results"))?;
    let text = (search_text(&first), search_text(&second));
    require(text.0 == text.1, || nondeterminism("text bytes"))?;
    let json = (
        search_json(&first, first.diagnostics()),
        search_json(&second, second.diagnostics()),
    );
    require(json.0 == json.1, || nondeterminism("JSON bytes"))
}

/// Two identical searches disagreed about `what`.
fn nondeterminism(what: &str) -> String {
    format!("two identical searches answered different {what}")
}

/// The composition case's needles did not all match.
fn missed_needles(everything: &BTreeSet<String>) -> String {
    format!("both derived needles must match, but the hits were {everything:?}")
}

/// A filtered search disagreed with enumeration-plus-predicate.
fn uncomposed(
    filter: &ListFilter,
    narrowed: &BTreeSet<String>,
    expected: &BTreeSet<String>,
) -> String {
    format!(
        "under {filter:?} the hits were {narrowed:?} where enumeration-plus-predicate says \
         {expected:?}"
    )
}

/// A construction-guaranteed arm found nothing to compose over.
fn starved(filter: &ListFilter) -> String {
    format!("under {filter:?} the composition ran over the empty set, which proves nothing")
}

/// A hit answered the same path twice: membership held, multiplicity did not.
fn duplicated_hits(subject: &str, answered: usize, distinct: usize) -> String {
    format!("{subject} answered {answered} hits over {distinct} distinct paths")
}

/// The invented word the filter-composition case plants.
const TERM: &str = "quixarine";

/// The derived corpus the composition arms run over, and the two facts the
/// seeding establishes about it.
struct Seeded {
    derived: Corpus,
    /// The bound type of the committed participant the term was seeded into
    /// — the type filter names it, so the type arm provably narrows to a
    /// nonempty set.
    participant_type: String,
    /// A declared-axis value some needle-carrying committed note holds, when
    /// the corpus holds any valued note at all; `starter`-like corpora are
    /// legitimately all-ordinary and skip the value arm.
    axis_value: Option<String>,
}

/// Seeds the term into one committed axis-participant note, one committed
/// value-carrying note where the corpus has one, and one planted untyped,
/// tagged note.
///
/// Deriving from committed notes is what keeps the case honest under
/// whichever encoding the contract declares: the participants' own states —
/// marked or unmarked — are real corpus data, not values this case invented.
fn seed_needles(corpus: &Corpus) -> Result<Seeded, String> {
    let (participant, participant_type) = first_participant(corpus)?;
    let derived = derive::derived_note(
        corpus,
        &NoteDerivation {
            label: "search-filters",
            note: &participant,
        },
        |text| Ok(format!("{text}\n{TERM}\n")),
    )?;
    derive::plant(
        &derived,
        "derived-search/untyped.md",
        &format!("---\ntags: [derived/topic]\n---\n{TERM}\n"),
    )?;
    let axis_value = seed_valued(corpus, &derived)?;
    Ok(Seeded {
        derived,
        participant_type,
        axis_value,
    })
}

/// The first committed note, in corpus order, whose bound type declares the
/// lifecycle axis property — and that type's name.
fn first_participant(corpus: &Corpus) -> Result<(String, String), String> {
    let contract = corpus.clean_contract()?;
    let axis = declared_axis(corpus)?;
    let notes = derive::notes(corpus)?;
    notes
        .notes()
        .iter()
        .find_map(|note| {
            let name = note.binding().type_name()?;
            contract.type_named(name)?.property(&axis)?;
            Some((note.path().as_str().to_owned(), name.to_owned()))
        })
        .ok_or_else(|| "no committed note participates in the declared axis".to_string())
}

/// Seeds the term into the first committed note carrying an axis value, and
/// answers that value — or `None` where the corpus is legitimately all
/// unmarked, in which case the value arm has nothing honest to filter for.
fn seed_valued(corpus: &Corpus, derived: &Corpus) -> Result<Option<String>, String> {
    let axis = declared_axis(corpus)?;
    let notes = derive::notes(corpus)?;
    let valued = notes.notes().iter().find_map(|note| {
        let value = note.property(&axis)?.scalar()?;
        Some((note.path().as_str().to_owned(), value.to_owned()))
    });
    valued
        .map(|(path, value)| {
            let seeded = format!("{}\n{TERM}\n", derive::note_text(corpus, &path)?);
            derive::plant(derived, &path, &seeded).map(|()| value)
        })
        .transpose()
}

/// The declared lifecycle axis, which every built profile carries.
fn declared_axis(corpus: &Corpus) -> Result<String, String> {
    corpus
        .clean_contract()?
        .lifecycle()
        .axis()
        .map(str::to_owned)
        .ok_or_else(|| "a built profile declares a lifecycle axis; none was found".to_string())
}

/// The corpus searched through the SDK, unfiltered and uncapped.
///
/// `pub(super)` because the derived situations in [`super::docs_native`] query
/// the same way; one spelling of *search this corpus* is what keeps the two
/// modules asking the SDK the same question.
pub(super) fn searched(corpus: &Corpus, query: &str) -> Result<SearchResult, String> {
    searched_with(corpus, query, ListFilter::default())
}

pub(super) fn searched_with(
    corpus: &Corpus,
    query: &str,
    filter: ListFilter,
) -> Result<SearchResult, String> {
    let contract = corpus.clean_contract()?;
    Ok(search(
        &corpus.vault_root()?,
        &contract,
        &SearchRequest {
            query: query.to_owned(),
            filter,
            limit: usize::MAX,
        },
    ))
}

/// Every hit's path, as the set membership assertions compare.
pub(super) fn hit_paths(result: &SearchResult) -> BTreeSet<String> {
    result
        .hits()
        .iter()
        .map(|hit| hit.path().as_str().to_owned())
        .collect()
}

/// The hits are exactly `expected` — membership and count, not ordering.
pub(super) fn require_hits(result: &SearchResult, expected: &[&str], subject: &str) -> Checked {
    let answered = hit_paths(result);
    let wanted: BTreeSet<String> = expected.iter().map(|path| (*path).to_owned()).collect();
    require(answered == wanted, || {
        format!("{subject} answered {answered:?} where the planted notes say {wanted:?}")
    })?;
    let doubled = duplicated_hits(subject, result.hits().len(), wanted.len());
    require(result.hits().len() == wanted.len(), || doubled)
}

#[cfg(test)]
mod tests {
    use super::super::derive::plant;
    use super::*;

    const NONE_CONTRACT: &str = concat!(
        "contract_version = 2\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    #[test]
    fn a_corpus_declaring_no_axis_refuses_the_filter_case_up_front() {
        let corpus = Corpus::holding("search-no-axis-refusal", NONE_CONTRACT);
        plant(&corpus, "a.md", "# Capture\n").expect("a note");
        let detail = composes_with_list_filters(&corpus)
            .expect_err("the case needs a declared axis to compose with");
        assert!(detail.contains("none was found"), "{detail}");
    }

    #[test]
    fn an_all_unmarked_corpus_skips_the_value_arm_and_composes_the_rest() {
        // Every note leaves the axis unmarked, so there is no honest value to
        // filter for: the value arm skips, and the other arms still compose.
        let contract = concat!(
            "contract_version = 2\n",
            "\n[dialect]\nlinks = \"wikilink\"\n",
            "\n[lifecycle]\naxis = \"stage\"\nordinary = { absent = true }\n",
            "\n[tags]\nproperty = \"tags\"\n",
            "\n[[type]]\nname = \"work\"\ncapabilities = [\"identity-bearing\"]\n",
            "  [[type.property]]\n  name = \"stage\"\n  kind = \"enum\"\n  values = [\"done\"]\n",
            "  [[type.property]]\n  name = \"tags\"\n  kind = \"list\"\n  of = \"string\"\n",
            "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
            "  [[type.property]]\n  name = \"tags\"\n  kind = \"list\"\n  of = \"string\"\n",
        );
        let corpus = Corpus::holding("search-all-unmarked", contract);
        plant(&corpus, "a.md", "---\ntype: work\n---\n# A participant\n").expect("a note");
        composes_with_list_filters(&corpus).expect("the case composes without a value arm");
    }

    #[test]
    fn a_corpus_whose_axis_no_note_participates_in_refuses_the_filter_case() {
        let contract = concat!(
            "contract_version = 2\n",
            "\n[dialect]\nlinks = \"wikilink\"\n",
            "\n[lifecycle]\naxis = \"stage\"\nordinary = { absent = true }\n",
            "\n[[type]]\nname = \"work\"\ncapabilities = [\"identity-bearing\"]\n",
            "  [[type.property]]\n  name = \"stage\"\n  kind = \"enum\"\n  values = [\"active\"]\n",
            "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
        );
        let corpus = Corpus::holding("search-no-participant", contract);
        plant(&corpus, "a.md", "# Capture, untyped\n").expect("a note");
        let detail = composes_with_list_filters(&corpus)
            .expect_err("the case needs a committed participant to derive from");
        assert!(
            detail.contains("no committed note participates"),
            "{detail}"
        );
    }

    #[test]
    fn the_details_a_misbehaving_sdk_would_earn_say_what_went_wrong() {
        // A passing suite can never run the closures that raise these, so the
        // text itself is held here: each detail names its subject and what
        // actually arrived.
        let hit = sample_hit();
        assert!(untyped_hit(&hit).contains("must say which type"));
        assert!(unquoted_hit(&hit).contains("must quote the matched context"));
        assert!(nondeterminism("results").contains("different results"));
        let everything: BTreeSet<String> = ["only.md".to_owned()].into();
        assert!(missed_needles(&everything).contains("only.md"));
        let narrowed = BTreeSet::new();
        let detail = uncomposed(&ListFilter::default(), &narrowed, &everything);
        assert!(detail.contains("enumeration-plus-predicate"), "{detail}");
        assert!(detail.contains("only.md"), "{detail}");
        let starved = starved(&ListFilter::default());
        assert!(starved.contains("proves nothing"), "{starved}");
        let doubled = duplicated_hits("the query", 3, 2);
        assert!(
            doubled.contains("3 hits over 2 distinct paths"),
            "{doubled}"
        );
    }

    /// One real hit, searched out of a one-note corpus, for the detail tests.
    fn sample_hit() -> dogtag::note::SearchHit {
        let corpus = Corpus::holding("search-sample-hit", NONE_CONTRACT);
        plant(&corpus, "sample.md", "# Sample\n\nquixarine\n").expect("a note");
        let result = searched(&corpus, "quixarine").expect("a clean search");
        result.hits()[0].clone()
    }

    #[test]
    fn a_committed_needle_fails_the_empty_result_case_honestly() {
        let corpus = Corpus::holding("search-committed-needle", NONE_CONTRACT);
        plant(&corpus, "a.md", "# The word quixarine, committed\n").expect("a note");
        let detail = empty_result_is_a_result(&corpus)
            .expect_err("a committed needle means the case proves nothing");
        assert!(detail.contains("yet the hits were"), "{detail}");
        assert!(detail.contains("a.md"), "{detail}");
    }

    #[test]
    fn a_committed_needle_fails_the_membership_cases_by_widening_the_hits() {
        // The planted-notes assertions demand *exactly* the planted set, so a
        // committed note that happens to carry a needle is a failure naming
        // the surplus hit rather than a quiet widening of the evidence.
        let corpus = Corpus::holding("search-widened", NONE_CONTRACT);
        plant(
            &corpus,
            "taken.md",
            "quixarine, zanthiqor veldrune, zanthiqal\n",
        )
        .expect("a note");
        for (case, name) in [
            (
                membership_by_body_term as fn(&Corpus) -> Checked,
                "membership",
            ),
            (phrase_matches_adjacent_words, "phrase"),
            (prefix_wildcard, "prefix"),
        ] {
            let detail = case(&corpus).expect_err(name);
            assert!(detail.contains("taken.md"), "{name}: {detail}");
        }
    }

    #[test]
    fn a_corpus_that_is_not_clean_refuses_every_derived_search_case() {
        let corpus = Corpus::holding("search-dirty", NONE_CONTRACT);
        plant(&corpus, "broken.md", "---\ntype: nothing\n---\n").expect("a broken note");
        for (case, name) in [
            (
                membership_by_body_term as fn(&Corpus) -> Checked,
                "membership",
            ),
            (phrase_matches_adjacent_words, "phrase"),
            (prefix_wildcard, "prefix"),
            (repeat_is_deterministic, "repeat"),
        ] {
            let detail = case(&corpus).expect_err(name);
            assert!(detail.contains("the corpus before"), "{name}: {detail}");
        }
    }

    #[test]
    fn a_broken_corpus_fails_the_empty_result_case_on_its_diagnostics() {
        let corpus = Corpus::holding("search-broken-empty", NONE_CONTRACT);
        plant(&corpus, "broken.md", "---\ntype: nothing\n---\n").expect("a broken note");
        let detail = empty_result_is_a_result(&corpus)
            .expect_err("a broken corpus surfaces its diagnostics on every retrieval");
        assert!(detail.contains("note.unknown-type"), "{detail}");
    }
}
