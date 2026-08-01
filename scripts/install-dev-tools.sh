#!/usr/bin/env bash
# install-dev-tools.sh — install the pinned developer toolchain.
#
# Every version comes from tools.toml, the single source of truth that
# scripts/check-tool-pins.py holds the workflows to. Installing anything here
# by hand at a different version will fail that check, which is the point.
#
# Installs: the Rust toolchains (pinned stable, MSRV floor, coverage
# nightly), cargo-llvm-cov, cargo-deny, cargo-semver-checks, cargo-cyclonedx,
# zizmor, git-cliff, osv-scanner, and the CodeScene CLI. Already-current tools
# are skipped, so re-running is cheap.
#
# The CodeScene CLI additionally needs CS_ACCESS_TOKEN in your environment to
# do anything; get a PAT from https://codescene.io/users/me/pat.
set -euo pipefail

cd "$(dirname "$0")/.."

pin() {
  python3 - "$1" "$2" <<'PY'
import sys, tomllib, pathlib
tools = tomllib.loads(pathlib.Path("tools.toml").read_text())
print(tools[sys.argv[1]][sys.argv[2]])
PY
}

have() {
  command -v "$1" >/dev/null 2>&1
}

# `cargo install` is idempotent but slow, so skip when the version matches.
# Any trailing arguments are the words the tool needs before `--version`
# answers: a cargo subcommand that refuses to run standalone — cargo-cyclonedx
# is one — reports its version only as `cargo-cyclonedx cyclonedx --version`,
# and probing it wrongly would silently reinstall on every run.
cargo_tool() {
  local name="$1" version="$2"
  shift 2
  if have "$name" && "$name" "$@" --version 2>/dev/null | grep -qF "$version"; then
    echo "install-dev-tools: $name $version already installed"
    return
  fi
  echo "install-dev-tools: installing $name $version"
  cargo install "$name" --version "$version" --locked
}

echo "== Rust toolchains =="
for key in toolchain msrv coverage_nightly; do
  channel="$(pin rust "$key")"
  case "$channel" in
    nightly-*) rustup toolchain install "$channel" --profile minimal --component llvm-tools-preview ;;
    *) rustup toolchain install "$channel" --profile minimal --component clippy --component rustfmt ;;
  esac
done

echo "== cargo tools =="
cargo_tool cargo-llvm-cov "$(pin cargo-llvm-cov version)"
cargo_tool cargo-deny "$(pin cargo-deny version)"
cargo_tool cargo-semver-checks "$(pin cargo-semver-checks version)"
cargo_tool cargo-cyclonedx "$(pin cargo-cyclonedx version)" cyclonedx
cargo_tool zizmor "$(pin zizmor version)"
cargo_tool git-cliff "$(pin git-cliff version)"

echo "== lefthook =="
lefthook_version="$(pin lefthook version)"
if have lefthook && lefthook version 2>/dev/null | grep -qF "$lefthook_version"; then
  echo "install-dev-tools: lefthook $lefthook_version already installed"
else
  echo "install-dev-tools: install lefthook $lefthook_version from"
  echo "  https://github.com/evilmartians/lefthook/releases/tag/v$lefthook_version"
  echo "  (verify against that release's lefthook_checksums.txt), then run \`just hooks\`"
fi

echo "== osv-scanner =="
osv_version="$(pin osv-scanner version)"
if have osv-scanner && osv-scanner --version 2>/dev/null | grep -qF "$osv_version"; then
  echo "install-dev-tools: osv-scanner $osv_version already installed"
else
  echo "install-dev-tools: install osv-scanner $osv_version from"
  echo "  https://github.com/google/osv-scanner/releases/tag/v$osv_version"
  echo "  (packaged builds vary by platform; verify the published SHA256SUMS)"
fi

echo "== CodeScene CLI =="
cs_version="$(pin codescene-cli version)"
if have cs && cs version 2>/dev/null | grep -qF "$cs_version"; then
  echo "install-dev-tools: cs $cs_version already installed"
else
  build_sha="$(pin codescene-cli build_sha)"
  expected="$(pin codescene-cli sha256)"
  # Only architectures whose checksum this repository has recorded. Upstream
  # publishes none, so an unrecorded architecture means installing an
  # unverified binary — refused rather than warned about, because a warning
  # in a setup script is read once and never again.
  case "$(uname -sm)" in
    "Linux x86_64") arch="linux-amd64" ;;
    *)
      echo "install-dev-tools: no recorded CodeScene sha256 for $(uname -sm)." >&2
      echo "  Download from https://codescene.io/docs/cli/, verify it yourself," >&2
      echo "  and add the architecture and its sha256 to tools.toml." >&2
      exit 1
      ;;
  esac
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  url="https://downloads.codescene.io/enterprise/cli/cs-$arch-$build_sha.zip"
  echo "install-dev-tools: installing cs $cs_version"
  curl -fsSL --proto '=https' --tlsv1.2 -o "$tmp/cs.zip" "$url"
  echo "$expected  $tmp/cs.zip" | sha256sum -c -
  unzip -q "$tmp/cs.zip" -d "$tmp"
  install -m 0755 "$tmp/cs" "${CARGO_HOME:-$HOME/.cargo}/bin/cs"
fi

echo
echo "install-dev-tools: done. \`just gate\` runs everything CI enforces locally."
