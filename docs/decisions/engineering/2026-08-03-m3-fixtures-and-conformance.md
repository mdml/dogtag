# The M3 fixture notes, the extended privacy gate, and the conformance additions

- Status: accepted (amended 2026-08-04 — see [Amendments](#amendments))
- Date: 2026-08-03

## Context

Notes land at M3 with the document model that defines them — [the M2 fixture record](2026-07-31-m2-fixtures-and-the-privacy-gate.md) deferred them by name and fixed the rule that binds this record: *a scenario whose Given describes notes may not graduate against a corpus that holds none; graduating the note scenarios and authoring the notes are one act.* The same record's machinery carries over: the numeric derivation and its privacy gate for anything sourced from a private corpus, derived-never-authored negative cases with the three-assertion rule, harness-enforced floors (per the 2026-08-01 amendment), and `built` never returning to `scheduled`.

Two later findings sharpen the job. The 2026-08-01 amendment records that `dense` reads *templated rather than accreted* — fifty-eight types with an identical property tail, no irregular edges — which is a gap against the profile's own claim of realism. And [the contract-version-2 record](2026-08-03-contract-version-2.md) adds constructs that exist because a real corpus asked for them; the kind-lattice record's consequence that "neither fixture demonstrates this gap" is the regret this record retires.

Nine conformance scenarios are pending at M3 (`conforming-corpus-zero-diagnostics`, `unknown-type-diagnostic`, `missing-required-property-diagnostic`, `dangling-typed-link-diagnostic`, `show-returns-document-model`, `list-filters-by-declared-lifecycle-axis`, `bare-name-link-resolves-when-unambiguous`, `ambiguous-bare-name-yields-link-diagnostic`, `path-qualified-link-resolves`). `docs` and `records` corpora stay scheduled — M4 and pre-E1 respectively — so M3's executable evidence is `dense` and `starter`, reported pending-on-corpus for the other two, exactly the M2 posture.

## Decision

### Both committed fixtures declare `contract_version = 2`

**`dense` migrates to version 2 and uses both new constructs. `starter` declares version 2 and uses no version-2-only key.** `starter` is the normative statement of what `init` must stamp, and a fresh install stamping a below-current version would be a strange product claim; keeping it free of version-2 constructs is what makes the derived version case below legal.

**The `supported`-but-not-current classification is demonstrated by derivation, not by a committed fixture**: a derived case copies `starter`'s contract, rewrites `contract_version` to `1`, asserts the bytes changed, asserts the contract loads fully, and asserts the `supported` classification and the `compat.newer-format-available` info diagnostic. The transformation is legal against `starter` by construction and fails loudly against any profile whose contract uses a version-2 construct — which is itself the guard that keeps the case honest.

### `dense`'s floor grows, in two directions

**Construct coverage:** at least one `record` property and at least one `list` `of = "record"` on an identity-bearing type; a `[tags]` declaration with at least one closed namespace, at least one open namespace, and at least one `required = true` namespace; and notes exercising each — a note carrying a conforming record value, a list of records, and tags in each declared namespace. The M2 floors deliberately declined to mandate kinds so that unused kinds stay visible; that argument protected kinds nobody had asked for, and does not extend to constructs a real corpus requested. `float` stays unmandated — its question is still open and now two-sided.

**Irregularity, mechanized where it can be:** at least one declared type with zero notes in the corpus; at least one optional property never used by any note; and notes-per-type deliberately non-uniform. Those three join the harness-enforced floor. The rest of the amendment's finding — irregular property tails, varied body shapes, the texture of accretion — is authored judgement, stated as guidance in `PROFILE.md` rather than pretended into a mechanical check.

`dense` holds roughly forty notes, as M0 sized it, including at least one untyped note (the catch-all binding demonstrated in a committed corpus) and a pair of notes sharing a name with nothing referencing the bare name — legitimate ambiguity at rest, which the resolution scenarios then exercise through derived references.

**Both committed corpora load with `check` reporting zero error diagnostics.** `info` findings are permitted in principle but `dense` and `starter` are authored without them; the corpus that legitimately carries permanent `info` findings is a private one, not a fixture.

### `starter`'s notes are the normative `init` output

**`starter` gains a small note set — one to three notes — authored by hand now, as the definition of what `init` stamps.** The M2 record already fixed the direction of authority (the fixture is normative; `init`, when it lands, must equal it byte for byte) for the contract; the same sentence now covers the notes. Minimal, typed, conforming, exercising the named-value lifecycle encoding `starter` exists to carry.

### `dense`'s notes: derived numerically, authored lexically

**The M2 privacy gate extends to note-level shape unchanged in kind.** One intermediate artifact of integers and format constants, written outside any repository working tree and deleted after authoring — roughly how many notes per type, roughly how often optional properties are filled, roughly how many links and tags a note carries, rough body-length spread — all as orders of magnitude, never a census. No title, name, tag value, prose fragment, or vocabulary word crosses. The fixture's notes are authored fiction consistent with the counts; before commit, the authored corpus is read once against the private source by the only party who can, confirming no name or phrase coincides; the commit message records that the gate ran. Ideally the notes are authored from the numeric artifact alone by someone or something that never saw the source — the record's standing ideal, and now a practical one, since authoring-from-counts is a delegable act.

### Negative note cases are derived, never authored

**The harness's derivation machinery extends from contracts to corpora: every negative note case copies a profile's own corpus into a temporary directory and transforms it.** The transformation inventory this milestone needs: rewrite a note's `type` to an undeclared name; delete a required property key; corrupt a declared kind's lexical form; repoint a typed link at a nonexistent target; duplicate a note under a second path and plant a bare-name reference to it; rewrite `[lifecycle]` to `none = true` (the no-axis filter diagnostic); rewrite `contract_version` to `1` (the supported case above). Every case keeps the three-assertion rule: the untransformed corpus is clean, the transformed bytes differ, the expected identifier appears. The transformations are code and get their own tests, as the M2 record already required of their contract-side siblings.

Committed broken notes under an excluded directory were rejected: the exclusion would need a traversal exemption, which is a waiver wearing a path, and it would weaken the dot-directory rule the document-model record just fixed.

### Graduation and the new scenarios

**All nine pending M3 scenarios graduate in the same change that authors the notes** — the one-act rule applied, with no scenario held back. **Five scenarios are added:**

- `required-tag-namespace-missing` — a note of a declaring type with no tag matching a required prefix yields `note.required-namespace-missing`.
- `closed-namespace-value-outside-vocabulary` — a tag matching a closed namespace's prefix with a remainder outside `values` yields `note.tag-outside-vocabulary`.
- `supported-contract-version-loads-with-info` — the derived version-1 case above: full load, `supported` classification, info diagnostic.
- `undeclared-key-reported-as-info` — an undeclared frontmatter key is `info`, appears in `check`'s report, and does not affect the exit code even under `--strict`.
- `untyped-note-binds-to-catch-all` — a note without a discriminator is a member of the catch-all and `show` reports the binding.

`contract-explain-renders-every-declaration` needs no sibling: it asserts the full declaration set renders in both formats, so the version-2 constructs flow into it by construction. The set totals 24 scenarios, 24 executable, running against `dense` and `starter` with `docs` and `records` pending on their corpora.

### Alternatives considered

- **`starter` committed at version 1.** Rejected: `init`'s normative output would be below current, and `starter` would never exercise the version-2 key set; the derived case demonstrates the classification at no such cost.
- **No committed v1 evidence at all.** Rejected: the classification beta.md says M3 makes reachable would be held only by unit tests against the injectable range.
- **Keeping the floors construct-silent.** Rejected: the matrix would never demonstrate the constructs a real corpus asked for — the kind-lattice record's stated regret, repeated knowingly.
- **Mechanizing irregularity wholesale.** Rejected: a statistical texture test would make the harness the author; the three floors chosen are the ones a reader can verify in one look.
- **Committed broken notes in a quarantined directory.** Rejected as above.
- **Inline Rust test inputs for note negatives.** Rejected: they never appear in the matrix — the M2 record's reasoning, unchanged.
- **Inventing `dense`'s note shape without counting.** Rejected: the notes become a guess about what a mature corpus looks like, the assumption the profile exists to test rather than embody — the same trade the M2 record rejected for the contract.

## Consequences

- **`corpus = "built"` now means notes, for `dense` and `starter`.** The M2 record's warning that the word means different things at M2 and M3 resolves in the direction it promised; the `PROFILE.md` files say what exists.
- **Authoring forty coherent fictional notes over a fictional taxonomy is real work**, again, and the irregularity floors make it slightly harder on purpose. The privacy cost is paid the same way it was at M2: once.
- **The privacy gate still has no mechanical enforcement**, and now covers more surface. The gate's receipt lives in the commit message; nothing in CI can prove it ran. Unchanged, and restated so growth of the gated surface is visible.
- **The derivation machinery gains corpus transformations**, which are more code that can be wrong in ways that vacuously pass; their tests are part of the same change, not a follow-up.
- **The matrix's M3 rows report two executions and two pendings** per scenario until M4 builds `docs`. Cross-profile evidence at M3 is `dense` and `starter`, and the printed matrix should not be read as more — the M2 record's caveat, carried forward verbatim.

## Amendments

The Decision above stands as written; these later entries change parts of it, and the original text is left intact so the change is legible. All three arise from the slices 1–5 implementation, adjudicated 2026-08-04.

- **2026-08-04 — the fixture contracts migrated at slice 1, not slice 9, because the packet forces it.** Widening the supported range makes a version-1 fixture classify as `supported` and earn an info diagnostic, and the executable M2 scenario `conforming-contract-loads-with-zero-diagnostics` demands zero diagnostics at any severity — so this record's "both fixtures declare version 2" binds the moment the range widens, fourteen scenario-by-profile pairs going red otherwise. The implementation migrated both committed contracts inside the version-machinery slice; the slice ordering here that placed migration at slice 9 was wrong about where the constraint bites, and the workflow fence that forbade touching fixtures was the brief contradicting the packet — the packet wins, recorded in [the slices record](2026-08-03-m3-implementation-slices.md#amendments).

- **2026-08-04 — `starter`'s claim narrows: the named ordinary value lives on every *typed* note, and the catch-all has no lifecycle.** The composed catch-all rule ([contract version 2](2026-08-03-contract-version-2.md#amendments)) removes the axis from `starter`'s catch-all entirely, which changes what `init` normatively stamps — an unclassified capture in a fresh vault carries no lifecycle state until it is classified. The floor's wording "a lifecycle axis whose ordinary state is a named value" now means: on every type that declares the axis, which is every type except the catch-all. The absence-versus-named-value seam the profile pair exists for is narrowed to participating notes, not broken.

- **2026-08-04 — two version-2 migrations of `dense`'s contract exist, and the slice 9–10 integration takes the corpus draft's.** The slices 1–5 branch carries a minimal migration made to keep the matrix green; the `m3/dense-corpus-draft` branch carries the full floor-meeting migration (namespaces, record properties) that the forty-four authored notes conform to. They will collide at integration, and the resolution is decided now rather than negotiated in the merge: the draft's contract supersedes the minimal one, adjusted only as needed to carry the slices branch's catch-all changes, and the notes are validated against it by the real implementation before the corpus status flips.
