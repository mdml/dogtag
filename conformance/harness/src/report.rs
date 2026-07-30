//! The scenarios × profiles cross product and its human-readable rendering.
//!
//! [`report`] takes no filter parameter — the signature is part of the
//! no-waiver enforcement, alongside the strict schemas and loaders.

use std::collections::BTreeMap;

use crate::error::HarnessError;
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
///
/// At M1 the only possible outcome is [`Outcome::Pending`]. Pass/fail
/// variants arrive with the execution path, at the milestone that graduates
/// the first scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Not runnable yet: the scenario is still a prose contract, the
    /// profile's corpus is not built, or both.
    Pending {
        /// The scenario's status is still `pending`.
        scenario_pending: bool,
        /// The profile's corpus is not yet built.
        corpus_missing: bool,
    },
}

/// Compute the full cross product: every scenario against every profile,
/// each pair exactly once. No filter parameter exists — the signature is
/// part of the no-waiver enforcement.
///
/// Errors with [`HarnessError::NotExecutable`] if a pair is runnable
/// (executable scenario × built corpus) because execution is not wired yet;
/// the harness refuses to invent an outcome for a pair it should be running.
pub fn report(scenarios: &[Scenario], profiles: &[Profile]) -> Result<Vec<Pair>, HarnessError> {
    let mut pairs = Vec::with_capacity(scenarios.len() * profiles.len());
    for scenario in scenarios {
        for profile in profiles {
            pairs.push(pair_outcome(scenario, profile)?);
        }
    }
    Ok(pairs)
}

/// Resolve one scenario/profile pair, refusing a runnable pair because
/// execution is not wired yet.
fn pair_outcome(scenario: &Scenario, profile: &Profile) -> Result<Pair, HarnessError> {
    let scenario_pending = scenario.status == ScenarioStatus::Pending;
    let corpus_missing = profile.corpus == CorpusStatus::Scheduled;
    if !scenario_pending && !corpus_missing {
        return Err(HarnessError::NotExecutable {
            scenario: scenario.id.clone(),
            profile: profile.name.clone(),
        });
    }
    Ok(Pair {
        scenario_id: scenario.id.clone(),
        profile_name: profile.name.clone(),
        outcome: Outcome::Pending {
            scenario_pending,
            corpus_missing,
        },
    })
}

/// Render the cross product as a human-readable matrix: one row per
/// scenario, one column per profile, every cell filled. Printed by the
/// harness's matrix test (`just conformance` runs it with `--nocapture`).
pub fn pending_matrix(scenarios: &[Scenario], profiles: &[Profile], pairs: &[Pair]) -> String {
    let outcomes = outcome_index(pairs);
    let table = MatrixTable::new(scenarios, profiles);

    let mut out = summary_line(scenarios, profiles, pairs);
    out.push_str(&table.header_row());
    out.push_str(&table.separator_row());
    for scenario in scenarios {
        out.push_str(&table.body_row(scenario, &outcomes));
    }
    out.push('\n');
    out.push_str(&corpora_line(profiles));
    out
}

/// The matrix's row-label column header.
const HEADER_LABEL: &str = "scenario (milestone)";

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
    /// Per-profile column widths; every cell renders `pending` (7 chars),
    /// so columns are at least that wide.
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
        let col_widths = profiles.iter().map(|p| p.name.len().max(7)).collect();
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

/// The headline: total cross-product size and how much of it is pending.
fn summary_line(scenarios: &[Scenario], profiles: &[Profile], pairs: &[Pair]) -> String {
    let pending_count = pairs
        .iter()
        .filter(|p| matches!(p.outcome, Outcome::Pending { .. }))
        .count();
    format!(
        "conformance cross product: {} scenarios x {} profiles = {} pairs ({} pending)\n\n",
        scenarios.len(),
        profiles.len(),
        pairs.len(),
        pending_count
    )
}

/// Render one cell. `MISSING` marks a pair the report failed to produce —
/// it should never appear, and the matrix test asserts it does not.
fn cell(outcome: Option<&Outcome>) -> &'static str {
    match outcome {
        Some(Outcome::Pending { .. }) => "pending",
        None => "MISSING",
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A pair absent from the report renders as `MISSING` rather than being
    /// silently dropped — the visible-alarm branch the integration tests
    /// assert is never reached on a complete report.
    #[test]
    fn absent_pair_renders_as_missing() {
        let scenarios = [Scenario {
            id: "lonely-scenario".to_string(),
            title: "A scenario with no reported pairs".to_string(),
            milestone: crate::schema::Milestone::M2,
            status: ScenarioStatus::Pending,
            contract: "Given/when/then.".to_string(),
        }];
        let profiles = [Profile {
            name: "minimal".to_string(),
            persona: "a rendering-test persona".to_string(),
            distinguishing_axes: vec!["one axis".to_string()],
            corpus: CorpusStatus::Scheduled,
            corpus_milestone: "M2".to_string(),
        }];
        let matrix = pending_matrix(&scenarios, &profiles, &[]);
        assert!(
            matrix.contains("MISSING"),
            "an unaccounted-for cell must be visible: {matrix}"
        );
    }

    /// The corpora summary renders a built corpus as `built` — the branch
    /// the all-scheduled M1 fixtures never reach.
    #[test]
    fn corpora_line_labels_a_built_corpus() {
        let profiles = [Profile {
            name: "built-profile".to_string(),
            persona: "a rendering-test persona".to_string(),
            distinguishing_axes: vec!["one axis".to_string()],
            corpus: CorpusStatus::Built,
            corpus_milestone: "M2".to_string(),
        }];
        let matrix = pending_matrix(&[], &profiles, &[]);
        assert!(
            matrix.contains("corpora: built-profile (built, M2)"),
            "the corpora line names the built status: {matrix}"
        );
    }

    /// The headline states the cross product's arithmetic — scenarios times
    /// profiles equals pairs — and counts the pending ones separately, so a
    /// shrunken matrix is visible in the first line rather than only in the
    /// cells.
    #[test]
    fn summary_line_counts_the_whole_cross_product() {
        let scenarios = [
            rendering_scenario("first-scenario"),
            rendering_scenario("second-scenario"),
        ];
        let profiles = [
            rendering_profile("one"),
            rendering_profile("two"),
            rendering_profile("three"),
        ];
        let pairs = report(&scenarios, &profiles).expect("all-pending pairs report");
        let matrix = pending_matrix(&scenarios, &profiles, &pairs);
        assert!(
            matrix.starts_with(
                "conformance cross product: 2 scenarios x 3 profiles = 6 pairs (6 pending)\n"
            ),
            "the headline counts the cross product: {matrix}"
        );
        assert!(
            !matrix.contains("MISSING"),
            "every cell of a complete report is accounted for: {matrix}"
        );
    }

    /// A pending scenario, the only status the M1 matrix renders.
    fn rendering_scenario(id: &str) -> Scenario {
        Scenario {
            id: id.to_string(),
            title: "A scenario for rendering tests".to_string(),
            milestone: crate::schema::Milestone::M2,
            status: ScenarioStatus::Pending,
            contract: "Given/when/then.".to_string(),
        }
    }

    /// A profile whose corpus is still scheduled, as every M1 corpus is.
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
