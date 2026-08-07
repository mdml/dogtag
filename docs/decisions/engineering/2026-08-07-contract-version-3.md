# Contract version 3

- Status: accepted
- Date: 2026-08-07

## Context

[The M5 surfaces record](2026-08-07-m5-capture-and-the-write-transaction.md) needs two pieces of committed, vault-shared configuration that no existing contract version can carry: where captures land, and whether the catch-all's notes are born flagged for triage. The contract's per-version key sets are closed — an unrecognized table in a version-2 contract is a diagnostic, not an extension point — so these seats are format motion, and format motion is a version bump under the machinery [contract version 2](2026-08-03-contract-version-2.md) built: per-version key sets, per-version default tables, a ceiling-only widening of the supported range, and a scope fence stating what the version deliberately does not carry.

The birth-state concept itself is a product stance already on the record: types may declare that their notes are born needing triage, and the table of which types do so is configuration, not product vocabulary ([the inbox-workflow PDR](../product/inbox-workflow.md), [the lifecycle PDR](../product/lifecycle.md)).

## Decision

### The version-3 key set: two write seats, nothing else

Version 3 adds exactly what M5 consumes:

- **`[capture]`** — `directory`, the vault-relative directory captures land in. A mechanic with a config seat, so it stays a mechanic: the location carries no meaning, and the seat exists so two agents working one vault agree about it.
- **A birth-state declaration** on a type: whether notes of that type are created carrying the needs-triage flag. Declarable anywhere a type is declared; at M5 the only writer that reads it is `capture`, so the catch-all's declaration is the one that acts.

Both seats have entries in the version-3 default table: an undeclared `[capture]` yields the default directory, an undeclared birth state stamps nothing. A version-3 contract that declares neither behaves identically to a version-2 contract — and `capture` works at every supported version, reading the defaults where the seats cannot exist. The seats configure the verb; they do not enable it.

### The range widens to `1..=3`, ceiling only

The supported range moves in the one change that carries version 3's key set and default table, exactly as the version-2 bump did. The floor stays at 1: migration tooling is still unshipped, and [the diagnostics record](2026-07-31-diagnostics-and-compatibility.md) forbids raising the floor before it ships. A version-2 vault keeps loading, keeps validating, and gains `capture` with defaults. `supported`-but-not-current now names versions 1 and 2.

### The scope fence

Version 3 carries **no** speculative seats — the version-2 fence discipline, restated because the temptation is larger this time:

- **No write-policy vocabulary** beyond the existing `closed-write` capability. The human-only/ai-only/mixed/named-authors vocabulary [the vault-contract record](2026-07-31-vault-contract-and-installation-record.md) deferred stays deferred: M5's one operation creates notes in the catch-all and never modifies an existing note, so nothing at this milestone would read it.
- **No relationship cardinality, no `targets`, no value constraints** — still no consumer.
- **No actor or provenance seats.** Actor identity's home is the installation record and the invocation, never the contract ([the settings PDR](../product/settings.md)).

### Alternatives considered

- **Avoiding the bump** — capture directory in local config or a flag. Rejected: the landing spot stops being committed vault-shared state, and two agents on one vault disagree about where captures go; "local config may narrow, never define" is the settings stance.
- **Riding the write-policy vocabulary along** since a write milestone is here anyway. Rejected: no M5 code path reads it; the version-2 fence held precisely by refusing seats without consumers, and it was right.
- **A birth-state seat scoped to the catch-all only.** Rejected: the product stance is per-type; scoping the seat narrower than the stance just forces version 4 when triage verbs arrive.
- **Defaulting `[capture] directory` to the vault root.** Rejected: root-level captures interleave with organized notes in every listing; a named default directory keeps the unfiled pile visibly a pile.

## Consequences

- **Three live versions.** The per-version machinery earns its keep or shows its cracks; conformance inherits the duty to keep a version-2 corpus loading and capturing.
- **The default directory becomes de-facto vocabulary** — documentation and skills will name it — which is the cost of defaulting instead of requiring a declaration, accepted for never-lossy's sake.
- **The birth-state seat lands mostly dormant**: one reader, one type that matters. Its full use arrives with triage verbs at a later milestone, and a dormant seat is the price of not scoping it narrower than the stance.
- **Fixture contracts face a version choice** ([the M5 fixtures record](2026-08-07-m5-fixtures-and-conformance.md) makes it): whichever corpora stay at version 2 become the standing proof that the floor is real.
