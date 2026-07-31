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

- **In-vault paths are vault-relative.** The installation record is reported as `$XDG_CONFIG_HOME/dogtag/installation.toml` rather than expanded, so no diagnostic emits the account name. Structured output always uses forward slashes.
- **Spans carry 1-based line, 1-based column measured in Unicode scalar values, and a byte offset**, with an optional end position for real spans.
- **Every diagnostic may carry related evidence**: a list of `{ location, message }` pairs.

Vault-relative paths are forced by determinism rather than chosen for ergonomics. Absolute paths in structured output would make a conformance golden machine-specific, so a fixture's expected output would differ between a working copy and CI, and there would be no golden file to write at all. The rule does double duty as a privacy control, which is worth saying so that nobody relaxes it later believing only goldens are at stake. Related evidence is likewise forced: *"this contract declares two catch-all types"* is unusable without pointing at both.

**Diagnostic messages may quote a corpus's own vocabulary** — type names, property names, lifecycle values — because a diagnostic that will not name what is wrong is not worth emitting. That deserves stating plainly next to the [fixture privacy gate](2026-07-31-m2-fixtures-and-the-privacy-gate.md), which treats exactly that vocabulary as material not to publish: the two are consistent only because one governs a committed artifact and the other governs transient output. The practical consequence lands during the cutover's seven-day parallel run, when output gets pasted into issues and agent transcripts: **diagnostic output from a private vault is private.** Where a location alone identifies the problem, a message should prefer the location to the name.

### Ordering, rendering, and exit codes

- **Deterministic total order**: by path, then line, then column, then byte offset, then identifier. Diagnostics with no location sort first, ordered by identifier; vault-relative paths sort before the installation record, so the two path kinds never interleave ambiguously. Never discovery order, which varies with the filesystem.
- **Human rendering** is `severity[identifier]: message`, a location line, related-evidence lines, and an optional `help:` line. Colour only when the stream is a terminal and `NO_COLOR` is unset.
- **Structured output is a single JSON document** carrying its own schema version, on a separate clock from `contract_version`.
- **Exit codes are `0`, `1`, and `2`**: clean, error-severity diagnostics reported, and usage error respectively. Warnings exit `0` unless `--strict` is given, which promotes them for exit purposes only.
- **Severity is the sole determinant of `0` versus `1`.** `2` is reserved for argument-parsing failures that produce no diagnostic at all. So an unregistered `--vault work` — which is an `installation.*` diagnostic — exits `1`, not `2`, even though it arrives as a bad argument.
- Any other exit code means an unmodelled internal failure and must be treated as failure. A Rust panic exits `101`, so a caller matching on `{0, 1, 2}` needs a default arm; claiming three codes without saying this would leave the one case this record predicts unhandled.

Three modelled codes rather than four is a deliberate forcing function. **Every foreseeable failure is a diagnostic with an identifier** — an unreadable contract, a permission denial, malformed TOML — rather than a bare nonzero exit. A distinct internal-failure code would become the home for every error nobody wanted to model, and the point of an envelope is that there is nowhere else to put things.

`--strict` exists because M2's own cutover is an unattended scheduled check whose only automatic signal is the exit code, and because a nested vault resolves a *different corpus* at warning severity. Without `--strict` that check reports healthy while inspecting the wrong vault. Deferring the flag would have meant deferring it past the one workflow that asked for it.

### Compatibility

**The SDK declares a contiguous supported range of `contract_version` values**, `1..=1` at M2, and classifies a contract's version against it:

| Found | Classification | Behavior |
| --- | --- | --- |
| below the floor | `below-supported-floor` | refuse: `compat.contract-below-supported-floor` names migration and the version-pinning recourse |
| in range, not the maximum | `supported` | load fully, plus `compat.newer-format-available` at `info` |
| the maximum | `current` | load fully, no diagnostic |
| above the range | `too-new` | refuse: `compat.contract-too-new` |

**The floor does not rise during the beta.** It may rise only in a release *after* migration tooling ships, and never in the same release that introduces the version it excludes. Without that policy the design contradicts the promise it was built to satisfy: a user on an excluded version is told to pin an older build — that is, *not to upgrade* — which negates both halves of "upgrade to a fix without manual vault repair" simultaneously, and fails the ship test's requirement that an older beta upgrade to the final candidate. Pinning is also bounded rather than indefinite, because [the supply-chain policy](2026-07-30-supply-chain-and-vulnerability-policy.md) fixes security response as forward-only: a pinned install forfeits later fixes.

The consequence of a floor that cannot rise is that the supported range grows monotonically and the SDK carries every historical version's parse rules and default tables. That cost is real and is accepted; there is no shedding mechanism that does not break the promise.

**`dogtag migrate` is scheduled at no milestone.** `architecture.md` commits to it as the schema-change escape hatch and the settings PDR lists it as a CLI surface, but no rung of the ladder delivers it. That is recorded here as an open question, in the same terms as `init`, rather than resolved inside an M2 packet.

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

- **Identifiers are now a public API surface with no deprecation mechanism**, and the first implementation freezes the whole M2 set. Rejecting a separate registry document was right — a second home for a list the enum already holds would only drift — but it leaves no review step for a permanent namespace, so **the identifier enum is called out for review in its own right when it lands**, not carried along with the code that raises each one.
- **The JSON output is a report containing the envelope, not the envelope alone**, and its field names and initial schema version are fixed by the implementation rather than by this record. That is the largest thing left underspecified here: `doctor`'s section names and its representation of *not evaluated* are what the cutover's seven-day triage will diff, so they need to be settled in the first implementation commit rather than accreted.
- **The `ext.` prefix is asserted but unexercised**, since no consumer has written a linter yet. The constructor's rejection is testable; the ergonomics are not, until someone builds one.
- **Every failure must be modelled as a diagnostic.** That is the intended pressure, and it means an unmodelled failure surfaces as a panic rather than as a tidy exit code — which is worse in the moment and better over time.
- **The compatibility machinery is almost entirely theory at M2**, with one supported version. What ships is the classification, the diagnostics, and the tests; the first real bump is when it is proven. Two of the four classifications are reachable from a real fixture — `current`, and `below-supported-floor` via `contract_version = 0` — and `too-new` via `2`. Only `supported`-but-not-current is unreachable at M2, which is why the conformance scenario asserts the three that are and leaves the fourth to the milestone whose range has two versions in it.
- **`classify` is public API purely for testability.** That is a real cost — a function on the public surface whose injectable parameter exists for the test suite — accepted because the alternative is either an untestable gate or a blocked coverage ratchet.
- **The JSON schema version is a third version to maintain**, alongside the crate version and `contract_version`. Three clocks is one more than anyone wants; the alternative was coupling output stability to format stability, which is worse.
