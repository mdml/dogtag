#!/usr/bin/env bash
# codescene-hook.sh — the git-hook wrapper around the Code Health delta.
#
#   codescene-hook.sh staged      pre-commit: delta over the staged changes
#   codescene-hook.sh branch      pre-push:   delta of this branch vs its base
#
# This exists for one reason: Code Health is enforced by the maintainer's
# `just gate`, not by a required check, and an external contributor has no
# CodeScene account. A hook that hard-failed without a token would make one a
# requirement for contributing; a hook that silently passed would be worse
# still. So without a token it prints a conspicuous NOT MEASURED banner and
# exits 0 — loudly unmeasured, never a false pass. `just gate`, which the
# maintainer runs before merging, stays fail-closed.
#
# The distinction matters and is deliberate: scripts/codescene-gate.sh
# refuses to run without a token. Only this wrapper is allowed to decline.
set -euo pipefail

cd "$(dirname "$0")/.."

mode="${1:-}"

unmeasured() {
  # stderr, and shouted, because a hook's normal output is skimmed. The
  # words "pass", "ok" and "clean" appear nowhere.
  cat >&2 <<'NOTICE'
========================================================================
  CODE HEALTH NOT MEASURED — CS_ACCESS_TOKEN is not set.
  Nothing was checked. This is not a pass.
  Maintainers: export a PAT from https://codescene.io/users/me/pat
  Contributors: no account needed — the maintainer runs `just gate`
  with the token before merging, and that run is the gate.
========================================================================
NOTICE
}

# The ref this branch is measured against. `origin/main` is what the
# maintainer's `just gate` and the merge target both mean; `main` covers a
# clone with no fetched remote. A base that resolves to nothing is reported
# rather than silently replaced, because a delta against the wrong base is
# indistinguishable from a clean one.
resolve_base() {
  local ref
  for ref in origin/main main; do
    if git rev-parse --verify --quiet "$ref^{commit}" >/dev/null; then
      printf '%s\n' "$ref"
      return 0
    fi
  done
  return 1
}

case "$mode" in
  staged)
    [ -n "${CS_ACCESS_TOKEN:-}" ] || { unmeasured; exit 0; }
    exec ./scripts/codescene-gate.sh --staged
    ;;
  branch)
    [ -n "${CS_ACCESS_TOKEN:-}" ] || { unmeasured; exit 0; }
    if ! base="$(resolve_base)"; then
      echo "codescene-hook: neither origin/main nor main resolves; cannot delta." >&2
      exit 1
    fi
    exec ./scripts/codescene-gate.sh --branch "$base"
    ;;
  *)
    echo "codescene-hook: usage: codescene-hook.sh staged|branch" >&2
    exit 2
    ;;
esac
