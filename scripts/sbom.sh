#!/usr/bin/env bash
# sbom.sh — generate a CycloneDX SBOM for the shipped dogtag binary.
#
# Usage: scripts/sbom.sh <target-triple>
#
# Produces:
#   dist/dogtag-<target-triple>.cdx.json   (CycloneDX 1.5, JSON)
#
# The document describes ONE BINARY ON ONE TARGET, which is the whole point:
# a whole-workspace SBOM would name the conformance harness's and the commit
# linter's dependencies, none of which is in the binary a user installs, and
# the supply-chain policy is explicit that a wrong SBOM is worse than none
# because it will be believed. `--describe binaries` is what scopes the
# document to the `dogtag` executable, and `--target` is what resolves the
# closure for the platform being shipped rather than for the builder's.
#
# Like scripts/package.sh, this is the single generation path shared by
# .github/workflows/release.yml and `just dist`, so the local rehearsal
# exercises exactly what CI ships. The tool is pinned in tools.toml.
set -euo pipefail

usage() {
  echo "usage: scripts/sbom.sh <target-triple>" >&2
  exit 2
}

if [ "$#" -ne 1 ]; then
  usage
fi
target="$1"

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

if ! command -v cargo-cyclonedx >/dev/null 2>&1; then
  echo "error: cargo-cyclonedx is not installed" >&2
  echo "hint: run ./scripts/install-dev-tools.sh, which installs the pinned version" >&2
  exit 1
fi

# cargo-cyclonedx writes beside each workspace member's manifest and takes no
# package filter, so one run scatters an SBOM through the tree for every
# member that builds a binary. Sweeping before and after leaves the working
# tree as it was found and makes a stale file impossible to mistake for a
# fresh one; dist/ is pruned so the sweep cannot eat its own output.
sweep() {
  find . \( -path ./target -o -path ./dist -o -path ./.git \) -prune \
    -o -name '*.cdx.json' -type f -exec rm -f {} +
}

emitted="crates/dogtag-cli/dogtag_bin.cdx.json"
output="dist/dogtag-$target.cdx.json"

sweep
trap sweep EXIT

echo "generating SBOM for the dogtag binary on $target"
cargo cyclonedx \
  --manifest-path crates/dogtag-cli/Cargo.toml \
  --target "$target" \
  --describe binaries \
  --all \
  --format json \
  --spec-version 1.5

if [ ! -f "$emitted" ]; then
  echo "error: cargo-cyclonedx produced no SBOM at $emitted" >&2
  echo "hint: it names the file after the binary target; check crates/dogtag-cli/Cargo.toml" >&2
  exit 1
fi

mkdir -p dist
install -m 0644 "$emitted" "$output"

echo "wrote $output"
