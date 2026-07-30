#!/usr/bin/env bash
# check-links.sh — verify that relative markdown links resolve to existing files.
#
# Scans every *.md file in the repo (excluding .git/, target/, dist/,
# node_modules/), extracts inline [text](destination) links outside fenced
# code blocks, skips absolute URLs (http/https/mailto) and pure-fragment
# links, strips #fragments, and checks that each remaining destination exists
# relative to the linking file (leading-slash destinations resolve from the
# repo root). Exits nonzero listing every missing target. No dependencies
# beyond find/grep/sed/awk.
set -euo pipefail

cd "$(dirname "$0")/.."

fail=0
checked=0

while IFS= read -r file; do
  dir="$(dirname "$file")"
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
  done < <(awk '/^[[:space:]]*(```|~~~)/ { fence = !fence; next } !fence' "$file" \
    | { grep -o '\[[^]]*\]([^)]*)' || true; } \
    | sed 's/^.*(\(.*\))$/\1/')
done < <(find . \
  \( -path ./.git -o -path ./target -o -path ./dist -o -name node_modules \) -prune \
  -o -name '*.md' -print | sort)

if [ "$fail" -ne 0 ]; then
  echo "check-links: broken relative markdown links found (see MISSING lines above)." >&2
  exit 1
fi
echo "check-links: all $checked relative markdown links resolve."
