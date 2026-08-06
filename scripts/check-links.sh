#!/usr/bin/env bash
# check-links.sh — verify that relative markdown links resolve to existing files.
#
# Scans every *.md file in the repo (excluding .git/, target/, dist/,
# node_modules/, and the conformance fixture corpora — see below), extracts
# inline [text](destination) links outside fenced code blocks, skips absolute
# URLs (http/https/mailto) and pure-fragment links, strips #fragments, and
# checks that each remaining destination exists relative to the linking file
# (leading-slash destinations resolve from the repo root). Exits nonzero
# listing every missing target. No dependencies beyond find/grep/sed/awk.
#
# Why the fixture corpora are excluded: a file under conformance/profiles/*/
# corpus/ is a note in a *vault*, and a vault resolves a reference by its own
# committed rule — against the vault root rather than the linking file, with
# the .md extension optional, and with a bare name resolving by name alone.
# `docs/corpus/reference/README.md` linking `guides/README.md` is correct there
# and unresolvable here, so checking those files against this script's rule
# would report a conforming corpus as broken. They are not unchecked: the
# harness reads every corpus through the SDK, and the floors in
# conformance/harness/tests/floors.rs require every internal reference the docs
# corpus commits to resolve under the rule that actually governs it.
set -euo pipefail

cd "$(dirname "$0")/.."

fail=0
checked=0

while IFS= read -r file; do
  dir="$(dirname "$file")"
  # awk's own failure must not read as "this file has no links": without
  # this the whole file is silently skipped and the run still reports that
  # every link resolved. `grep` is the one command allowed to find nothing.
  if ! outside_fences="$(awk '/^[[:space:]]*(```|~~~)/ { fence = !fence; next } !fence' "$file")"; then
    echo "check-links: cannot read $file" >&2
    exit 1
  fi
  while IFS= read -r dest; do
    [ -n "$dest" ] || continue
    case "$dest" in
      http://* | https://* | mailto:* | '#'*) continue ;;
    esac
    dest="${dest%%#*}"    # strip fragment
    dest="${dest%% \"*}"  # strip optional "title"
    dest="${dest#<}"      # strip <...> wrapping
    dest="${dest%>}"
    [ -n "$dest" ] || continue
    case "$dest" in
      /*) resolved=".$dest" ;;
      *) resolved="$dir/$dest" ;;
    esac
    checked=$((checked + 1))
    if [ ! -e "$resolved" ]; then
      printf 'MISSING: %s -> %s\n' "$file" "$dest"
      fail=1
    fi
  done < <(printf '%s\n' "$outside_fences" \
    | { grep -o '\[[^]]*\]([^)]*)' || true; } \
    | sed 's/^.*(\(.*\))$/\1/')
done < <(find . \
  \( -path ./.git -o -path ./target -o -path ./dist -o -name node_modules \
     -o -path './conformance/profiles/*/corpus' \) -prune \
  -o -name '*.md' -print | sort)

if [ "$fail" -ne 0 ]; then
  echo "check-links: broken relative markdown links found (see MISSING lines above)." >&2
  exit 1
fi
# Zero links checked means the walk found nothing, not that everything
# resolved: a broken prune, a bad cd, or a changed layout would otherwise
# report success having verified nothing at all.
if [ "$checked" -eq 0 ]; then
  echo "check-links: no relative markdown links found; the scan matched nothing." >&2
  exit 1
fi
echo "check-links: all $checked relative markdown links resolve."
