#!/usr/bin/env python3
"""Adversarial tests for the Code Health git hooks.

These hooks carry more weight than hooks usually do. Every other rule they
check is re-checked by a required status check, so a bypassed hook costs
nothing; Code Health has no remote counterpart at all, so these deltas are
the only automatic signal before `just gate`. That makes exactly one failure
mode unacceptable: reporting success without measuring.

The wrapper is allowed to *decline* — an external contributor has no
CodeScene account, and a hook that demanded one would make an account a
condition of contributing. What it must never do is look like a pass. So the
tests below check the words as well as the exit status, and check that
`scripts/codescene-gate.sh` itself keeps refusing to run token-less, since
that is what `just gate` depends on.

Stdlib only (unittest). Run directly, or via `just check`.
"""

import os
import re
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
HOOK = "scripts/codescene-hook.sh"
GATE = "scripts/codescene-gate.sh"

# Claims that would make an unmeasured run read as a measured one. Matched on
# word boundaries: the notice says "this is not a pass", and forbidding the
# bare substring would forbid the very sentence that makes it honest.
PASS_CLAIMS = (
    r"\bpassed\b",
    r"\bok\b",
    r"\bclean\b",
    r"\bsucce",
    r"\bno findings\b",
    r"\b10\.0\b",
)


def run(argv: list[str], token: str | None) -> subprocess.CompletedProcess:
    """Run a script with CS_ACCESS_TOKEN present or definitively absent."""
    env = dict(os.environ)
    env.pop("CS_ACCESS_TOKEN", None)
    if token is not None:
        env["CS_ACCESS_TOKEN"] = token
    return subprocess.run(
        argv, cwd=REPO_ROOT, env=env, capture_output=True, text=True, check=False
    )


class WithoutTokenTest(unittest.TestCase):
    """No token: decline loudly, and never resemble a pass."""

    def declined(self, mode: str) -> subprocess.CompletedProcess:
        done = run([HOOK, mode], token=None)
        self.assertEqual(done.returncode, 0, done.stderr)
        return done

    def test_the_staged_hook_declines_rather_than_blocking_a_commit(self) -> None:
        self.assertIn("NOT MEASURED", self.declined("staged").stderr)

    def test_the_branch_hook_declines_rather_than_blocking_a_push(self) -> None:
        self.assertIn("NOT MEASURED", self.declined("branch").stderr)

    def test_the_notice_says_plainly_that_nothing_was_checked(self) -> None:
        self.assertIn("This is not a pass", self.declined("staged").stderr)

    def test_the_notice_never_reads_as_a_pass(self) -> None:
        """The load-bearing case: an unmeasured run must not look measured."""
        for mode in ("staged", "branch"):
            done = self.declined(mode)
            output = (done.stdout + done.stderr).lower()
            for claim in PASS_CLAIMS:
                self.assertIsNone(
                    re.search(claim, output), f"{mode!r} output matched {claim!r}"
                )

    def test_the_notice_goes_to_stderr_where_a_hook_is_read(self) -> None:
        self.assertEqual(self.declined("staged").stdout, "")

    def test_the_notice_tells_a_contributor_they_need_no_account(self) -> None:
        self.assertIn("no account needed", self.declined("staged").stderr)

    def test_the_notice_tells_a_maintainer_where_to_get_a_token(self) -> None:
        self.assertIn("codescene.io/users/me/pat", self.declined("staged").stderr)


class FailClosedTest(unittest.TestCase):
    """The gate itself must keep refusing, or `just gate` stops being a gate."""

    def test_the_underlying_gate_still_refuses_to_run_token_less(self) -> None:
        """It must refuse; *which* prerequisite it names depends on the host.

        On a CI runner the `cs` CLI is not installed either, so the gate
        stops at that check first. Either way it declines and says why —
        asserting the specific message would be asserting the machine.
        """
        done = run([GATE], token=None)
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("codescene-gate:", done.stderr)

    @unittest.skipUnless(shutil.which("cs"), "the `cs` CLI is not installed")
    def test_with_the_cli_present_it_names_the_missing_token(self) -> None:
        self.assertIn("CS_ACCESS_TOKEN", run([GATE], token=None).stderr)

    def test_only_the_wrapper_may_decline(self) -> None:
        """The wrapper's leniency must not have leaked into the gate script."""
        self.assertNotIn("NOT MEASURED", (REPO_ROOT / GATE).read_text())


class UsageTest(unittest.TestCase):
    """An unrecognised mode is an error, not a quiet success."""

    def test_an_unknown_mode_is_rejected(self) -> None:
        done = run([HOOK, "bogus"], token=None)
        self.assertEqual(done.returncode, 2)

    def test_no_mode_at_all_is_rejected(self) -> None:
        self.assertEqual(run([HOOK], token=None).returncode, 2)


class BaseResolutionTest(unittest.TestCase):
    """The branch delta must measure against the ref the merge will use.

    Built as throwaway repositories with exactly the refs under test rather
    than asked of whatever repository happens to be checked out: a CI
    runner's checkout of a pull-request merge ref has no `origin/main`
    remote-tracking branch at all, so asserting against the ambient tree
    would be asserting the machine.
    """

    def repo(self, *refs: str) -> Path:
        """A one-commit repository carrying exactly the named refs."""
        root = Path(tempfile.mkdtemp(prefix="test-hooks-"))
        self.addCleanup(shutil.rmtree, root, ignore_errors=True)
        for argv in (
            ["git", "init", "-q", "--initial-branch=work"],
            ["git", "config", "user.email", "probe@example.invalid"],
            ["git", "config", "user.name", "Probe"],
            ["git", "commit", "-q", "--allow-empty", "-m", "chore: base"],
        ):
            subprocess.run(argv, cwd=root, check=True)
        head = subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=root, capture_output=True, text=True, check=True
        ).stdout.strip()
        for ref in refs:
            subprocess.run(["git", "update-ref", ref, head], cwd=root, check=True)
        return root

    def resolved(self, cwd: Path) -> str:
        """What resolve_base() picks, asked of the script's own logic."""
        script = (REPO_ROOT / HOOK).read_text()
        body = script[script.index("resolve_base()") : script.index('case "$mode"')]
        done = subprocess.run(
            ["bash", "-c", f"{body}\nresolve_base"],
            cwd=cwd,
            capture_output=True,
            text=True,
            check=False,
        )
        return done.stdout.strip()

    def test_origin_main_is_preferred_because_it_is_the_merge_target(self) -> None:
        both = self.repo("refs/remotes/origin/main", "refs/heads/main")
        self.assertEqual(self.resolved(both), "origin/main")

    def test_a_local_main_covers_a_clone_with_no_fetched_remote(self) -> None:
        self.assertEqual(self.resolved(self.repo("refs/heads/main")), "main")

    def test_neither_ref_resolves_to_nothing_rather_than_to_a_guess(self) -> None:
        self.assertEqual(self.resolved(self.repo()), "")

    def test_a_base_that_resolves_to_nothing_is_an_error_not_a_silent_pass(self) -> None:
        """A delta against the wrong base is indistinguishable from a clean one.

        The hook `cd`s to its own repository root on entry, so it has to be
        copied into the throwaway repository to be exercised against it —
        passing a working directory would not reach it.
        """
        root = self.repo()
        (root / "scripts").mkdir()
        copied = root / HOOK
        shutil.copy(REPO_ROOT / HOOK, copied)
        done = subprocess.run(
            [str(copied), "branch"],
            env={**os.environ, "CS_ACCESS_TOKEN": "unused-here"},
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("cannot delta", done.stderr)


class WorkflowCleanlinessTest(unittest.TestCase):
    """No CodeScene credential or command may live in GitHub Actions."""

    def workflow_text(self) -> str:
        directory = REPO_ROOT / ".github" / "workflows"
        return "\n".join(p.read_text() for p in sorted(directory.glob("*.yml")))

    def test_no_workflow_mentions_a_codescene_credential_or_command(self) -> None:
        for marker in ("CS_ACCESS_TOKEN", "codescene", "CodeScene", "cs-linux-amd64"):
            self.assertNotIn(marker, self.workflow_text(), marker)

    def test_no_workflow_reads_a_repository_secret_at_all(self) -> None:
        """The inventory is empty, so provisioning installs no secrets."""
        self.assertNotIn("secrets.", self.workflow_text())


if __name__ == "__main__":
    unittest.main(verbosity=2)
