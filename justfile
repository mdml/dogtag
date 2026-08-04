# Dogtag task runner. `just` alone lists the recipes.
#
# Four commands carry the day-to-day work, in a strict ladder:
#
#   just fast          while implementing   seconds, offline
#   just check         before handing off   seconds, offline, deterministic
#   just gate          before a pull request  needs network + pinned tools + CS_ACCESS_TOKEN
#   just gate-verbose  the same run, with every gate's full output
#
# Each is a superset of the one above it, and every gate is declared once in
# scripts/gate.py as the exact command CI runs — `just gates` prints that
# table. CI and the repository rulesets remain authoritative; nothing here
# is a merge signal.

# List available recipes
default:
    @just --list

# Print the gate table: every gate, the suites it belongs to, and the command it runs
gates:
    @python3 scripts/gate.py --list

# Inner loop while implementing — format, clippy, tests, commits, cheap policy checks (seconds, offline; NOT a merge gate)
fast:
    python3 scripts/gate.py fast

# The complete offline gate — `fast` plus docs, links, and every deterministic policy check. Run before handing work off
check:
    python3 scripts/gate.py check

# Everything CI enforces that can run locally — `check` plus coverage, MSRV, deny, OSV, zizmor, Code Health (network + pinned tools + CS_ACCESS_TOKEN)
gate:
    python3 scripts/gate.py gate

# `gate` with every gate's full output — identical checks, thresholds and exit codes; more evidence
gate-verbose:
    python3 scripts/gate.py gate --verbose

# Format all Rust code (writes; `just fast` checks it)
fmt:
    cargo fmt --all

# Run the full test suite
test:
    python3 scripts/gate.py tests

# The full test suite with every harness's output
test-verbose:
    python3 scripts/gate.py tests --verbose

# Run the conformance harness and print the scenario x profile matrix
conformance:
    cargo test -p dogtag-conformance --locked -- --nocapture

# Debug build of the whole workspace
build:
    cargo build --workspace --locked

# Measure coverage and enforce the thresholds and ratchet baseline (nightly toolchain; ~seconds)
coverage:
    python3 scripts/gate.py coverage

# The scripted smoke sequence over the fixtures — release criterion 10; run before every tag
smoke:
    bash scripts/smoke.sh

# Coverage with the full per-file evidence table
coverage-verbose:
    python3 scripts/gate.py coverage --verbose

# Build and test against the declared MSRV floor (needs the 1.85.0 toolchain)
msrv:
    python3 scripts/gate.py msrv-build msrv-test

# MSRV build and tests with full output
msrv-verbose:
    python3 scripts/gate.py msrv-build msrv-test --verbose

# Rust advisories, licenses, bans, and sources (network: fetches the RustSec DB)
deny:
    python3 scripts/gate.py deny

# cargo-deny with its full per-check output
deny-verbose:
    python3 scripts/gate.py deny --verbose

# Cross-ecosystem vulnerability scan of every lockfile (network; CI's pinned scanner is the verdict)
osv:
    python3 scripts/gate.py osv

# OSV scan with the full report
osv-verbose:
    python3 scripts/gate.py osv --verbose

# Lint every workflow for supply-chain and permission mistakes (offline)
zizmor:
    python3 scripts/gate.py zizmor

# zizmor with every finding and suppression listed
zizmor-verbose:
    python3 scripts/gate.py zizmor --verbose

# Check that internal Markdown links resolve (offline)
links:
    python3 scripts/gate.py links

# Link check, listing every link it resolved
links-verbose:
    python3 scripts/gate.py links --verbose

# Code Health of every supported file — the floor (network + CS_ACCESS_TOKEN; one round trip per file)
codescene:
    python3 scripts/gate.py codescene

# Code Health of every supported file, listing each file's score
codescene-verbose:
    python3 scripts/gate.py codescene --verbose

# Code Health of specific paths, e.g. `just codescene-files src/lib.rs` (delta; prints per-file scores)
codescene-files *FILES:
    ./scripts/codescene-gate.sh --files {{FILES}}

# Code Health of staged changes — the pre-commit check (delta; findings print in full)
codescene-staged:
    ./scripts/codescene-gate.sh --staged

# Code Health of this branch against a base (delta; findings print in full)
codescene-branch BASE="main":
    ./scripts/codescene-gate.sh --branch {{BASE}}

# API-compatibility check against the last release tag (advisory until the first stable tag)
semver BASELINE="v0.1.0-beta.0":
    cargo semver-checks --package dogtag --baseline-rev {{BASELINE}} --release-type patch

# Validate commit messages as Conventional Commits over an explicit range (the
# default matches what `just fast` resolves; CI validates the range a pull
# request introduces, which is not necessarily either of them)
commits RANGE="origin/main..HEAD":
    cargo run --quiet --locked -p commit-lint -- --range {{RANGE}}

# Install the git hooks defined in lefthook.yml
hooks:
    lefthook install

# Preview the release notes the next tag would publish
notes:
    git-cliff --config cliff.toml --unreleased --strip all

# Install every pinned developer tool from tools.toml
install-dev-tools:
    ./scripts/install-dev-tools.sh

# Release-build the CLI for this machine's release target (Linux ships musl), package it into dist/ and generate its SBOM (the same scripts the release workflow runs)
dist:
    #!/usr/bin/env bash
    set -euo pipefail
    triple="$(rustc -vV | sed -n 's/^host: //p')"
    case "$triple" in
        *-unknown-linux-gnu) triple="${triple%-gnu}-musl" ;;
    esac
    rustup target add "$triple" >/dev/null
    # Byte-for-byte the release workflow's invocation, cargo-auditable wrapper
    # and argument order included; only the target is substituted.
    cargo auditable build --release --locked -p dogtag-cli --target "$triple"
    ./scripts/package.sh "$triple"
    ./scripts/sbom.sh "$triple"

# Rehearse install.sh end-to-end against the locally packaged dist/
install-local: dist
    #!/usr/bin/env bash
    set -euo pipefail
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT
    DOGTAG_DOWNLOAD_BASE="file://$PWD/dist" \
        DOGTAG_INSTALL_DIR="$tmpdir" \
        sh install.sh
    "$tmpdir/dogtag" version
