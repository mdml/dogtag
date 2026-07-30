#!/usr/bin/env python3
"""Adversarial tests for the pin checker.

The property under test is the one that makes the checker worth having: it
reads the declaration that actually runs, so a stale command cannot be
excused by an accurate comment sitting next to it. Every case below mutates
a copy of the real tree — a checker that passed against a synthetic fixture
but not the repository would prove nothing.

Stdlib only (unittest). Run directly, or via `just check`.
"""

import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
check_tool_pins = __import__("check-tool-pins")

REPO_ROOT = Path(__file__).resolve().parent.parent
CI = ".github/workflows/ci.yml"
SECURITY = ".github/workflows/security.yml"
GATE = "scripts/gate.py"


class PinCheckerTest(unittest.TestCase):
    """Each test mutates a throwaway copy of the tree and checks the verdict."""

    def setUp(self) -> None:
        self.tmp = Path(tempfile.mkdtemp(prefix="check-tool-pins-"))
        self.root = self.tmp / "repo"
        shutil.copytree(
            REPO_ROOT,
            self.root,
            ignore=shutil.ignore_patterns(
                ".git", "target", "dist", "node_modules", "__pycache__"
            ),
        )
        self.addCleanup(shutil.rmtree, self.tmp, ignore_errors=True)

    def edit(self, relative: str, old: str, new: str) -> None:
        """Rewrite one exact substring, failing loudly if it is not present."""
        path = self.root / relative
        text = path.read_text()
        self.assertIn(old, text, f"fixture text missing from {relative}")
        path.write_text(text.replace(old, new, 1))

    def violations(self) -> list[str]:
        return check_tool_pins.check(self.root)

    def assert_rejected(self, needle: str) -> None:
        found = self.violations()
        self.assertTrue(found, "expected a violation, got none")
        self.assertTrue(
            any(needle in v for v in found),
            f"no violation mentioned {needle!r}: {found}",
        )

    def test_the_unmodified_tree_passes(self) -> None:
        self.assertEqual(self.violations(), [])

    def test_stale_tool_pin_is_caught_despite_an_accurate_comment(self) -> None:
        """The load-bearing case: a comment cannot vouch for a stale command."""
        self.edit(
            CI,
            "tool: cargo-llvm-cov@0.8.7",
            "tool: cargo-llvm-cov@0.8.6 # pinned to 0.8.7 per tools.toml",
        )
        self.assert_rejected("cargo-llvm-cov")

    def test_a_comment_alone_does_not_satisfy_a_pin(self) -> None:
        """Removing the declaration but naming the version in prose fails."""
        self.edit(
            SECURITY,
            "tool: cargo-deny@0.20.2",
            "tool: cargo-deny@0.19.0\n          # tools.toml pins cargo-deny 0.20.2",
        )
        self.assert_rejected("cargo-deny")

    def test_a_tool_that_disappears_entirely_is_caught(self) -> None:
        self.edit(CI, "tool: cargo-llvm-cov@0.8.7", "tool: cargo-nextest@0.9.0")
        self.assert_rejected("not declared anywhere")

    def test_a_stale_action_version_input_is_caught(self) -> None:
        self.edit(CI, 'version: "1.28.0"', 'version: "1.27.0" # 1.28.0')
        self.assert_rejected("zizmor-action")

    def test_a_swapped_download_sha_is_caught(self) -> None:
        """The URL is the pin; a correct comment beside it changes nothing."""
        self.edit(
            CI,
            "cs-linux-amd64-5f703ce1f9c264701f32c795fa7104467f1e4ab4.zip",
            "cs-linux-amd64-" + "0" * 40 + ".zip",
        )
        self.assert_rejected("CodeScene download URL")

    def test_a_swapped_checksum_assertion_is_caught(self) -> None:
        self.edit(
            CI,
            "b6a1b259c6b53d94d34c85b85bb725b6665973ab2bec9f6c678a371d7a0202ee",
            "d" * 64,
        )
        self.assert_rejected("CodeScene checksum")

    def test_an_unpinned_toolchain_input_is_caught(self) -> None:
        self.edit(CI, 'toolchain: "1.85.0"', 'toolchain: "1.86.0"')
        self.assert_rejected("does not pin")

    def test_an_unpinned_cargo_plus_toolchain_is_caught(self) -> None:
        self.edit(GATE, "cargo +1.85.0 build", "cargo +1.84.0 build")
        self.assert_rejected("cargo +1.84.0")

    def test_a_toolchain_hidden_in_a_recipe_file_is_still_read(self) -> None:
        """The MSRV moved out of the justfile; the checker had to follow it."""
        self.edit("justfile", "python3 scripts/gate.py msrv-build", "cargo +9.9.9 build")
        self.assert_rejected("cargo +9.9.9")

    def test_dropping_the_msrv_invocation_entirely_is_caught(self) -> None:
        """A pinned floor nothing builds is a pin that stopped meaning anything."""
        self.edit(GATE, "cargo +1.85.0 build --workspace --locked", "cargo build")
        self.edit(GATE, "cargo +1.85.0 test --workspace --locked", "cargo test")
        self.assert_rejected("declared and never built")

    def test_an_msrv_prefix_is_not_accepted_as_equality(self) -> None:
        """1.85.1 shares a prefix with the pin and is still a different pin."""
        self.edit("Cargo.toml", 'rust-version = "1.85"', 'rust-version = "1.85.1"')
        self.assert_rejected("rust-version in Cargo.toml")

    def test_a_drifted_toolchain_file_is_caught(self) -> None:
        self.edit("rust-toolchain.toml", 'channel = "1.97.1"', 'channel = "1.97.0"')
        self.assert_rejected("rust-toolchain.toml")

    def test_a_drifted_coverage_nightly_is_caught(self) -> None:
        self.edit(
            "coverage-baseline.toml",
            'toolchain = "nightly-2026-07-30"',
            'toolchain = "nightly-2026-07-29"',
        )
        self.assert_rejected("coverage-baseline.toml")

    def test_a_semver_compatible_sdk_version_bump_is_caught(self) -> None:
        """Cargo resolves 0.1.1 against a ^0.1.0-beta.0 requirement silently."""
        self.edit("Cargo.toml", 'version = "0.1.0-beta.0"', 'version = "0.1.1"')
        self.assert_rejected("bump both together")


if __name__ == "__main__":
    unittest.main(verbosity=2)
