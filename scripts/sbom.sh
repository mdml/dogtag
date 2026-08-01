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
closure="$(mktemp)"

cleanup() {
  sweep
  rm -f "$closure"
}

sweep
trap cleanup EXIT

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

# The closure the build actually resolves, feature gates included. cargo tree
# honours them; cargo metadata's resolve graph, which the generator reads,
# does not. Reconciling against this is what keeps the attested document from
# claiming crates the binary never links.
cargo tree \
  --locked \
  --package dogtag-cli \
  --edges normal \
  --target "$target" \
  --prefix none \
  | awk 'NF >= 2 { print $1 "@" substr($2, 2) }' \
  | sort -u >"$closure"

mkdir -p dist
python3 scripts/sbom_filter.py "$emitted" "$output" \
  --closure "$closure" \
  --root dogtag-cli

# The SBOM gets the same sidecar every other published asset gets. It is an
# attestation *predicate*, not a subject, so `gh attestation verify` reads the
# copy in the transparency log and never looks at the published file — which
# leaves the asset sitting beside the archive with nothing checking it unless
# the aggregate covers it.
(
  cd dist
  name="$(basename "$output")"
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$name" >"$name.sha256"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$name" >"$name.sha256"
  else
    echo "error: neither sha256sum nor shasum is available" >&2
    exit 1
  fi
)

echo "wrote $output"
echo "wrote $output.sha256"
