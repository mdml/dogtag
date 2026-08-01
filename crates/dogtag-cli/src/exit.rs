//! Severity to exit code.
//!
//! Three codes exist and no more. `0` and `1` are decided by **severity
//! alone**, and `2` is reserved for an argument-parsing failure that produces
//! no diagnostic at all — which is why an unregistered `--vault work` exits
//! `1`: it arrives as a bad argument and leaves as an installation-area
//! diagnostic, and a caller distinguishing "you typed something impossible"
//! from "your vault has a problem" needs those to be different answers.
//!
//! Argument parsing is `clap`'s, and its failures exit `2` without reaching
//! this module.

use dogtag::diagnostic::{Diagnostic, DiagnosticList, SeverityCounts};

/// Nothing needs attention.
pub const SUCCESS: i32 = 0;

/// Something does.
pub const FAILURE: i32 = 1;

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
