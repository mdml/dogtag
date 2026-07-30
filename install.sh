#!/bin/sh
# install.sh — install a released dogtag binary.
#
#   curl -fsSL https://dogtag.dev/install.sh | sh
#
# Environment overrides:
#   DOGTAG_VERSION        install this version (e.g. 0.1.0-beta.0 or v0.1.0-beta.0)
#                         instead of the newest release
#   DOGTAG_INSTALL_DIR    install directory (default: $XDG_BIN_HOME, else ~/.local/bin)
#   DOGTAG_DOWNLOAD_BASE  fetch <base>/<asset> directly instead of GitHub releases
#                         (enables local rehearsal via file:// and dogtag.dev fronting)
#   DOGTAG_NO_VERIFY=1    skip sha256 verification (not recommended)
#
# No sudo, no /usr/local: installs a single static binary into a user directory.
#
# Everything below is function definitions until the `main "$@"` on the last
# line, so a truncated `curl | sh` download executes nothing.
set -u

REPO="mdml/dogtag"

err() {
  printf 'install.sh: error: %s\n' "$*" >&2
  exit 1
}

note() {
  printf 'install.sh: %s\n' "$*"
}

# fetch <url> <dest>  (dest "-" writes to stdout)
# curl gets TLS hardening flags only for https:// URLs, so file:// (local
# rehearsal) and plain-http bases keep working.
fetch() {
  if command -v curl >/dev/null 2>&1; then
    case "$1" in
      https://*) curl --proto '=https' --tlsv1.2 -fsSL -o "$2" "$1" ;;
      *) curl -fsSL -o "$2" "$1" ;;
    esac
  elif command -v wget >/dev/null 2>&1; then
    wget -q -O "$2" "$1"
  else
    err "neither curl nor wget is available; install one and re-run"
  fi
}

main() {
  # --- detect platform -> release target triple (Linux always gets musl assets) ---
  platform="$(uname -sm)"
  case "$platform" in
    "Darwin x86_64") target="x86_64-apple-darwin" ;;
    "Darwin arm64") target="aarch64-apple-darwin" ;;
    "Linux x86_64") target="x86_64-unknown-linux-musl" ;;
    "Linux aarch64") target="aarch64-unknown-linux-musl" ;;
    *) err "unsupported platform: $platform (supported: Darwin x86_64/arm64, Linux x86_64/aarch64)" ;;
  esac
  asset="dogtag-$target.tar.gz"

  command -v curl >/dev/null 2>&1 || command -v wget >/dev/null 2>&1 \
    || err "neither curl nor wget is available; install one and re-run"

  # --- resolve download base URL ---
  if [ -n "${DOGTAG_DOWNLOAD_BASE:-}" ]; then
    base="${DOGTAG_DOWNLOAD_BASE%/}"
    note "using download base $base"
  else
    version="${DOGTAG_VERSION:-}"
    if [ -z "$version" ]; then
      # GitHub's /releases/latest resolves only the newest NON-prerelease
      # release; every beta release is a prerelease, so ask the API for the
      # newest release instead (one unauthenticated call, no jq needed).
      releases_url="https://api.github.com/repos/$REPO/releases?per_page=1"
      response="$(fetch "$releases_url" -)" || response=""
      [ -n "$response" ] \
        || err "could not query GitHub for the latest release ($releases_url); set DOGTAG_VERSION to install a specific version"
      version="$(printf '%s\n' "$response" \
        | grep '"tag_name"' \
        | head -n 1 \
        | sed 's/.*"tag_name"[^"]*"\([^"]*\)".*/\1/')"
      [ -n "$version" ] \
        || err "could not parse a tag_name from the GitHub response (no releases yet?); set DOGTAG_VERSION to install a specific version"
    fi
    version="${version#v}"
    base="https://github.com/$REPO/releases/download/v$version"
    note "installing dogtag $version for $target"
  fi

  # --- download ---
  tmp="$(mktemp -d)" || err "mktemp failed"
  tmp_bin=""
  trap 'rm -rf "$tmp" "$tmp_bin"' EXIT

  fetch "$base/$asset" "$tmp/$asset" || err "download failed: $base/$asset"

  # --- verify checksum ---
  if [ "${DOGTAG_NO_VERIFY:-}" = "1" ]; then
    note "warning: skipping sha256 verification (DOGTAG_NO_VERIFY=1)"
  else
    fetch "$base/$asset.sha256" "$tmp/$asset.sha256" \
      || err "download failed: $base/$asset.sha256"
    if command -v sha256sum >/dev/null 2>&1; then
      (cd "$tmp" && sha256sum -c "$asset.sha256" >/dev/null 2>&1) \
        || err "sha256 verification failed for $asset"
    elif command -v shasum >/dev/null 2>&1; then
      (cd "$tmp" && shasum -a 256 -c "$asset.sha256" >/dev/null 2>&1) \
        || err "sha256 verification failed for $asset"
    else
      err "no sha256sum or shasum tool found; set DOGTAG_NO_VERIFY=1 to skip verification (not recommended)"
    fi
    note "sha256 verified"
  fi

  # --- install ---
  INSTALL_DIR="${DOGTAG_INSTALL_DIR:-${XDG_BIN_HOME:-$HOME/.local/bin}}"
  mkdir -p "$INSTALL_DIR" || err "could not create $INSTALL_DIR"
  tar -xzf "$tmp/$asset" -C "$tmp" || err "could not extract $asset"
  [ -f "$tmp/dogtag" ] || err "archive $asset did not contain a dogtag binary"
  # Stage next to the destination, then rename over it: the swap is atomic,
  # and a running `dogtag` is replaced instead of written into (no ETXTBSY).
  tmp_bin="$INSTALL_DIR/.dogtag.tmp.$$"
  cp "$tmp/dogtag" "$tmp_bin" || err "could not write $tmp_bin"
  chmod +x "$tmp_bin"
  mv -f "$tmp_bin" "$INSTALL_DIR/dogtag" || err "could not install $INSTALL_DIR/dogtag"
  tmp_bin=""
  note "installed $INSTALL_DIR/dogtag"

  case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) note "warning: $INSTALL_DIR is not on your PATH; add it, e.g.: export PATH=\"$INSTALL_DIR:\$PATH\"" ;;
  esac

  "$INSTALL_DIR/dogtag" version
}

main "$@"
