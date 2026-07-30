#!/usr/bin/env bash
# codescene-gate.sh — enforce CodeScene Code Health on this repository.
#
# The invariant is that every supported file scores 10.0, always. Verifying
# that from scratch means scoring every file — one network round trip each —
# which is too slow to run on every commit. So the invariant is held
# inductively instead: the floor was established once, and `cs delta` gates
# every change after it.
#
#   codescene-gate.sh                 score every supported file (the floor)
#   codescene-gate.sh --files P...    score just the named paths
#   codescene-gate.sh --staged        delta over staged changes (pre-commit)
#   codescene-gate.sh --branch [BASE] delta of this branch vs BASE (default main)
#
# Delta is sufficient for induction because it fails on both halves of the
# rule: it reports a file whose score dropped, and it reports a *new* file
# born below 10.0 (verified against cs 1.0.36 — a new file at 8.67 exits
# nonzero). What delta cannot see is a file no change touched, so the full
# scan still runs periodically: rules evolve between CLI versions, and a
# scored floor is only as current as its last measurement.
#
# Scoring uses stock CodeScene rules. There is deliberately no rules file,
# threshold override, or allowlist anywhere in this repository — the only way
# to pass is to refactor. A null score means the file holds no scorable code
# (its language is analyzed but the file has no functions), which passes as
# out of scope.
#
# Requires the `cs` CLI, jq, and CS_ACCESS_TOKEN in the environment (CI takes
# it from a secret; locally export a PAT from https://codescene.io/users/me/pat).
# Network-dependent: the CLI validates the token against codescene.io.
set -euo pipefail

cd "$(dirname "$0")/.."

# A cheap prefilter for the repository sweep, so unscorable files (Markdown,
# TOML, shell) cost no network round trip. It is deliberately a superset of
# what any one CLI version accepts; the authority on whether a file is
# scorable is the CLI's own answer, handled in score_files. A file the CLI
# does not analyze has no Code Health score at all, so it is outside the gate
# by nature rather than by waiver.
SUPPORTED='\.(c|h|cc|cpp|cxx|ipp|hh|hpp|hxx|cs|java|groovy|js|mjs|cjs|sj|ts|mts|cts|jsx|tsx|vue|m|mm|scala|py|pyi|swift|go|dart|vb|php|rs|rb|kt|kts|pl|pm|erl|hrl|ex|exs|clj|cljc|cljs|ps1|psm1|psd1|tcl|cls|trigger|tgr|brs|bs|efx|emx)$'

require() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "codescene-gate: $1 is required but not on PATH (see \`just install-dev-tools\`)." >&2
    exit 1
  }
}

require cs
require jq
[ -n "${CS_ACCESS_TOKEN:-}" ] || {
  echo "codescene-gate: CS_ACCESS_TOKEN is not set; the CodeScene CLI cannot authenticate." >&2
  exit 1
}
export CS_DISABLE_VERSION_CHECK=1

# Every source file under version control, tracked or newly added, whose
# language CodeScene scores. Ignored files stay ignored.
supported_files() {
  git ls-files --cached --others --exclude-standard | grep -E "$SUPPORTED" | sort
}

# Score each named file, printing one evidence line per file. Fails unless
# every score is exactly 10.0 (or null).
score_files() {
  local fail=0 scored=0 skipped=0 file review score
  for file in "$@"; do
    if ! review="$(cs review --output-format json "$file" 2>&1)"; then
      # The CLI rejects file types it does not analyze. That is not a
      # finding: an unanalyzed language has no score to meet.
      if printf '%s' "$review" | grep -q 'Unsupported file-type'; then
        printf 'skip     %s  (language not analyzed)\n' "$file"
        skipped=$((skipped + 1))
        continue
      fi
      printf 'ERROR    %s  (cs review failed)\n' "$file"
      fail=1
      continue
    fi
    score="$(printf '%s' "$review" | jq '.score')"
    scored=$((scored + 1))
    if [ "$score" = "null" ]; then
      printf 'n/a      %s  (no scorable code)\n' "$file"
    elif [ "$(printf '%s' "$review" | jq '.score == 10')" = "true" ]; then
      printf '10.0     %s\n' "$file"
    else
      printf '%-8s %s  (must be 10.0)\n' "$score" "$file"
      fail=1
    fi
  done

  if [ "$fail" -ne 0 ]; then
    echo "codescene-gate: files below Code Health 10.0. Refactor them; never adjust the rules." >&2
    return 1
  fi
  printf 'codescene-gate: %d file(s) at 10.0 (or holding no scorable code)' "$scored"
  [ "$skipped" -eq 0 ] && printf '.\n' || printf ', %d skipped as unanalyzed.\n' "$skipped"
}

mode="${1:-}"
case "$mode" in
  "")
    mapfile -t files < <(supported_files)
    [ "${#files[@]}" -gt 0 ] || { echo "codescene-gate: no supported source files found." >&2; exit 1; }
    score_files "${files[@]}"
    ;;
  --files)
    shift
    [ "$#" -gt 0 ] || { echo "codescene-gate: --files needs at least one path." >&2; exit 2; }
    score_files "$@"
    ;;
  --staged)
    # Pre-commit: refuses a commit that would degrade any file it touches.
    cs delta --staged --error-on-warnings
    echo "codescene-gate: staged changes introduce no Code Health findings."
    ;;
  --branch)
    # Whole-branch view, as a reviewer sees it. In CI the base ref must be
    # fetched first (actions/checkout with fetch-depth: 0).
    base="${2:-main}"
    cs delta "$base" --error-on-warnings
    echo "codescene-gate: no Code Health findings against $base."
    ;;
  -h | --help)
    sed -n '2,26p' "$0"
    ;;
  *)
    echo "codescene-gate: unknown mode \`$mode\` (try --help)." >&2
    exit 2
    ;;
esac
