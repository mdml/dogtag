# Release pipeline and artifacts

- Status: accepted
- Date: 2026-07-30

## Context

M1 is "clean repository and empty release": the CLI does nothing useful beyond `dogtag version`, but the release path must be the real one — tag push to installable binary, end to end. That path has to be small enough to audit line by line, reproducible locally without CI, and safe by default (no accidental publishes, no unpinned third-party code running with write tokens). The decisions below were made against the current (2026-07-30) state of GitHub Actions runners, `gh`, and the Rust release-tooling ecosystem.

## Decision

### Target set and runners

Four release targets, all built natively (no cross-compilation):

| Target | Runner |
| --- | --- |
| `x86_64-unknown-linux-musl` | `ubuntu-latest` (+ musl-tools) |
| `aarch64-unknown-linux-musl` | `ubuntu-24.04-arm` (+ musl-tools) |
| `aarch64-apple-darwin` | `macos-15` |
| `x86_64-apple-darwin` | `macos-15-intel` |

Linux builds are **musl-static only** — no gnu/glibc artifacts. Precedent: ripgrep, just, and zoxide all ship musl binaries as their portable Linux artifacts. A static musl binary runs on any Linux distribution with no glibc version floor, which removes the single most common "works on the build machine, fails on the user's machine" failure for downloaded binaries, and it spares `install.sh` any libc detection. The known musl cost — slower default allocator under heavy multithreaded allocation — is irrelevant to a CLI that prints its version; if a future milestone becomes allocation-heavy, revisiting with jemalloc (ripgrep's approach) is recorded as the escape hatch. `macos-15` is pinned over `macos-latest` (currently macOS 26) for stability; `macos-15-intel` covers the legacy Intel fleet, accepting that GitHub sunsets Intel runners around August 2027 — fine for a beta-era commitment.

### Artifact naming: versionless

Artifacts are named `dogtag-<target-triple>.tar.gz` — **no version in the filename** (the uv/deno pattern). The version is scoped by the release tag that owns the asset, so the name never disagrees with the tag, URLs are constructible from the target alone, and `DOGTAG_DOWNLOAD_BASE` mirrors (local rehearsal, later dogtag.dev fronting) need no per-version renaming.

The caveat this surfaces: GitHub's `/releases/latest/download/<asset>` convenience URL resolves only the newest **non-prerelease** release, and during the beta every release is a prerelease — so "latest" cannot be a static URL. `install.sh` therefore resolves the newest release via one unauthenticated API call (`/releases?per_page=1`, `tag_name` parsed with grep/sed — no jq dependency) and downloads from `releases/download/v<version>/<asset>`. Once stable releases exist, `/latest` also works, but the API path stays as the uniform mechanism.

### Checksums

Each archive gets a `<asset>.sha256` sidecar in `sha256sum -c` format ("hex, two spaces, filename"), produced at packaging time next to the binary that was just built. The release job re-verifies every sidecar after the artifact download round-trip and concatenates them (sorted) into one aggregate `sha256.sum` on the release (uv's pattern): sidecars serve per-asset verification in `install.sh`; the aggregate serves humans and mirrors that want one file to check everything.

### Release creation: `gh` CLI, not a third-party action

The release job runs `gh release create "$TAG" --draft --prerelease --verify-tag --title "dogtag $TAG" --generate-notes` with the built-in `GITHUB_TOKEN`, uploading the archives, sidecars, and `sha256.sum`. `gh` is preinstalled on every hosted runner and maintained by GitHub, so this removes an entire third-party action (and its pinning/audit burden) from the one job that holds a `contents: write` token. `--verify-tag` refuses to mint a release for a tag that does not exist on the remote.

**Draft-first, always**: the job never publishes. Tag push produces a draft prerelease; a human inspects the assets and notes and clicks publish. Automation proves the path; publishing stays a human act.

A guard step fails the release job when the pushed tag does not equal `v` + the `[workspace.package]` version parsed from `Cargo.toml`, so a mis-tagged commit can never produce artifacts that disagree with what the binary reports.

### Supply-chain posture: SHA pins, dependabot, attestations, least privilege

- Every third-party action is pinned to a **full commit SHA** with a `# vX.Y.Z` comment (motivated concretely by the tj-actions compromise, CVE-2025-30066, where a moved tag injected malicious code into thousands of workflows). `dtolnay/rust-toolchain` is pinned the same way, with the toolchain passed explicitly (`toolchain: "1.97.1"`) because a SHA pin loses the `@rev`-derived default.
- `.github/dependabot.yml` (github-actions + cargo ecosystems, weekly) keeps the SHA pins from going stale — a pin without an update feed is just slow-motion rot.
- The release job runs `actions/attest-build-provenance` over the archives, producing signed SLSA provenance linking each asset to the exact workflow run and commit. Attestation covers the automated path only; there is deliberately no local signing story at M1.
- CI runs with workflow-level `permissions: contents: read`; the release workflow is `permissions: {}` at the top with job-level elevation (`contents: read` to build; `contents: write, id-token: write, attestations: write` only on the release job). All builds and tests use `--locked` against the committed `Cargo.lock`.

### `scripts/package.sh`: one packaging path for CI and local

Both `release.yml` and `just dist` call `scripts/package.sh <target-triple> [binary-path]`, which stages `dogtag` + `LICENSE` + `README.md`, tars them (with `--sort=name` where GNU tar is available; BSD tar falls back to a fixed explicit member list), and emits the `.sha256` sidecar. Because the local rehearsal and CI share the exact script, "it packaged locally" is evidence about the release path, not a parallel implementation of it. Full byte-for-byte reproducible archives (fixed mtimes, owners) are explicitly not an M1 goal.

### `install.sh` contract

POSIX sh (`#!/bin/sh`, `set -u`, shellcheck-clean), so it runs under dash and BSD sh alike. Behavior: `uname -sm` maps to exactly the four release triples (Linux always gets the musl asset — static linking makes libc detection unnecessary); newest-release resolution as above; download via `curl -fsSL` with `wget` fallback; sha256 verification by default (`sha256sum`, then `shasum -a 256`; hard failure if neither exists) with `DOGTAG_NO_VERIFY=1` as the explicit escape; installs to `$XDG_BIN_HOME`, else `~/.local/bin` — never sudo, never `/usr/local`; warns when the install dir is not on `PATH`; ends by running `dogtag version` as the proof-of-life. Environment contract: `DOGTAG_VERSION` (pin a version), `DOGTAG_INSTALL_DIR`, `DOGTAG_DOWNLOAD_BASE` (fetch `<base>/<asset>` directly — enables `file://` rehearsal against `dist/` and later dogtag.dev fronting; note `file://` requires curl, as wget does not speak it), `DOGTAG_NO_VERIFY`.

### Link integrity: own `scripts/check-links.sh`

Internal markdown link checking is a ~50-line bash script (find/grep/sed: extract relative links, strip fragments, resolve against the linking file, list missing targets) wired into CI and `just links`. Chosen over lychee/lychee-action: the need is "do relative links in this repo resolve", which does not justify another pinned third-party binary in CI. If external-URL checking is ever wanted, that is the moment to reconsider lychee.

## Alternatives considered

- **cargo-dist** — generates exactly this pipeline (matrix builds, installers, checksums) and would remove hand-rolled YAML. Rejected for M1: its 2025 was turbulent (axodotdev's maintenance stalled, the astral-sh fork spun up and has since been archived, upstream activity has only recently resumed), and adopting a release tool mid-recovery means betting the whole publish path on it. At four targets and one binary, the hand-rolled matrix is ~150 lines of auditable YAML with zero third-party release tooling; cargo-dist's value curve starts winning at many-targets/many-installers, which is not where this repo is. Revisit if the target set or installer set grows.
- **Versioned artifact names** (`dogtag-v0.1.0-beta.0-<triple>.tar.gz`) — self-describing files after download, but every consumer (install.sh, mirrors, docs) must then interpolate the version into the name, and the name can silently disagree with the tag that hosts it. The tag already scopes the version; the filename repeating it buys little and costs a template everywhere.
- **gnu (glibc) Linux builds** — marginally faster allocator and the "default" triple, but introduces a glibc version floor determined by the build runner and either a second Linux artifact pair or a libc-detection dance in install.sh. Static musl makes the entire problem not exist at M1 scale.
- **Third-party release actions** (softprops/action-gh-release and kin) — convenient upload ergonomics, but another SHA-pinned dependency running with `contents: write`, replacing a first-party preinstalled tool (`gh`) that does the same job.
- **lychee for link checking** — better markdown parsing and external-URL checking, at the cost of a pinned binary/action for a need a small script covers. Recorded above.
- **Publishing releases automatically** (no draft) — faster, but removes the one human checkpoint between "tag pushed" and "the world installs it". Draft-first is the whole point of "publishing is a human act".

## Consequences

- A tag push yields a complete draft prerelease with four attested archives, per-asset sidecars, and an aggregate `sha256.sum`; nothing reaches users without a human publish.
- `install.sh` works throughout the beta despite `/releases/latest` ignoring prereleases, at the cost of one unauthenticated API call (rate limits are generous for this shape; `DOGTAG_VERSION` bypasses the call entirely).
- The pipeline has exactly five pinned third-party action SHAs (checkout, upload/download-artifact, rust-toolchain, attest-build-provenance) and dependabot watches all of them; everything else is first-party runner tooling or scripts in this repo.
- Local `just dist` + `DOGTAG_DOWNLOAD_BASE` rehearsal exercises the same packaging and install code paths CI uses, so release bugs are findable without pushing tags.
- Costs accepted: hand-rolled YAML is ours to maintain (the cargo-dist trade), Intel macOS support inherits GitHub's ~Aug 2027 runner sunset, and archives are deterministic-ish rather than byte-reproducible.
