#!/usr/bin/env bash
# The scripted smoke sequence over the conformance fixtures — release
# acceptance criterion 10, the M3 replacement for interactive manual testing.
#
# Every surface of the built binary runs against a throwaway copy of each
# profile corpus: the pristine subset (version, doctor, contract explain,
# check, list, search) on every profile that ships a corpus, and the full
# seeded sequence (list with content, show, search, find, refusals) on
# `starter`, the normative initialization profile. The script writes only
# under mktemp and leaves the tree untouched.

set -euo pipefail
cd "$(dirname "$0")/.."

steps=0
step() {
  steps=$((steps + 1))
  printf 'smoke  %-52s ok\n' "$1"
}

fail() {
  printf 'smoke  FAIL: %s\n' "$1" >&2
  exit 1
}

cargo build --locked --quiet --bin dogtag
dogtag="$PWD/target/debug/dogtag"

scratch=$(mktemp -d)
trap 'rm -rf "$scratch"' EXIT
export HOME="$scratch/home"
mkdir -p "$HOME"

json_ok() {
  python3 -m json.tool >/dev/null 2>&1
}

# --- every profile with a corpus: the pristine subset ---------------------
for profile in conformance/profiles/*/corpus; do
  [ -f "$profile/.dogtag/contract.toml" ] || continue
  name=$(basename "$(dirname "$profile")")
  vault="$HOME/vaults/$name"
  mkdir -p "$HOME/vaults"
  cp -R "$profile" "$vault"
  chmod -R go-w "$vault"
  chmod 700 "$vault"

  "$dogtag" version >/dev/null || fail "$name: version"
  step "$name: version answers"

  (cd "$vault" && "$dogtag" doctor >/dev/null) || fail "$name: doctor"
  (cd "$vault" && "$dogtag" doctor --format json | json_ok) || fail "$name: doctor json"
  step "$name: doctor is clean in both formats"

  (cd "$vault" && "$dogtag" contract explain >/dev/null) || fail "$name: explain"
  (cd "$vault" && "$dogtag" contract explain --format json | json_ok) || fail "$name: explain json"
  step "$name: contract explain renders in both formats"

  (cd "$vault" && "$dogtag" check --strict >/dev/null) || fail "$name: check --strict"
  (cd "$vault" && "$dogtag" check --format json | json_ok) || fail "$name: check json"
  step "$name: check is green under strict"

  (cd "$vault" && "$dogtag" list >/dev/null) || fail "$name: list"
  (cd "$vault" && "$dogtag" list --format json | json_ok) || fail "$name: list json"
  step "$name: list enumerates in both formats"

  # An invented word no corpus carries: an empty result is a result, exit 0.
  (cd "$vault" && "$dogtag" search smoke-pristine-sweep >/dev/null) || fail "$name: search"
  (cd "$vault" && "$dogtag" search smoke-pristine-sweep --format json | json_ok) \
    || fail "$name: search json"
  step "$name: search answers in both formats"
done

# --- starter, seeded: content flows through list and show -----------------
vault="$HOME/vaults/starter"
mkdir -p "$vault/people"
printf -- '---\ntype: person\nfull_name: Smoke Tester\nstatus: active\n---\n# Smoke Tester\n\nA seeded smoke note.\n' \
  >"$vault/people/smoke.md"
printf -- '# A loose thought\n' >"$vault/inbox.md"

(cd "$vault" && "$dogtag" check --strict >/dev/null) || fail "seeded corpus: check"
step "starter: seeded corpus stays green under strict"

(cd "$vault" && "$dogtag" list | grep -q "people/smoke.md") || fail "list names the note"
(cd "$vault" && "$dogtag" list --type person | grep -q "people/smoke.md") || fail "type filter"
(cd "$vault" && "$dogtag" list --type person | grep -q "inbox.md") && fail "type filter leaks"
step "starter: list filters compose"

(cd "$vault" && "$dogtag" show people/smoke | grep -q "Smoke Tester") || fail "show text"
(cd "$vault" && "$dogtag" show smoke --format json | json_ok) || fail "show json"
step "starter: show renders a note by path and by name"

(cd "$vault" && "$dogtag" search seeded | grep -q "people/smoke.md") || fail "search body term"
(cd "$vault" && "$dogtag" search seeded --format json | json_ok) || fail "search json"
(cd "$vault" && "$dogtag" search seeded --type person | grep -q "people/smoke.md") \
  || fail "search type filter"
(cd "$vault" && "$dogtag" search thought --type person | grep -q "inbox.md") \
  && fail "search type filter leaks"
step "starter: search finds the seeded note and filters compose"

(cd "$vault" && "$dogtag" search '"seeded smoke"' | grep -q "people/smoke.md") \
  || fail "phrase in order"
(cd "$vault" && "$dogtag" search '"smoke seeded"' | grep -q "people/smoke.md") \
  && fail "phrase out of order matched"
(cd "$vault" && "$dogtag" search 'seed*' | grep -q "people/smoke.md") || fail "prefix wildcard"
step "starter: the phrase and prefix forms match as written"

[ -z "$(cd "$vault" && "$dogtag" search seeded --limit 0)" ] || fail "limit caps the hits"
step "starter: search caps its hits at the limit"

(cd "$vault" && "$dogtag" find smoke | grep -q "people/smoke.md") || fail "find by name"
(cd "$vault" && "$dogtag" find SMOKE | grep -q "people/smoke.md") || fail "find any case"
(cd "$vault" && "$dogtag" find smoke --format json | json_ok) || fail "find json"
step "starter: find resolves the note by name, any case"

printf -- '# A doubled name\n' >"$vault/smoke.md"
if (cd "$vault" && "$dogtag" find smoke >/dev/null 2>&1); then
  fail "a doubled name must refuse as ambiguous"
fi
(cd "$vault" && "$dogtag" find smoke 2>&1 || true) | grep -q "people/smoke.md" \
  || fail "the ambiguity lists its candidates"
(cd "$vault" && "$dogtag" find smoke --type person | grep -q "people/smoke.md") \
  || fail "the type filter narrows the doubled name"
(cd "$vault" && "$dogtag" find people/smoke | grep -q "people/smoke.md") \
  || fail "the path picks one bearer"
step "starter: find refuses a doubled name and the filter or path picks one"

# --- refusals keep their shape --------------------------------------------
if (cd "$vault" && "$dogtag" show nowhere >/dev/null 2>&1); then
  fail "a missing reference must exit nonzero"
fi
step "starter: a missing reference refuses"

if (cd "$vault" && "$dogtag" search '"never closed' >/dev/null 2>&1); then
  fail "an unbalanced quote must exit nonzero"
fi
(cd "$vault" && "$dogtag" search '"never closed' 2>&1 || true) | grep -q "search.invalid-query" \
  || fail "the query fault carries its identifier"
step "starter: an unreadable query refuses as a query fault"

broken="$HOME/vaults/broken"
mkdir -p "$broken/.dogtag"
chmod 700 "$broken"
printf 'contract_version = 3\n' >"$broken/.dogtag/contract.toml"
if (cd "$broken" && "$dogtag" check >/dev/null 2>&1); then
  fail "an unresolved contract must refuse"
fi
(cd "$broken" && "$dogtag" check 2>&1 || true) | grep -q "dogtag doctor" \
  || fail "the refusal points at doctor"
step "broken: an unresolved contract refuses toward doctor"

printf 'smoke pass — %d steps\n' "$steps"
