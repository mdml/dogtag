# Dogtag task runner. `just` alone lists the recipes.
#
# `just check` is the offline, deterministic gate to run before handing off
# work. `just gate` adds everything CI enforces that can run locally, which
# means network and, for Code Health, a CodeScene token. CI is authoritative.

# List available recipes
default:
    @just --list

# Format check, clippy, tests, docs, and the offline policy checks — the gate to run before handing off
check:
    cargo fmt --all --check
    cargo clippy --all-targets --workspace --locked -- -D warnings
    cargo test --workspace --locked
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
    python3 scripts/check-tool-pins.py
    python3 scripts/check_security_exceptions.py
    cargo run --quiet --locked -p commit-lint -- --range main..HEAD

# Everything CI enforces that can run locally (network required; slower than check)
gate: check links coverage msrv deny osv zizmor codescene
    @echo "gate: every locally runnable CI check passed."

# Format all Rust code
fmt:
    cargo fmt --all

# Run the full test suite
test:
    cargo test --workspace

# Run the conformance harness and print the scenario x profile matrix
conformance:
    cargo test -p dogtag-conformance -- --nocapture

# Debug build of the whole workspace
build:
    cargo build --workspace

# Measure coverage and enforce the thresholds and ratchet baseline
coverage:
    ./scripts/coverage-gate.sh

# Build and test against the declared MSRV floor
msrv:
    cargo +1.85.0 build --workspace --locked
    cargo +1.85.0 test --workspace --locked

# Rust advisories, licenses, bans, and sources
deny:
    cargo deny --workspace --locked check

# Cross-ecosystem vulnerability scan of every lockfile
osv:
    osv-scanner scan source -r .

# Lint every workflow for supply-chain and permission mistakes
zizmor:
    zizmor --persona regular --min-severity low --no-online-audits .github/workflows

# Code Health of every supported file — the floor (network + CS_ACCESS_TOKEN)
codescene:
    ./scripts/codescene-gate.sh

# Code Health of specific paths, e.g. `just codescene-files src/lib.rs`
codescene-files *FILES:
    ./scripts/codescene-gate.sh --files {{FILES}}

# Code Health of staged changes — the pre-commit check
codescene-staged:
    ./scripts/codescene-gate.sh --staged

# Code Health of this branch against a base (default main)
codescene-branch BASE="main":
    ./scripts/codescene-gate.sh --branch {{BASE}}

# API-compatibility check against the last release tag (advisory until the first stable tag)
semver BASELINE="v0.1.0-beta.0":
    cargo semver-checks --package dogtag --baseline-rev {{BASELINE}} --release-type patch

# Validate commit messages as Conventional Commits (default: this branch vs main)
commits RANGE="main..HEAD":
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

# Release-build the CLI for this machine's release target (Linux ships musl) and package it into dist/ (same scripts/package.sh as the release workflow)
dist:
    #!/usr/bin/env bash
    set -euo pipefail
    triple="$(rustc -vV | sed -n 's/^host: //p')"
    case "$triple" in
        *-unknown-linux-gnu) triple="${triple%-gnu}-musl" ;;
    esac
    rustup target add "$triple" >/dev/null
    # Matches the release workflow exactly, cargo-auditable wrapper included.
    cargo auditable build --release --locked --target "$triple" -p dogtag-cli
    ./scripts/package.sh "$triple"

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

# Check that internal Markdown links resolve (same script as the CI links job)
links:
    ./scripts/check-links.sh
