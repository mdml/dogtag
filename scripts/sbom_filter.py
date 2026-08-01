#!/usr/bin/env python3
"""Reconcile a generated CycloneDX document against the binary's real closure.

`cargo cyclonedx` builds its component list from `cargo metadata`'s resolve
graph. That graph is filtered by platform but NOT by feature, so a dependency
that exists only behind an unenabled feature is emitted as a required
component of a binary that never links it. At M2 that is `toml`'s optional
`preserve_order`, which pulls `indexmap` and its two dependencies into a
document describing a binary built without it.

The supply-chain policy is explicit that a wrong SBOM is worse than none
because it will be believed, and the release signs this document, so the
overstatement would be attested and — tags being immutable — permanent.

This script takes the raw document and the closure `cargo tree` resolves for
the same target, drops any component the closure does not contain, and then
asserts the two agree exactly. The filter removes overstatement; the
assertion catches understatement, which no filter can. Anything dropped is
named on stderr, because a silent correction is how the next one hides.
"""

from __future__ import annotations

import argparse
import json
import sys


def identifier(name: str, version: str) -> str:
    return f"{name}@{version}"


def read_closure(path: str, root: str) -> set[str]:
    """Read `name@version` lines, dropping the package being described.

    The root package is the SBOM's `metadata.component`, not one of its
    `components`, so leaving it in would make the comparison fail by one.
    """
    with open(path, encoding="utf-8") as handle:
        entries = {line.strip() for line in handle if line.strip()}
    return {entry for entry in entries if not entry.startswith(f"{root}@")}


def within(component: dict, closure: set[str]) -> bool:
    """Whether the build closure contains this component."""
    return identifier(component["name"], component["version"]) in closure


def without(graph: list[dict], stale: set[str]) -> list[dict]:
    """The dependency graph with every reference to a dropped component gone."""
    kept = [entry for entry in graph if entry.get("ref") not in stale]
    for entry in kept:
        entry["dependsOn"] = [r for r in entry.get("dependsOn", []) if r not in stale]
    return kept


def prune(document: dict, closure: set[str]) -> tuple[dict, list[str]]:
    """Drop components outside the closure, and every reference to them."""
    components = document.get("components", [])
    dropped = [c for c in components if not within(c, closure)]
    stale = {c["bom-ref"] for c in dropped if "bom-ref" in c}

    document["components"] = [c for c in components if within(c, closure)]
    document["dependencies"] = without(document.get("dependencies", []), stale)

    return document, sorted(identifier(c["name"], c["version"]) for c in dropped)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw", help="the document cargo-cyclonedx generated")
    parser.add_argument("output", help="where to write the reconciled document")
    parser.add_argument("--closure", required=True, help="name@version per line")
    parser.add_argument("--root", required=True, help="the package being described")
    args = parser.parse_args()

    closure = read_closure(args.closure, args.root)
    with open(args.raw, encoding="utf-8") as handle:
        document = json.load(handle)

    document, dropped = prune(document, closure)

    for name in dropped:
        print(f"sbom: dropped {name} — resolved by cargo metadata, not by the build", file=sys.stderr)

    present = {identifier(c["name"], c["version"]) for c in document["components"]}
    missing = sorted(closure - present)
    if missing:
        # Filtering can only ever remove, so a shortfall means the generator
        # and the build disagree in the direction no post-processing can fix.
        print("error: the SBOM omits components the binary links:", file=sys.stderr)
        for name in missing:
            print(f"  {name}", file=sys.stderr)
        return 1

    with open(args.output, "w", encoding="utf-8") as handle:
        json.dump(document, handle, indent=2)
        handle.write("\n")

    print(f"sbom: {len(present)} components reconciled against the build closure")
    return 0


if __name__ == "__main__":
    sys.exit(main())
