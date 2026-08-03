# Contract version 2: the tag vocabulary, the record kind, and the per-version machinery

- Status: accepted
- Date: 2026-08-03

## Context

[beta.md](../../beta.md#milestones) widened M3 on 2026-08-03 to carry the first `contract_version` bump: a tag-vocabulary construct — because [note-types](../product/note-types.md) routed subtypes into tags and version 1 provides no way to describe tags — plus the `record` kind [the kind-lattice record](2026-08-03-the-kind-lattice-against-a-real-corpus.md) scopes. The bump raises the ceiling only; the floor stays at 1 per [the compatibility record](2026-07-31-diagnostics-and-compatibility.md), so a version-1 vault keeps loading.

Three standing constraints bind the execution. The [vault-contract record's](2026-07-31-vault-contract-and-installation-record.md) 2026-08-01 amendment makes a per-version key set and default table due *before* the supported range widens — "widening it without them is the regression" — and the kind-lattice amendment notes the bump is the event that makes those mechanisms' second branch reachable at all. The same record's amendments list two version-1 readings the parser inferred without a decision: `[dialect]`'s mandatoriness (identifier `contract.missing-dialect` already minted) and the two implemented defaults. And the M2 cutover blocker names exactly what the tag construct must express: the subtype vocabulary and the per-type required tag prefixes the incumbent's schema-printing command renders.

One shape constraint comes from the trail's own principles: behavior binds to declarations, never to names, and reserving a vocabulary word is what the capability model exists to avoid ([the lifecycle record](2026-07-31-lifecycle-declaration-and-the-seam.md) rejected `ordinary = "absent"` on exactly that ground). A hardcoded `tags:` frontmatter key would be the format's first reserved corpus-vocabulary word.

## Decision

### The version and the machinery

**`contract_version = 2`. The supported range becomes `1..=2`, and the range widens only in the change that carries the per-version key sets and per-version default tables.** That ordering is an M3 acceptance criterion ([the release record](2026-08-03-m3-release-and-cutover.md)), not a hope. Key legality and resolution are version-scoped exactly as the vault-contract record promised: a version-1 contract is judged against version 1's key set, its omissions resolve against version 1's default table, and a construct only version 2 defines is absent from — never defaulted into — a version-1 model. The `supported`-but-not-current classification and its `compat.newer-format-available` info diagnostic become reachable for the first time; [the M3 fixtures record](2026-08-03-m3-fixtures-and-conformance.md) makes them reachable from a derived case rather than only from the injectable range.

**The version-1 inferences are ratified rather than left inferred.** `[dialect]` is mandatory — in version 1 and version 2 both — and its absence is `contract.missing-dialect`, now record-backed. Version 1's default table is exactly the two implemented literals: `required = false` on a property, `capabilities = []` on a type. Version 2's table is version 1's plus the defaults this record declares below. A later decision to default `links` remains unavailable without a version to carry it, as the amendment warned.

**There is no upgrade-on-read.** The compatibility record invited revisiting in-memory upgrade at the first real bump; revisited, it is declined on the record's own standing ground: an upgraded value comes from neither the file nor a format default, so provenance would need a fourth source or would lie. A version-1 model simply has no tag vocabulary and no record kinds.

### The tag vocabulary

**A top-level `[tags]` table names the corpus's tag-carrying property; per-type `[[type.tag-namespace]]` entries declare the vocabulary.** Sketched with invented names:

```toml
[tags]
property = "labels"

[[type]]
name = "log"
capabilities = []

  [[type.property]]
  name = "labels"
  kind = "list"
  of = "string"

  [[type.tag-namespace]]
  prefix = "log/"
  required = true
  values = ["workout", "meditation", "reading"]

  [[type.tag-namespace]]
  prefix = "topic/"
  required = false
  open = true
```

- **`[tags]` follows the lifecycle seam**: it names a declared property rather than reserving a word, so the kernel never learns any corpus's word for tags. The named property must be declared, with `kind = "list"`, `of = "string"`, on every type that declares a tag-namespace; the existing shared-kind rule already guarantees one corpus-wide answer for the name. A type declaring a namespace in a contract with no `[tags]` table, or without the named property, is a load error. `[tags]` is optional — a corpus without tag vocabulary declares nothing and loses nothing.
- **`prefix` is a literal, non-empty string matched against the start of a tag, and includes its own separator** (`log/` above). The kernel owns no separator convention and never splits a tag; a namespace is a prefix test and nothing more. Prefixes are unique within a type; a repeat is a load error with both spans, like every other duplicate.
- **Exactly one of `values` or `open = true`**, mirroring the lifecycle table's XOR discipline rather than making omission a decision. `values` is a non-empty list of unique members, each naming the remainder after the prefix — the closed vocabulary. `open = true` declares the namespace without bounding its membership. Declaring both, or neither, is a load error.
- **`required` defaults to `false`** (version 2's default table carries it, alongside `required = false` on a record field).

**What `check` enforces, per note of the declaring type:** a `required` namespace must have at least one tag matching its prefix (`note.required-namespace-missing`); a tag matching a closed namespace's prefix must have its remainder in `values` (`note.tag-outside-vocabulary`). Namespaces are evaluated independently; a tag matching no declared namespace is untouched, at any severity — tags are content, and the construct describes what the corpus chose to schematize, never a license to enumerate all tagging.

This is deliberately not the pattern facility the kind-lattice record refused. A namespace is a first-class construct with two fixed behaviors, and it lives on the *tag* plane, where note-types already made prefixes the corpus's own model; the unknown-key rule on the contract itself is untouched.

### The record kind

**The kind-lattice record's sketch is adopted as written**: `kind = "record"` declares its own `[[type.property.field]]` list; `list` may name `record` in `of`. A field has `name`, `kind`, and `required`, and **a field's kind may be any member of the scalar lattice, including `enum` with `values`** — "drawn from the existing scalar lattice" is read at full width, and the record kind's canonical instance, the labeled multi-value channel, plausibly wants a closed label set. A field may not be `record` or `list`: one level of nesting, exactly as scoped, because both observed shapes fit inside it and a second level is far easier to add than to remove.

In frontmatter, a record value is a mapping one level deep (the depth [the document-model record](2026-08-03-m3-document-model.md) admits); its fields validate exactly as properties do — missing required field and kind mismatch reuse `note.missing-required-property` and `note.property-kind-invalid` with the field path in the message and evidence, and an undeclared field inside a record value is `note.undeclared-property` at `info`, consistent with the note-side unknown-key posture. Field names obey the same never-empty, never-dotted rule as every other declaration name, under the same identifier.

### The catch-all validity rule

**In a version-2 contract, the catch-all type may declare no `required = true` property, no `required = true` relationship, and no `required = true` tag-namespace; a violation is a load error (`contract.catch-all-requires`) pointing at the capability and the offending declaration.** The document-model record binds every untyped note to the catch-all, so a requiring catch-all would have `contract explain` render "accepts anything" beside requirements every untyped note instantly fails — the misleading-declaration shape this trail rejects everywhere else.

The rule is **version-scoped to 2 and above, deliberately.** A version-1 contract that loaded clean at `0.1.0-beta.1` must keep loading, or the upgrade promise breaks; validity is part of a version's schema, and version 1's is frozen. A version-1 corpus in that shape simply collects missing-required findings on its untyped notes.

### The scope fence

**Version 2 carries exactly the constructs named here and nothing else.** Relationship cardinality, `targets`, the write-policy vocabulary, and value constraints all remain deferred: none has an M3 consumer, and inventing them inside a bump that has one is how a version accretes guesses. The migration cost is as the vault-contract record priced it — a hand edit, paid by the two committed fixtures and any founder vault that wants the new constructs.

### Alternatives considered

- **A reserved `tags:` frontmatter key.** Rejected: the format's first reserved corpus-vocabulary word, superseding the binds-to-declarations property for a convenience the `[tags]` seam provides anyway.
- **Per-type tag-property naming.** Rejected: more general than note-types asks for; the shared-kind rule already makes one corpus-wide property the natural unit, and per-type naming invites two words for one plane.
- **`values` optional, omission meaning open.** Rejected: omission-as-decision, the shape the lifecycle record rejected for its own table.
- **Enumerated subtypes without prefixes.** Rejected: cannot render the per-type required tag prefixes the cutover blocker names, so the M2-deferred cutover would stay blocked at the milestone built to unblock it.
- **Scalar-only record fields (no `enum`).** Rejected: a narrower reading of "the scalar lattice" than the sketch's own words, costing enforcement exactly where the canonical instance wants it.
- **A recursive record kind.** Rejected already by the kind-lattice record; nothing observed needs it and the rendering cost is unbounded.
- **Upgrade-on-read.** Rejected as reasoning above, honoring the compatibility record's revisit clause by answering it.
- **The catch-all rule in version 1 as well.** Rejected: it would stop a previously-loading vault from loading on upgrade, which negates the promise the floor policy exists to keep.
- **Folding another deferral into version 2.** Rejected: no consumer, and the freeze-a-guess failure arriving dressed as efficiency.

## Consequences

- **The SDK now genuinely carries two versions** — two key sets, two default tables, both compatibility branches live — which is the cost the compatibility record accepted in prose and M3 pays in code. The mechanisms are testable against real fixtures for the first time.
- **`contract explain` grows nested rendering**: namespace tables and record fields under their types. The one-level bound is what keeps that rendering finite, and the existing `contract-explain-renders-every-declaration` scenario holds the equivalence across the new constructs by construction.
- **The founder vault's gap one closes on migration**: the omitted record-valued properties become declarable, and the omission comments come out. Gap two — the foreign editor's prefix — remains, permanently, expressed at read time as the `info` findings the document-model record defines.
- **A version-1 vault sees a new info diagnostic** (`compat.newer-format-available`) on every run once `1..=2` ships. That is the classification working as designed, and it is `info` for precisely the reason the severity got created.
- **`installation_version` stays at 1.** Nothing in this milestone touches the installation record's schema, and bumping it in sympathy would be symmetry for its own sake.
- **The `[tags]` property is declared per type like any property**, so a corpus can have typed tag vocabularies on some types and no tag property on others; a namespace-less type with the tag property is ordinary tagging, undescribed and unenforced.
