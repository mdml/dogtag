# The M3 surfaces: `check`, `list`, `show`, and the boundary they inherit

- Status: accepted
- Date: 2026-08-03

## Context

M3 ships three commands over the document model ([beta.md](../../beta.md#milestones)). The boundary questions were settled at M2 and are inherited, not reopened: the SDK owns the semantic model and every rendering, the CLI owns argument parsing, environment and current-directory resolution, colour, stream routing, and the severity-to-exit mapping ([the M2 surfaces record](2026-07-31-m2-surfaces-and-the-sdk-boundary.md), as amended); semantic operations take a resolved contract and cannot be reached without one; severity is the sole determinant of exit `0` versus `1`, `--strict` promotes warnings only, and every foreseeable failure is a diagnostic with an identifier ([the diagnostics record](2026-07-31-diagnostics-and-compatibility.md)).

Two M0 scenarios constrain the surfaces directly: `list-filters-by-declared-lifecycle-axis` requires filtering by the declared axis with neither vocabulary reaching the core, and [the lifecycle record](2026-07-31-lifecycle-declaration-and-the-seam.md) makes filtering a no-axis corpus an error diagnostic while rejecting magic value words. `show-returns-document-model` fixes the result shape. The identifiers these surfaces raise are permanent public API, so their namespace is a decision.

## Decision

### Grammar and shared behavior

```
dogtag check [--vault <name-or-path>] [--format text|json] [--strict]
dogtag list  [--vault …] [--format …] [--strict] [--type <name>] [--tag <tag>]
             [--lifecycle <value>] [--ordinary]
dogtag show <ref> [--vault …] [--format …] [--strict] 
```

All three refuse on an unresolved contract exactly as `contract explain` does — the diagnostics, exit `1`, and a pointer at `doctor`. Reading a corpus against rules that did not resolve would be the same fiction the M2 record refused to hand an agent. Results go to standard output; in text mode diagnostics go to standard error; in JSON mode the output is one document carrying the result and the diagnostic envelope together, on the structured-output schema's own clock (the report shapes added here bump that schema version — its clock exists for exactly this). Diagnostics are sorted by the standing total order.

`--strict` behaves identically everywhere: it reaches the one severity-promotion predicate and nothing else. An `info` finding is never promoted, which is load-bearing for the cutover — [the document-model record](2026-08-03-m3-document-model.md) puts the founder vault's permanent prefix-gap findings at `info` precisely so `check --strict` on a clean corpus exits `0`.

### The two new diagnostic areas

**M3 mints `note` and `link`.** `note.*` is a single note's own structure — typing, required properties, kind lexical validation, undeclared keys, tag-namespace enforcement, frontmatter and encoding faults. `link.*` is resolution between notes — dangling typed links, ambiguous references, unresolvable targets. The split follows the semantic concern rather than the command, because the same finding must carry the same identifier whichever door raises it — `check`, `show`, the TypeScript binding, or the M6 MCP server. A per-command area would fork identifiers across doors, and a single `corpus` area would repeat the note/link noun in every slug anyway. The full enum lands under the same review obligation the M2 identifier set carried.

### `check`

`check` walks the corpus under the document-model record's traversal rule, validates every note against the resolved contract, resolves every typed link, and reports the aggregate: the diagnostic list in total order plus summary counts per severity and per identifier. It writes nothing. Exit follows severity alone, so a corpus whose only findings are `info` is a passing corpus — that sentence is the scheduled cutover's contract with this command.

`check` does not restate `doctor`: contract and installation problems surface through the shared loading path as they would for any command, but the discovery/installation report is `doctor`'s job, and `check` on a broken vault refuses rather than half-diagnosing. The two commands answer the two questions the M2 record separated — "is my vault configured coherently?" and "is my corpus healthy?" — and M3 is the milestone at which the second finally has an answerer.

### `list`

`list` enumerates notes with four composable filters, ANDed: `--type <name>` (the declared discriminator value), `--tag <tag>` (literal, exact, full-tag match), `--lifecycle <value>` (the axis property equals the given value), and `--ordinary` (the ordinary state, whatever its declared encoding — named value or absence). `--lifecycle` and `--ordinary` are mutually exclusive; either against a contract declaring `none = true` is the lifecycle record's error diagnostic. `--ordinary` is its own flag because a literal `ordinary` value word would reserve vocabulary — the magic-string shape the lifecycle record rejected — and because the flag is what lets both encodings answer one question without either vocabulary reaching the core, which is the seam the M0 scenario exists to test.

Output per note: the vault-relative path, the bound type, and the axis value where an axis is declared — one line per note in text, an array of document-model summaries in JSON. Sorted by path. The title is not fetched at M3: listing must not require opening every body, and the H1 is display metadata `show` supplies.

`--tag` is deliberately exact-match. A prefix query (`log/…` as a family) is real and unshipped; a namespace-aware filter belongs to the milestone that needs it, and is recorded as absent rather than half-shipped.

### `show`

`show <ref>` accepts a vault-relative path (anything containing `/`, `.md` optional) or a bare name. A path resolves exactly; a bare name resolves iff unambiguous, and ambiguity is the `link.ambiguous-reference` error listing every candidate as related evidence — the markdown-flavor model surfaced verbatim, and the CLI demonstration of the M0 ambiguity scenario. The result is the document-model shape: identity, type and how it bound, properties, relationships, tags, body, title. Text rendering is SDK-owned like every other rendering, for the M2 record's reason — three consumers must not grow three notions of what a note looks like.

### Alternatives considered

- **A single `corpus` diagnostic area.** Rejected: every slug repeats the noun the split carries structurally.
- **Per-command areas (`check.*`).** Rejected: identifiers would fork across doors for one finding.
- **`--lifecycle ordinary` as a value word.** Rejected: reserves vocabulary; superseding the lifecycle record's rejection was not warranted by saving one flag.
- **`--type`-only filtering, deferring `--tag`.** Rejected: the tags construct would land with no read-path consumer, and subtype-scoped listing is what note-types names as `list`'s job.
- **Path-only `show`.** Rejected: daily reading would be worse than the incumbent it must displace, and the cutover is the milestone's evidence.
- **`check` re-reporting the full `doctor` sections.** Rejected: two commands owning one report drift; the loading path already surfaces what refusal needs.
- **Titles in `list` output.** Rejected for M3: it makes listing cost a full-corpus body read.

## Consequences

- **`info` findings are invisible to every exit code, permanently.** A consumer that wants them fatal cannot get that from the CLI; that is the designed trade, and the linter-over-the-SDK path is the answer.
- **The JSON report schema version bumps**, and `check`'s report shape — like `doctor`'s before it — is what the cutover's parallel-run triage will diff, so it settles in the first implementation commit rather than accreting.
- **Bare-name `show` makes corpus ambiguity user-visible** in daily work, which is the design intent: ambiguity belongs to the reference that meets it.
- **`--tag` exact match will surprise namespace users** expecting prefix semantics; the limit is stated in this record and the help text should state it too.
- **Three commands share one loading, traversal, and validation path**, so a policy change in the document model reaches all three identically — which is also why none of them may own a private variant of it.

## Amendments

The Decision above stands as written; these later entries change parts of it, and the original text is left intact so the change is legible.

- **2026-08-04 — the structured-output schema version ticked at slice 2, not at this record's surfaces, and the tick is per-milestone.** The Consequences attribute the JSON schema bump to the report shapes added here; the field set actually changed first when the tag vocabulary joined the resolved model, so `SCHEMA_VERSION` moved to 2 there. Adjudicated as one tick per milestone rather than one per shape change — the surfaces this record adds ride the same version. A consumer pinning the schema version sees one bump for the whole of M3.
- **2026-08-04 — `show` carries the whole corpus's diagnostics, not the shown note's alone.** This record fixed `show`'s result shape but not its diagnostics scope, and the slice-8 trial implementations split — two carried the shared corpus read's diagnostics (an unrelated broken note fails the run), one filtered to the shown note (the same vault exits 0). Adjudicated for corpus scope, with the merged implementation: `show` renders through the one shared loading/traversal/validation path, and that path's diagnostics are the run's diagnostics; the severity-to-exit mapping applies to them unchanged. A reader who wants one note's problems in isolation reads the note-scoped evidence inside the report, not a filtered exit code.
- **2026-08-04 — refusal shape is a family rule, stated once: the unresolved contract refuses everywhere exactly as `contract explain`; a run that proceeds emits one JSON document with explicit nulls.** The per-surface refusal texts left the composition implicit, and the trial produced three variants. The rule the merged surfaces obey: an unresolved contract refuses identically on every surface — diagnostics, exit 1, pointer at `doctor`, no structured document; past that gate, JSON mode answers with exactly one document per run, absent results as explicit `null` fields (`show`'s missing note is `"note": null`), and text mode routes diagnostics to stderr with results on stdout. The next surface inherits this rule rather than a precedent to reinterpret.
- **2026-08-05 — `show <ref>`'s path spelling includes the separator-less root-level path.** "Anything containing `/`, `.md` optional" left a root-level note's path parsing as a bare name that names nothing. The reference grammar is amended at its home — [the document model record](2026-08-03-m3-document-model.md#amendments): a trailing `.md` marks a reference path-qualified exactly as a `/` does. `show welcome.md` and `show welcome` now both resolve the root-level note.
