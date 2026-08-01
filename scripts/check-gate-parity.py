#!/usr/bin/env python3
"""Hold the local gate table to the workflows it claims to mirror.

`scripts/gate.py` declares each gate as the command string CI runs. Two
copies of anything drift, and the failure mode is quiet: a flag added to a
workflow and not to the table leaves `just gate` green while measuring
something narrower than the required check it stands in for. So this asserts
the agreement structurally, the way scripts/check-tool-pins.py asserts
versions — against the command that actually runs, never against a comment
naming it.

Matching is **whole-line against a parsed `run:` step**, not a substring
search of the file. A substring search would accept three different lies: a
command sitting in a YAML comment, a CI line that is a strict superset of the
local one (`… --locked --offline` satisfying `… --locked`), and a trailing
flag added in CI only. Each of those is precisely the drift this file exists
to catch, so the parse is worth its lines. A step that legitimately takes a
runtime argument declares the suffix, so the tolerance is per-step and
checked rather than global.

Six rules:

1. Every step in the `gate` suite is a whole `run:` line in a workflow, or is
   listed in DIVERGENCES with the reason it cannot be.
2. Every environment variable a step declares appears in the workflows too, so
   the docs gate cannot lose `-D warnings` on one side only.
3. Every required status check in .github/rulesets/main-branch.json maps to
   gate steps, or is listed in NOT_LOCAL with the reason; each mapped step
   really does run in the job that reports that context; and every required
   context is reported by a job that still exists under that name. That last
   part catches the fragility the workflow-security ADR names — contexts are
   display names, so renaming a job silently un-requires it and blocks every
   merge until someone edits the ruleset.
4. Every repository script a workflow runs is claimed by a gate step, or is
   listed in CI_ONLY with the reason. This is the direction that notices a new
   check added inside an already-required job.
5. No table carries a stale entry: a divergence whose command has since become
   verbatim-identical is dropped rather than kept, and neither context table
   names a check the ruleset stopped requiring.
6. A step recorded in LOCAL_ONLY is genuinely absent from CI — not merely its
   command, but every marker that would signal it creeping back in a
   different shape. A local-only gate weakens the merge rules by definition,
   so it has to stay a deliberate, recorded choice.

Stdlib only (itertools, json, re). Usage:
    check-gate-parity.py [repo-root]
"""

import itertools
import json
import re
import sys
from pathlib import Path
from typing import NamedTuple

sys.path.insert(0, str(Path(__file__).resolve().parent))
import gate  # noqa: E402  (path shim above; scripts/ is not a package)

# release.yml is scanned even though it reports no required status check: it
# is the only path that produces the bytes users install, so rule 4 watching
# it is the difference between "a script CI runs is accounted for" and "a
# script a *required* job runs is accounted for". The release path's scripts
# are rehearsed by `just dist` rather than by a gate step, which is what the
# CI_ONLY entries below record.
WORKFLOWS = (
    ".github/workflows/ci.yml",
    ".github/workflows/security.yml",
    ".github/workflows/release.yml",
)
RULESET = ".github/rulesets/main-branch.json"

# Steps whose local command cannot be byte-identical to CI's, and why. Each
# reason names what CI does differently, so the divergence is a decision on
# the record rather than a difference nobody noticed.
#
# `commits` is deliberately absent: its command *is* verbatim, and what
# differs is the range appended at runtime, declared in ARGUMENT_SUFFIX below.
DIVERGENCES = {
    "msrv-build": (
        "CI sets RUSTUP_TOOLCHAIN because rust-toolchain.toml outranks "
        "`rustup default`; `cargo +1.85.0` selects the same toolchain "
        "locally without needing the env var"
    ),
    "msrv-test": (
        "same RUSTUP_TOOLCHAIN mechanism as msrv-build; CI additionally "
        "asserts `cargo --version` really is 1.85.0, which has no local "
        "analogue because the +toolchain form cannot silently fall back"
    ),
    "osv": (
        "CI runs Google's reusable osv-scanner workflow, pinned at v2.3.8 "
        "and carrying its own scanner image; the local run uses the "
        "tools.toml-pinned binary. The two versions drift by design, so a "
        "local scan is a pre-flight and CI is the verdict"
    ),
    "zizmor": (
        "CI runs the pinned zizmor action, which takes its settings as "
        "inputs rather than flags; the local command mirrors those inputs "
        "as flags and audits the same scope — the repository root, which is "
        "what the action defaults to. The mechanism may differ; the audited "
        "scope may not. It did once: the local run was scoped to "
        ".github/workflows, so it never read .github/dependabot.yml and a "
        "finding there passed every local gate before failing CI"
    ),
}

# Arguments CI appends to an otherwise identical command. Declared per step so
# the tolerance is a checked fact rather than a substring match's blind spot.
ARGUMENT_SUFFIX = {
    "commits": '--range "$range"',
}

# Repository commands a workflow runs that no gate step covers, and why.
CI_ONLY = {
    'scripts/package.sh "$TARGET"': (
        "a release-path script, not a gate step: it stages a built binary "
        "into the published archive, so it runs on a tag rather than on "
        "every commit. `just dist` rehearses it with the same arguments the "
        "release workflow passes, which is where a break surfaces locally"
    ),
    'scripts/sbom.sh "$TARGET"': (
        "same release path as package.sh: it generates the per-target "
        "CycloneDX SBOM the release publishes and attests, and `just dist` "
        "runs it immediately after the packaging step for the same reason"
    ),
}

# Gate steps CI does not run at all, and the strings that must therefore stay
# out of the workflows. A local-only gate is a real weakening of the merge
# rules, so it is spelled out here rather than left as an absence: if any of
# these markers reappears in a workflow, the ruleset payloads, the required
# contexts and this table all have to move together, and the check below is
# what forces that conversation.
LOCAL_ONLY = {
    "codescene": (
        "Code Health is a maintainer-local invariant, not a required check. "
        "Enforcing it in CI needs a CodeScene account per contributor or a "
        "repository secret that forked pull requests cannot reach, and the "
        "fork case has no honest resolution — the job either fails every "
        "external contribution or passes it unmeasured. The maintainer runs "
        "`just gate` with the token before merging, and that run is the gate",
        ("CS_ACCESS_TOKEN", "codescene", "CodeScene", "cs-linux-amd64"),
    ),
}

# Required status checks and the gate steps that cover them.
CONTEXT_STEPS = {
    "Format, lint, test (Linux)": (
        "fmt",
        "clippy",
        "tests",
        "docs",
        "tool-pins",
        "tool-pins-test",
        "rulesets",
        "exceptions",
        "gate-parity",
        "gate-test",
        "hooks-test",
    ),
    "Commit messages": ("commits",),
    "MSRV (Rust 1.85)": ("msrv-build", "msrv-test"),
    "Coverage thresholds": ("coverage",),
    "Workflow security (zizmor)": ("zizmor",),
    "Markdown link integrity": ("links",),
    "cargo-deny": ("deny",),
    "osv-pr / osv-scan": ("osv",),
}

# Required checks with no local step, and why not.
NOT_LOCAL = {
    "Test (macOS arm64)": (
        "runs the suite on another platform; a Linux developer cannot "
        "reproduce it, and `just gate` does not pretend to"
    ),
    "Release build check (x86_64 musl)": (
        "needs the x86_64-unknown-linux-musl cross target, which is not "
        "available on every supported development platform; `just dist` "
        "rehearses the same build where it is"
    ),
}

JOB_HEADER = re.compile(r"^  ([A-Za-z0-9_-]+):$")
JOB_NAME = re.compile(r'^    name: "?(.+?)"?$', re.MULTILINE)
RUN_SCALAR = re.compile(r"^\s*run: (?!\|)(.+)$")
RUN_BLOCK = re.compile(r"^(\s*)run: \|-?\s*$")


class CiJob(NamedTuple):
    """One workflow job: the context it reports, and the commands it runs."""

    label: str
    commands: tuple[str, ...]


def jobs_section(text: str) -> str:
    """Everything after the `jobs:` key, so `on:` cannot look like a job."""
    _, _, rest = text.partition("\njobs:\n")
    return rest


def job_blocks(section: str) -> list[tuple[str, str]]:
    """(job id, body) for each job, split on the two-space job headers."""
    lines = section.splitlines()
    starts = [i for i, line in enumerate(lines) if JOB_HEADER.match(line)]
    return [
        (JOB_HEADER.match(lines[start]).group(1), "\n".join(lines[start + 1 : end]))
        for start, end in zip(starts, starts[1:] + [len(lines)])
    ]


def meaningful(line: str) -> bool:
    """A line that runs something, rather than a blank or a comment."""
    stripped = line.strip()
    return bool(stripped) and not stripped.startswith("#")


def indented_past(line: str, indent: int) -> bool:
    return not line.strip() or len(line) - len(line.lstrip()) > indent


def block_commands(lines: list[str], index: int) -> list[str]:
    """The command lines of a `run: |` block opening at `index`."""
    opener = RUN_BLOCK.match(lines[index])
    if not opener:
        return []
    indent = len(opener.group(1))
    body = itertools.takewhile(
        lambda line: indented_past(line, indent), lines[index + 1 :]
    )
    return [line.strip() for line in body if meaningful(line)]


def scalar_command(line: str) -> list[str]:
    found = RUN_SCALAR.match(line)
    return [found.group(1).strip()] if found else []


def run_commands(body: str) -> list[str]:
    """Every command line this job's `run:` steps execute."""
    lines = body.splitlines()
    nested = (scalar_command(line) + block_commands(lines, i) for i, line in enumerate(lines))
    return [command for group in nested for command in group]


def ci_jobs(root: Path) -> list[CiJob]:
    """Every workflow job, labelled by the status-check context it reports.

    A job that declares a `name` reports under it; one that does not reports
    under its id — which is how `cargo-deny` becomes a required context.
    """
    jobs = []
    for path in WORKFLOWS:
        text = (root / path).read_text()
        for job_id, body in job_blocks(jobs_section(text)):
            found = JOB_NAME.search(body)
            label = found.group(1).strip() if found else job_id
            jobs.append(CiJob(label, tuple(run_commands(body))))
    return jobs


def expected_line(step_id: str) -> str:
    """The whole `run:` line CI must carry for this step."""
    command = gate.BY_ID[step_id].command
    suffix = ARGUMENT_SUFFIX.get(step_id)
    return f"{command} {suffix}" if suffix else command


def check_commands(commands: set[str]) -> list[str]:
    """Rule 1: every gate step is a whole CI command line, or is excused."""
    excused = set(DIVERGENCES) | set(LOCAL_ONLY)
    return [
        f"gate.py step {step_id!r} runs `{expected_line(step_id)}`, which is "
        f"not a whole `run:` line in any workflow, and neither DIVERGENCES "
        f"nor LOCAL_ONLY gives a reason"
        for step_id in gate.GATE
        if step_id not in excused and expected_line(step_id) not in commands
    ]


def check_local_only(root: Path, commands: set[str]) -> list[str]:
    """Rule 6: a local-only gate really is absent from CI, markers and all.

    Checking only that the *command* is missing would let Code Health creep
    back as a differently-shaped job — a token here, an install step there —
    while the ruleset still required nothing. The markers are what make the
    absence deliberate instead of incidental.
    """
    text = "\n".join((root / path).read_text() for path in WORKFLOWS)
    violations = [
        f"LOCAL_ONLY names {step_id!r}, which is not a gate step"
        for step_id in LOCAL_ONLY
        if step_id not in gate.BY_ID
    ]
    violations += [
        f"LOCAL_ONLY excuses {step_id!r}, but a workflow now runs "
        f"`{expected_line(step_id)}` — restore its required status check and "
        f"drop the entry"
        for step_id in LOCAL_ONLY
        if step_id in gate.BY_ID and expected_line(step_id) in commands
    ]
    return violations + [
        f"a workflow mentions {marker!r}, but {step_id!r} is recorded as "
        f"local-only — CI and the ruleset payloads must change together"
        for step_id, (_reason, markers) in LOCAL_ONLY.items()
        for marker in markers
        if marker in text
    ]


def check_env(root: Path) -> list[str]:
    """Rule 2: a variable a step needs is declared on the CI side too."""
    text = "\n".join((root / path).read_text() for path in WORKFLOWS)
    return [
        f"gate.py step {step.id!r} runs with {name}={value!r}, which no "
        f"workflow declares — the two would enforce different things"
        for step in gate.STEPS
        if step.id in gate.GATE
        for name, value in step.env
        if f"{name}: {value}" not in text
    ]


def required_contexts(root: Path) -> list[str]:
    """The contexts the branch ruleset requires before a merge."""
    payload = json.loads((root / RULESET).read_text())
    for rule in payload["rules"]:
        if rule["type"] == "required_status_checks":
            checks = rule["parameters"]["required_status_checks"]
            return [check["context"] for check in checks]
    return []


def unmapped_contexts(contexts: list[str]) -> list[str]:
    """Rule 3a: a required check is covered locally, or recorded as not."""
    return [
        f"required check {context!r} maps to no gate step and is not in "
        f"NOT_LOCAL — `just gate` is weaker than it claims"
        for context in contexts
        if context not in CONTEXT_STEPS and context not in NOT_LOCAL
    ]


def missing_mapped_steps() -> list[str]:
    """Rule 3b: every mapped step exists and is in the `gate` suite."""
    return [
        f"context {context!r} maps to {step_id!r}, which is not a step in "
        f"the gate suite"
        for context, step_ids in CONTEXT_STEPS.items()
        for step_id in step_ids
        if step_id not in gate.GATE
    ]


def context_job(context: str) -> str:
    """The job label that reports this context.

    A reusable workflow renders its context as `caller-job-id /
    callee-job-name`, so the job to look for is the caller named before the
    slash.
    """
    return context.split(" / ")[0]


def missing_context_jobs(jobs: list[CiJob], contexts: list[str]) -> list[str]:
    """Rule 3c: every required context is actually reported by some job.

    Contexts are display names, so **renaming a job silently un-requires
    it** — the rule keeps demanding a check that will never report again,
    and every merge blocks until someone edits the ruleset. This is the one
    fragility the workflow-security ADR calls out by name; here it fails a
    build instead of a merge.
    """
    labels = {job.label for job in jobs}
    return [
        f"required check {context!r} is reported by no job — a job rename "
        f"un-requires its context until {RULESET} is updated to match"
        for context in contexts
        if context_job(context) not in labels
    ]


def misplaced_mapped_steps(jobs: list[CiJob]) -> list[str]:
    """Rule 3d: a mapped step really runs in the job reporting that context.

    Asserting the mapping against the job that provides the context is what
    turns CONTEXT_STEPS from a comment into a check: a step could otherwise be
    verbatim in some *other* job and still leave its context uncovered.
    """
    by_label = {job.label: set(job.commands) for job in jobs}
    return [
        f"context {context!r} maps to {step_id!r}, but the job reporting "
        f"that context does not run `{expected_line(step_id)}`"
        for context, step_ids in CONTEXT_STEPS.items()
        if context_job(context) in by_label
        for step_id in step_ids
        if step_id in gate.BY_ID
        and step_id not in DIVERGENCES
        and expected_line(step_id) not in by_label[context_job(context)]
    ]


def unclaimed_repo_commands(commands: set[str]) -> list[str]:
    """Rule 4: a repository script CI runs is a gate step, or is recorded.

    The direction that notices a check added to an already-required job. It
    is scoped to commands naming `scripts/`, so ordinary shell plumbing —
    `set -euo pipefail`, an apt-get, a fetch — needs no entry.
    """
    claimed = {expected_line(step_id) for step_id in gate.GATE}
    return [
        f"a workflow runs `{command}`, which no gate step covers and CI_ONLY "
        f"does not excuse — `just gate` would not notice it failing"
        for command in sorted(commands)
        if "scripts/" in command and command not in claimed and command not in CI_ONLY
    ]


def stale_divergences(commands: set[str]) -> list[str]:
    """Rule 5a: a divergence that stopped being one, or never named a step."""
    unknown = [
        f"DIVERGENCES names {step_id!r}, which is not a gate step"
        for step_id in DIVERGENCES
        if step_id not in gate.BY_ID
    ]
    return unknown + [
        f"DIVERGENCES excuses {step_id!r}, but a workflow now runs "
        f"`{expected_line(step_id)}` verbatim — drop the entry"
        for step_id in DIVERGENCES
        if step_id in gate.BY_ID and expected_line(step_id) in commands
    ]


def stale_entries(commands: set[str], contexts: list[str]) -> list[str]:
    """Rule 5b: no table describes something that stopped existing."""
    named = set(CONTEXT_STEPS) | set(NOT_LOCAL)
    stale_contexts = [
        f"{context!r} is mapped here but is no longer a required status "
        f"check in {RULESET}"
        for context in sorted(named - set(contexts))
    ]
    return stale_contexts + [
        f"CI_ONLY excuses `{command}`, which no workflow runs any more"
        for command in sorted(set(CI_ONLY) - commands)
    ]


def unclaimed_suffixes() -> list[str]:
    """Rule 5c: an argument allowance must name a real step."""
    return [
        f"ARGUMENT_SUFFIX names {step_id!r}, which is not a gate step"
        for step_id in ARGUMENT_SUFFIX
        if step_id not in gate.BY_ID
    ]


def check(root: Path) -> list[str]:
    """Every parity rule, against one tree. Returns the violations found."""
    jobs = ci_jobs(root)
    commands = {command for job in jobs for command in job.commands}
    contexts = required_contexts(root)
    return (
        check_commands(commands)
        + check_local_only(root, commands)
        + check_env(root)
        + unmapped_contexts(contexts)
        + missing_mapped_steps()
        + missing_context_jobs(jobs, contexts)
        + misplaced_mapped_steps(jobs)
        + unclaimed_repo_commands(commands)
        + stale_divergences(commands)
        + stale_entries(commands, contexts)
        + unclaimed_suffixes()
    )


def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent
    violations = check(root)
    if violations:
        print("check-gate-parity: the gate table and CI disagree:", file=sys.stderr)
        for violation in violations:
            print(f"  - {violation}", file=sys.stderr)
        return 1

    verbatim = len(gate.GATE) - len(DIVERGENCES) - len(LOCAL_ONLY)
    print(
        f"check-gate-parity: OK — {verbatim} commands whole-line identical to "
        f"CI, {len(DIVERGENCES)} recorded divergences, {len(LOCAL_ONLY)} "
        f"local-only, {len(required_contexts(root))} required contexts mapped."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
