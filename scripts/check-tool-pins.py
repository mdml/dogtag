#!/usr/bin/env python3
"""Verify that pinned tool versions agree across the repository.

`tools.toml` is the single source of truth. The workflows spell their
versions out literally, because a workflow that computes its own pins is
unreadable — so something has to keep the two from drifting.

The checks are *structural*: every version is read out of the declaration
that actually runs — the `tool:` input handed to an install action, the
`toolchain:` input on a toolchain action, the URL a step downloads, the
`cargo +toolchain` a recipe invokes. A comment naming the right version
therefore proves nothing, and a stale command with an accurate comment
beside it fails. Comparisons are by equality; a prefix match would accept
1.85.1 where 1.85.0 was pinned.

Stdlib only (tomllib, re). Usage:
    check-tool-pins.py [repo-root]
"""

import re
import sys
import tomllib
from pathlib import Path

# Files that declare a pin. Workflows are read for action inputs and run
# steps; the justfile for the toolchains its recipes invoke.
WORKFLOWS = (
    ".github/workflows/ci.yml",
    ".github/workflows/security.yml",
    ".github/workflows/release.yml",
)
# Files that invoke a toolchain by name. scripts/gate.py declares each gate
# as the command string it runs, so the MSRV `cargo +toolchain` lives there
# rather than in a recipe; reading only the justfile would leave this check
# passing having found nothing to compare.
RECIPE_FILES = ("justfile", "scripts/gate.py")

# Each pattern anchors to declaration syntax, never to prose.
INSTALL_ACTION_TOOL = re.compile(r"^\s*tool:\s*([A-Za-z0-9_.-]+)@(\S+)\s*$", re.MULTILINE)
TOOLCHAIN_INPUT = re.compile(r'^\s*toolchain:\s*"([^"]+)"\s*$', re.MULTILINE)
CARGO_PLUS = re.compile(r"cargo\s+\+(\S+)")


class Repo:
    """The tree under inspection, so tests can point at a fixture."""

    def __init__(self, root: Path):
        self.root = root

    def text(self, relative: str) -> str:
        return (self.root / relative).read_text()

    def workflow_text(self) -> str:
        return "\n".join(self.text(path) for path in WORKFLOWS)

    def recipe_text(self) -> str:
        return "\n".join(self.text(path) for path in RECIPE_FILES)


def declared_tool_versions(text: str, tool: str) -> set[str]:
    """Versions declared for `tool` in `tool: <name>@<version>` inputs."""
    return {
        version for name, version in INSTALL_ACTION_TOOL.findall(text) if name == tool
    }


def declared_action_input(text: str, action: str, version: str) -> set[str]:
    """Versions given as a `version:` input to a specific pinned action."""
    pattern = re.compile(
        re.escape(action) + r"@[0-9a-f]{40}[^\n]*\n(?:[^\n]*\n)*?\s*version:\s*\"?([^\"\n]+)\"?"
    )
    return {match.strip() for match in pattern.findall(text)}


def equality_violation(label: str, found: set[str], want: str) -> list[str]:
    """`found` must be exactly the one pinned value, and must be non-empty."""
    if not found:
        return [f"{label} is not declared anywhere; tools.toml pins {want}"]
    if found != {want}:
        return [f"{label} declares {sorted(found)}, but tools.toml pins {want}"]
    return []


def check_install_action(repo: Repo, name: str, spec: dict) -> list[str]:
    found = declared_tool_versions(repo.workflow_text(), name)
    return equality_violation(f"`tool: {name}@…`", found, spec["version"])


def check_action_input(repo: Repo, name: str, spec: dict) -> list[str]:
    found = declared_action_input(repo.workflow_text(), spec["action"], spec["version"])
    return equality_violation(f"{spec['action']} `version:`", found, spec["version"])


FORMS = {
    "install-action": check_install_action,
    "action-input": check_action_input,
    "none": lambda repo, name, spec: [],
}


def check_codescene_sweep(repo: Repo, tools: dict) -> list[str]:
    """The 10.0 floor was last swept with the CLI version now pinned.

    Code Health is enforced locally rather than by a required check, so
    nothing in CI would notice the floor going stale. It goes stale in one
    specific way: CodeScene's rules change between CLI versions, so a bump
    can drop a file below 10.0 without any change touching that file — which
    is exactly what the delta the hooks run cannot see. Bumping the pin
    without re-sweeping is therefore the one move that silently inherits a
    floor nobody measured.
    """
    spec = tools["codescene-cli"]
    if spec["version"] == spec.get("swept_at"):
        return []
    return [
        f"tools.toml pins CodeScene CLI {spec['version']} but records the "
        f"10.0 sweep at {spec.get('swept_at')!r} — run `just codescene` to "
        f"re-establish the floor on the new CLI, then move swept_at with it"
    ]


def check_declared_tools(repo: Repo, tools: dict) -> list[str]:
    """Every tool is declared exactly as tools.toml pins it."""
    violations = []
    for name, spec in tools.items():
        if name == "rust":
            continue
        form = spec.get("form")
        if form not in FORMS:
            violations.append(f"tools.toml: {name} has unknown form {form!r}")
            continue
        violations.extend(FORMS[form](repo, name, spec))
    return violations


def check_toolchains(repo: Repo, rust: dict) -> list[str]:
    """Every toolchain a workflow or recipe invokes is one of the three pinned."""
    pinned = {rust["toolchain"], rust["msrv"], rust["coverage_nightly"]}
    declared = set(TOOLCHAIN_INPUT.findall(repo.workflow_text()))
    recipes = set(CARGO_PLUS.findall(repo.recipe_text()))

    violations = [
        f"a workflow declares `toolchain: \"{name}\"`, which tools.toml does "
        f"not pin (pinned: {sorted(pinned)})"
        for name in sorted(declared - pinned)
    ]
    violations += [
        f"a recipe file invokes `cargo +{name}`, which tools.toml does not "
        f"pin (pinned: {sorted(pinned)})"
        for name in sorted(recipes - pinned)
    ]
    if rust["msrv"] not in recipes:
        violations.append(
            f"tools.toml pins the MSRV toolchain {rust['msrv']}, but no "
            f"recipe file invokes `cargo +{rust['msrv']}` — the floor would "
            f"be declared and never built"
        )
    violations += [
        f"tools.toml pins the {key} toolchain {rust[key]}, but no workflow "
        f"declares it"
        for key in ("toolchain", "msrv", "coverage_nightly")
        if rust[key] not in declared
    ]
    return violations


def first_capture(pattern: str, text: str) -> str | None:
    match = re.search(pattern, text, re.MULTILINE)
    return match.group(1) if match else None


def check_cross_file_pins(repo: Repo, rust: dict) -> list[str]:
    """The Rust pins equal what the files declaring them independently say."""
    expected = [
        (
            rust["toolchain"],
            first_capture(r'^channel = "([^"]+)"', repo.text("rust-toolchain.toml")),
            "the toolchain channel in rust-toolchain.toml",
        ),
        (
            rust["msrv_manifest"],
            first_capture(r'^rust-version = "([^"]+)"', repo.text("Cargo.toml")),
            "rust-version in Cargo.toml",
        ),
        (
            rust["coverage_nightly"],
            first_capture(r'^toolchain = "([^"]+)"', repo.text("coverage-baseline.toml")),
            "the toolchain in coverage-baseline.toml",
        ),
    ]
    return [
        f"{label} is {found!r}, but tools.toml pins {want!r}"
        for want, found, label in expected
        if found != want
    ]


def check_internal_dependency_version(repo: Repo) -> list[str]:
    """The SDK's own version requirement must equal the workspace version.

    Cargo only refuses to resolve a path dependency when the requirement is
    unsatisfiable, and a caret requirement accepts every semver-compatible
    bump — so `0.1.0-beta.0` would silently keep matching at `0.1.1`. Cargo
    cannot catch that; this can.
    """
    manifest = tomllib.loads(repo.text("Cargo.toml"))
    declared = manifest["workspace"]["package"]["version"]
    required = manifest["workspace"]["dependencies"]["dogtag"]["version"]
    if declared != required:
        return [
            f"[workspace.dependencies].dogtag pins version {required}, but "
            f"[workspace.package].version is {declared} — bump both together"
        ]
    return []


def check(root: Path) -> list[str]:
    """Every pin check, against one tree. Returns the violations found."""
    repo = Repo(root)
    tools = tomllib.loads(repo.text("tools.toml"))
    rust = tools["rust"]
    return (
        check_declared_tools(repo, tools)
        + check_toolchains(repo, rust)
        + check_cross_file_pins(repo, rust)
        + check_internal_dependency_version(repo)
        + check_codescene_sweep(repo, tools)
    )


def main() -> int:
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent
    violations = check(root)
    if violations:
        print("check-tool-pins: pinned versions disagree:", file=sys.stderr)
        for violation in violations:
            print(f"  - {violation}", file=sys.stderr)
        return 1

    # Counted as what was actually compared, not as tables in the file: the
    # `[rust]` table is toolchains rather than a tool, and a `form = "none"`
    # entry declares nothing for this check to verify.
    tools = tomllib.loads(Repo(root).text("tools.toml"))
    compared = sum(
        1
        for name, spec in tools.items()
        if name != "rust" and spec.get("form") not in (None, "none")
    )
    print(
        f"check-tool-pins: {compared} declared tool pins and 3 toolchains "
        f"agree with the commands that use them."
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
