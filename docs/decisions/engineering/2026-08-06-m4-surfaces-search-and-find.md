# The M4 surfaces: `search`, `find`, and retrieval without an index

- Status: accepted
- Date: 2026-08-06

## Context

M4 ships lexical retrieval over the common model: basic search and entity lookup ([beta.md](../../beta.md#milestones)). The boundary questions are inherited, not reopened: the SDK owns the semantic model and every rendering, the CLI owns argument parsing and environment resolution ([the M2 surfaces record](2026-07-31-m2-surfaces-and-the-sdk-boundary.md), as amended); severity alone maps to exit codes and `--strict` reaches only the promotion predicate ([the diagnostics record](2026-07-31-diagnostics-and-compatibility.md)); the family refusal rule and the one-JSON-document rule apply to every new surface as written ([the M3 surfaces record's amendments](2026-08-03-m3-surfaces-check-list-show.md#amendments)); references parse under the document model's grammar, including the trailing-`.md` path-qualification amendment ([the document-model record](2026-08-03-m3-document-model.md#amendments)).

The incumbent these surfaces displace is `pkm search`: SQLite FTS5 with BM25 ranking, OR-by-default multi-term queries, quoted phrases, explicit `AND`, trailing-`*` prefix, date bounds, a fileClass filter, and a persistent index with a `reindex` verb. The cutover rule requires daily search to move onto the installed binary and not move back, so the M4 surface must be good enough to displace that incumbent for interactive and agent use — which is a bar about resolution and latency, not about feature parity.

## Decision

### Grammar and shared behavior

```
dogtag search <query> [--vault <name-or-path>] [--format text|json] [--strict]
                      [--type <name>] [--tag <tag>] [--lifecycle <value>] [--ordinary]
                      [--limit <n>]
dogtag find <name>    [--vault …] [--format …] [--strict] [--type <name>]
```

Both surfaces inherit the family rules unchanged: refusal on an unresolved contract exactly as `contract explain`, one JSON document per run with explicit nulls, diagnostics to standard error in text mode, the standing total order, and `--strict` reaching the promotion predicate and nothing else.

### `search`

`search` matches the query against a note's body and its identity — vault-relative path, title, and aliases. The query grammar at M4 is the floor the incumbent's daily use actually exercises: bare multi-term queries are OR-combined and relevance-ranked, `"quoted phrases"` match adjacent words in order, and a trailing `*` is a prefix wildcard. Explicit `AND`, date bounds, tag-text matching, and typed-link-target matching are recorded as absent rather than half-shipped — the `--tag`-prefix precedent — and accrue to the milestone that needs them.

`list`'s four filters compose with the query, ANDed, under `list`'s own rules: `--type`, `--tag` (exact match), `--lifecycle`, `--ordinary`, with the same mutual exclusion and the same no-axis error diagnostic. Search is enumeration plus a text predicate; it must not grow a second filter vocabulary.

Output is relevance-ordered with a deterministic tie-break on the vault-relative path, so identical runs produce identical bytes. **Ordering is not contract.** Conformance scenarios assert membership and count; the record states this so ranking can improve without an amendment. Per hit: the vault-relative path, the bound type, and a matched-context snippet — one line per hit in text, an array of hit objects in JSON. `--limit` defaults to 20.

`search` mints one narrow diagnostic area, `search.*`, scoped to query-expression faults (an unbalanced quote is `search.invalid-query`, an error). An empty result is a result, not a diagnostic: exit `0`, empty membership.

### Retrieval is a corpus scan, not an index

Each search walks the corpus through the same shared loading, traversal, and validation path `check`, `list`, and `show` use. No persistent index, no staleness semantics, no `reindex` verb, no write path in a read milestone.

The evidence: the full walk over the larger founder vault — roughly 13,800 notes, validation and body-link scanning included — measured 0.44 seconds, compute-bound, on 2026-08-06. Text matching on top of that walk keeps interactive search comfortably under one second on the worst corpus the project can currently observe. The trigger is recorded with the decision: **if a measured search on a real vault exceeds one second at the median, the persistent index lands at M6**, where the MCP server wants a resident process anyway. Skepticism about the scan is on the record and answered by measurement per vault, not by argument.

### `find`

`find <name>` is M3's reference resolution exposed as a verb: a case-insensitive match over note basenames and aliases, narrowed by `--type` when given. An unambiguous match returns the document-model summary. An ambiguous one raises `link.ambiguous-reference` with every candidate as related evidence — the same identifier, shape, and severity `show` raises, because it is the same semantic event at a different door. `find` mints no diagnostic area of its own.

A caller who wants the candidate list *is served by the diagnostic*: the enumeration is the related evidence, surfaced verbatim. The exit code still follows severity, which keeps one ambiguity contract across `show`, `find`, and every future door rather than a per-verb interpretation.

### Alternatives considered

- **Full incumbent parity (AND, date bounds, tag text, link targets, `--explain`).** Rejected: front-loads contract surface the milestone doesn't need; each absent form is named above rather than implied.
- **Body-only matching.** Rejected: a search that cannot find a note by its own title loses the cutover comparison on day one.
- **A persistent FTS index at M4.** Rejected on measurement: the scan meets the interactive bar on the largest real corpus, and the index brings location, invalidation, and a write path into a read-only milestone. Deferred with a named trigger, not refused.
- **Ranking as contract.** Rejected: scenarios would pin an ordering the fixture corpora are too small to make meaningful, and every ranking improvement would become an amendment.
- **`find` as a `search` mode flag.** Rejected: exact resolution and ranked matching are different contracts, and the incumbent habit being displaced (`pkm find`) is a distinct verb.
- **`find` treating ambiguity as a successful multi-result.** Rejected: it would fork the ambiguity contract `show` fixed; the diagnostic's evidence already carries the candidates.
- **A richer entity surface (relationships, backlinks in one shape).** Rejected for M4: a synthesis surface with no consumer until M6; it would front-run the model.

## Consequences

- **The scan makes every search a full validation pass**, so a broken corpus surfaces its diagnostics on every retrieval — consistent with `show`'s corpus-scope amendment, and occasionally noisy by design.
- **Search latency scales with corpus size and nothing bounds it but the trigger.** The one-second median trigger must actually be checked when real vaults grow; the cutover receipt records the measured latency at cutover time.
- **Ordering not being contract means two conforming builds may order differently.** Determinism is still required per build, so triage diffs stay byte-stable within a version.
- **The absent grammar forms will be asked for** — `AND` by agents porting FTS5 habits first. The help text should name what is absent, as `--tag`'s exact-match note does.
- **`find`'s error-on-ambiguity will read as failure to callers expecting a match list**; the record and help text say where the list lives.

## Amendments

The Decision above stands as written; these later entries change parts of it, and the original text is left intact so the change is legible.

- **2026-08-06 — a note's aliases are the values of the declared property named `aliases` on its bound type.** The Decision has retrieval match "aliases" without saying where a note's aliases live. Adjudicated with the implementation: the kernel reads the property a type declares under that name — scalar as one alias, list as each element — per type, the tag-property precedent, so the values surface only where a type declares the property and an undeclared `aliases` key never reaches the model. The cost is on the record rather than denied: a corpus that declares `aliases` meaning something else is still matched by it, which is what a `[tags]`-style contract seat would avoid; the convention stands until a record seats or overturns it.
- **2026-08-06 — the case planes are split: text matching is case-insensitive, path resolution is not.** `search` matches words case-insensitively — the incumbent behavior daily use established, and the rule this record already fixed for `find`'s name plane — while a path-qualified reference resolves under the document model's exact rule on every surface. One verb answering both planes obeys both at once.
- **2026-08-06 — a query naming no word is `search.invalid-query`.** An expression with nothing to match is a query-expression fault, not an empty result: `search '!!!'` exits 1 naming the query rather than 0 hiding the caller's error.
- **2026-08-06 — body matching reads the body as uninterpreted text, fenced code included.** Distinct from M3's fence-aware body *link* scanning, which stands: link extraction skips code fences so an example cannot claim an edge; text matching reads them so a search can find the flag or identifier a docs corpus keeps inside one. Different concerns, different rules, both deliberate.
- **2026-08-06 — the snippet is rendering, not contract.** Per hit: the note's own text around the earliest body match with ellipses marking cut edges; the matched alias where only an alias matched; nothing where the path alone matched, which the hit already names. Conformance asserts membership and count — the ordering rule's own boundary — so snippet improvements never need an amendment.
- **2026-08-06 — `find`'s answer is the established document-model summary.** "The document-model summary" reads as M3's term: `list`'s summary shape (path, type, lifecycle), rendered as `list` renders a line — not `show`'s full document rendering, which stays `show`'s.
- **2026-08-06 — a type-narrowed miss keeps `link.target-not-found` and says what the filter did.** A name only the `--type` filter excluded refuses with the same identifier and severity, its message naming the type that emptied the candidate set — message honesty without a second identifier for the same semantic event.
- **2026-08-06 — an alias equal to another note's name is ordinary ambiguity.** Both bearers are candidates in the one refusal; the name plane does not rank declarations above filenames.
- **2026-08-06 — `find` validates through the full corpus read; its answer stays name-based.** As first merged, `find` walked `list`'s body-free summary read, so a vault broken only by a finding prose alone can raise exited 0 at `find`'s door while `search`, `show`, and `check` exited 1 on the same corpus — a fork the cutover's parallel-run triage would read as a resolution regression, confirmed against the built binary. Amended to the record's own sentence: both retrieval verbs read through the one shared loading, traversal, and validation path, so every door agrees about a corpus's health; the *answer* remains the body-free summary, bodies never participate in name matching, and `list` alone keeps the body-free license M3 granted it.
