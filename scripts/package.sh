#!/usr/bin/env bash
# package.sh — stage a built dogtag binary into a release archive.
#
# Usage: scripts/package.sh <target-triple> [binary-path]
#
# Produces:
#   dist/dogtag-<target-triple>.tar.gz         (contains dogtag, LICENSE, README.md)
#   dist/dogtag-<target-triple>.tar.gz.sha256  ("<hex>  <filename>", sha256sum -c compatible)
#
# Artifact names are deliberately VERSIONLESS; the version is scoped by the
# release tag. This script is the single packaging path shared by
# .github/workflows/release.yml and `just dist`, so the local rehearsal
# exercises exactly what CI ships.
set -euo pipefail

usage() {
  echo "usage: scripts/package.sh <target-triple> [binary-path]" >&2
  exit 2
}

if [ "$#" -lt 1 ] || [ "$#" -gt 2 ]; then
  usage
fi
target="$1"

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

if [ "$#" -eq 2 ]; then
  binary="$2"
elif [ -f "target/$target/release/dogtag" ]; then
  binary="target/$target/release/dogtag"
else
  binary="target/release/dogtag"
fi

if [ ! -f "$binary" ]; then
  echo "error: binary not found at $binary" >&2
  echo "hint: build it first, e.g. cargo build --release --locked -p dogtag-cli --target $target" >&2
  exit 1
fi

# Sanity echo only — the version never appears in the artifact name.
version="$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version = "\([^"]*\)".*/\1/p' Cargo.toml | head -n 1)"
echo "packaging dogtag ${version:-unknown-version} for $target"

archive="dogtag-$target.tar.gz"
stage="dist/stage/dogtag-$target"
rm -rf "$stage"
mkdir -p "$stage"

install -m 0755 "$binary" "$stage/dogtag"
install -m 0644 LICENSE "$stage/LICENSE"
install -m 0644 README.md "$stage/README.md"

# Byte-reproducible archive: fixed mtimes on every staged file, a fixed
# explicit member ordering, and gzip -n so no timestamp is embedded in the
# gzip header — rebuilding the same binary yields the same bytes. GNU tar
# additionally normalizes ownership to 0:0; BSD tar (macOS) has no such
# flags, so they are guarded.
touch -t 202607300000 "$stage/dogtag" "$stage/LICENSE" "$stage/README.md"
if tar --version 2>/dev/null | grep -q 'GNU tar'; then
  tar --owner=0 --group=0 --numeric-owner -cf - -C "$stage" dogtag LICENSE README.md \
    | gzip -n > "dist/$archive"
else
  tar -cf - -C "$stage" dogtag LICENSE README.md | gzip -n > "dist/$archive"
fi

(
  cd dist
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$archive" > "$archive.sha256"
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$archive" > "$archive.sha256"
  else
    echo "error: neither sha256sum nor shasum is available" >&2
    exit 1
  fi
)

echo "wrote dist/$archive"
echo "wrote dist/$archive.sha256"
