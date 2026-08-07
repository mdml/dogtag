# Fixture profile: `dense`

**Stands for:** the PKM enthusiast with an established corpus.
**Corpus:** built at M2 (the milestone that defines the committed vault-contract format).

## Distinguishing axes

- **Many types, several identity-bearing.** A mature taxonomy with dozens of declared types, several of which carry the identity-bearing capability — the corpus distinguishes multiple entity kinds, not one entity type plus impostors. This stresses capability enumeration, per-type required properties, and typed-relationship validation at realistic scale.
- **Wikilink dialect.** References are written as wikilinks, with bare names the common case. This stresses name resolution against a large corpus where most references never spell a path.
- **Lifecycle where the ordinary state is absence.** The contract names a lifecycle axis whose ordinary state is encoded as the *absence* of a value: an unmarked note is in the ordinary state, and a named value marks departure from it. This is one side of the sharpest seam axis — `starter` holds the other — and the pair exists to prove that lifecycle filtering can be answered from the declaration alone in both encodings.

## What the fixture is

A committed vault contract at M2, and roughly forty hand-authored fictional notes on top of it at M3, when the document model those notes must be written in is decided.

**The contract's shape is derived; its vocabulary is not.** Only approximate counts and axis facts cross from the established private corpus this profile is modelled on — roughly how many types, how many carry each capability, how many predicates and how many are required, which property kinds appear, the lifecycle encoding, the dialect. They cross as approximations rather than as a transcription: an exact census is a corpus statistic, which the extraction rules remove, and a stable fingerprint linking this fixture to its source for anyone who sees both. Every name in this fixture is authored fiction: no type name, property name, predicate, lifecycle value, or tag is copied. Because no vocabulary crosses, leakage is accidental rather than routine — not impossible, since whoever counts a schema has read it, which is why the gate also reads the authored names back against the source once before the commit. The reasoning, and the rejected alternative of scrubbing a derived vocabulary, are in [the M2 fixture and privacy record](../../../docs/decisions/engineering/2026-07-31-m2-fixtures-and-the-privacy-gate.md).

## What it must cover

At least two identity-bearing types, exactly one catch-all, at least one closed-write; at least two relationship predicates, at least one of them required; a lifecycle axis whose ordinary state is absence; the wikilink dialect. The contract loads with zero diagnostics at any severity.

**Held at contract version 2, deliberately.** From M5 the supported range reaches version 3, and this corpus stays where it is: together with the other below-ceiling profile it is the standing witness that the floor is real — a version-2 vault keeps loading, keeps validating, and gains `capture` through version 3's default table — and that version 3's write seats configure the verb rather than enable it. The cost is one `info` per run, `compat.newer-format-available`, which the conforming-contract scenario admits by name and by severity and nothing else. See [the M5 fixtures record](../../../docs/decisions/engineering/2026-08-07-m5-fixtures-and-conformance.md); moving this corpus to the current version is that record's decision to revisit, not an edit.

The floor deliberately does not require every property value kind to be used: a kind nobody reaches for should be visible as unused, and mandating coverage would make this fixture, rather than a real corpus, the reason a kind stays in the lattice.

## What the corpus holds now

`corpus/.dogtag/contract.toml` — a vault root and its committed contract, and nothing else. The notes land at M3, with the document model they must be written in.
