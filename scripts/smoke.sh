#!/usr/bin/env bash
# The scripted smoke sequence over the conformance fixtures — release
# acceptance criterion 10, the M3 replacement for interactive manual testing.
#
# Every surface of the built binary runs against a throwaway copy of each
# profile corpus: the pristine subset (version, doctor, contract explain,
# check, list, search) on every profile that ships a corpus, then the one
# mutation (preview, capture, read-back) on each of them, and the full seeded
# sequence (list with content, show, search, find, refusals) on `starter`, the
# normative initialization profile. The script writes only under mktemp and
# leaves the tree untouched.
#
# The capture steps run LAST inside the per-profile loop, after every read
# assertion, so a capture cannot perturb what the reads are about. Two ambient
# facts make them what they are: `HOME` is a fresh mktemp directory, so there is
# no installation record and every capture here is unattributed by design; and
# the copies are not repositories, so every capture exercises guest mode.
#
# Expect informational output on standard error from two of the three profiles.
# `dense` and `docs` declare contract version 2 while the current version is 3,
# deliberately — they are the standing witnesses that a below-ceiling vault
# keeps loading and gains `capture` through the default table — so every run
# against them prints `compat.newer-format-available`. It is information, it is
# true, and hiding it here would be the wrong instinct: a green run that says
# what it noticed is worth more than a quiet one.

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

# The path a capture's structured result says it created.
created_path() {
  python3 -c 'import json,sys; print(json.load(sys.stdin)["created"] or "")'
}

# Every file under a vault, as one comparable listing.
files_under() {
  find "$1" -type f | sort
}

# An invented phrase no fixture corpus carries, so finding it back proves the
# capture rather than a coincidence.
THOUGHT="vellichor sweep of the harbour ledgers"

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

  # --- the one mutation ---------------------------------------------------
  before=$(files_under "$vault")
  (cd "$vault" && "$dogtag" capture --preview "smoke preview for $name" >/dev/null 2>&1) \
    || fail "$name: capture --preview"
  [ "$before" = "$(files_under "$vault")" ] || fail "$name: the preview wrote something"
  step "$name: capture --preview writes nothing"

  captured=$(cd "$vault" && "$dogtag" capture "$THOUGHT for $name" --format json 2>/dev/null) \
    || fail "$name: capture"
  printf '%s' "$captured" | json_ok || fail "$name: capture json"
  landed=$(printf '%s' "$captured" | created_path)
  case "$landed" in
    */*) ;;
    *) fail "$name: the capture named no created path" ;;
  esac
  step "$name: capture creates one note and names it"

  # The read-back, through the ordinary doors: the note is listed, it shows the
  # thought that was captured, and the corpus is still green under strict —
  # which is the structural exemption made visible, since a capture binds to a
  # catch-all that can require nothing.
  (cd "$vault" && "$dogtag" list | grep -q "$landed") || fail "$name: list omits the capture"
  (cd "$vault" && "$dogtag" show "$landed" | grep -q "$THOUGHT") \
    || fail "$name: show omits the captured thought"
  (cd "$vault" && "$dogtag" check --strict >/dev/null) || fail "$name: check after capture"
  step "$name: the captured note reads back and leaves the corpus green"

  # No installation record is in scope, so provenance is unattributed — a
  # warning that never gates the act, which is why this still exits 0.
  (cd "$vault" && "$dogtag" capture "smoke unattributed for $name" 2>&1 >/dev/null) \
    | grep -q "write.actor-unattributed" || fail "$name: an unattributed capture must warn"
  step "$name: an unattributed capture warns and still lands"
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
printf 'contract_version = 4\n' >"$broken/.dogtag/contract.toml"
if (cd "$broken" && "$dogtag" check >/dev/null 2>&1); then
  fail "an unresolved contract must refuse"
fi
(cd "$broken" && "$dogtag" check 2>&1 || true) | grep -q "dogtag doctor" \
  || fail "the refusal points at doctor"
step "broken: an unresolved contract refuses toward doctor"

printf 'smoke pass — %d steps\n' "$steps"
