#!/usr/bin/env python3
"""Verify the checked-in ruleset payloads against the record that explains them.

`.github/rulesets/*.json` are the files the provisioning commands actually
POST. The workflow-security ADR quotes them verbatim so a reader can see the
rules and the reasoning together. Two copies of anything drift, so this
asserts they are the same JSON, and checks the properties that would be
silently wrong rather than loudly broken:

1. Each payload appears in the ADR, parsed-equal to the checked-in file.
2. Every required-status-check list holds exactly the expected contexts —
   counted, so a context dropped in an edit fails rather than passing as a
   shorter list.
3. Neither ruleset carries a bypass actor, and the branch ruleset permits
   only rebase merges: a squash merge would put a commit on main whose
   message no gate validated.

Stdlib only (json, re). Usage: check-ruleset-payloads.py [repo-root]
"""

import json
import re
import sys
from pathlib import Path

ADR = "docs/adr/2026-07-30-workflow-security-and-repository-rules.md"
RULESET_DIR = ".github/rulesets"

# The eight CI jobs plus cargo-deny, each of which runs on a pull request and
# on a push to main; the OSV context differs because the pull-request scan
# and the full scan are separate jobs with separate triggers.
#
# Code Health is deliberately absent. It is enforced locally by `just gate`
# against the pinned CodeScene CLI, not by a required check — see the
# [code health ADR](../docs/adr/2026-07-30-code-health-and-coverage-gates.md).
# Adding it back here without also restoring the CI job would create a
# required context nothing reports, which blocks every merge permanently.
COMMON_CONTEXTS = [
    "Format, lint, test (Linux)",
    "Commit messages",
    "Test (macOS arm64)",
    "MSRV (Rust 1.85)",
    "Coverage thresholds",
    "Workflow security (zizmor)",
    "Release build check (x86_64 musl)",
    "Markdown link integrity",
    "cargo-deny",
]

EXPECTED = {
    "main-branch.json": {
        "name": "main",
        "target": "branch",
        "contexts": COMMON_CONTEXTS + ["osv-pr / osv-scan"],
        "merge_methods": ["rebase"],
    },
    "release-tags.json": {
        "name": "release-tags",
        "target": "tag",
        "contexts": COMMON_CONTEXTS + ["osv-full / osv-scan"],
        "merge_methods": None,
    },
}


def adr_payloads(root: Path) -> list[dict]:
    """Every fenced JSON block in the ADR, parsed."""
    text = (root / ADR).read_text()
    return [json.loads(b) for b in re.findall(r"```json\n(.*?)\n```", text, re.DOTALL)]


def rule_of(payload: dict, kind: str) -> dict | None:
    """The parameters of the named rule, if the payload declares it."""
    for rule in payload["rules"]:
        if rule["type"] == kind:
            return rule.get("parameters", {})
    return None


def declared_contexts(payload: dict) -> list[str]:
    params = rule_of(payload, "required_status_checks") or {}
    return [c["context"] for c in params.get("required_status_checks", [])]


def check_contexts(label: str, payload: dict, expected: list[str]) -> list[str]:
    """Rule 2: the context list is exactly the expected one, in count and content."""
    found = declared_contexts(payload)
    if found == expected:
        return []
    missing = [c for c in expected if c not in found]
    extra = [c for c in found if c not in expected]
    detail = f"{len(found)} contexts, expected {len(expected)}"
    if missing:
        detail += f"; missing {missing}"
    if extra:
        detail += f"; unexpected {extra}"
    return [f"{label}: {detail}"]


def check_posture(label: str, payload: dict, merge_methods: list[str] | None) -> list[str]:
    """Rule 3: no bypass actors, and merges cannot synthesize a commit."""
    violations = []
    if payload.get("bypass_actors"):
        violations.append(
            f"{label}: bypass_actors is non-empty, so the rules would not bind "
            f"the admin they mainly exist to bind"
        )
    if merge_methods is None:
        return violations
    found = (rule_of(payload, "pull_request") or {}).get("allowed_merge_methods")
    if found != merge_methods:
        violations.append(
            f"{label}: allowed_merge_methods is {found}, expected "
            f"{merge_methods} — a squash merge lands a commit message no gate "
            f"validated"
        )
    return violations


def check_payload(root: Path, filename: str, expected: dict, quoted: list[dict]) -> list[str]:
    """One payload: matches its ADR copy, and states what it should."""
    payload = json.loads((root / RULESET_DIR / filename).read_text())
    label = f"{RULESET_DIR}/{filename}"

    violations = []
    if payload["target"] != expected["target"]:
        violations.append(f"{label}: target is {payload['target']!r}")
    if payload not in quoted:
        violations.append(
            f"{label}: no JSON block in {ADR} matches it — the record and the "
            f"payload the provisioning command posts have drifted apart"
        )
    violations += check_contexts(label, payload, expected["contexts"])
    violations += check_posture(label, payload, expected["merge_methods"])
    return violations


def check(root: Path) -> list[str]:
    quoted = adr_payloads(root)
    violations = []
    for filename, expected in EXPECTED.items():
        violations += check_payload(root, filename, expected, quoted)
    return violations


def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent
    violations = check(root)
    if violations:
        print("check-ruleset-payloads: the ruleset payloads are wrong:", file=sys.stderr)
        for violation in violations:
            print(f"  - {violation}", file=sys.stderr)
        return 1

    counts = ", ".join(
        f"{name} ({len(spec['contexts'])} contexts)" for name, spec in EXPECTED.items()
    )
    print(f"check-ruleset-payloads: OK — {counts}, matching {ADR}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
