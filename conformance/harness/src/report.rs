//! The scenarios × profiles cross product and its human-readable rendering.
//!
//! [`report`] takes **no filter parameter** — the signature is part of the
//! no-waiver enforcement, alongside the strict schemas and loaders. The
//! execution parameter it does take is not a filter: an executor cannot
//! decline a pair, because answering "no execution path" refuses the whole
//! report rather than skipping the pair (see [`crate::Execution`]).

use std::collections::BTreeMap;

use crate::error::HarnessError;
use crate::execution::Execution;
use crate::schema::{CorpusStatus, Profile, Scenario, ScenarioStatus};

/// One cell of the scenarios × profiles cross product.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    /// The scenario's id.
    pub scenario_id: String,
    /// The profile's name.
    pub profile_name: String,
    /// What the harness can say about the pair.
    pub outcome: Outcome,
}

/// The outcome of one scenario/profile pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Not runnable: the scenario is still a prose contract, the profile's
    /// corpus is not built, or both. The two reasons are carried separately
    /// because they are different facts, and a rendering that conflated them
    /// would let a run covering two of four profiles read as a complete
    /// matrix.
    Pending {
        /// The scenario's status is still `pending`.
        scenario_pending: bool,
        /// The profile's corpus is not yet built — a **skip**, not a result.
        corpus_missing: bool,
    },
    /// The pair ran and passed.
    Passed,
    /// The pair ran and failed.
    Failed {
        /// What the case reported.
        detail: String,
    },
}

/// Compute the full cross product: every scenario against every profile,
/// each pair exactly once.
///
/// **No filter parameter exists** — that absence is part of the no-waiver
/// enforcement, and `execution` does not reintroduce one. An executor decides
/// *how* a runnable pair runs, never *whether* it runs: a runnable pair whose
/// scenario has no execution path is refused below rather than skipped.
///
/// # Errors
///
/// [`HarnessError::NotExecutable`] if a pair is runnable (an `executable`
/// scenario against a `built` corpus) and `execution` has no case for it. The
/// harness refuses to invent an outcome for a pair it should be running, which
/// is what makes graduation all-or-nothing.
pub fn report(
    scenarios: &[Scenario],
    profiles: &[Profile],
    execution: &dyn Execution,
) -> Result<Vec<Pair>, HarnessError> {
    let mut pairs = Vec::with_capacity(scenarios.len() * profiles.len());
    for scenario in scenarios {
        for profile in profiles {
            pairs.push(pair_outcome(scenario, profile, execution)?);
        }
    }
    Ok(pairs)
}

/// Resolve one scenario/profile pair.
fn pair_outcome(
    scenario: &Scenario,
    profile: &Profile,
    execution: &dyn Execution,
) -> Result<Pair, HarnessError> {
    let scenario_pending = scenario.status == ScenarioStatus::Pending;
    let corpus_missing = profile.corpus == CorpusStatus::Scheduled;
    let outcome = if scenario_pending || corpus_missing {
        Outcome::Pending {
            scenario_pending,
            corpus_missing,
        }
    } else {
        executed(scenario, profile, execution)?
    };
    Ok(Pair {
        scenario_id: scenario.id.clone(),
        profile_name: profile.name.clone(),
        outcome,
    })
}

/// Run a runnable pair, refusing the whole report when there is no case for it.
fn executed(
    scenario: &Scenario,
    profile: &Profile,
    execution: &dyn Execution,
) -> Result<Outcome, HarnessError> {
    match execution.run(scenario, profile) {
        None => Err(HarnessError::NotExecutable {
            scenario: scenario.id.clone(),
            profile: profile.name.clone(),
        }),
        Some(Ok(())) => Ok(Outcome::Passed),
        Some(Err(detail)) => Ok(Outcome::Failed { detail }),
    }
}

/// Render the cross product as a human-readable matrix: one row per scenario,
/// one column per profile, every cell filled, with a legend naming each cell
/// and a summary counting each category separately. Printed by the harness's
/// matrix test (`just conformance` runs it with `--nocapture`).
pub fn matrix(scenarios: &[Scenario], profiles: &[Profile], pairs: &[Pair]) -> String {
    let outcomes = outcome_index(pairs);
    let table = MatrixTable::new(scenarios, profiles);

    let mut out = summary_line(scenarios, profiles, pairs);
    out.push_str(&table.header_row());
    out.push_str(&table.separator_row());
    for scenario in scenarios {
        out.push_str(&table.body_row(scenario, &outcomes));
    }
    out.push('\n');
    out.push_str(&legend());
    out.push('\n');
    out.push_str(&corpora_line(profiles));
    out.push_str(&failures_block(pairs));
    out
}

/// The matrix's row-label column header.
const HEADER_LABEL: &str = "scenario (milestone)";

/// Every cell spelling, with what it means. The order is the order the legend
/// prints and the order the summary counts, so the two cannot drift.
const CELLS: &[(&str, &str)] = &[
    ("pass", "ran and passed"),
    ("FAIL", "ran and failed; the detail is printed below"),
    (
        "pending",
        "the scenario is still prose; the corpus is built",
    ),
    (
        "no-corpus",
        "the scenario is executable; the corpus is not built - a skip, not a result",
    ),
    ("pending,no-corpus", "both"),
];

/// The legend printed under the matrix, generated from [`CELLS`] so a cell
/// cannot exist without a line explaining it.
fn legend() -> String {
    let width = CELLS.iter().map(|(cell, _)| cell.len()).max().unwrap_or(0);
    let mut out = String::from("legend\n");
    for (spelling, meaning) in CELLS {
        out.push_str(&format!("  {spelling:<width$}  {meaning}\n"));
    }
    out
}

/// Lookup from (scenario id, profile name) to the pair's outcome.
type OutcomeIndex<'a> = BTreeMap<(&'a str, &'a str), &'a Outcome>;

/// Index the pairs for cell lookup while rendering.
fn outcome_index(pairs: &[Pair]) -> OutcomeIndex<'_> {
    pairs
        .iter()
        .map(|p| {
            (
                (p.scenario_id.as_str(), p.profile_name.as_str()),
                &p.outcome,
            )
        })
        .collect()
}

/// A scenario's row label: `<id> (<milestone>)`.
fn row_label(scenario: &Scenario) -> String {
    format!("{} ({})", scenario.id, scenario.milestone)
}

/// The matrix table: the profile columns plus the column geometry computed
/// from them, so each row renders against the same widths.
struct MatrixTable<'a> {
    /// The profiles, one column each, in report order.
    profiles: &'a [Profile],
    /// Width of the row-label column: the widest label, header included.
    label_width: usize,
    /// Per-profile column widths, at least as wide as the widest cell spelling.
    col_widths: Vec<usize>,
}

impl<'a> MatrixTable<'a> {
    /// Measure the labels and columns once, up front.
    fn new(scenarios: &[Scenario], profiles: &'a [Profile]) -> Self {
        let label_width = scenarios
            .iter()
            .map(|s| row_label(s).len())
            .chain([HEADER_LABEL.len()])
            .max()
            .unwrap_or(0);
        let cell_width = CELLS.iter().map(|(cell, _)| cell.len()).max().unwrap_or(0);
        let col_widths = profiles
            .iter()
            .map(|p| p.name.len().max(cell_width))
            .collect();
        MatrixTable {
            profiles,
            label_width,
            col_widths,
        }
    }

    /// The column-header row: the row-label header, then one profile per
    /// column.
    fn header_row(&self) -> String {
        let mut row = format!("{HEADER_LABEL:<width$}", width = self.label_width);
        for (profile, width) in self.profiles.iter().zip(&self.col_widths) {
            row.push_str(&format!("  {:<width$}", profile.name));
        }
        row.push('\n');
        row
    }

    /// The dashed rule under the header row.
    fn separator_row(&self) -> String {
        let mut row = "-".repeat(self.label_width);
        for width in &self.col_widths {
            row.push_str("  ");
            row.push_str(&"-".repeat(*width));
        }
        row.push('\n');
        row
    }

    /// One scenario's row: its label, then one cell per profile.
    fn body_row(&self, scenario: &Scenario, outcomes: &OutcomeIndex<'_>) -> String {
        let mut row = format!("{:<width$}", row_label(scenario), width = self.label_width);
        for (profile, width) in self.profiles.iter().zip(&self.col_widths) {
            let outcome = outcomes
                .get(&(scenario.id.as_str(), profile.name.as_str()))
                .copied();
            row.push_str(&format!("  {:<width$}", cell(outcome)));
        }
        row.push('\n');
        row
    }
}

/// The headline: the cross product's arithmetic, then how many pairs fall in
/// each category.
///
/// Each category is counted separately on purpose. A single "pending" total
/// would let a run that reached two of four profiles read as a complete
/// matrix, which is the thing acceptance criterion 1 is about.
fn summary_line(scenarios: &[Scenario], profiles: &[Profile], pairs: &[Pair]) -> String {
    let tally: Vec<String> = CELLS
        .iter()
        .map(|(spelling, _)| {
            let count = pairs
                .iter()
                .filter(|pair| cell(Some(&pair.outcome)) == *spelling)
                .count();
            format!("{count} {spelling}")
        })
        .collect();
    format!(
        "conformance cross product: {} scenarios x {} profiles = {} pairs ({})\n\n",
        scenarios.len(),
        profiles.len(),
        pairs.len(),
        tally.join(", ")
    )
}

/// Render one cell. `MISSING` marks a pair the report failed to produce —
/// it should never appear, and the matrix test asserts it does not.
fn cell(outcome: Option<&Outcome>) -> &'static str {
    match outcome {
        Some(Outcome::Passed) => "pass",
        Some(Outcome::Failed { .. }) => "FAIL",
        Some(Outcome::Pending {
            scenario_pending,
            corpus_missing,
        }) => pending_cell(*scenario_pending, *corpus_missing),
        None => "MISSING",
    }
}

/// Which pending cell a pair earns, from the two reasons it can be pending for.
///
/// `IMPOSSIBLE` is a pending outcome with no reason to be pending. [`report`]
/// cannot produce one — a pair that is neither pending nor corpus-less is run
/// or refused — so it is rendered rather than hidden, on the same principle as
/// `MISSING`.
fn pending_cell(scenario_pending: bool, corpus_missing: bool) -> &'static str {
    match (scenario_pending, corpus_missing) {
        (true, true) => "pending,no-corpus",
        (true, false) => "pending",
        (false, true) => "no-corpus",
        (false, false) => "IMPOSSIBLE",
    }
}

/// The trailing corpora summary: each profile's corpus status and milestone.
fn corpora_line(profiles: &[Profile]) -> String {
    let corpora: Vec<String> = profiles.iter().map(corpus_summary).collect();
    format!("corpora: {}\n", corpora.join(", "))
}

/// One profile's corpora-summary entry: `<name> (<status>, <milestone>)`.
fn corpus_summary(profile: &Profile) -> String {
    format!(
        "{} ({}, {})",
        profile.name,
        corpus_status_label(profile.corpus),
        profile.corpus_milestone
    )
}

/// The lowercase status word used in the corpora summary.
fn corpus_status_label(status: CorpusStatus) -> &'static str {
    match status {
        CorpusStatus::Scheduled => "scheduled",
        CorpusStatus::Built => "built",
    }
}

/// Every failed pair's detail, or nothing when none failed.
fn failures_block(pairs: &[Pair]) -> String {
    let failures: Vec<String> = pairs
        .iter()
        .filter_map(|pair| match &pair.outcome {
            Outcome::Failed { detail } => Some(format!(
                "  {} x {}: {detail}",
                pair.scenario_id, pair.profile_name
            )),
            Outcome::Passed | Outcome::Pending { .. } => None,
        })
        .collect();
    if failures.is_empty() {
        return String::new();
    }
    format!("\nfailures\n{}\n", failures.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::NoExecution;

    /// A pair absent from the report renders as `MISSING` rather than being
    /// silently dropped — the visible-alarm branch the integration tests
    /// assert is never reached on a complete report.
    #[test]
    fn absent_pair_renders_as_missing() {
        let scenarios = [rendering_scenario("lonely-scenario")];
        let profiles = [rendering_profile("minimal")];
        let rendered = matrix(&scenarios, &profiles, &[]);
        assert!(
            rendered.contains("MISSING"),
            "an unaccounted-for cell must be visible: {rendered}"
        );
    }

    /// A pending outcome with no reason to be pending is rendered rather than
    /// hidden. [`report`] cannot produce one, so the branch is reached here.
    #[test]
    fn pending_for_no_reason_renders_as_impossible() {
        assert_eq!(pending_cell(false, false), "IMPOSSIBLE");
    }

    /// The corpora summary renders a built corpus as `built`.
    #[test]
    fn corpora_line_labels_a_built_corpus() {
        let profiles = [Profile {
            corpus: CorpusStatus::Built,
            ..rendering_profile("built-profile")
        }];
        let rendered = matrix(&[], &profiles, &[]);
        assert!(
            rendered.contains("corpora: built-profile (built, M2)"),
            "the corpora line names the built status: {rendered}"
        );
    }

    /// The headline states the cross product's arithmetic and counts every
    /// category separately, so a shrunken matrix is visible in the first line
    /// rather than only in the cells.
    #[test]
    fn summary_line_counts_every_category_separately() {
        let scenarios = [
            rendering_scenario("first-scenario"),
            rendering_scenario("second-scenario"),
        ];
        let profiles = [
            rendering_profile("one"),
            rendering_profile("two"),
            rendering_profile("three"),
        ];
        let pairs = report(&scenarios, &profiles, &NoExecution).expect("all-pending pairs report");
        let rendered = matrix(&scenarios, &profiles, &pairs);
        assert!(
            rendered.starts_with(
                "conformance cross product: 2 scenarios x 3 profiles = 6 pairs \
                 (0 pass, 0 FAIL, 0 pending, 0 no-corpus, 6 pending,no-corpus)\n"
            ),
            "the headline counts each category: {rendered}"
        );
        assert!(
            !rendered.contains("MISSING"),
            "every cell of a complete report is accounted for: {rendered}"
        );
        assert!(
            rendered.contains(&legend()),
            "the legend prints: {rendered}"
        );
        for (spelling, meaning) in CELLS {
            assert!(
                rendered.contains(spelling),
                "the legend names `{spelling}`: {rendered}"
            );
            assert!(
                rendered.contains(meaning),
                "the legend says what `{spelling}` means: {rendered}"
            );
        }
    }

    /// A failed pair renders as `FAIL` and prints its detail beneath the
    /// matrix, so the run says what went wrong without a second command.
    #[test]
    fn a_failed_pair_prints_its_detail() {
        let pairs = [Pair {
            scenario_id: "first-scenario".to_owned(),
            profile_name: "one".to_owned(),
            outcome: Outcome::Failed {
                detail: "the contract did not resolve".to_owned(),
            },
        }];
        let rendered = matrix(
            &[rendering_scenario("first-scenario")],
            &[rendering_profile("one")],
            &pairs,
        );
        assert!(rendered.contains("FAIL"), "the cell says FAIL: {rendered}");
        assert!(
            rendered.contains("failures\n  first-scenario x one: the contract did not resolve"),
            "the detail prints beneath the matrix: {rendered}"
        );
    }

    /// A pending scenario, the status every M3 scenario still carries.
    fn rendering_scenario(id: &str) -> Scenario {
        Scenario {
            id: id.to_string(),
            title: "A scenario for rendering tests".to_string(),
            milestone: crate::schema::Milestone::M2,
            status: ScenarioStatus::Pending,
            contract: "Given/when/then.".to_string(),
        }
    }

    /// A profile whose corpus is still scheduled.
    fn rendering_profile(name: &str) -> Profile {
        Profile {
            name: name.to_string(),
            persona: "a rendering-test persona".to_string(),
            distinguishing_axes: vec!["one axis".to_string()],
            corpus: CorpusStatus::Scheduled,
            corpus_milestone: "M2".to_string(),
        }
    }
}
