# Dogtag task runner. `just` alone lists the recipes.

# List available recipes
default:
    @just --list

# Format check + clippy (warnings as errors) + full test suite — the CI gate
check:
    cargo fmt --all --check
    cargo clippy --all-targets --workspace -- -D warnings
    cargo test --workspace

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

# Release-build the CLI for this machine's release target (Linux ships musl) and package it into dist/ (same scripts/package.sh as the release workflow)
dist:
    #!/usr/bin/env bash
    set -euo pipefail
    triple="$(rustc -vV | sed -n 's/^host: //p')"
    case "$triple" in
        *-unknown-linux-gnu) triple="${triple%-gnu}-musl" ;;
    esac
    rustup target add "$triple" >/dev/null
    cargo build --release --locked --target "$triple" -p dogtag-cli
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
