#!/usr/bin/env bash
# coverage-gate.sh — measure workspace coverage and enforce the ratchet.
#
# Runs cargo-llvm-cov on the pinned nightly toolchain (branch coverage needs
# nightly instrumentation) with a full JSON export — the per-file summaries
# feed the kernel 100%-line check — then hands the report and the committed
# baseline (coverage-baseline.toml) to scripts/coverage_check.py, which
# enforces the thresholds and prints the evidence table. Exits nonzero on
# any violated rule.
set -euo pipefail

cd "$(dirname "$0")/.."

toolchain="$(sed -n 's/^toolchain = "\(.*\)"$/\1/p' coverage-baseline.toml)"
if [ -z "$toolchain" ]; then
  echo "coverage-gate: no toolchain pin found in coverage-baseline.toml" >&2
  exit 1
fi

mkdir -p target/llvm-cov
cargo "+$toolchain" llvm-cov --workspace --locked --branch --json \
  --output-path target/llvm-cov/coverage.json

exec python3 scripts/coverage_check.py target/llvm-cov/coverage.json coverage-baseline.toml
