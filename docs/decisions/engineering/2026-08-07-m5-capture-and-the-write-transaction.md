# The M5 surfaces: `capture` and the write transaction

- Status: accepted
- Date: 2026-08-07

## Context

M5 ships safe mutation: the plan/validate/apply transaction with one operation, `capture` ([beta.md](../../beta.md#milestones)). The transaction's required contents are fixed — actor, provenance, intended file scope, preview, post-write validation, structured result, recovery ([beta.md](../../beta.md#required-properties), [architecture.md](../../architecture.md)) — but not their form. The product stances that bind the write path were decided long before the milestone: capture is never lossy, instant, and unfiled by default, with capture and filing as separate acts ([the inbox-workflow PDR](../product/inbox-workflow.md)); notes commit at birth with pathspec-scoped commits, and guest mode means the substrate may not own the commit path at all ([the git-integration PDR](../product/git-integration.md)); attribution derives from the version record, never hand-maintained frontmatter ([the authorship PDR](../product/authorship.md)); a writing surface never alters bytes it did not semantically touch ([the markdown-flavor PDR](../product/markdown-flavor.md)).

Two of those stances collide on their face: the inbox-workflow PDR gives capture "zero gates — no tags, no commit, lint-exempt by design," while the milestone ladder gives every M5 mutation "post-write validation." This record's first decision is the composition.

The surface boundary questions are inherited, not reopened: the SDK owns the semantic model and every rendering ([the M2 surfaces record](2026-07-31-m2-surfaces-and-the-sdk-boundary.md)); severity alone maps to exit codes ([the diagnostics record](2026-07-31-diagnostics-and-compatibility.md)); the family refusal rule and the one-JSON-document rule apply to every new surface ([the M3 surfaces record](2026-08-03-m3-surfaces-check-list-show.md)); every door reads through the one shared loading, traversal, and validation path ([the M4 surfaces record's amendments](2026-08-06-m4-surfaces-search-and-find.md#amendments)).

## Decision

### The exemption is structural, not a waiver

Capture's zero-gates guarantee and post-write validation compose without machinery, because the composition is already frozen: [the version-2 record's](2026-08-03-contract-version-2.md) `contract.catch-all-requires` rule forbids the catch-all from requiring any property, relationship, or tag namespace, and a capture binds to the catch-all. **A capture therefore cannot fail contract rules by construction — and symmetrically receives none of the vault's services: no template, no defaults, no lifecycle** (the catch-all is axis-less under the named-ordinary composition, per [the seam record's amendments](2026-07-31-lifecycle-declaration-and-the-seam.md#amendments)). You may ignore the vault's rules only by also forgoing what the vault provides; that trade is the product stance, and the catch-all's emptiness is its mechanism.

Post-write validation still runs — the shared read path over the result, uniform with every other door — and **reports**; it never rolls back or refuses a capture. Refusals exist only for non-lint non-negotiables: an unresolved contract (the family rule, identical at every door), an ownership violation, an unwritable target.

### What `capture` creates, and where

```
dogtag capture [<text> | --file <path> | -] [--vault <name-or-path>] [--format text|json] [--preview]
```

A capture creates one new note: frontmatter that **omits `type` entirely** — absence binds to the catch-all through the document model's own mechanism, so nothing is stamped that the model derives — plus the needs-triage flag if and only if the contract declares a birth state for the catch-all ([contract version 3](2026-08-07-contract-version-3.md) owns that seat), and the captured text as the body, byte-for-byte. Nothing else.

The note lands in the capture directory the contract declares (`[capture] directory`, a version-3 seat), or the default where no declaration applies — the mechanic the inbox-workflow PDR says carries no meaning gets a config seat precisely so it stays a mechanic. The filename derives from a timestamp plus a slug of the first line; a collision appends a suffix rather than refusing, because a name coincidence is not a reason to lose a thought. Identity remains the path, as everywhere.

Capture works at **every supported contract version**: the version-3 seats configure it, they do not enable it. A version-1 or version-2 vault captures into the default directory with no birth flag, which is exactly what a version-3 vault that declares neither does.

### The transaction: a value, one verb, no artifact

The plan is an SDK value, never an on-disk artifact. Its contents are the fixed list — actor, provenance, intended file scope, the diagnostics known pre-write, compatibility impact (null for a note-level write; the field exists because the transaction shape is shared, not because capture moves the contract) — and it is rendered inside the one JSON document the run emits. `--preview` emits the plan and writes nothing; the bare verb plans and applies in one act, which is what "capture is instant" requires. There is no plan file, no journal, no two-invocation handshake: a persisted plan is state that can go stale between invocations, machinery a one-note write does not need and a later milestone can add without breaking this shape.

A capture creates exactly one file, so a partial apply cannot exist at M5 — recorded as a property of the operation, not a guarantee of the machinery.

### Commit, recovery, and guest mode

When dogtag owns the commit path, apply writes the file and commits it **pathspec-scoped** — the commit takes exactly the one created file, never a concurrent writer's work — honoring commit-at-birth. That commit *is* the recovery path: the structured result names it, and reverting it is recovery. In guest mode the write lands uncommitted and the result names the created path as the recovery information — delete is recovery for a create. Recovery is therefore concrete in both modes without backup copies or an undo verb.

The commit carries the actor and provenance as a trailer pair (machine-parseable; exact keys fixed at implementation with the identifier review). **The trailer standard the authorship PDR deferred lands here**, at the first commit-writing verb, so no attributed-history gap accrues.

### Actor and provenance

The actor resolves from the installation record, overridable per invocation; the provenance kind is one of a closed set — `human`, `agent`, `automation` — seated the same way. The contract never carries actor identity ([the settings PDR](../product/settings.md)).

**A missing actor does not gate a capture.** The write lands, provenance records as unattributed, and the result carries a warning — the advance-warning role [the M2 vault-contract record](2026-07-31-vault-contract-and-installation-record.md) gave `doctor` ("telling you now that provenance will be unattributed later") resolves this way: unattributed-with-warning at the write, never a refusal between thought and capture.

### Exit semantics: the transaction is the verdict

Read verbs answer "what is true of the corpus"; write verbs answer "did my act land." A successful capture exits `0` even on a corpus whose validation carries pre-existing errors — those diagnostics ride the structured result for triage, exactly as the shared read path produces them, but they are not the transaction's verdict. A refused plan or a failed apply carries error diagnostics and exits `1` under the standing severity rule. This is a deliberate, recorded split from the every-door-agrees rule the read surfaces follow: a capture that exited `1` on a permanently red vault would train callers to ignore exit codes, which is the worse outcome for the one signal severity is supposed to own.

### The `write.*` diagnostic area

M5 mints the `write.*` area. Refusing an act is not a finding about the corpus, so the standing lattice rule's own test — a diagnostic that genuinely fits neither `note` nor `link` nor the M2 areas — is met for the first time. The area holds the transaction's refusals (an unwritable target, a target resolving outside the vault root, an ownership violation), enumerated exactly at implementation, where the identifier enum receives its own review as the diagnostics record requires. The post-write report reuses `note.*` and `link.*` unchanged; the missing-actor warning's seat is fixed in that same review.

### Writes go through the verified handle

Every write resolves its target through the opaque, verified `VaultRoot` handle — never re-resolved from a string — which is what [the discovery record](2026-07-31-vault-discovery-and-selection.md) kept the handle opaque *for*. A target resolving outside the vault root is refused. Paths derived from `XDG_CONFIG_HOME` are discovery inputs only, never write targets, which closes the write-target question [the vault-contract record](2026-07-31-vault-contract-and-installation-record.md) parked for this milestone. The installation record's writer stays explicitly unscheduled: M5 writes notes, nothing else.

### Alternatives considered

- **Validate-then-write with capture-exempt rules.** Rejected: a rules carve-out is the waiver pattern this repository refuses elsewhere; the structural composition needs no second rule set.
- **Refusing an unconfigured capture (no actor, no `[capture]` declaration).** Rejected: both would gate capture on configuration, against never-lossy; defaults and warnings carry the same information without the loss.
- **Two verbs, `plan` then `apply`, with a persisted plan document.** Rejected for M5: auditable but stateful; capture's product shape is one instant act, and the settings model's plan/apply precedent describes plan *availability*, not plan persistence.
- **Stamping `type` explicitly.** Rejected: hand-writes what the model derives, and renaming the catch-all would orphan every old capture.
- **Never committing at M5.** Rejected: walks back commit-at-birth and leaves the trailer standard with no home; guest mode already covers the commit-less case honestly.
- **Exit reflecting corpus health (every door agrees, no exceptions).** Rejected as above; the split is recorded rather than smuggled.
- **Stretching `note.*` to cover act-refusals.** Rejected: the exact bend the lattice rule exists to prevent.
- **An undo verb or backup copies as recovery.** Rejected: the commit (or the named path) already is the recovery; an undo verb is a second mutation the scope contract does not admit.

## Consequences

- **The one-mutation scope is load-bearing.** Everything above is sized to a single-file create; the first multi-file or in-place mutation (triage verbs, edits) will need plan scope, collision, and atomicity decisions this record deliberately does not make.
- **The write-verbs/read-verbs exit split is new surface** for callers to learn, and the help text and record must carry it; the structured result is the one place both stories are always complete.
- **Unattributed captures can accrue** on an unconfigured installation — by design, with warnings; the attribution index deferred by the authorship PDR inherits the gap.
- **The trailer keys become permanent public surface** the moment they land, like diagnostic identifiers; their review is not optional polish.
- **`--preview` doubles the walk** (plan-time read, then post-write validation on apply) — acceptable at current corpus sizes under the M4 latency evidence, and covered by the same one-second trigger logic if it stops being acceptable.

## Amendments

The Decision above stands as written; these later entries change parts of it, and the original text is left intact so the change is legible.

- **2026-08-08 — the `write.*` enum is five identifiers.** The Decision enumerated the area's refusals without fixing their spellings and deferred the set to this review. Adjudicated with the implementation: `write.target-outside-vault`, `write.target-unwritable` and `write.closed-write` are the three refusals the Decision named — a target escaping the root, a target that cannot be written, an ownership violation — each an error; `write.actor-unattributed` and `write.commit-failed` are warnings. The area holds nothing else, and the post-write report reuses `note.*` and `link.*` unchanged, as written.
- **2026-08-08 — the missing-actor warning is seated in `write.*`, not `installation.*`.** The Decision fixed the seat's *timing* here without fixing its *area*. Adjudicated: an installation record is entitled to name nobody, and reading one is silent about it — only writing makes the absence matter — so the warning belongs to the act rather than to the configuration it read. `write.actor-unattributed` is emitted by the transaction, at the write, exactly as the advance-warning role the M2 vault-contract record described.
- **2026-08-08 — `write.commit-failed` is minted beyond the Decision's list, as a warning.** The Decision's enumeration covered refusals only, and a commit that fails *after* the file is written is not a refusal: the capture landed, and never-lossy means the verdict says so. The act therefore exits `0` carrying a warning, and the structured result names the created path as recovery — the guest-mode shape — rather than a commit that does not exist. The cost is recorded rather than denied: a capture can succeed uncommitted on a substrate that owns the commit path, so commit-at-birth is honored where it can be and reported where it cannot.
- **2026-08-08 — the commit trailer pair is `Dogtag-Actor` and `Dogtag-Provenance`.** The keys the Decision deferred to this review, namespaced so they can never be confused with another tool's trailers. An unattributed act writes **no** actor trailer rather than one naming nobody, and still writes the provenance trailer, so the capacity is always on the record even when the identity is not. An actor name carrying a line break cannot forge a second trailer — the writer's own rule, regression-tested — because a trailer is a line and attribution a caller can fabricate is worse than none.
- **2026-08-08 — `--strict` is deliberately absent from `capture`.** The read verbs' flag promotes findings for the exit code, and a write verb's exit code answers a different question — did the act land — which this record's exit split already fixed. A `--strict` capture could only mean "refuse the write on a corpus finding", which is the gating never-lossy forbids. The absence is the composition, not an omission.
- **2026-08-08 — a flag name that cannot round-trip through frontmatter is a known, deferred gap.** A contract may declare a property whose name contains a newline, name it as a flag, and give it as a type's birth state; all three declarations validate and the contract loads with no findings. A capture into such a vault stamps the name verbatim and writes frontmatter that no longer parses — the created note is reported `note.frontmatter-invalid` on the next read, so the defect is loud rather than silent, and the captured text survives in the file's bytes. Reaching it requires all three declaration sites poisoned consistently — each partial spelling is already refused by `contract.flag-property-undeclared` or `contract.birth-flag-not-a-flag` — so no ordinary vault trips it. **Deferred, with its destination named:** closing it belongs either to contract validity, where a rule about what a name may contain is version-scoped work this packet does not open, or to the writer, which could refuse to stamp a flag name that does not round-trip — the same "ask the reader's own rule" discipline the frontmatter fence guard adopted. The milestone that opens either question owns it; recorded here so it is not rediscovered as a surprise.
