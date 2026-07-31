# Conformance harness shape

- Status: accepted
- Date: 2026-07-30

## Context

The beta contract commits to a no-waiver conformance rule: every scenario runs against every fixture profile, with no profile-specific escape hatch. Rules like that erode when they live in policy — someone under deadline adds `skip_profiles = ["dense"]` "temporarily" and the temporary becomes load-bearing. The harness structure itself has to make the waiver inexpressible. Separately, at M1 the eleven golden scenarios are enumerated but nothing is executable, and the fixture corpora cannot honestly exist yet: the committed vault-contract format they must be written in is an M2 decision.

## Decision

- **A dedicated harness crate**, `conformance/harness` (package `dogtag-conformance`, `publish = false`). At M1 it does not depend on the `dogtag` crate — there is nothing to call; when scenarios become executable it will consume only the SDK's public API, like every other consumer.
- **Scenarios are data**: one TOML file per scenario under `conformance/scenarios/`, with exactly the fields `id`, `title`, `milestone`, `status`, `contract`. Deserialization uses `#[serde(deny_unknown_fields)]`. There is deliberately no field that could name a profile — a file that adds `profiles = [...]`, `skip`, `waive`, or `only` fails to parse. This is the structural no-waiver mechanism, and the harness's own tests assert the rejection.
- **The report is a cross product**: scenarios × all profiles, including profiles whose corpora are not yet built. At M1 every cell is Pending; the matrix prints on every `just conformance` run, so the outstanding debt is visible rather than filtered out.
- **Profile specs land now, corpora later.** All four fixture profiles from [beta.md](../../beta.md) exist at M1 as prose specs plus metadata (`PROFILE.md` + `PROFILE.toml` with a `corpus = "scheduled"` marker and target milestone). The corpora themselves are deferred to the milestone that defines the committed vault-contract format — this records the reading of "built for the first release" as *the first release that can load a contract*, not the empty M1 slice: a corpus authored before the format exists would be written against a guess and rewritten at M2.
- **Graduation is binary.** A scenario flips `status = "pending"` to `"executable"`; the harness then requires it to execute against every profile. There is no per-profile partial graduation, by construction.

### Alternatives considered

- **A `profiles` allowlist field plus a written policy against using it.** Rejected: this is precisely the erosion path the rule exists to prevent. A schema field that cannot exist beats a review comment that must be remembered.
- **Building the corpora now against a guessed contract format.** Rejected: it would force the M2 format decision implicitly and early (the worst way to make it), and guarantee churn across four corpora when the real format lands.
- **Excluding scheduled profiles from the report until their corpora exist.** Rejected: a filtered matrix hides exactly the debt the cross product is designed to keep in view, and creates a soft waiver (a profile that is perpetually "not built yet" is a profile being skipped).
- **Encoding scenarios as Rust tests instead of data files.** Rejected: code can trivially special-case a profile; data behind a closed schema cannot. It also keeps scenario authorship reviewable by non-Rust readers.

## Consequences

- Introducing any waiver now requires changing the scenario schema in the harness source — a named, reviewable, greppable act — and updating the tests that assert unknown fields are rejected. The friction is the feature.
- Every harness run prints an all-Pending matrix at M1. That is honest but noisy; the pressure it creates to turn cells green is intended.
- `deny_unknown_fields` makes the scenario schema rigid: legitimate future fields require a deliberate schema change too. Accepted — schema evolution should be exactly that deliberate.
- Because the harness will consume only the public SDK API, conformance doubles as a permanent test that the public API is sufficient — any private hook it needs is an architecture bug.
