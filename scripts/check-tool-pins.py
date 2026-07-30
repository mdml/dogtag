#!/usr/bin/env python3
"""Verify that pinned tool versions agree across the repository.

`tools.toml` is the single source of truth for developer and CI tool
versions. The workflows spell their versions out literally, because a
workflow that computes its own pins is unreadable — so something has to keep
the two from drifting. This does, offline and deterministically:

1. Every tool in tools.toml whose `checked_in` names a file must have its
   version appear in that file.
2. The Rust toolchain pin must match rust-toolchain.toml, the MSRV must match
   Cargo.toml's rust-version, and the coverage nightly must match
   coverage-baseline.toml.

Stdlib only (tomllib). Run from anywhere; paths resolve against the repo root.
"""

import re
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def read_text(relative: str) -> str:
    return (REPO_ROOT / relative).read_text()


def pinned_values(spec: dict) -> list[str]:
    """Every literal in a tool's entry that a pinning file must reproduce.

    For most tools that is just the version. The CodeScene CLI is fetched by
    raw URL keyed on a build SHA and verified against a checksum, so those
    two strings are the pin — a workflow could name the right version while
    downloading something else entirely.
    """
    keys = ("version", "build_sha", "sha256")
    return [spec[key] for key in keys if key in spec]


def missing_from(target: str, values: list[str], label: str) -> list[str]:
    """Which of `values` the file at `target` fails to mention."""
    text = read_text(target)
    return [
        f"{target} does not reference {label} {value} "
        f"(tools.toml is the source of truth)"
        for value in values
        if value not in text
    ]


def check_workflow_references(tools: dict) -> list[str]:
    """Rule 1: each tool's pinned literals appear in every file that pins it."""
    violations = []
    for name, spec in tools.items():
        checked_in = spec.get("checked_in", [])
        if not isinstance(checked_in, list):
            continue  # per-key form, handled with the Rust pins
        for target in checked_in:
            violations.extend(missing_from(target, pinned_values(spec), name))
    return violations


def check_rust_references(rust: dict) -> list[str]:
    """The Rust pins, each against the files that must carry that one."""
    violations = []
    for key, targets in rust.get("checked_in", {}).items():
        for target in targets:
            violations.extend(missing_from(target, [rust[key]], f"rust {key}"))
    return violations


def extract(pattern: str, text: str, label: str) -> tuple[str | None, str | None]:
    """First capture group of `pattern`, or a violation describing its absence."""
    match = re.search(pattern, text, re.MULTILINE)
    if match is None:
        return None, f"could not find {label}"
    return match.group(1), None


def check_rust_pins(rust: dict) -> list[str]:
    """Rule 2: the Rust pins agree with the files that also declare them."""
    expected = [
        (
            rust["toolchain"],
            r'^channel = "([^"]+)"',
            read_text("rust-toolchain.toml"),
            "the toolchain channel in rust-toolchain.toml",
        ),
        (
            rust["msrv"],
            r'^rust-version = "([^"]+)"',
            read_text("Cargo.toml"),
            "rust-version in Cargo.toml",
        ),
        (
            rust["coverage_nightly"],
            r'^toolchain = "([^"]+)"',
            read_text("coverage-baseline.toml"),
            "the toolchain in coverage-baseline.toml",
        ),
    ]

    violations = []
    for want, pattern, text, label in expected:
        found, problem = extract(pattern, text, label)
        if problem:
            violations.append(problem)
        elif not want.startswith(found):
            violations.append(
                f"{label} is {found}, but tools.toml pins {want}"
            )
    return violations


def check_internal_dependency_version() -> list[str]:
    """The SDK's own version requirement must equal the workspace version.

    Cargo only refuses to resolve a path dependency when the requirement is
    unsatisfiable, and a caret requirement accepts every semver-compatible
    bump — so `0.1.0-beta.0` would silently keep matching at `0.1.1`. Cargo
    cannot catch that; this can.
    """
    manifest = tomllib.loads(read_text("Cargo.toml"))
    declared = manifest["workspace"]["package"]["version"]
    required = manifest["workspace"]["dependencies"]["dogtag"]["version"]
    if declared != required:
        return [
            f"[workspace.dependencies].dogtag pins version {required}, but "
            f"[workspace.package].version is {declared} — bump both together"
        ]
    return []


def main() -> int:
    tools = tomllib.loads(read_text("tools.toml"))
    violations = (
        check_workflow_references(tools)
        + check_rust_references(tools["rust"])
        + check_rust_pins(tools["rust"])
        + check_internal_dependency_version()
    )

    if violations:
        print("check-tool-pins: pinned versions disagree:", file=sys.stderr)
        for violation in violations:
            print(f"  - {violation}", file=sys.stderr)
        return 1

    print(f"check-tool-pins: {len(tools)} pinned tool versions agree.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
