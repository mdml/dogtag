# The diagnostic envelope and the compatibility direction

- Status: accepted
- Date: 2026-07-31

## Context

M2 ships structured diagnostics and compatibility checks ([beta.md](../../beta.md#milestones)), and both are long-lived commitments rather than milestone-local ones. [beta.md](../../beta.md#required-properties) binds Rust and TypeScript to *shared fixtures, diagnostic identifiers, and compatibility rules*, so an identifier chosen here is a name every binding and every consumer's error handling depends on. The compatibility direction is constrained from two sides at once: the beta promises *"upgrade to a fix without manual vault repair"* while listing *self-mutating upgrades* among the Deferred, and [the contract format](2026-07-31-vault-contract-and-installation-record.md) makes unknown keys fatal.

One further constraint shapes the implementation rather than the contract. `coverage-baseline.toml` requires 100% line coverage across the semantic kernel, and coverage-exclusion attributes are not permitted, so a compatibility branch that no real vault can reach at M2 would block the gate rather than sit harmlessly unused.

## Decision

### Identifiers

- **Kernel identifiers are `<area>.<slug>`**, lowercase kebab-case, drawn from an exhaustive enum in the SDK. The enum is the single source; a test asserts every variant's identifier is unique, well-formed, and matches its variant. There is no separate registry document to drift from it.
- **Consumer identifiers must begin `ext.`** — `ext.<namespace>.<slug>` — enforced by the constructor, which rejects any identifier outside that prefix.
- **Identifiers are permanent.** Renaming one is a breaking change to every consumer; a superseded diagnostic keeps its identifier or the change is announced as breaking.

The `ext.` prefix rather than a reserved-area list follows from the [architecture obligation](../../architecture.md) that the public API is sufficient to author corpus-specific validation. Once a consumer can construct diagnostics, it must not be able to mint one indistinguishable from the kernel's — and a reserved-area list would have to predict today every area the kernel will want years from now, so a consumer using `corpus` would break when the kernel claimed it. A mandatory prefix gives the SDK the entire remaining namespace permanently and costs consumers four characters.

M2's kernel areas are `discovery`, `contract`, `installation`, and `compat`.

### Severity

Three levels: **`error`** — the operation cannot produce a trustworthy result; **`warning`** — the result stands but something needs attention; **`info`** — context the caller did not ask for and should see.

`info` earns its place from two M2 diagnostics that recur on every run in a legitimate setup: a root resolved through a symlink, and a supported-but-not-current contract version. As warnings, those would fire on every invocation forever and train their reader to ignore warnings, which is how a warning level dies. A fourth `hint` level was rejected: no M2 diagnostic wants to be one rather than a `help:` line on an existing diagnostic.

### Locations and evidence

- **In-vault paths are vault-relative**; out-of-vault paths — the installation record — are absolute. Structured output always uses forward slashes.
- **Spans carry 1-based line, 1-based column measured in Unicode scalar values, and a byte offset**, with an optional end position for real spans.
- **Every diagnostic may carry related evidence**: a list of `{ location, message }` pairs.

Vault-relative paths are forced by determinism rather than chosen for ergonomics. Absolute paths in structured output would make a conformance golden machine-specific, so a fixture's expected output would differ between a working copy and CI, and there would be no golden file to write at all. Related evidence is likewise forced: *"this contract declares two catch-all types"* is unusable without pointing at both.

### Ordering, rendering, and exit codes

- **Deterministic total order**: by path, then line, then column, then byte offset, then identifier. Diagnostics with no location sort first, ordered by identifier. Never discovery order, which varies with the filesystem.
- **Human rendering** is `severity[identifier]: message`, a location line, related-evidence lines, and an optional `help:` line. Colour only when the stream is a terminal and `NO_COLOR` is unset.
- **Structured output is a single JSON document** carrying its own schema version, on a separate clock from `contract_version`.
- **Exit codes are `0`, `1`, and `2` only**: clean, error-severity diagnostics reported, and usage error respectively. Warnings exit `0`.

Three codes rather than four is a deliberate forcing function. **Every foreseeable failure is a diagnostic with an identifier** — an unreadable contract, a permission denial, malformed TOML — rather than a bare nonzero exit. A distinct internal-failure code would become the home for every error nobody wanted to model, and the point of an envelope is that there is nowhere else to put things.

### Compatibility

**The SDK declares a contiguous supported range of `contract_version` values**, `1..=1` at M2, and classifies a contract's version against it:

| Found | Classification | Behavior |
| --- | --- | --- |
| below the floor | `below-supported-floor` | refuse; the diagnostic names migration and the version-pinning recourse |
| in range, not the maximum | `supported` | load fully, plus an `info` diagnostic that a newer format exists |
| the maximum | `current` | load fully |
| above the range | `too-new` | refuse with `compat.contract-too-new` |

A range rather than a single supported version is forced by M0 rather than chosen: with self-mutating upgrades deferred and manual vault repair ruled out, a newer tool **must** keep loading an older contract. The other direction is forced by the format: with unknown keys fatal, an older tool physically cannot best-effort a newer contract, so refusal is the only honest answer.

**`dogtag doctor` never refuses to run.** Refusal is per-operation; diagnosis is always available. When the contract version is out of range, `doctor` still reports discovery, the installation record, the registry, and the version classification itself, and marks every contract-dependent section explicitly *not evaluated* with the reason. Stopping at the version check would hide whether the right vault was even found, exactly when the user is most confused about what is wrong.

**The version gate is a pure classification function over an injectable range.** `classify(found, supported)` is public and exhaustively unit-tested across the whole space; the SDK's entry points pass the real constant. At M2 the supported range is a single version, so the below-floor and supported-but-not-current branches are unreachable through any real vault — and with coverage exclusions banned and a 100% kernel floor, unreachable branches would block `just gate`. Injecting the range makes every branch reachable in tests without fabricating impossible vaults, and it makes the compatibility contract exist in code rather than only in this document.

**No migration tooling ships at M2.** The below-floor diagnostic names that migration arrives later and points at pinning an older version through the installer's `DOGTAG_VERSION` as the interim recourse.

### Alternatives considered

- **A reserved-area list instead of an `ext.` prefix.** Rejected as reasoning above: it must predict the kernel's future areas.
- **Numeric codes.** Compact and stable under rewording. Rejected: they convey nothing at the point of failure, require a registry lookup to interpret, and are ungreppable in a codebase — the opposite of what an agent reading output needs.
- **Two severity levels.** Rejected: the two recurring benign M2 diagnostics would become permanent warnings.
- **A fourth `hint` level.** Rejected as unearned.
- **Absolute paths everywhere.** Copy-pasteable into an editor. Rejected: conformance goldens become machine-specific.
- **Line numbers without columns.** Simplest to produce and assert. Rejected: no caret rendering, and no way to point at one key inside a line, which is most of what a contract diagnostic wants to do.
- **A separate Markdown registry of diagnostic identifiers.** Rejected: a second home for a list the enum already holds, with nothing keeping them equal.
- **A distinct exit code for internal failure.** Rejected as reasoning above.
- **Warnings exiting nonzero by default.** Rejected: benign warnings would fail the daily health check until the vault was rearranged, and `--strict` is deferred.
- **Supporting exactly one contract version at a time.** Smallest surface. Rejected: it breaks every existing vault at each format bump, with no migration command in the beta to repair them.
- **A supported range with in-memory upgrade on read.** Attractive — one internal model regardless of on-disk version. Rejected for now: it needs an upgrade path per version, and it muddies provenance, since a value produced by an upgrade came from neither the file nor a format default and would need a fourth source. Worth revisiting at the first real bump, when the cost is concrete.
- **Hardcoding the supported range and accepting unreachable branches.** Rejected: the coverage floor blocks it and exclusions are banned.
- **Omitting the unreachable branches until a second version exists.** Fully covered, minimal code. Rejected: the compatibility contract this record freezes would not exist in code, and the M2 scenarios asserting it could not run.

## Consequences

- **Identifiers are now a public API surface with no deprecation mechanism.** The M2 set is small, which is the moment to get the areas right; a mistake in an area name is permanent or breaking.
- **The `ext.` prefix is asserted but unexercised**, since no consumer has written a linter yet. The constructor's rejection is testable; the ergonomics are not, until someone builds one.
- **Every failure must be modelled as a diagnostic.** That is the intended pressure, and it means an unmodelled failure surfaces as a panic rather than as a tidy exit code — which is worse in the moment and better over time.
- **The compatibility machinery is almost entirely theory at M2**, with one supported version. What ships is the classification, the diagnostics, and the tests; the first real bump is when it is proven.
- **`classify` is public API purely for testability.** That is a real cost — a function on the public surface whose injectable parameter exists for the test suite — accepted because the alternative is either an untestable gate or a blocked coverage ratchet.
- **The JSON schema version is a third version to maintain**, alongside the crate version and `contract_version`. Three clocks is one more than anyone wants; the alternative was coupling output stability to format stability, which is worse.
