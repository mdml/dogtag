# Conformance

Every scenario runs against every fixture profile. **There are no waivers.** A scenario expressible against only one profile fails the harness and is triaged as either an incomplete configuration model or a personal convention mistaken for an invariant. This rule — not prose review — is what keeps one author's assumptions out of the kernel; see [beta.md](../docs/beta.md) (required properties) and [architecture.md](../docs/architecture.md).

## Layout

```
conformance/
  harness/       # crate `dogtag-conformance` (publish = false): loads, validates, cross-products
  scenarios/     # one TOML per golden scenario — the M0 set of eleven plus the M2 packet's seven
  profiles/      # one directory per fixture profile: PROFILE.toml (parsed) + PROFILE.md (spec)
    dense/  starter/  docs/  records/
```

## Scenarios

A scenario is a contract first and a test second:

```toml
id = "missing-required-property-diagnostic"   # kebab-case, equals the filename stem
title = "A missing required property yields a stable diagnostic"
milestone = "M2"                              # when it becomes executable (M2 | M3 so far)
status = "pending"                            # pending | executable — all pending at M1
contract = """
Given/when/then prose in profile-agnostic terms.
"""
```

Contract prose binds behavior to **declared capabilities and declared axes** — "the declared catch-all type", "the declared lifecycle axis" — never to any corpus's vocabulary. No scenario may mention a type name, a lifecycle word, or a dialect assumption; vocabulary lives in profiles, and only declarations reach the core.

Which milestone a scenario carries follows what it operates on: a scenario that opens and diagnoses a vault and its committed contract is M2, and one that validates the corpus's notes is M3. Four scenarios written at M0 moved from M2 to M3 on that test when the M2 packet closed, and seven M2 scenarios were added alongside them — a metadata correction, not a waiver, since every scenario still runs against every profile.

### Non-conforming inputs are derived, never authored

A scenario that needs a broken contract — no catch-all type, two of them, an unsupported version, a missing lifecycle declaration — does not get a checked-in broken fixture. The harness copies **each profile's own contract** into a temporary directory and transforms it there.

That is not a convenience. A hand-authored broken contract has to be written in *some* corpus's vocabulary, which makes it a profile-specific fixture wearing a shared-sounding name — the exact shape this suite exists to catch. Deriving the input runs one assertion against every profile's vocabulary by construction, and no broken contract is checked in anywhere.

### The no-waiver rule is structural

The scenario schema has **no field that could name a profile**. Deserialization uses serde's `deny_unknown_fields`, so an added `profiles = ["dense"]`, `skip`, `waive`, or `only` key fails parsing instead of creating an exemption — the place where a personal invariant would hide does not exist in the format. The profile schema is locked the same way in the other direction: a profile has no field with which to exempt itself from scenarios. Harness tests assert both rejections (`waiver_shaped_fields_fail_scenario_parsing`, `waiver_shaped_fields_fail_profile_parsing`); if either test ever fails, someone widened the schema, and that is the exact change this suite exists to forbid. Rejected alternatives — declared-and-reviewed exemptions, a two-tier shared/profile-specific split — both make single-profile scenarios routine, and an exemption list is exactly where a personal invariant hides.

## Profiles

Four profiles, each standing for one persona and together spreading across every axis the configuration seam claims to absorb — type taxonomy, capability assignment, property requirements, predicate vocabulary, lifecycle encoding, name resolution, and dialect. The roster is exact: the harness fails if a profile is missing or a fifth appears unspecified. See each profile's `PROFILE.md` for its full specification.

| Profile | Stands for | Corpus built |
| --- | --- | --- |
| `dense` | the PKM enthusiast with an established corpus | M2 |
| `starter` | a fresh install | M2 |
| `docs` | the dev team | M4 (lexical retrieval) |
| `records` | the decision maker | pre-E1 (before assisted adoption) |

### Why no corpus is built at M1

The committed vault-contract format is an M2 decision. A fixture corpus written before the format exists would freeze a guess and then defend it. So the profile *specifications* land now — early enough that no scenario can be written with one profile's axes quietly assumed away — and each corpus lands at the milestone that defines what it must be written in (`dense`, `starter`) or the milestone whose hypothesis it stresses (`docs`, `records`).

### What `built` means

`corpus = "built"` means **the fixture vault exists**: a vault root and its committed contract. It does not promise notes. No M2 scenario reads a note, and notes for `dense` and `starter` land at M3 alongside the document model that defines how they are written — authoring them earlier would freeze a guess about a format M3 decides.

The schema deliberately gained no third status for this. Any state between `scheduled` and `built` is somewhere a profile can sit indefinitely while the matrix reports something other than a skip, which is what a waiver looks like from the inside. Each `PROFILE.md` states exactly what its corpus holds at each stage instead.

## The harness

`cargo test -p dogtag-conformance` (or `just conformance`, which adds `--nocapture` so the matrix prints):

1. every scenario file parses under the strict schema, with unique kebab-case ids equal to their filename stems;
2. waiver-shaped fields fail parsing, on scenarios and on profiles;
3. the report is the full **scenarios × profiles cross product** — every pair present exactly once, |report| = |scenarios| × |profiles|, and at M1 every pair's outcome is *pending* (scenario pending and corpus not built);
4. the profile roster is exactly the four above;
5. the pending matrix prints, one row per scenario, one column per profile, every cell filled.

The harness crate depends on `serde` and `toml` only. It does **not** depend on the `dogtag` SDK at M1 — every scenario is pending and there is nothing to call. When scenarios become executable, the harness consumes only the SDK's public API, same as any other consumer.

## Graduating a scenario

A scenario graduates by flipping `status = "pending"` to `"executable"` at its milestone, alongside the execution wiring in the harness. Graduation is all-or-nothing: an executable scenario runs against **every** profile, and the harness refuses to produce a report at all — rather than quietly marking pairs pending — if a runnable pair exists without an execution path. There is no partial graduation, no per-profile rollout, and no mechanism to exclude a profile that fails.
