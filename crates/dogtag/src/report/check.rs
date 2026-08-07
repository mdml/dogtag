//! The `check` report: one corpus's aggregate health, shared by both renderings.
//!
//! `check` walks the corpus under the shared read path, which validates every
//! note against the resolved contract and resolves every typed link. This
//! module owns the aggregate that walk produces — the diagnostic list in the
//! total order plus summary counts per severity and per identifier — and its
//! renderings. It writes nothing; the severity-to-exit mapping stays with the
//! CLI.

use std::collections::BTreeMap;

use crate::diagnostic::{Diagnostic, DiagnosticList, SeverityCounts};
use crate::note::Corpus;

/// The aggregate corpus-health report behind both of `check`'s renderings.
pub struct CheckReport {
    diagnostics: Vec<Diagnostic>,
    counts: SeverityCounts,
    by_identifier: Vec<(String, usize)>,
}

impl CheckReport {
    /// Every finding, in the diagnostic total order.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Summary counts per severity.
    pub fn counts(&self) -> SeverityCounts {
        self.counts
    }

    /// Summary counts per identifier, in identifier order.
    pub fn by_identifier(&self) -> &[(String, usize)] {
        &self.by_identifier
    }
}

/// Aggregates one corpus walk into the report both renderings share.
///
/// `extra` carries the loading path's diagnostics, so the report is the whole
/// run's answer — selection, opening, and corpus in one list, exactly the
/// diagnostics the exit code weighs.
pub fn check_report(corpus: &Corpus, extra: &[Diagnostic]) -> CheckReport {
    let mut collected = DiagnosticList::new();
    collected.extend(extra.iter().cloned());
    collected.extend(corpus.diagnostics().iter().cloned());
    let counts = collected.counts();
    let diagnostics = collected.sorted();
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    for diagnostic in &diagnostics {
        *tally.entry(diagnostic.id.as_str()).or_default() += 1;
    }
    let by_identifier = tally
        .into_iter()
        .map(|(id, count)| (id.to_string(), count))
        .collect();
    CheckReport {
        diagnostics,
        counts,
        by_identifier,
    }
}

/// Renders the aggregate summary as SDK-owned plain text.
///
/// The findings themselves are diagnostics and travel on the diagnostic
/// stream; this rendering is the result — the counts a reader scans before
/// deciding whether to read them.
pub fn check_text(report: &CheckReport) -> String {
    let counts = report.counts();
    if report.diagnostics().is_empty() {
        return "no findings\n".to_string();
    }
    let mut rendered = format!(
        "findings: {} error(s), {} warning(s), {} info\n",
        counts.error, counts.warning, counts.info
    );
    let width = report
        .by_identifier()
        .iter()
        .fold(0, |widest, (id, _)| widest.max(id.len()));
    for (id, count) in report.by_identifier() {
        rendered.push_str(&format!("  {id:width$}  {count}\n"));
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::parse_contract;
    use crate::diagnostic::{KernelDiagnostic, Severity};
    use crate::note::read_corpus;
    use crate::vault::{SENTINEL, VaultRoot, tree::Tree};
    use std::fs;

    const CONTRACT: &str = concat!(
        "contract_version = 3\n",
        "\n[dialect]\nlinks = \"wikilink\"\n",
        "\n[lifecycle]\nnone = true\n",
        "\n[tags]\nproperty = \"labels\"\n",
        "\n[[type]]\nname = \"person\"\ncapabilities = [\"identity-bearing\"]\n",
        "\n  [[type.property]]\n  name = \"name\"\n  kind = \"string\"\n",
        "\n  [[type.property]]\n  name = \"labels\"\n  kind = \"list\"\n  of = \"string\"\n",
        "\n  [[type.relationship]]\n  predicate = \"knows\"\n",
        "\n[[type]]\nname = \"capture\"\ncapabilities = [\"catch-all\"]\n",
    );

    fn read(tree: &Tree, notes: &[(&str, &str)]) -> Corpus {
        let root = tree.vault("vault");
        fs::write(root.join(SENTINEL), CONTRACT).expect("contract");
        for (path, contents) in notes {
            let target = root.join(path);
            let parent = target.parent().expect("a note path is never the root");
            fs::create_dir_all(parent).expect("note directory");
            fs::write(&target, contents).expect("note");
        }
        let contract = parse_contract(CONTRACT)
            .contract
            .expect("resolved contract");
        read_corpus(&VaultRoot::new(root), &contract)
    }

    #[test]
    fn a_clean_corpus_reports_no_findings() {
        let tree = Tree::new("check-clean");
        let corpus = read(
            &tree,
            &[(
                "people/ada.md",
                "---\ntype: person\nname: Ada\n---\n# Ada\n",
            )],
        );
        let report = check_report(&corpus, &[]);
        assert!(report.diagnostics().is_empty());
        assert_eq!(report.counts(), SeverityCounts::zero());
        assert!(report.by_identifier().is_empty());
        assert_eq!(check_text(&report), "no findings\n");
    }

    #[test]
    fn findings_arrive_in_the_total_order_with_loading_diagnostics_merged() {
        let tree = Tree::new("check-merged");
        let corpus = read(
            &tree,
            &[(
                "people/ada.md",
                "---\ntype: person\nname: Ada\nknows: \"[[missing]]\"\n---\n",
            )],
        );
        let planted = Diagnostic::kernel(KernelDiagnostic::DiscoveryNestedVault, "an ancestor");
        let report = check_report(&corpus, &[planted]);
        let ids: Vec<&str> = report
            .diagnostics()
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect();
        assert!(ids.contains(&"link.dangling-typed-link"), "{ids:?}");
        assert!(ids.contains(&"discovery.nested-vault"), "{ids:?}");
        assert_eq!(report.counts().error, 1);
        assert_eq!(report.counts().warning, 1);
    }

    #[test]
    fn identifier_counts_tally_and_sort_by_identifier() {
        let tree = Tree::new("check-tally");
        let corpus = read(
            &tree,
            &[
                (
                    "people/ada.md",
                    "---\ntype: person\nname: Ada\nknows: \"[[missing]]\"\n---\n",
                ),
                (
                    "people/grace.md",
                    "---\ntype: person\nname: Grace\nknows: \"[[also-missing]]\"\n---\n",
                ),
                ("marked.md", "\u{feff}plain\n"),
            ],
        );
        let report = check_report(&corpus, &[]);
        let tally: Vec<(&str, usize)> = report
            .by_identifier()
            .iter()
            .map(|(id, count)| (id.as_str(), *count))
            .collect();
        assert_eq!(
            tally,
            [("link.dangling-typed-link", 2), ("note.byte-order-mark", 1)]
        );
        let rendered = check_text(&report);
        assert!(rendered.starts_with("findings: 2 error(s), 1 warning(s), 0 info\n"));
        assert!(
            rendered.contains("  link.dangling-typed-link  2\n"),
            "{rendered}"
        );
        assert!(
            rendered.contains("  note.byte-order-mark      1\n"),
            "{rendered}"
        );
    }

    #[test]
    fn an_info_only_report_counts_no_failures() {
        let tree = Tree::new("check-info");
        let corpus = read(&tree, &[]);
        let planted = Diagnostic::new(
            crate::diagnostic::DiagnosticId::kernel(KernelDiagnostic::DiscoveryNestedVault),
            Severity::Info,
            "a permanent, accepted condition",
        );
        let report = check_report(&corpus, &[planted]);
        assert_eq!(report.counts().error, 0);
        assert_eq!(report.counts().warning, 0);
        assert_eq!(report.counts().info, 1);
    }
}
