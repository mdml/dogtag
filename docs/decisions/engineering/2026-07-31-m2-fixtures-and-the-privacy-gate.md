# The M2 fixture corpora, negative cases, and the privacy gate

- Status: accepted
- Date: 2026-07-31

## Context

The `dense` and `starter` fixture corpora land at M2, because the committed contract format they are written in is an M2 decision ([the conformance harness ADR](2026-07-30-conformance-harness-shape.md)). Both carry obligations from elsewhere. `dense` stands for *the PKM enthusiast with an established corpus*, and the only established corpus available to model it on is a maintainer's private vault — which makes it the one artifact in this milestone sourced from private material rather than written fresh. [The extraction ADR](2026-07-30-document-extraction-sanitization.md) anticipated exactly this: a derived fixture contract *"publishes that corpus's type names, property vocabulary, and lifecycle words, and therefore requires its own dedicated privacy pass."*

`starter` carries a different problem. Its specification says the fixture is *"the contract and starter notes produced by initialization, committed verbatim"* — but `init` appears in no milestone of [beta.md](../../beta.md#milestones), and M2's surfaces are `doctor` and `contract explain`. The fixture is defined as the output of a command that does not exist and is not scheduled.

## Decision

### `dense` is derived numerically and authored lexically

**Only the shape crosses the private/public boundary, expressed as counts and axis facts. Every name in the public fixture is authored fiction.**

**The derivation produces exactly one intermediate artifact**: a short file of integers and format constants — roughly how many types, how many carry each capability, how many predicates and how many are required, which property kinds appear, which lifecycle encoding, which dialect. It is written **outside any repository working tree** and deleted once the fixture is authored. What does not cross: any corpus vocabulary. Type names, property names, predicate names, lifecycle values, and tag vocabularies are written for the fixture.

**The counts cross as approximations, not as a transcription.** This matters more than it first appears. Rule 5 of [the extraction record](2026-07-30-document-extraction-sanitization.md) deletes *corpus statistics* as archaeology, and the product decision trail independently forbids corpus-specific numbers — and an exact census (thirty-one types, four identity-bearing, twelve predicates) is a stable, distinctive fingerprint that links the fixture to the private vault for anyone who ever sees both, while disclosing that corpus's scale, which is otherwise absent from the public tree. An order of magnitude serves `dense`'s purpose completely: what makes it stress the kernel is that it is *realistically dense*, not that it matches one vault's arithmetic. The coverage floor below is already written as a floor for this reason.

**The gate is therefore: open the intermediate artifact and confirm it holds only integers and format constants, with no corpus vocabulary anywhere in it.** It is performed by whoever authors the fixture, the fixture's commit message records that it was performed, and it runs **before the commit** — not before the push, because once history is public a file's removal does not remove it from history, and this repository fixes forward and never rewrites.

This changes the gate's nature rather than merely tightening it. A vocabulary-scrubbing pass is judgement-based, unbounded, and never provably complete — and it cannot be mechanized in the open, because a scanner would need a deny-list of the private vocabulary and that deny-list is itself the secret. Numeric derivation makes lexical leakage **accidental rather than routine**, and reduces the human check to something a reader can complete in one look.

It does not make leakage impossible, and the earlier claim that it did was too strong. Whoever counts the private schema has read its vocabulary, and priming is real: a distinctive predicate or lifecycle word can resurface in "invented" naming. So one step survives from the rejected alternative — **before the commit, the authored vocabulary is read once against the private source, by the only party who can, confirming no name coincides.** Ideally the fixture is authored from the numeric artifact alone, by someone or something that never saw the source.

**Coverage floor for `dense`:** at least two identity-bearing types, exactly one catch-all, at least one closed-write, at least two predicates with at least one `required`, a lifecycle axis whose ordinary state is absence, and the wikilink dialect. It loads with zero diagnostics.

The floor deliberately does **not** require every property kind to be used. Mandating that would make the fixture, rather than a real corpus, the reason a kind stays in the lattice — and a kind nobody reaches for should be visible as unused, which is the only evidence that the lattice can shrink.

### `starter` is authored as the normative initialization profile

**The `starter` contract is authored by hand at M2 and *is* the definition of what `init` must stamp.** When `init` lands, a test asserts its output equals this fixture byte for byte. The profile specification is corrected accordingly: the fixture is normative rather than generated.

This keeps M2's scope intact — no vault-creating write at the milestone whose job is opening and diagnosing — and keeps the fixture honest, because the direction of authority is stated rather than assumed. Pulling `init` into M2 would be an M0 scope change made inside an M2 packet; deferring `starter`'s corpus would leave M2 with one fixture and no absence-versus-named-value pair, which is the sharpest seam axis available and the reason the two profiles exist together.

**Coverage floor for `starter`:** exactly one catch-all, at least one identity-bearing type, a lifecycle axis whose ordinary state is a named value, and otherwise minimal. It loads with zero diagnostics — doubly load-bearing, since `starter` is the standing test that the product's own defaults satisfy the product's own rules.

**Three commitments are scheduled at no milestone, and are recorded here as open questions rather than resolved.** All three are the same shape: an obligation another document makes, with no rung of the ladder delivering it.

- **`init`.** `README.md` documents `dogtag init` and `dogtag import`; no milestone of `beta.md` schedules either. So the only route to a working vault at M2 is hand-authoring `.dogtag/contract.toml`, whose sole specification is an engineering record — and `beta.md`'s own promise to *"discover, configure, inspect, and validate a vault"* has nothing behind "configure" at the milestone that owns configuration.
- **`dogtag migrate`.** `architecture.md` commits to it as the schema-change escape hatch and the settings PDR lists it as a CLI surface. No milestone delivers it, which is why [the compatibility record](2026-07-31-diagnostics-and-compatibility.md) has to freeze the supported floor for the whole beta.
- **Generating the vault's agent contract on disk.** `architecture.md` makes it a beta obligation and `README.md`'s sketch shows the generated file in the vault. `contract explain` renders it to standard output at M2; nothing writes it, at any milestone.

None of the three blocks M2, and amending the beta scope contract is not this packet's to do. They are listed together because they are one gap, not three: the beta ladder has no rung that *creates or maintains* vault configuration, only rungs that read it.

### The M2 corpora contain a vault, not notes

**At M2 each built corpus is a vault root and its `.dogtag/contract.toml`. Notes land at M3, with the document model that defines them.**

No M2 scenario reads a note, so `corpus = "built"` at M2 means the fixture vault exists — which is exactly what every M2 scenario needs. Authoring notes now would mean writing them against an undecided document model, since frontmatter language and link syntax are M3 decisions, and rewriting all of them when M3 lands. That is the freeze-a-guess failure the harness ADR rejected for contracts, applied to notes. Each `PROFILE.md` states precisely what exists at each stage, so `built` does not overclaim by silence.

A third `CorpusStatus` between `scheduled` and `built` was rejected: any status short of built is a place a profile can sit indefinitely while the matrix reports something other than a skip, which is what a waiver looks like from the inside.

That leaves one obligation the schema cannot carry, so it is stated as a rule instead: **a scenario whose Given describes notes may not graduate against a corpus that holds none.** The loader checks only that `corpus/` is a directory, so an M3 note scenario run against a contract-only corpus would find zero notes, satisfy "every note carries a declared type" vacuously over the empty set, and report green. Graduating the note scenarios and authoring the notes are one act, not two.

**`corpus = "scheduled"` is itself the profile-side exemption channel**, which `conformance/README.md` previously denied existed. Setting it removes a profile from every scenario at once, and the loader's only check is disk consistency — so deleting a corpus directory and flipping the status back is a mechanically valid way to make a failing profile stop failing. Nothing in the harness ratchets. The rule is therefore written down: **a corpus that has been `built` never returns to `scheduled`**, and the harness should enforce it rather than trusting that nobody would.

### Negative cases are derived, never authored

**The harness produces every negative case by copying a profile's own contract into a temporary directory and transforming it** — removing the catch-all, duplicating it, rewriting `contract_version`, deleting `[lifecycle]`.

**A transformation must prove it changed something.** Every derived case asserts three things, not one: the untransformed contract loads clean, the transformed bytes differ from the original, and the expected diagnostic identifier appears. Without the middle assertion a transformation that fails to find its target — because one profile spells a table where another spells an array of tables — produces a copy identical to the original and the case passes or fails for reasons unrelated to what it tests.

**Two kinds of input are not contract transformations, and saying so is part of the rule.** The installation-record cases have no per-profile source to derive from, since one machine-local record serves every profile; running them once per profile runs one identical input four times. The three discovery scenarios need a synthetic *tree* — a nested directory, a symlink, a second configuration directory with no contract inside — where the profile contributes a contract that discovery never parses, because the sentinel is a path test. Those cases are honestly one case each, and pretending the cross product multiplies them is the kind of overclaim this suite exists to prevent.

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
