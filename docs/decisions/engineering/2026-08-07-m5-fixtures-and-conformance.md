# Writes under conformance and the M5 scenario set

- Status: accepted
- Date: 2026-08-07

## Context

M5 introduces the first scenarios that mutate a corpus, under standing rules built for reading: every scenario runs against every fixture profile with no waivers ([conformance/README.md](../../../conformance/README.md)), corpus-specific situations are derived by transformation into a fresh copy ([the M4 fixtures record's amendments](2026-08-06-m4-fixtures-and-conformance.md#amendments)), and committed corpora never change during a run. No record has said how a write scenario fits that frame — and [contract version 3](2026-08-07-contract-version-3.md) puts the fixture contracts to a version choice.

## Decision

### Writes mutate the per-pair copy — that is the whole mechanism

Every corpus-backed case already runs against a temporary copy of the profile's corpus, created fresh per pair, that the run may write into. **That existing design is the write-scenario mechanism, now load-bearing and recorded as such**: a write scenario derives any situation it needs into the copy, applies the mutation to the copy, and asserts the post-state through the ordinary read surfaces — `list`, `show`, `check` — the same doors any consumer would use. No restore machinery, no fixture mutation, no new schema field, no third status. Committed corpora stay byte-identical through every run, which the checkout's cleanliness already proves.

Every write scenario carries the construction guarantees the graduation change established: no empty expected set can pass vacuously, and what a case asserts is its own planted work, never a committed coincidence.

Conformance copies are not git repositories, so the write scenarios exercise the **guest-mode** path — write without commit, result names the path — by construction. The one commit-behavior scenario constructs a repository inside its copy first (a derivation like any other planting) and asserts the pathspec-scoped commit-at-birth; git assertions appear only where the copy is a repository, file-state assertions everywhere.

### The fixture contracts split across the version range — deliberately

- **`starter` moves to version 3**, declaring `[capture]` and a birth state on its catch-all. It is the fresh-install profile and the normative `init` output; capture is its story, and the flag-at-birth path needs one committed corpus that declares it.
- **`dense` and `docs` stay at version 2.** Capture runs on them through the default table, which makes them the standing proof of two claims at once: the floor is real (a version-2 vault keeps working), and the version-3 seats configure rather than enable. `records` remains scheduled, unbuilt.

The migration of `starter` lands with the same receipts discipline as every contract motion: the supported range widens only in the change carrying the per-version key sets and defaults, and every new floor is harness-enforced in the change that raises it.

### The M5 scenario set

Universal — derived onto every built corpus, version differences carried by the default table:

- `capture-lands-unfiled` — a capture creates a note in the declared or default directory, bound to the catch-all by the absence of `type`, visible to `list`.
- `capture-body-is-verbatim` — the captured text round-trips byte-for-byte; frontmatter contains nothing beyond what the record allows.
- `capture-preview-writes-nothing` — `--preview` emits the plan; the copy is byte-identical afterward.
- `capture-collision-appends-suffix` — two captures colliding on a filename both land; neither is lost, neither overwritten.
- `capture-exit-is-the-transaction-verdict` — on a copy transformed to carry a pre-existing validation error, a successful capture exits `0` and the result carries the corpus diagnostics.
- `capture-without-actor-warns` — no installation record in scope: the write lands, provenance is unattributed, the result carries the warning.
- `capture-result-names-recovery` — the structured result names the created path (and, in the repository scenario below, the commit).
- `capture-commits-at-birth` — in a copy constructed as a repository: one pathspec-scoped commit containing exactly the created file, carrying the trailer pair.
- `capture-birth-state-stamps-the-flag` — where the contract declares a birth state for the catch-all (committed on `starter`; derived onto the others by transforming the copy's contract to version 3), the note is born flagged; where nothing is declared, it is not.
- `capture-repeat-is-deterministic` — the plan for identical input is identical minus the timestamped identity, and two runs produce two notes.

The version machinery's existing scenarios extend to the widened range in the same change that widens it — the `1..=3` counterpart of what the version-2 bump did — including `supported`-but-not-current now naming two versions.

The matrix keeps distinguishing ran from pending; `records` stays the only skip. Coverage floors move only under the standing adjudication discipline: proposed, marked, ratified — never silently.

### Alternatives considered

- **A restore/snapshot mechanism for write scenarios.** Rejected: the per-pair copy already is the isolation; machinery on top would imply the copies were ever shared.
- **A `mutates: true` schema field.** Rejected: the schema deliberately has no field that changes how a scenario is treated; a write scenario is just a scenario whose assertions follow an act.
- **Moving all three corpora to version 3.** Rejected: it would leave the floor's "a version-2 vault keeps working" claim with no committed witness, exactly when capture-through-defaults needs one.
- **Keeping `starter` at version 2 too.** Rejected: the birth-state path would exist only in derived form, and the normative init output would model none of the write milestone's configuration.
- **Asserting git state in every write scenario.** Rejected: the copies are not repositories and guest mode is a first-class product stance, not a degraded case; one constructed-repository scenario covers the commit path.

## Consequences

- **The per-pair copy is now a correctness dependency, not a hygiene measure** — a future optimization that shares copies between pairs would break write scenarios silently; this record is the tripwire.
- **The version split makes every conformance run a mixed-version run**, which is the point, and also means a version-3-only bug can hide from two of three corpora; `starter`'s coverage carries that weight.
- **The constructed-repository scenario brings git into the harness's dependency surface** for one case — accepted; asserting commit-at-birth nowhere would be the larger hole.
- **Ten new cases will dilute the coverage ratchet again**; the floors move by the established marked-proposal path if they move at all.
