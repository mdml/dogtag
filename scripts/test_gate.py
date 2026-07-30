#!/usr/bin/env python3
"""Adversarial tests for the gate runner and the gate/CI parity checker.

The properties under test are the ones that would let a gate lie. Summary
mode is the mode people read, so the tests that matter are the ones proving
it hides nothing: that it runs the same commands verbose does, that a child's
failure survives being captured, that a metric which cannot be found stays
absent rather than becoming a number, and that a check with nothing to
measure reports a skip rather than a pass.

The equivalence tests deliberately compare the child's **whole** argv, cwd
and environment rather than a hand-picked variable or two. An earlier version
sampled `$RUSTDOCFLAGS` and tty-ness and stayed green against four separately
injected mode divergences — dropping a step when verbose, changing the cwd,
appending an argument, adding an environment variable — so it proved nothing
it claimed to.

Steps are exercised with real child processes rather than mocks wherever a
real one will do: the bugs this file exists to catch live in the seam between
Python and the shell, and a mock has no such seam.

Stdlib only (unittest). Run directly, or via `just check`.
"""

import json
import os
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import gate  # noqa: E402  (path shim above; scripts/ is not a package)

check_gate_parity = __import__("check-gate-parity")  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parent.parent
CI = ".github/workflows/ci.yml"


def scratch(case: unittest.TestCase) -> Path:
    """A temporary directory removed when the test ends."""
    tmp = Path(tempfile.mkdtemp(prefix="test-gate-"))
    case.addCleanup(shutil.rmtree, tmp, ignore_errors=True)
    return tmp


def quoted_step(step_id: str, script: str, env: tuple = ()) -> gate.Step:
    """A step that runs `script` under sh, quoted so a path may hold spaces."""
    return gate.Step(step_id, f"sh -c {shlex.quote(script)}", None, env)


class SuiteShapeTest(unittest.TestCase):
    """The step table's own invariants, which no run can violate."""

    def test_the_suites_nest_strictly(self) -> None:
        self.assertEqual(gate.CHECK[: len(gate.FAST)], gate.FAST)
        self.assertEqual(gate.GATE[: len(gate.CHECK)], gate.CHECK)

    def test_no_step_is_declared_twice(self) -> None:
        ids = [step.id for step in gate.STEPS]
        self.assertEqual(len(ids), len(set(ids)))

    def test_a_suite_names_only_real_steps(self) -> None:
        for name, ids in gate.SUITES.items():
            for step_id in ids:
                self.assertIn(step_id, gate.BY_ID, f"{name} names {step_id}")

    def test_resolving_a_suite_twice_runs_each_step_once(self) -> None:
        """`gate.py check check fmt` must not compile the workspace twice."""
        steps, unknown = gate.resolve(["check", "check", "fmt"])
        self.assertEqual(unknown, [])
        self.assertEqual([step.id for step in steps], list(gate.CHECK))

    def test_an_unknown_name_is_reported_rather_than_ignored(self) -> None:
        _, unknown = gate.resolve(["chekc"])
        self.assertEqual(unknown, ["chekc"])

    def test_every_declared_metric_names_a_real_extractor(self) -> None:
        for step in gate.STEPS:
            if step.metric is not None:
                self.assertIn(step.metric, gate.EXTRACTORS, step.id)

    def test_every_command_parses_into_an_argv(self) -> None:
        for step in gate.STEPS:
            self.assertTrue(step.argv, step.id)

    def test_the_docs_gate_still_carries_the_flag_that_makes_it_a_gate(self) -> None:
        self.assertEqual(gate.BY_ID["docs"].env, gate.RUSTDOC_STRICT)


class ModeEquivalenceTest(unittest.TestCase):
    """Summary and verbose must differ in rendering and in nothing else."""

    def child_state(self, verbose: bool) -> dict:
        """Everything the child can see about how it was launched."""
        tmp = scratch(self)
        script = (
            "import json, os, sys; "
            "print(json.dumps({'argv': sys.argv, 'cwd': os.getcwd(), "
            "'env': dict(os.environ), 'tty': sys.stdin.isatty()}))"
        )
        step = gate.Step(
            "probe", f"python3 -c {shlex.quote(script)} tail-arg", None, gate.RUSTDOC_STRICT
        )
        with mock.patch("sys.stdout"):
            gate.run_step(step, tmp, verbose)
        return json.loads((tmp / "probe.log").read_text())

    def test_both_modes_launch_an_identical_child(self) -> None:
        """argv, cwd, the whole environment, and a closed stdin — all equal."""
        self.assertEqual(self.child_state(False), self.child_state(True))

    def test_the_child_gets_the_environment_and_stdin_the_contract_promises(self) -> None:
        state = self.child_state(False)
        self.assertEqual(state["env"]["RUSTDOCFLAGS"], "-D warnings")
        self.assertEqual(state["cwd"], str(gate.ROOT))
        self.assertFalse(state["tty"])
        self.assertEqual(state["argv"][-1], "tail-arg")

    def ran_under(self, verbose: bool) -> list[str]:
        """The step ids a run of the same table actually executed."""
        tmp = scratch(self)
        marker = tmp / "ran"
        steps = tuple(
            quoted_step(name, f"echo {name} >> {shlex.quote(str(marker))}")
            for name in ("alpha", "beta", "gamma")
        )
        with mock.patch("sys.stdout"):
            gate.run_steps(steps, verbose, tmp)
        return marker.read_text().split()

    def test_both_modes_execute_the_same_steps(self) -> None:
        self.assertEqual(self.ran_under(False), ["alpha", "beta", "gamma"])
        self.assertEqual(self.ran_under(False), self.ran_under(True))

    def test_stdin_is_closed_so_a_prompting_tool_cannot_hang(self) -> None:
        """A tool that reads stdin gets EOF at once, in both modes.

        If this regresses to an inherited stdin the test hangs rather than
        fails — which is the same way the bug would present in a real run.
        """
        tmp = scratch(self)
        sink = tmp / "stdin"
        for verbose in (False, True):
            step = quoted_step("reader", f"cat > {shlex.quote(str(sink))}")
            with mock.patch("sys.stdout"):
                result = gate.run_step(step, tmp, verbose)
            self.assertEqual(result.status, "pass")
            self.assertEqual(sink.read_text(), "")


class ExitStatusTest(unittest.TestCase):
    """A failure must survive being captured, and keep its shell meaning."""

    def run_quiet(self, command: str, verbose: bool = False) -> gate.Result:
        with mock.patch("sys.stdout"):
            return gate.run_step(gate.Step("child", command), scratch(self), verbose)

    def test_a_failing_child_propagates_its_own_status_in_both_modes(self) -> None:
        for verbose in (False, True):
            with self.subTest(verbose=verbose):
                self.assertEqual(self.run_quiet("sh -c 'exit 3'", verbose).code, 3)

    def test_a_child_killed_by_a_signal_reports_the_shell_convention(self) -> None:
        """subprocess reports -15; a shell reports 143, and so must this."""
        for verbose in (False, True):
            with self.subTest(verbose=verbose):
                self.assertEqual(self.run_quiet("sh -c 'kill -TERM $$'", verbose).code, 143)

    def test_a_failure_can_never_render_as_a_zero_exit(self) -> None:
        for returncode in (-15, -2, 1, 3, 255, 256):
            self.assertNotEqual(gate.exit_code(returncode), 0, returncode)

    def pipeline_code(self, verbose: bool) -> int:
        steps = (
            gate.Step("ok", "true"),
            gate.Step("bad", "sh -c 'exit 7'"),
            gate.Step("worse", "sh -c 'exit 9'"),
        )
        with mock.patch("sys.stdout"), mock.patch("sys.stderr"):
            results = gate.run_steps(steps, verbose, scratch(self))
            return gate.finish("gate", results, 0.0, verbose)

    def test_both_modes_exit_with_the_first_failure(self) -> None:
        self.assertEqual(self.pipeline_code(False), 7)
        self.assertEqual(self.pipeline_code(True), 7)


class FailureVisibilityTest(unittest.TestCase):
    """Summary mode must diagnose a failure without a second run."""

    def failure_report(self, verbose: bool) -> str:
        """The stderr dump for a step whose output is not in its command."""
        script = "printf '%s\\n' \"the\" \"real\" \"reason\" >&2; exit 4"
        step = quoted_step("noisy", script)
        with mock.patch("sys.stdout"):
            results = [gate.run_step(step, scratch(self), verbose)]
        return capture_stderr(lambda: gate.report_failures(results, verbose))

    def test_the_failing_command_output_reaches_stderr_in_summary_mode(self) -> None:
        report = self.failure_report(False)
        self.assertIn("the\nreal\nreason", report)
        self.assertIn("exit 4", report)

    def test_verbose_names_the_failure_without_repeating_the_streamed_body(self) -> None:
        """The body already streamed; repeating it doubles a long failure."""
        report = self.failure_report(True)
        self.assertIn("exit 4", report)
        self.assertNotIn("the\nreal\nreason", report)

    def test_the_reported_command_is_the_one_that_ran(self) -> None:
        """`commits` gains a range and `docs` a flag; the dump must show them."""
        job = gate.prepare(gate.BY_ID["docs"])
        self.assertIn("RUSTDOCFLAGS=", job.rendered())
        self.assertIn("cargo doc", job.rendered())

    def test_a_skip_is_counted_rather_than_folded_into_the_passes(self) -> None:
        results = [
            gate.Result(gate.prepare(gate.BY_ID["fmt"]), "pass", "", 0, None),
            gate.Result(gate.prepare(gate.BY_ID["links"]), "skip", "nothing to do", 0, None),
        ]
        self.assertEqual(gate.tally(results), "1/2 steps, 1 skipped")


class LabelTest(unittest.TestCase):
    """A partial run must not sign off with the full suite's authority."""

    def test_a_suite_run_is_labelled_by_its_suite(self) -> None:
        for suite in gate.SUITES:
            self.assertEqual(gate.label_for([suite]), suite)

    def test_a_run_of_named_steps_is_not_labelled_gate(self) -> None:
        self.assertEqual(gate.label_for(["coverage"]), gate.STEPS_LABEL)
        self.assertEqual(gate.label_for(["fmt", "clippy"]), gate.STEPS_LABEL)

    def test_every_label_has_an_authority_note(self) -> None:
        for label in list(gate.SUITES) + [gate.STEPS_LABEL]:
            self.assertIn(label, gate.AUTHORITY)

    def test_a_partial_run_does_not_claim_the_gate_is_authoritative(self) -> None:
        self.assertNotIn("rulesets", gate.AUTHORITY[gate.STEPS_LABEL])


class ArgumentTest(unittest.TestCase):
    """A mistyped flag must not silently run something else."""

    def run_main(self, *argv: str) -> int:
        with mock.patch.object(sys, "argv", ["gate.py", *argv]):
            with mock.patch("sys.stdout"), mock.patch("sys.stderr"):
                return gate.main()

    def test_an_unknown_flag_is_a_usage_error(self) -> None:
        self.assertEqual(self.run_main("fmt", "--verbos"), 2)

    def test_an_unknown_step_is_a_usage_error(self) -> None:
        self.assertEqual(self.run_main("fmtt"), 2)

    def test_no_arguments_is_a_usage_error(self) -> None:
        self.assertEqual(self.run_main(), 2)

    def test_list_refuses_to_be_combined_with_a_suite(self) -> None:
        self.assertEqual(self.run_main("--list", "gate"), 2)


class ExtractorTest(unittest.TestCase):
    """A metric is rendering. It may be absent; it may never be invented."""

    def test_no_extractor_invents_a_number_from_noise(self) -> None:
        for name, extract in gate.EXTRACTORS.items():
            self.assertIsNone(extract("unrelated output\nnothing to see\n"), name)
            self.assertIsNone(extract(""), name)

    def test_test_counts_sum_every_harness_in_the_run(self) -> None:
        text = (
            "test result: ok. 11 passed; 0 failed; 0 ignored\n"
            "test result: ok. 40 passed; 0 failed; 0 ignored\n"
        )
        self.assertEqual(gate.test_count(text), "51 tests")

    def test_a_failed_harness_line_is_not_counted_as_passes(self) -> None:
        self.assertIsNone(gate.test_count("test result: FAILED. 0 passed; 3 failed\n"))

    def test_metrics_survive_the_colour_ci_forces_through_the_pipe(self) -> None:
        """CI sets CARGO_TERM_COLOR=always, so captured text carries escapes."""
        coloured = "\x1b[32mtest result\x1b[0m: ok. 51 passed; 0 failed\n"
        self.assertEqual(gate.test_count(gate.strip_ansi(coloured)), "51 tests")

    def test_cargo_deny_warnings_are_not_hidden_behind_an_ok(self) -> None:
        clean = "advisories ok, bans ok, licenses ok, sources ok\n"
        noisy = "warning[unmaintained]: foo\n" + clean
        self.assertEqual(gate.deny_summary(clean), clean.strip())
        self.assertIn("1 warnings", gate.deny_summary(noisy))

    def test_the_zizmor_findings_line_is_matched_as_zizmor_writes_it(self) -> None:
        self.assertEqual(gate.zizmor_summary("10 findings: 9 informational, 1 medium\n"), "10 findings")
        self.assertEqual(
            gate.zizmor_summary("No findings to report. Good job! (10 suppressed)\n"),
            "no findings (10 filtered out)",
        )

    def test_the_repository_summary_line_is_read_from_the_last_match(self) -> None:
        text = "check-links: stale\nnoise\ncheck-links: all 201 relative markdown links resolve.\n"
        self.assertEqual(gate.repo_summary(text, "check-links"), "all 201 relative markdown links resolve")

    def test_an_absent_metric_renders_as_nothing_not_as_zero(self) -> None:
        log = scratch(self) / "empty.log"
        log.write_text("no summary here\n")
        with mock.patch("sys.stderr"):
            self.assertEqual(gate.measure(gate.BY_ID["links"], log), "")


class RepositoryContractTest(unittest.TestCase):
    """The in-repo checkers really do print the line the runner reads.

    This is the half an env-var handshake could not prove: it runs each
    checker and reads its actual output, so a reworded summary fails here
    instead of silently emptying the metric column. The three not covered
    need a nightly toolchain, a CodeScene token, and a commit range that
    exists — `coverage-check`, `codescene-gate`, and commit-lint outside a
    branch — so `just gate` is what exercises those.
    """

    def assert_emits_summary(self, step_id: str, *extra: str) -> None:
        step = gate.BY_ID[step_id]
        done = subprocess.run(
            step.argv + list(extra), cwd=REPO_ROOT, capture_output=True, text=True, check=False
        )
        self.assertEqual(done.returncode, 0, done.stderr)
        found = gate.EXTRACTORS[step.metric](gate.strip_ansi(done.stdout))
        self.assertIsNotNone(found, f"{step_id} printed no `{step.metric}: …` line")

    def test_tool_pins_prints_its_summary(self) -> None:
        self.assert_emits_summary("tool-pins")

    def test_ruleset_payloads_prints_its_summary(self) -> None:
        self.assert_emits_summary("rulesets")

    def test_security_exceptions_prints_its_summary(self) -> None:
        self.assert_emits_summary("exceptions")

    def test_gate_parity_prints_its_summary(self) -> None:
        self.assert_emits_summary("gate-parity")

    def test_links_prints_its_summary(self) -> None:
        self.assert_emits_summary("links")

    def test_commit_lint_prints_its_summary(self) -> None:
        self.assert_emits_summary("commits", "--range", "HEAD~1..HEAD")


class ScopeTest(unittest.TestCase):
    """Neither mode may narrow what an expensive gate actually measures."""

    def test_codescene_scores_the_whole_repository(self) -> None:
        command = gate.BY_ID["codescene"].command
        for narrowing in ("--files", "--staged", "--branch"):
            self.assertNotIn(narrowing, command)
        self.assertEqual(command, "scripts/codescene-gate.sh")

    def test_coverage_runs_the_enforcing_script_with_no_flags(self) -> None:
        """Thresholds, the ratchet and the kernel rule all live behind this."""
        self.assertEqual(gate.BY_ID["coverage"].command, "scripts/coverage-gate.sh")

    def test_an_ordinary_step_is_never_rewritten_before_it_runs(self) -> None:
        for step_id in ("coverage", "codescene", "clippy", "tests"):
            step = gate.BY_ID[step_id]
            self.assertNotIn(step_id, gate.PREPARE)
            self.assertEqual(gate.prepare(step).argv, step.argv)


class CommitRangeTest(unittest.TestCase):
    """An empty range validated nothing, and must not report a pass."""

    def test_an_empty_range_is_a_skip(self) -> None:
        with mock.patch.object(gate, "git_output", return_value="0"):
            job = gate.prepare(gate.BY_ID["commits"])
        self.assertIn("no commits in", job.skip)

    def test_a_non_empty_range_runs_with_the_range_appended(self) -> None:
        with mock.patch.object(gate, "git_output", return_value="4"):
            job = gate.prepare(gate.BY_ID["commits"])
        self.assertEqual(job.skip, "")
        self.assertIn("--range", job.argv)


class PrerequisiteTest(unittest.TestCase):
    """A missing prerequisite fails its step, and only its step.

    Three behaviours that have to hold together. It must be a **failure**,
    never a skip: a Code Health gate that reports success without a token
    measured nothing. It must be scoped to the steps that need it, so an
    absent token cannot cost eighteen unrelated gates. And it must be
    resolved per step rather than as a suite-wide preflight, so the run
    still happens and the report is still complete.
    """

    def without_token(self):
        return mock.patch.dict(os.environ, {"CS_ACCESS_TOKEN": ""}, clear=False)

    def test_a_missing_token_names_where_to_get_one(self) -> None:
        with self.without_token():
            missing = gate.missing_prereqs({"codescene"})
        self.assertTrue(any("codescene.io/users/me/pat" in m for m in missing), missing)

    def test_a_missing_tool_names_how_to_install_it(self) -> None:
        with mock.patch.object(gate.shutil, "which", return_value=None):
            missing = gate.missing_prereqs({"deny", "osv"})
        self.assertTrue(any("just install-dev-tools" in m for m in missing), missing)
        self.assertTrue(any("github.com/google/osv-scanner" in m for m in missing), missing)

    def test_several_missing_prerequisites_are_all_named(self) -> None:
        """cs, jq and the token are three separate things to fix."""
        with self.without_token(), mock.patch.object(gate.shutil, "which", return_value=None):
            missing = gate.missing_prereqs({"codescene"})
        self.assertEqual(len(missing), 3, missing)
        self.assertTrue(any("`cs`" in m for m in missing), missing)
        self.assertTrue(any("`jq`" in m for m in missing), missing)
        self.assertTrue(any("CS_ACCESS_TOKEN" in m for m in missing), missing)

    def test_prerequisites_are_resolved_per_step_not_across_the_suite(self) -> None:
        with self.without_token():
            self.assertEqual(gate.missing_prereqs({"fmt", "clippy", "tests"}), [])
            self.assertTrue(gate.missing_prereqs({"codescene"}))

    def test_every_prerequisite_names_steps_that_exist(self) -> None:
        for prereq in gate.PREREQS:
            for step_id in prereq.steps:
                self.assertIn(step_id, gate.BY_ID, prereq.label)

    def blocked_step(self, verbose: bool) -> gate.Result:
        with self.without_token(), mock.patch("sys.stdout"):
            return gate.run_step(gate.BY_ID["codescene"], scratch(self), verbose)

    def test_a_blocked_step_fails_rather_than_skipping(self) -> None:
        result = self.blocked_step(False)
        self.assertEqual(result.status, "fail")
        self.assertNotEqual(result.code, 0)
        self.assertIn("CS_ACCESS_TOKEN", result.detail)

    def test_a_blocked_step_never_spawns_anything(self) -> None:
        self.assertIsNone(self.blocked_step(False).log)

    def test_both_modes_block_the_step_identically(self) -> None:
        summary, verbose = self.blocked_step(False), self.blocked_step(True)
        self.assertEqual(
            (summary.status, summary.detail, summary.code),
            (verbose.status, verbose.detail, verbose.code),
        )

    def suite_without_token(self, verbose: bool) -> tuple[list[gate.Result], int]:
        steps = (gate.BY_ID["fmt"], gate.BY_ID["codescene"], gate.BY_ID["rulesets"])
        with self.without_token(), mock.patch("sys.stdout"), mock.patch("sys.stderr"):
            results = gate.run_steps(steps, verbose, scratch(self))
            return results, gate.finish("gate", results, 0.0, verbose)

    def test_unaffected_steps_still_run_and_the_suite_still_fails(self) -> None:
        results, code = self.suite_without_token(False)
        self.assertEqual([r.status for r in results], ["pass", "fail", "pass"])
        self.assertNotEqual(code, 0, "a blocked step must fail the suite")

    def test_the_suite_result_is_the_same_in_verbose_mode(self) -> None:
        summary, summary_code = self.suite_without_token(False)
        verbose, verbose_code = self.suite_without_token(True)
        self.assertEqual([r.status for r in summary], [r.status for r in verbose])
        self.assertEqual(summary_code, verbose_code)

    def test_the_actionable_message_reaches_the_summary_and_the_dump(self) -> None:
        results, _ = self.suite_without_token(False)
        blocked = results[1]
        self.assertIn("export a PAT", blocked.detail)
        dump = capture_stderr(lambda: gate.report_failures(results, False))
        self.assertIn("export a PAT", dump)
        self.assertIn("codescene", dump)

    def test_verbose_still_prints_the_reason_it_could_not_run(self) -> None:
        """There is no streamed body to fall back on, so the dump must say it."""
        results, _ = self.suite_without_token(True)
        dump = capture_stderr(lambda: gate.report_failures(results, True))
        self.assertIn("export a PAT", dump)


class TemporaryFileTest(unittest.TestCase):
    """Captured output is temporary, and leaves nothing behind."""

    def capture_dirs(self) -> set[Path]:
        return set(Path(tempfile.gettempdir()).glob("dogtag-gate-*"))

    def run_main(self, *argv: str) -> int:
        before = self.capture_dirs()
        with mock.patch.object(sys, "argv", ["gate.py", *argv]):
            with mock.patch("sys.stdout"), mock.patch("sys.stderr"):
                code = gate.main()
        self.assertEqual(self.capture_dirs(), before, "a capture directory survived")
        return code

    def test_a_passing_run_removes_its_capture_directory(self) -> None:
        self.assertEqual(self.run_main("fmt"), 0)

    def test_a_failing_run_still_removes_its_capture_directory(self) -> None:
        with mock.patch.dict(gate.BY_ID, {"fmt": gate.Step("fmt", "sh -c 'exit 5'")}):
            self.assertEqual(self.run_main("fmt"), 5)


class ParityTest(unittest.TestCase):
    """The parity checker must reject what a substring search would accept.

    Each case mutates a copy of the real workflows: a checker that passed
    against a synthetic fixture but not the repository would prove nothing.
    """

    def setUp(self) -> None:
        self.root = scratch(self)
        for relative in (CI, ".github/workflows/security.yml", ".github/rulesets/main-branch.json"):
            target = self.root / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy(REPO_ROOT / relative, target)

    def edit(self, relative: str, old: str, new: str) -> None:
        path = self.root / relative
        text = path.read_text()
        self.assertIn(old, text, f"fixture text missing from {relative}")
        path.write_text(text.replace(old, new, 1))

    def assert_rejected(self, needle: str) -> None:
        found = check_gate_parity.check(self.root)
        self.assertTrue(found, "expected a violation, got none")
        self.assertTrue(any(needle in v for v in found), f"no violation mentioned {needle!r}: {found}")

    def test_the_unmodified_tree_passes(self) -> None:
        self.assertEqual(check_gate_parity.check(self.root), [])

    def test_a_ci_command_that_is_a_superset_of_the_local_one_is_caught(self) -> None:
        """The exact drift a substring match would have accepted."""
        self.edit(
            CI,
            "run: cargo clippy --all-targets --workspace --locked -- -D warnings",
            "run: cargo clippy --all-targets --workspace --locked --offline -- -D warnings",
        )
        self.assert_rejected("clippy")

    def test_a_command_demoted_to_a_comment_no_longer_vouches_for_itself(self) -> None:
        self.edit(CI, "run: cargo fmt --all --check", "run: true # cargo fmt --all --check")
        self.assert_rejected("fmt")

    def test_dropping_a_step_environment_variable_from_ci_is_caught(self) -> None:
        self.edit(CI, "RUSTDOCFLAGS: -D warnings", "RUSTDOCFLAGS: ")
        self.assert_rejected("RUSTDOCFLAGS")

    def test_a_repository_check_added_to_ci_alone_is_caught(self) -> None:
        self.edit(
            CI,
            "run: python3 scripts/check-ruleset-payloads.py",
            "run: python3 scripts/check-ruleset-payloads.py\n"
            "      - name: New\n        run: python3 scripts/check-new.py",
        )
        self.assert_rejected("scripts/check-new.py")

    def test_a_required_context_losing_its_job_is_caught(self) -> None:
        self.edit(CI, "name: Markdown link integrity", "name: Link check")
        self.assert_rejected("Markdown link integrity")

    def test_a_required_context_with_no_mapping_is_caught(self) -> None:
        self.edit(".github/rulesets/main-branch.json", '"context": "cargo-deny"', '"context": "brand-new"')
        self.assert_rejected("brand-new")


def capture_stderr(action) -> str:
    """Whatever `action` writes to stderr, as text."""
    with tempfile.TemporaryFile("w+") as sink:
        with mock.patch.object(sys, "stderr", sink):
            action()
        sink.seek(0)
        return sink.read()


if __name__ == "__main__":
    unittest.main(verbosity=2)
