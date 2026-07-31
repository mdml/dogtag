# The M2 fixture corpora, negative cases, and the privacy gate

- Status: accepted
- Date: 2026-07-31

## Context

The `dense` and `starter` fixture corpora land at M2, because the committed contract format they are written in is an M2 decision ([the conformance harness ADR](2026-07-30-conformance-harness-shape.md)). Both carry obligations from elsewhere. `dense` stands for *the PKM enthusiast with an established corpus*, and the only established corpus available to model it on is a maintainer's private vault — which makes it the one artifact in this milestone sourced from private material rather than written fresh. [The extraction ADR](2026-07-30-document-extraction-sanitization.md) anticipated exactly this: a derived fixture contract *"publishes that corpus's type names, property vocabulary, and lifecycle words, and therefore requires its own dedicated privacy pass."*

`starter` carries a different problem. Its specification says the fixture is *"the contract and starter notes produced by initialization, committed verbatim"* — but `init` appears in no milestone of [beta.md](../../beta.md#milestones), and M2's surfaces are `doctor` and `contract explain`. The fixture is defined as the output of a command that does not exist and is not scheduled.

## Decision

### `dense` is derived numerically and authored lexically

**Only the shape crosses the private/public boundary, expressed as counts and axis facts. Every name in the public fixture is authored fiction.**

What crosses: how many types, how many carry each capability, how many predicates and how many are required, which property kinds are used and how often, the lifecycle encoding, the dialect. What does not cross: any string. Type names, property names, predicate names, lifecycle values, and tag vocabularies are written for the fixture.

**The privacy gate is therefore: does the artifact that crossed the boundary contain any string? If not, the gate is satisfied.** It runs **before the commit**, not before the push, because once history is public a file's removal does not remove it from history, and this repository fixes forward and never rewrites.

This changes the gate's nature rather than merely tightening it. A vocabulary-scrubbing pass is judgement-based, unbounded, and never provably complete — and it cannot be mechanized in the open, because a scanner would need a deny-list of the private vocabulary and that deny-list is itself the secret. Numeric derivation makes lexical leakage structurally impossible instead of reviewed-away, and reduces the human check to something a reader can complete in one look. `dense` loses nothing it exists for: density and capability distribution are what make it stress the kernel at realistic scale, and both are numbers.

**Coverage floor for `dense`:** at least two identity-bearing types, exactly one catch-all, at least one closed-write, every one of the eight property kinds used at least once, at least two predicates with at least one `required` and at least one carrying `targets`, a lifecycle axis whose ordinary state is absence, and the wikilink dialect. It loads with zero diagnostics.

### `starter` is authored as the normative initialization profile

**The `starter` contract is authored by hand at M2 and *is* the definition of what `init` must stamp.** When `init` lands, a test asserts its output equals this fixture byte for byte. The profile specification is corrected accordingly: the fixture is normative rather than generated.

This keeps M2's scope intact — no vault-creating write at the milestone whose job is opening and diagnosing — and keeps the fixture honest, because the direction of authority is stated rather than assumed. Pulling `init` into M2 would be an M0 scope change made inside an M2 packet; deferring `starter`'s corpus would leave M2 with one fixture and no absence-versus-named-value pair, which is the sharpest seam axis available and the reason the two profiles exist together.

**Coverage floor for `starter`:** exactly one catch-all, at least one identity-bearing type, a lifecycle axis whose ordinary state is a named value, and otherwise minimal. It loads with zero diagnostics — doubly load-bearing, since `starter` is the standing test that the product's own defaults satisfy the product's own rules.

**`init`'s absence from every milestone of `beta.md` is recorded here as an open question**, not resolved. A fresh install currently has no documented way to create a vault. It does not block M2 under this decision, and amending the beta scope contract is not this packet's to do.

### The M2 corpora contain a vault, not notes

**At M2 each built corpus is a vault root and its `.dogtag/contract.toml`. Notes land at M3, with the document model that defines them.**

No M2 scenario reads a note, so `corpus = "built"` at M2 means the fixture vault exists — which is exactly what every M2 scenario needs. Authoring notes now would mean writing them against an undecided document model, since frontmatter language and link syntax are M3 decisions, and rewriting all of them when M3 lands. That is the freeze-a-guess failure the harness ADR rejected for contracts, applied to notes. Each `PROFILE.md` states precisely what exists at each stage, so `built` does not overclaim by silence.

A third `CorpusStatus` between `scheduled` and `built` was rejected: any status short of built is a place a profile can sit indefinitely while the matrix reports something other than a skip, which is what a waiver looks like from the inside.

### Negative cases are derived, never authored

**The harness produces every negative case by copying a profile's own contract into a temporary directory and transforming it** — removing the catch-all, duplicating it, rewriting `contract_version`, deleting `[lifecycle]`, adding a contract-owned key to an installation record.

This is what makes a negative case profile-agnostic by construction rather than by review. A hand-authored broken contract has to be written in *some* vocabulary, which makes it a profile-specific fixture wearing a shared-sounding name — the exact shape the no-waiver rule exists to catch. Derivation runs the same assertion against four vocabularies with no broken contract checked in anywhere. Putting the cases inline in Rust tests was rejected for a different reason: they would not be conformance scenarios at all, would never appear in the matrix, and the printed cross product would stop describing what is actually verified.

### Alternatives considered

- **Mechanical derivation of `dense` plus a vocabulary-scrubbing pass.** The most faithful to a real corpus, and what the extraction ADR anticipated. Rejected: private vocabulary would exist in a working copy, the pass is judgement-based and never provably complete, no CI check can ever back it, and an escaped name is public permanently.
- **Authoring `dense` entirely, shape included.** Nothing crosses the boundary in any form. Rejected: `dense`'s claim is that it is *realistically* dense, and an invented shape makes it a guess about what a mature corpus looks like — which is the assumption the profile exists to test rather than to embody.
- **Adding `dogtag init` to M2** so `starter` is genuinely generated. Rejected as above.
- **Deferring `starter`'s corpus to the milestone where `init` lands.** Rejected as above.
- **Authoring notes for both corpora at M2.** Rejected as above.
- **A third corpus status.** Rejected as above.
- **Hand-authored broken contracts under a `cases/` directory.** Explicit and easy to eyeball. Rejected as above.
- **Negative cases inline in the harness's Rust tests.** Rejected as above.

## Consequences

- **Authoring a coherent fictional taxonomy at realistic density is real work**, and it must be internally consistent enough that the contract loads clean and the property kinds are each genuinely used. That work is the cost of the privacy decision, and it is paid once.
- **The extraction ADR's anticipated vocabulary pass no longer applies to `dense`**, and is amended there rather than left to look unperformed.
- **The privacy gate has no mechanical enforcement, by necessity.** It is reduced to a check a human can complete reliably, but nothing in CI can prove it happened — the weakest link in a repository that mechanizes everything else, and stated here rather than left implicit.
- **`corpus = "built"` means two different things at M2 and M3** — the vault exists, then the vault has notes. The profile specifications carry the distinction, and the harness schema does not, which is a deliberate choice to keep the schema unwidened at the cost of the word carrying context.
- **The `docs` and `records` corpora stay scheduled**, so M2's executable scenarios run against two of four profiles. The no-waiver machinery is intact — an executable scenario against a scheduled corpus is reported pending on the corpus, and the harness refuses a runnable pair with no execution path — but M2's cross-profile evidence is `dense` and `starter`, not four profiles, and the printed matrix should not be read as more.
- **Derived negative cases make the harness a program rather than a data set.** It gains transformation logic that must itself be correct, and a bug there could make a negative case vacuously pass. Those transformations need their own tests.
