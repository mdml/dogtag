//! Severity to exit code — and, for a write verb, the transaction to one.
//!
//! Three codes exist and no more. For a **read** verb `0` and `1` are decided
//! by **severity alone**, and `2` is reserved for an argument-parsing failure
//! that produces no diagnostic at all — which is why an unregistered
//! `--vault work` exits `1`: it arrives as a bad argument and leaves as an
//! installation-area diagnostic, and a caller distinguishing "you typed
//! something impossible" from "your vault has a problem" needs those to be
//! different answers.
//!
//! Argument parsing is `clap`'s, and its failures exit `2` without reaching
//! this module. Two more faults take that code: a vault selector given but
//! empty, which names no vault and so leaves nothing to diagnose, and a
//! capture whose thought could not be read at all.
//!
//! # A write verb answers a different question
//!
//! A read verb answers *what is true of the corpus*, so severity is exactly
//! its answer. A write verb answers *did my act land*, and that is a different
//! question about a different subject: a capture into a corpus carrying
//! pre-existing errors landed, and reporting failure because the vault was
//! untidy would teach callers to ignore the one signal an exit code exists to
//! carry. So [`for_write`] follows the transaction's verdict, the corpus's
//! findings ride the structured result for triage, and there is no `--strict`
//! on a write verb to blur the two. This split is deliberate and recorded; it
//! is the one place in this crate where severity does not decide.

use dogtag::diagnostic::{Diagnostic, DiagnosticList, SeverityCounts};

/// Nothing needs attention.
pub const SUCCESS: i32 = 0;

/// Something does.
pub const FAILURE: i32 = 1;

/// The argument or environment named nothing this run could act on.
///
/// Reserved for faults that produce no diagnostic because there was nothing to
/// diagnose. clap exits `2` without reaching this module; an empty vault
/// selector is the same kind of fault and takes the same code.
pub const USAGE: i32 = 2;

/// The code a run's diagnostics earn.
///
/// `strict` promotes warnings **for this decision only**. It changes no
/// rendering, no severity and no message, so a strict run and an ordinary one
/// over the same vault differ by their exit code and by nothing else — which
/// is what lets a scheduled check detect a wrong-vault resolution from its
/// exit code without parsing anything.
pub fn code(counts: SeverityCounts, strict: bool) -> i32 {
    if counts.error > 0 || promoted(counts, strict) {
        FAILURE
    } else {
        SUCCESS
    }
}

/// Whether strictness turns this run's warnings into a failure.
fn promoted(counts: SeverityCounts, strict: bool) -> bool {
    strict && counts.warning > 0
}

/// The code a write verb's transaction earns.
///
/// `landed` is the transaction's own verdict and never a tally of what the
/// corpus had to say. A preview landed: the act it was asked to perform is
/// *emit the plan and write nothing*, and that is what happened.
pub fn for_write(landed: bool) -> i32 {
    if landed { SUCCESS } else { FAILURE }
}

/// The code a list of diagnostics earns.
///
/// The tally is the SDK's rather than a second count kept here, so a severity
/// can never be weighed differently by the consumer that reports it.
pub fn code_for(diagnostics: &[Diagnostic], strict: bool) -> i32 {
    let mut list = DiagnosticList::new();
    list.extend(diagnostics.iter().cloned());
    code(list.counts(), strict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dogtag::diagnostic::KernelDiagnostic;

    fn counts(error: usize, warning: usize, info: usize) -> SeverityCounts {
        SeverityCounts {
            error,
            warning,
            info,
        }
    }

    #[test]
    fn nothing_at_all_succeeds() {
        assert_eq!(code(SeverityCounts::zero(), false), SUCCESS);
        assert_eq!(code(SeverityCounts::zero(), true), SUCCESS);
    }

    #[test]
    fn an_error_fails_however_the_run_was_asked_for() {
        assert_eq!(code(counts(1, 0, 0), false), FAILURE);
        assert_eq!(code(counts(1, 0, 0), true), FAILURE);
    }

    #[test]
    fn a_warning_fails_only_under_strict() {
        assert_eq!(code(counts(0, 1, 0), false), SUCCESS);
        assert_eq!(code(counts(0, 1, 0), true), FAILURE);
    }

    #[test]
    fn information_never_decides_anything() {
        assert_eq!(code(counts(0, 0, 3), true), SUCCESS);
    }

    /// The write verb's mapping, and the whole of it: the act decides, and
    /// nothing the corpus said reaches this.
    #[test]
    fn a_write_verbs_code_is_its_transactions_verdict() {
        assert_eq!(for_write(true), SUCCESS);
        assert_eq!(for_write(false), FAILURE);
    }

    #[test]
    fn a_list_is_weighed_by_the_severities_it_holds() {
        let refused = vec![Diagnostic::kernel(
            KernelDiagnostic::DiscoveryNoVaultFound,
            "no vault here",
        )];
        assert_eq!(code_for(&refused, false), FAILURE);
        assert_eq!(code_for(&[], false), SUCCESS);
    }
}
