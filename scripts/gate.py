#!/usr/bin/env python3
"""Run the repository's gates, one summary line each.

Three suites nest strictly — `fast` inside `check` inside `gate` — and every
step is declared once, as the exact command string CI runs. Declaring the
command rather than a recipe name is what lets
`scripts/check-gate-parity.py` hold these strings to the workflows: a local
gate cannot quietly measure something different from the required check it
mirrors, and a required check with no local equivalent has to say so.

Summary and verbose differ in **rendering only**. Both resolve the same
steps, run the same argv with the same environment, the same working
directory and the same closed stdin, and capture the same merged output;
`--verbose` additionally echoes that output as it arrives. No threshold,
scanner flag, file scope or exit code depends on the mode — a property
scripts/test_gate.py asserts rather than assumes.

Pass and fail come from the child's exit status alone. The metric column is
rendering: an extractor that finds nothing prints no number and says so on
stderr, so a reworded tool summary degrades visibly instead of inventing a
figure. Failing steps print their captured output to stderr, in summary mode
too, so one run diagnoses the failure.

Stdlib only (re, shlex, subprocess, tempfile). Usage:
    gate.py <suite|step>... [--verbose]
    gate.py --list
"""

from __future__ import annotations

import os
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable
from functools import partial
from pathlib import Path
from typing import NamedTuple

# The floor the repository's other checkers already assume (tomllib). Stated
# here because this module is on the `git commit` path via the pre-commit
# hook, where an unhandled TypeError from a `str | None` annotation would be
# a traceback rather than a sentence. The `annotations` import above keeps
# this module importable far enough down to print it.
MINIMUM_PYTHON = (3, 11)
if sys.version_info < MINIMUM_PYTHON:
    found = ".".join(str(part) for part in sys.version_info[:3])
    sys.exit(f"gate.py needs Python 3.11 or newer (found {found}).")

ROOT = Path(__file__).resolve().parent.parent

# Colour survives a pipe whenever CARGO_TERM_COLOR is set (CI sets it), so
# every extractor sees the text with the escapes removed rather than working
# on the developer's machine and silently finding nothing in CI.
ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


class Step(NamedTuple):
    """One gate: the command as written, and how to read its evidence."""

    id: str
    command: str
    metric: str | None = None
    env: tuple[tuple[str, str], ...] = ()

    @property
    def argv(self) -> list[str]:
        return shlex.split(self.command)


RUSTDOC_STRICT = (("RUSTDOCFLAGS", "-D warnings"),)

# The command strings are the declaration. `just` recipes and lefthook call
# into this table; the workflows spell their own copies out literally and
# check-gate-parity.py proves the two agree.
STEPS = (
    Step("fmt", "cargo fmt --all --check"),
    Step("clippy", "cargo clippy --all-targets --workspace --locked -- -D warnings"),
    Step("tests", "cargo test --workspace --locked", "cargo-test"),
    Step("tool-pins", "python3 scripts/check-tool-pins.py", "check-tool-pins"),
    Step("tool-pins-test", "python3 scripts/test_check_tool_pins.py"),
    Step(
        "exceptions",
        "python3 scripts/check_security_exceptions.py",
        "check_security_exceptions",
    ),
    Step("commits", "cargo run --quiet --locked -p commit-lint --", "commit-lint"),
    Step("docs", "cargo doc --workspace --no-deps --locked", None, RUSTDOC_STRICT),
    Step("links", "scripts/check-links.sh", "check-links"),
    Step("rulesets", "python3 scripts/check-ruleset-payloads.py", "check-ruleset-payloads"),
    Step("gate-parity", "python3 scripts/check-gate-parity.py", "check-gate-parity"),
    Step("gate-test", "python3 scripts/test_gate.py"),
    Step("hooks-test", "python3 scripts/test_hooks.py"),
    Step("coverage", "scripts/coverage-gate.sh", "coverage-check"),
    Step("msrv-build", "cargo +1.85.0 build --workspace --locked"),
    Step("msrv-test", "cargo +1.85.0 test --workspace --locked", "cargo-test"),
    Step("deny", "cargo deny --workspace --locked check", "cargo-deny"),
    Step("osv", "osv-scanner scan source -r .", "osv-scanner"),
    Step(
        "zizmor",
        "zizmor --persona regular --min-severity low --no-online-audits .github/workflows",
        "zizmor",
    ),
    Step("codescene", "scripts/codescene-gate.sh", "codescene-gate"),
)

BY_ID = {step.id: step for step in STEPS}

# Suites are literal supersets, so `fast` inside `check` inside `gate` is a
# fact about this data rather than a convention someone maintains. Ordered
# cheapest first, which is also roughly the order a failure is most likely.
FAST = ("fmt", "clippy", "tests", "tool-pins", "tool-pins-test", "exceptions", "commits")
CHECK = FAST + ("docs", "links", "rulesets", "gate-parity", "gate-test", "hooks-test")
GATE = CHECK + ("coverage", "msrv-build", "msrv-test", "deny", "osv", "zizmor", "codescene")
SUITES = {"fast": FAST, "check": CHECK, "gate": GATE}

# Printed with the final line so a green run never reads as more authority
# than it has. `fast` in particular contains the work behind six required
# checks and is still not a merge signal.
# The label a run of individually named steps carries. It is deliberately not
# a suite name: `just coverage` must not sign off as though it were `gate`.
STEPS_LABEL = "steps"

AUTHORITY = {
    "fast": "feedback only; run `just check` before handing work off",
    "check": "offline subset; run `just gate` before opening a pull request",
    "gate": "CI and the repository rulesets remain authoritative",
    STEPS_LABEL: "named gates only, not a suite; run `just gate` before a pull request",
}


# How each kind of prerequisite reads when it is absent, so the message a
# step fails with is a sentence rather than a label and a hint.
ABSENT = {
    "command": "is not on PATH",
    "toolchain": "is not installed",
    "env": "is not set",
}


class Prereq(NamedTuple):
    """Something a step cannot run without, and the one command that fixes it."""

    label: str
    fix: str
    kind: str
    value: str
    steps: tuple[str, ...]

    def message(self) -> str:
        return f"{self.label} {ABSENT[self.kind]}; {self.fix}"


INSTALL = "run `just install-dev-tools`"
PREREQS = (
    Prereq(
        "the `cs` command",
        "run `just install-dev-tools` (Linux x86_64 only; other architectures "
        "need a sha256 recorded in tools.toml first)",
        "command",
        "cs",
        ("codescene",),
    ),
    Prereq("the `jq` command", "install jq", "command", "jq", ("codescene",)),
    Prereq(
        "CS_ACCESS_TOKEN",
        "export a PAT from https://codescene.io/users/me/pat",
        "env",
        "CS_ACCESS_TOKEN",
        ("codescene",),
    ),
    Prereq("cargo-llvm-cov", INSTALL, "command", "cargo-llvm-cov", ("coverage",)),
    Prereq("cargo-deny", INSTALL, "command", "cargo-deny", ("deny",)),
    # The two `just install-dev-tools` cannot install for you: osv-scanner's
    # packaging is platform-specific, and the CodeScene CLI has a recorded
    # sha256 only for linux-amd64. Naming the recipe would send a macOS
    # developer to a script that prints instructions and moves on.
    Prereq(
        "osv-scanner",
        "install it from https://github.com/google/osv-scanner/releases "
        "(packaging varies by platform; `just install-dev-tools` prints the "
        "pinned version)",
        "command",
        "osv-scanner",
        ("osv",),
    ),
    Prereq("zizmor", INSTALL, "command", "zizmor", ("zizmor",)),
    Prereq(
        "the Rust 1.85.0 toolchain",
        INSTALL,
        "toolchain",
        "1.85.0",
        ("msrv-build", "msrv-test"),
    ),
)


def have_command(name: str) -> bool:
    return shutil.which(name) is not None


def have_toolchain(name: str) -> bool:
    """Whether rustup has the toolchain installed, not merely pinned."""
    listed = capture(["rustup", "toolchain", "list"])
    return any(line.startswith(name) for line in listed.splitlines())


def have_env(name: str) -> bool:
    return bool(os.environ.get(name))


PROBES: dict[str, Callable[[str], bool]] = {
    "command": have_command,
    "toolchain": have_toolchain,
    "env": have_env,
}


def missing_prereqs(step_ids: set[str]) -> list[str]:
    """Prerequisites these steps need and this machine lacks.

    Resolved per step, and never suite-wide. A missing CodeScene token is a
    reason for the Code Health gate to **fail** — not a reason to skip it,
    which would report success without measuring, and not a reason to abort
    the suite, which would throw away eighteen gates that had nothing to do
    with the token.
    """
    return [
        prereq.message()
        for prereq in PREREQS
        if step_ids & set(prereq.steps) and not PROBES[prereq.kind](prereq.value)
    ]


def capture(argv: list[str]) -> str:
    """Stdout of a short read-only helper; empty when it cannot run."""
    try:
        done = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True, check=False)
    except OSError:
        return ""
    return done.stdout


def strip_ansi(text: str) -> str:
    return ANSI.sub("", text)


def trim(summary: str) -> str:
    """Drop the `OK —` lead-in and trailing period the in-repo checkers write."""
    return summary.removeprefix("OK — ").removeprefix("OK - ").rstrip(". ")


def repo_summary(text: str, prefix: str) -> str | None:
    """The last `<prefix>: <summary>` line every in-repo checker prints.

    That line is the checkers' shared output contract — one stdout line
    naming the tool and what it verified — and test_gate.py runs each of
    them to prove the contract still holds, so this is a checked interface
    rather than a guess about human text.
    """
    found = re.findall(rf"^{re.escape(prefix)}: (.+)$", text, re.MULTILINE)
    return trim(found[-1]) if found else None


TEST_RESULT = re.compile(r"^test result: ok\. (\d+) passed", re.MULTILINE)
DENY_SUMMARY = re.compile(r"^(\w+ (?:ok|FAILED)(?:, \w+ (?:ok|FAILED))*)$", re.MULTILINE)
OSV_SCANNED = re.compile(r"^Scanned .* and found (\d+) packages$", re.MULTILINE)
ZIZMOR_CLEAN = re.compile(r"^No findings to report\. Good job!(?: \((\d+) suppressed\))?$", re.M)
ZIZMOR_FOUND = re.compile(r"^(\d+) findings?(?: \([^)]*\))?: ", re.MULTILINE)
DENY_WARNING = re.compile(r"^warning\[", re.MULTILINE)


def test_count(text: str) -> str | None:
    """Tests that ran, summed over every libtest harness in the run.

    libtest offers a machine-readable format only on nightly, so this reads
    the `test result:` line — stable for years, and anchored so a reworded
    line yields nothing rather than a wrong number.
    """
    counts = [int(n) for n in TEST_RESULT.findall(text)]
    return f"{sum(counts)} tests" if counts else None


def deny_summary(text: str) -> str | None:
    """cargo-deny's own one-line verdict; `--format json` would discard it.

    The warning count is appended because cargo-deny reports `ok` for a check
    that emitted warnings, so the verdict line alone cannot distinguish a
    clean run from one accumulating advisories that have not yet become
    errors.
    """
    found = DENY_SUMMARY.findall(text)
    if not found:
        return None
    warnings = len(DENY_WARNING.findall(text))
    return found[-1] + (f" ({warnings} warnings)" if warnings else "")


def osv_summary(text: str) -> str | None:
    """Package entries scanned, per source. The verdict is the exit status.

    "Entries across sources", not "packages": the counts are summed per
    scanned manifest and a package present in two of them is counted twice,
    so the total is a measure of work done rather than of distinct packages.
    """
    counts = [int(n) for n in OSV_SCANNED.findall(text)]
    if not counts:
        return None
    return f"{sum(counts)} package entries across {len(counts)} source(s)"


def zizmor_summary(text: str) -> str | None:
    """Findings, and how many the persona and severity filters removed.

    "Filtered", not "suppressed": zizmor's parenthetical counts what the
    `--persona regular --min-severity low` flags excluded, which is a
    reporting choice. It is not a suppression in this repository's sense —
    those are registered in docs/security/exceptions.toml — and calling it
    one would imply a registry entry that does not and should not exist.
    """
    clean = ZIZMOR_CLEAN.search(text)
    if clean:
        return f"no findings ({clean.group(1) or '0'} filtered out)"
    found = ZIZMOR_FOUND.search(text)
    return f"{found.group(1)} findings" if found else None


EXTRACTORS: dict[str, Callable[[str], str | None]] = {
    "cargo-test": test_count,
    "cargo-deny": deny_summary,
    "osv-scanner": osv_summary,
    "zizmor": zizmor_summary,
    "check-tool-pins": partial(repo_summary, prefix="check-tool-pins"),
    "check_security_exceptions": partial(repo_summary, prefix="check_security_exceptions"),
    "check-ruleset-payloads": partial(repo_summary, prefix="check-ruleset-payloads"),
    "check-gate-parity": partial(repo_summary, prefix="check-gate-parity"),
    "check-links": partial(repo_summary, prefix="check-links"),
    "coverage-check": partial(repo_summary, prefix="coverage-check"),
    "codescene-gate": partial(repo_summary, prefix="codescene-gate"),
    "commit-lint": partial(repo_summary, prefix="commit-lint"),
}


def git_output(*args: str) -> str | None:
    """Stdout of a git command, or None when git rejected it."""
    done = subprocess.run(
        ["git", *args], cwd=ROOT, capture_output=True, text=True, check=False
    )
    return done.stdout.strip() if done.returncode == 0 else None


def commit_base() -> str:
    """The ref this branch is measured against, closest to CI's first.

    CI validates the commits a pull request introduces; `origin/main` is the
    nearest local stand-in. `main` covers a clone with no fetched remote,
    and `HEAD~1` mirrors the fallback ci.yml uses when nothing defines an
    incoming range.
    """
    for ref in ("origin/main", "main"):
        if git_output("rev-parse", "--verify", "--quiet", f"{ref}^{{commit}}"):
            return ref
    return "HEAD~1"


def prepare_commits(step: Step) -> tuple[list[str], str]:
    """Resolve the range, and refuse to call an empty one a pass.

    `main..HEAD` is empty on `main` itself, and commit-lint reports "0
    commit(s) are Conventional Commits" and exits 0 — a green line for a
    check that validated nothing. An empty range is a skip, and it says so.

    The range is also the one part of this step CI computes differently: the
    `Commit messages` job validates exactly the commits a pull request
    introduces. A green local `commits` is evidence, not the required
    check — a stale `origin/main` moves the range under it.
    """
    span = f"{commit_base()}..HEAD"
    argv = step.argv + ["--range", span]
    count = git_output("rev-list", "--count", span)
    if count is None:
        return argv, ""
    return argv, f"no commits in {span}" if count == "0" else ""


def prepare_default(step: Step) -> tuple[list[str], str]:
    return step.argv, ""


PREPARE: dict[str, Callable[[Step], tuple[list[str], str]]] = {
    "commits": prepare_commits,
}


class Invocation(NamedTuple):
    """A step with its runtime arguments resolved, or a reason not to run."""

    step: Step
    argv: list[str]
    skip: str

    def rendered(self) -> str:
        """The command as a shell would have to be given it, env included.

        Rendered from the resolved argv rather than from `step.command`, so
        what is printed is what ran — `commits` gains a `--range`, and `docs`
        carries the flag that makes its warnings errors.
        """
        prefix = "".join(f"{name}={shlex.quote(value)} " for name, value in self.step.env)
        return prefix + shlex.join(self.argv)


def prepare(step: Step) -> Invocation:
    argv, skip = PREPARE.get(step.id, prepare_default)(step)
    return Invocation(step, argv, skip)


class Result(NamedTuple):
    """One step's verdict, its rendered metric, and where its output went."""

    invocation: Invocation
    status: str
    detail: str
    code: int
    log: Path | None

    @property
    def step(self) -> Step:
        return self.invocation.step


def exit_code(returncode: int) -> int:
    """A child's status as a shell reports it: a signal death is 128+n.

    subprocess reports a signal as a negative returncode, and sys.exit of a
    negative number is truncated to eight bits — SIGTERM would surface as
    241 rather than 143. Clamped so a failed step can never yield 0.
    """
    code = 128 - returncode if returncode < 0 else returncode
    return min(max(code, 1), 255)


def absolute(argv: list[str]) -> list[str]:
    """Resolve a repository-relative program against the root, not the cwd."""
    if "/" in argv[0]:
        return [str(ROOT / argv[0]), *argv[1:]]
    return argv


def child_env(overrides: tuple[tuple[str, str], ...]) -> dict[str, str]:
    return {**os.environ, **dict(overrides)}


def spawn(argv: list[str], step: Step, log: Path, verbose: bool) -> int:
    """Run one gate, capturing merged output; echo it live when verbose.

    stdin is closed in both modes: a tool that decides to prompt then fails
    fast instead of hanging behind a summary line with nothing to show.
    """
    with log.open("wb") as sink, subprocess.Popen(
        absolute(argv),
        cwd=ROOT,
        env=child_env(step.env),
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    ) as child:
        for chunk in child.stdout:
            sink.write(chunk)
            if verbose:
                sys.stdout.buffer.write(chunk)
                sys.stdout.flush()
        return child.wait()


def measure(step: Step, log: Path) -> str:
    """The step's metric, or nothing at all — never a fabricated number."""
    if step.metric is None:
        return ""
    text = strip_ansi(log.read_text(errors="replace"))
    found = EXTRACTORS[step.metric](text)
    if found is None:
        print(f"gate: no metric for {step.id}", file=sys.stderr)
        return ""
    return found


PREREQ_EXIT = 1


def run_invocation(job: Invocation, tmp: Path, verbose: bool) -> Result:
    """Run one resolved invocation to completion and read its evidence.

    A step whose prerequisites are absent fails here, before anything is
    spawned, carrying the message that names the fix. Every other step in the
    run is unaffected and still runs.
    """
    missing = missing_prereqs({job.step.id})
    if missing:
        return Result(job, "fail", "; ".join(missing), PREREQ_EXIT, None)
    if job.skip:
        return Result(job, "skip", job.skip, 0, None)
    log = tmp / f"{job.step.id}.log"
    returncode = spawn(job.argv, job.step, log, verbose)
    if returncode != 0:
        return Result(job, "fail", "", exit_code(returncode), log)
    return Result(job, "pass", measure(job.step, log), 0, log)


def run_step(step: Step, tmp: Path, verbose: bool) -> Result:
    """Resolve a step's arguments and run it."""
    return run_invocation(prepare(step), tmp, verbose)


def line(name: str, status: str, detail: str, width: int) -> str:
    suffix = f" — {detail}" if detail else ""
    return f"{name:<{width}}  {status}{suffix}"


def elapsed(seconds: float) -> str:
    if seconds < 60:
        return f"{seconds:.1f}s"
    return f"{int(seconds) // 60}m{int(seconds) % 60:02d}s"


def tally(results: list[Result]) -> str:
    """`passed/total`, with skips named so they cannot hide in a green run."""
    passed = sum(1 for r in results if r.status == "pass")
    skipped = sum(1 for r in results if r.status == "skip")
    total = len(results)
    return f"{passed}/{total} steps" + (f", {skipped} skipped" if skipped else "")


def report_failures(results: list[Result], verbose: bool) -> None:
    """Every failing step's captured output, on stderr, in summary mode too.

    In verbose mode the body already streamed as it arrived, so only the
    header is repeated — printing it twice would bury the second copy under
    the first and make a long failure twice as expensive to read.
    """
    for result in (r for r in results if r.status == "fail"):
        print(f"\n── {result.step.id} (exit {result.code}) ──", file=sys.stderr)
        print(f"$ {result.invocation.rendered()}", file=sys.stderr)
        if result.log is None:
            # A prerequisite failure never ran anything, so the message it
            # carries is the whole diagnosis.
            print(result.detail, file=sys.stderr)
        elif not verbose:
            sys.stderr.write(result.log.read_text(errors="replace"))


def render_start(job: Invocation, width: int, verbose: bool) -> None:
    """Show the step before it runs, so a slow gate never looks hung."""
    if verbose:
        print(f"\n── {job.step.id} ── $ {job.rendered()}", flush=True)
    else:
        sys.stdout.write(f"{job.step.id:<{width}}  ")
        sys.stdout.flush()


def render_result(result: Result, width: int, verbose: bool) -> None:
    if verbose:
        print(line(result.step.id, result.status, result.detail, width), flush=True)
    else:
        suffix = f" — {result.detail}" if result.detail else ""
        print(f"{result.status}{suffix}", flush=True)


def run_steps(steps: tuple[Step, ...], verbose: bool, tmp: Path) -> list[Result]:
    """Run every step, in order, without stopping at the first failure.

    Stopping early would leave a summary that reads as complete while nine
    lines are simply missing. The suite is seconds of compute outside the
    CodeScene sweep, so one run reporting every failure is worth more than
    the seconds an early exit would save.
    """
    width = max(len(step.id) for step in steps)
    results = []
    for step in steps:
        job = prepare(step)
        render_start(job, width, verbose)
        result = run_invocation(job, tmp, verbose)
        render_result(result, width, verbose)
        results.append(result)
    return results


def resolve(names: list[str]) -> tuple[tuple[Step, ...], list[str]]:
    """Expand suite and step names into the ordered, de-duplicated step list."""
    ids: list[str] = []
    unknown = [n for n in names if n not in SUITES and n not in BY_ID]
    for name in names:
        for step_id in SUITES.get(name, (name,)):
            if step_id in BY_ID and step_id not in ids:
                ids.append(step_id)
    return tuple(BY_ID[i] for i in ids), unknown


def print_list() -> None:
    """The table itself, so what runs is readable without reading the code."""
    width = max(len(step.id) for step in STEPS)
    for step in STEPS:
        suites = " ".join(name for name, ids in SUITES.items() if step.id in ids)
        print(f"{step.id:<{width}}  {suites:<16}  {Invocation(step, step.argv, '').rendered()}")


def label_for(names: list[str]) -> str:
    """What the final line calls this run.

    A run of individually named steps is labelled `steps`, never `gate`:
    `just coverage` must not sign off with the full suite's name and the full
    suite's authority behind it.
    """
    return names[0] if names and names[0] in SUITES else STEPS_LABEL


def finish(name: str, results: list[Result], started: float, verbose: bool) -> int:
    """The final line, then every failure's detail, then the exit status."""
    failed = [r for r in results if r.status == "fail"]
    status = "fail" if failed else "pass"
    note = f"{tally(results)}, {elapsed(time.monotonic() - started)}"
    width = max([len(r.step.id) for r in results] + [len(name)])
    print(line(name, status, f"{note} · {AUTHORITY[name]}", width), flush=True)
    # Flushed first: stdout is a pipe in every hook and CI run, so without
    # this the buffered verdict lands after the unbuffered stderr dump and
    # the log reads as though the run ended before it was judged.
    report_failures(results, verbose)
    return failed[0].code if failed else 0


FLAGS = ("--verbose", "--list")


def usage(problem: str) -> int:
    print(f"usage: gate.py <{'|'.join(SUITES)}|step>... [--verbose]", file=sys.stderr)
    print("       gate.py --list", file=sys.stderr)
    print(f"gate: {problem}", file=sys.stderr)
    return 2


def argument_problem(arguments: list[str], names: list[str]) -> str | None:
    """The first thing wrong with the command line, or None.

    A mistyped flag is a usage error rather than a word quietly dropped:
    `--verbos` must not run the whole suite in summary mode and exit 0.
    """
    unknown_flags = [a for a in arguments if a.startswith("-") and a not in FLAGS]
    if unknown_flags:
        return f"unknown option(s) {unknown_flags}"
    if "--list" in arguments and names:
        return "--list takes no other arguments"
    return None


def step_problem(steps: tuple[Step, ...], unknown: list[str]) -> str | None:
    """Why the requested names did not resolve to anything runnable."""
    if unknown:
        return f"unknown suite or step {unknown}"
    return None if steps else "no steps requested"


def run_suite(steps: tuple[Step, ...], names: list[str], verbose: bool) -> int:
    """Run the steps under a capture directory removed on the way out."""
    started = time.monotonic()
    tmp = Path(tempfile.mkdtemp(prefix="dogtag-gate-"))
    try:
        results = run_steps(steps, verbose, tmp)
        return finish(label_for(names), results, started, verbose)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main() -> int:
    arguments = sys.argv[1:]
    names = [a for a in arguments if not a.startswith("-")]
    problem = argument_problem(arguments, names)
    if problem:
        return usage(problem)
    if "--list" in arguments:
        print_list()
        return 0
    steps, unknown = resolve(names)
    problem = step_problem(steps, unknown)
    if problem:
        return usage(problem)
    return run_suite(steps, names, "--verbose" in arguments)


if __name__ == "__main__":
    sys.exit(main())
