#!/usr/bin/env python3
"""check_security_exceptions.py — enforce the security-exception registry.

Cross-checks docs/security/exceptions.toml against every security tool
config, in both directions: every suppression (deny.toml [advisories].ignore,
osv-scanner.toml IgnoredVulns, bunfig.toml minimumReleaseAgeExcludes) must
have a registry entry of the matching tool, every registry entry must be
valid and unexpired, and every registry entry must point at a real
suppression (bun one-offs marked `oneoff = true` are exempt from the reverse
check). Also asserts the bun quarantine window is never weakened below seven
days. Exits nonzero listing every violation. Stdlib only (tomllib).
"""

import datetime
import sys
import tomllib
from collections.abc import Callable
from pathlib import Path
from typing import NamedTuple

ROOT = Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "docs" / "security" / "exceptions.toml"
DENY = ROOT / "deny.toml"
OSV = ROOT / "osv-scanner.toml"
BUNFIG = ROOT / "bindings" / "typescript" / "bunfig.toml"

ALLOWED_TOOLS = {"cargo-deny", "osv-scanner", "bun"}
REQUIRED_FIELDS = ("id", "tool", "rationale", "owner", "expires", "record")
MIN_RELEASE_AGE = 604800  # seven days in seconds


class Suppressions(NamedTuple):
    """One tool config's suppression list: where it lives, how to read it."""

    path: Path
    field: str
    tool: str
    identify: Callable[[object], object]


def deny_id(item: object) -> object:
    """deny.toml takes a bare advisory id or an { id, reason } table."""
    return item.get("id") if isinstance(item, dict) else item


def osv_id(item: object) -> object:
    """osv-scanner.toml IgnoredVulns entries are always tables with an id."""
    return item.get("id") if isinstance(item, dict) else None


def bun_id(item: object) -> object:
    """minimumReleaseAgeExcludes entries are plain package names."""
    return item


DENY_IGNORES = Suppressions(DENY, "[advisories].ignore", "cargo-deny", deny_id)
OSV_IGNORES = Suppressions(OSV, "IgnoredVulns", "osv-scanner", osv_id)
BUN_EXCLUDES = Suppressions(BUNFIG, "minimumReleaseAgeExcludes", "bun", bun_id)


def load(path: Path) -> dict:
    """Parse one TOML config."""
    with path.open("rb") as f:
        return tomllib.load(f)


def rel(path: Path) -> str:
    """Path as written in messages: relative to the repo root."""
    return str(path.relative_to(ROOT))


def as_date(value: object) -> datetime.date | None:
    """A TOML date (or datetime) narrowed to a date; None when neither."""
    if isinstance(value, datetime.datetime):
        return value.date()
    if isinstance(value, datetime.date):
        return value
    return None


def entry_error(where: str, entry: object, today: datetime.date) -> str | None:
    """The first rule a registry entry breaks; None when it is well-formed."""
    if not isinstance(entry, dict):
        return f"{where}: not a table"
    missing = [f for f in REQUIRED_FIELDS if f not in entry]
    if missing:
        return f"{where}: missing required field(s): {', '.join(missing)}"
    if entry["tool"] not in ALLOWED_TOOLS:
        return (
            f"{where} ({entry['id']}): tool '{entry['tool']}' not one of "
            f"{sorted(ALLOWED_TOOLS)}"
        )
    expires = as_date(entry["expires"])
    if expires is None:
        return f"{where} ({entry['id']}): expires must be a TOML date"
    if expires < today:
        return f"{where} ({entry['id']}): expired on {expires}"
    # A record that does not resolve is a citation of nothing; the link
    # checker only reads Markdown, so this is the one place it can be caught.
    if not (ROOT / str(entry["record"])).exists():
        return (
            f"{where} ({entry['id']}): record '{entry['record']}' does not "
            f"exist (paths are relative to the repository root)"
        )
    return None


def check_registry(registry: dict) -> tuple[list[dict], list[str]]:
    """Rule 1: every entry well-formed and unexpired. Returns the valid ones."""
    entries = registry.get("exception", [])
    if not isinstance(entries, list):
        return [], [f"{rel(REGISTRY)}: 'exception' must be an array of tables"]
    today = datetime.date.today()
    valid: list[dict] = []
    violations: list[str] = []
    for i, entry in enumerate(entries):
        error = entry_error(f"{rel(REGISTRY)}: entry {i + 1}", entry, today)
        if error is None:
            valid.append(entry)
        else:
            violations.append(error)
    return valid, violations


def registered_ids(valid_entries: list[dict], tool: str) -> set[str]:
    """The registry ids claimed for one tool."""
    return {entry["id"] for entry in valid_entries if entry["tool"] == tool}


def check_ignores(
    source: Suppressions, items: list, valid_entries: list[dict]
) -> tuple[set[str], list[str]]:
    """Rule 2: each suppression needs a registry entry of the matching tool."""
    registered = registered_ids(valid_entries, source.tool)
    found: set[str] = set()
    violations: list[str] = []
    for item in items:
        identifier = source.identify(item)
        if not isinstance(identifier, str):
            violations.append(
                f"{rel(source.path)}: unrecognized {source.field} entry: {item!r}"
            )
            continue
        found.add(identifier)
        if identifier not in registered:
            violations.append(
                f"{rel(source.path)}: {source.field} '{identifier}' has no "
                f'tool = "{source.tool}" entry in {rel(REGISTRY)}'
            )
    return found, violations


def discover(filename: str) -> list[Path]:
    """Every config of this name in the tree, not just the one at the root.

    osv-scanner reads the config sitting beside each lockfile and never walks
    upward, and bun resolves bunfig.toml relative to the working directory —
    so a nested config is honored by the tool while a root-only checker sees
    nothing. Searching for all of them is the only way the registry's promise
    ("every suppression, in every tool") can actually hold.
    """
    skip = {"target", "node_modules", ".git", "dist"}
    return sorted(
        path
        for path in ROOT.rglob(filename)
        if not skip & set(path.relative_to(ROOT).parts)
    )


def check_quarantine(bunfigs: list[tuple[Path, dict]]) -> list[str]:
    """Rule 3: no bunfig anywhere weakens the quarantine below seven days."""
    violations = []
    for path, install in bunfigs:
        age = install.get("minimumReleaseAge")
        if not (isinstance(age, int) and age >= MIN_RELEASE_AGE):
            violations.append(
                f"{rel(path)}: [install].minimumReleaseAge must be an integer "
                f">= {MIN_RELEASE_AGE} (found {age!r})"
            )
    if not bunfigs:
        violations.append(
            f"{rel(BUNFIG)} is missing: the quarantine must be configured "
            f"before the first dependency is ever resolved"
        )
    return violations


def suppression_sources(
    bunfigs: list[tuple[Path, dict]],
) -> list[tuple[Suppressions, list]]:
    """Each tool config paired with the raw suppression list it declares."""
    sources = [(DENY_IGNORES, load(DENY).get("advisories", {}).get("ignore", []))]
    for path in discover("osv-scanner.toml"):
        sources.append(
            (OSV_IGNORES._replace(path=path), load(path).get("IgnoredVulns", []))
        )
    for path, install in bunfigs:
        sources.append(
            (
                BUN_EXCLUDES._replace(path=path),
                install.get("minimumReleaseAgeExcludes", []),
            )
        )
    return sources


def check_tool_configs(
    bunfigs: list[tuple[Path, dict]], valid_entries: list[dict]
) -> tuple[dict[str, set[str]], list[str]]:
    """Rule 2 across every tool config. Returns the suppressions found, by tool."""
    suppressed: dict[str, set[str]] = {}
    violations: list[str] = []
    for source, items in suppression_sources(bunfigs):
        found, source_violations = check_ignores(source, items, valid_entries)
        # Union rather than assign: one tool can have several config files.
        suppressed.setdefault(source.tool, set()).update(found)
        violations += source_violations
    return suppressed, violations


# The deny.toml settings that constitute the policy, and the only values
# that count as "not weakened". Turning a check off wholesale is a larger
# suppression than ignoring one advisory, so it cannot be the one move that
# needs no rationale, owner, or expiry — which, before this, it was.
DENY_POLICY_FLOOR = {
    ("advisories", "yanked"): {"deny"},
    ("advisories", "unmaintained"): {"all"},
    ("advisories", "unsound"): {"all"},
    ("bans", "multiple-versions"): {"deny"},
    ("bans", "wildcards"): {"deny"},
    ("sources", "unknown-registry"): {"deny"},
    ("sources", "unknown-git"): {"deny"},
}

# Keys whose mere presence widens policy: a non-empty value is a suppression
# with no identifier for the registry to match, so the floor is "empty".
DENY_MUST_BE_EMPTY = [
    ("bans", "skip"),
    ("bans", "skip-tree"),
    ("sources", "allow-git"),
    ("sources", "allow-registry-extra"),
]

ALLOWED_LICENSES = {"Apache-2.0", "MIT"}


def check_deny_policy(deny: dict) -> list[str]:
    """The cargo-deny policy itself has not been weakened.

    `[advisories].ignore` is the suppression the registry tracks by
    identifier. Everything here is a suppression with no identifier at all —
    disabling a check, skipping a crate tree, allowing a git source, widening
    the license list. Left unguarded, the cheapest way to silence cargo-deny
    would be the one requiring no record.
    """
    return (
        weakened_settings(deny)
        + non_empty_allowances(deny)
        + widened_licenses(deny)
    )


def weakened_settings(deny: dict) -> list[str]:
    """Policy settings turned down from their required value."""
    return [
        f"{rel(DENY)}: [{section}].{key} is {deny[section][key]!r}, weakening "
        f"the policy (expected one of {sorted(allowed)})"
        for (section, key), allowed in DENY_POLICY_FLOOR.items()
        if key in deny.get(section, {}) and deny[section][key] not in allowed
    ]


def non_empty_allowances(deny: dict) -> list[str]:
    """Blanket allowances, which carry no identifier to register."""
    return [
        f"{rel(DENY)}: [{section}].{key} is non-empty; a reviewed exception "
        f"belongs in {rel(REGISTRY)}, not in a bare allowance"
        for section, key in DENY_MUST_BE_EMPTY
        if deny.get(section, {}).get(key)
    ]


def widened_licenses(deny: dict) -> list[str]:
    """License identifiers admitted beyond the approved set."""
    extra = set(deny.get("licenses", {}).get("allow", [])) - ALLOWED_LICENSES
    if not extra:
        return []
    return [
        f"{rel(DENY)}: [licenses].allow adds {sorted(extra)} beyond the "
        f"approved set {sorted(ALLOWED_LICENSES)}; widen it deliberately "
        f"with a record, or scope it as a per-crate exception"
    ]


def is_oneoff(entry: dict) -> bool:
    """A one-off quarantine override leaves no standing config trace."""
    return entry["tool"] == "bun" and entry.get("oneoff") is True


def check_stale(
    valid_entries: list[dict], suppressed: dict[str, set[str]]
) -> list[str]:
    """Rule 4 (reverse): every entry must trace to an actual suppression."""
    violations = []
    for entry in valid_entries:
        if is_oneoff(entry) or entry["id"] in suppressed.get(entry["tool"], set()):
            continue
        violations.append(
            f"{rel(REGISTRY)}: entry '{entry['id']}' (tool = \"{entry['tool']}\") "
            f"matches no suppression in any tool config — remove the stale entry"
        )
    return violations


def report(violations: list[str]) -> int:
    """Print every violation precisely, one per line, and fail."""
    for violation in violations:
        print(f"ERROR: {violation}", file=sys.stderr)
    print(f"check_security_exceptions: {len(violations)} violation(s).", file=sys.stderr)
    return 1


def main() -> int:
    valid_entries, violations = check_registry(load(REGISTRY))
    bunfigs = [(path, load(path).get("install", {})) for path in discover("bunfig.toml")]
    violations += check_quarantine(bunfigs) + check_deny_policy(load(DENY))
    suppressed, config_violations = check_tool_configs(bunfigs, valid_entries)
    violations += config_violations + check_stale(valid_entries, suppressed)
    if violations:
        return report(violations)

    counts = ", ".join(
        f"{len(suppressed.get(tool, set()))} {tool}" for tool in sorted(ALLOWED_TOOLS)
    )
    print(
        f"check_security_exceptions: OK — {len(valid_entries)} registry "
        f"entrie(s); suppressions: {counts}."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
