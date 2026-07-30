#!/usr/bin/env python3
"""Enforce the coverage contract from a cargo-llvm-cov JSON export.

Reads the full JSON export (llvm-cov export format: .data[0].totals plus
per-file .data[0].files[] summaries) and the committed baseline
(coverage-baseline.toml), then enforces:

1. Global line percent >= thresholds.line and >= the global.line baseline.
2. Global branch percent >= thresholds.branch and >= the global.branch
   baseline — failing loudly if zero branches were instrumented, which
   means branch coverage did not run (e.g. a stable toolchain).
3. Every file under a [kernel] path: line percent == kernel.line (100%).

Prints the per-file table and global totals either way, so CI logs and a
verbose local run show the same evidence — `scripts/gate.py` captures this
output and renders one line unless asked for more, which is why
`just coverage-verbose` and `just gate-verbose` are the local invocations
that show the table. Exits nonzero listing every violated rule precisely;
exits 0 with a one-line summary otherwise. That summary line is also the
metric the gate runner reads, so its shape is a contract, not a nicety.

Stdlib only (json, tomllib). Usage:
    coverage_check.py <coverage.json> <coverage-baseline.toml>
"""

import json
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def relative_to_repo(filename: str) -> str:
    """Relativize an absolute export path against the repo root."""
    path = Path(filename)
    try:
        return path.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return path.as_posix()


def summarize_files(export: dict) -> list[tuple[str, float, float]]:
    """Per-file (relative path, line %, branch %) rows, sorted by path."""
    rows = []
    for entry in export["files"]:
        summary = entry["summary"]
        rows.append(
            (
                relative_to_repo(entry["filename"]),
                summary["lines"]["percent"],
                summary["branches"]["percent"],
            )
        )
    return sorted(rows)


def print_table(rows: list[tuple[str, float, float]], totals: dict) -> None:
    """The evidence table: one row per file, then the global totals."""
    width = max(len(path) for path, _, _ in rows + [("TOTAL", 0.0, 0.0)])
    print(f"{'file':<{width}}  {'line %':>7}  {'branch %':>8}")
    print(f"{'-' * width}  {'-' * 7}  {'-' * 8}")
    for path, line, branch in rows:
        print(f"{path:<{width}}  {line:>7.2f}  {branch:>8.2f}")
    print(
        f"{'TOTAL':<{width}}  {totals['lines']['percent']:>7.2f}"
        f"  {totals['branches']['percent']:>8.2f}"
    )


def check_global(totals: dict, baseline: dict) -> list[str]:
    """Rules 1-2: global line/branch against thresholds and the ratchet."""
    violations = []
    line = totals["lines"]["percent"]
    branch = totals["branches"]["percent"]
    checks = [
        ("line", line, baseline["thresholds"]["line"], "threshold"),
        ("line", line, baseline["global"]["line"], "baseline ratchet"),
        ("branch", branch, baseline["thresholds"]["branch"], "threshold"),
        ("branch", branch, baseline["global"]["branch"], "baseline ratchet"),
    ]
    for kind, measured, required, rule in checks:
        if measured < required:
            violations.append(
                f"global {kind} coverage {measured:.2f}% is below the {rule} "
                f"of {required:.2f}%"
            )
    if totals["branches"]["count"] == 0:
        violations.append(
            "branch instrumentation produced zero branches — branch coverage "
            "did not run (is the pinned nightly toolchain in use?)"
        )
    return violations


def unmatched_kernel_paths(
    rows: list[tuple[str, float, float]], paths: list[str]
) -> list[str]:
    """Kernel paths matching no measured file.

    A path that matches nothing — renamed module, changed layout, a typo —
    would otherwise let the strictest rule in the contract pass by covering
    nothing at all, while still reporting success.
    """
    return [
        f"kernel path {prefix!r} matched no measured file; the rule it "
        f"states is not being enforced against anything"
        for prefix in paths
        if not any(path.startswith(prefix) for path, _, _ in rows)
    ]


def kernel_files_below_bar(
    rows: list[tuple[str, float, float]], paths: list[str], bar: float
) -> list[str]:
    """Kernel files whose line coverage falls short of the kernel bar."""
    return [
        f"kernel file {path} has {line:.2f}% line coverage; the kernel bar "
        f"is {bar:.2f}%"
        for path, line, _branch in rows
        if any(path.startswith(prefix) for prefix in paths) and line < bar
    ]


def worst_kernel_line(
    rows: list[tuple[str, float, float]], paths: list[str]
) -> float:
    """The lowest line percentage any kernel file reached."""
    return min(
        line
        for path, line, _branch in rows
        if any(path.startswith(prefix) for prefix in paths)
    )


def check_kernel(
    rows: list[tuple[str, float, float]], baseline: dict
) -> list[str]:
    """Rule 3: the kernel paths match real files, and each meets the bar."""
    kernel = baseline["kernel"]
    paths = kernel["paths"]
    return unmatched_kernel_paths(rows, paths) + kernel_files_below_bar(
        rows, paths, kernel["line"]
    )


def main() -> int:
    if len(sys.argv) != 3:
        print(
            "usage: coverage_check.py <coverage.json> <coverage-baseline.toml>",
            file=sys.stderr,
        )
        return 2

    export = json.loads(Path(sys.argv[1]).read_text())["data"][0]
    baseline = tomllib.loads(Path(sys.argv[2]).read_text())

    totals = export["totals"]
    rows = summarize_files(export)
    print_table(rows, totals)

    violations = check_global(totals, baseline) + check_kernel(rows, baseline)
    if violations:
        print(f"\ncoverage-check: {len(violations)} rule(s) violated:", file=sys.stderr)
        for violation in violations:
            print(f"  - {violation}", file=sys.stderr)
        return 1

    # The measured worst kernel file, not the configured bar: printing the
    # bar back would report the rule as though it were the result, and read
    # identically whether the kernel was at 100% or had no files at all.
    print(
        f"\ncoverage-check: OK — line {totals['lines']['percent']:.2f}%, "
        f"branch {totals['branches']['percent']:.2f}%, worst kernel file "
        f"{worst_kernel_line(rows, baseline['kernel']['paths']):.2f}%."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
