# The M4 release and the search cutover

- Status: accepted
- Date: 2026-08-06

## Context

M4's prerelease, nominally `0.1.0-beta.3`, ships lexical retrieval: the `search` and `find` surfaces ([the surfaces record](2026-08-06-m4-surfaces-search-and-find.md)) and the `docs` corpus with the retrieval scenario set ([the fixtures record](2026-08-06-m4-fixtures-and-conformance.md)). The cutover rule from [beta.md](../../beta.md#required-properties) applies as amended at M2: each milestone moves one real workflow onto the installed binary with seven days of parallel running, the incumbent retires, and an obligation that turns out to have no incumbent rolls forward with an honest receipt.

This packet was frozen 2026-08-06, while M3's three cutover clocks were still running (earliest M3 close 2026-08-12). That is the M3 precedent applied deliberately: a packet freezes when its decisions are ready, and implementation follows the packet — milestone activation gates the roadmap's acceptance-criteria section, not the work.

## Decision

### Acceptance criteria

1. `search` and `find` implemented per the surfaces record, over the shared loading, traversal, and validation path, with the family refusal and one-document rules intact.
2. The M4 scenario set executable and green: the universal set on `dense`, `starter`, and `docs`; the `docs`-only set on the new corpus; the matrix distinguishing ran from pending throughout.
3. The `docs` corpus landed under the fixtures record's derivation and gate: numeric shape artifact, source-blind authoring, coincidence read-back receipt recorded, artifact deleted.
4. `contract_version` stays 2, affirmed here so the non-motion is a decision rather than a silence; the supported range does not move. The JSON report schema takes its one tick for the milestone when the first M4 report shape lands, per the M3 amendment's per-milestone rule.
5. The scripted smoke sequence extended with search and find steps over the fixture corpora, green before the tag.
6. The prerelease published from a passing tag and installed from the public release with verified attestations on macOS and Linux, the M2/M3 verification sequence unchanged.
7. The search cutover complete: one part, seven days of parallel running, the incumbents retired, the named residue recorded — as specified below.

### The cutover: one part, with named residue

Day one repoints the daily search paths to the installed binary: the interactive `pkm search` / `pkm find` habit and the skills' search paths — the residue that part two of M3's cutover explicitly left with the incumbent. Triage during the parallel week follows the M3 evidence design: local `--format json` output diffed against the incumbent's answers for the same queries, receipts without vocabulary, and a report-changing fix restarting only this part's clock.

After seven clean days the local incumbents retire: `pkm search`, `pkm find`, and the `reindex` verb's local scheduling.

**Named residue, carried to M6:** the claude.ai Dogtag connector is served by the incumbent's server-side FTS index, and the SDK-backed MCP server that could replace it is M6's deliverable — it cannot repoint earlier. The index infrastructure therefore survives M4 scoped to serving the connector, and `pkm tags`, whose counts ride that index, survives with it. Both are residue by name, with M6 as the recorded destination, so the retirement receipt can honestly say what stayed and why.

The cutover receipt also records the measured search latency on the founder vault at cutover time, which is the standing check on the surfaces record's scan-versus-index trigger.

### Alternatives considered

- **Two parts (search, then find).** Rejected: the two verbs share consumers and one triage; a second clock doubles bookkeeping without adding evidence.
- **Deferring the cutover to M5.** Rejected: it violates the dogfooding rule — M4 would ship and produce no E0 evidence, the failure the rule exists to prevent.
- **Retiring the server-side index at M4.** Rejected as impossible without breaking the connector; naming the residue is the honest form of the same intent.
- **Waiting for M3 to close before freezing this packet.** Rejected: the clocks gate M3's receipts, not M4's decisions; M3's own packet froze while M2's cutover question was open.

## Consequences

- **These criteria land in the public roadmap when M4 becomes the active rung** — on M3's close, expected 2026-08-12 at the earliest — from this record, unchanged; the roadmap section is a mirror, not a second source.
- **The parallel week runs both search stacks side by side**, so daily work briefly answers every search twice; that cost is the evidence.
- **The residue means M4's retirement is partial by design.** The receipt must say so plainly; a reader comparing M4's receipt to M3's should see a named difference, not a quiet one.
- **If the parallel week surfaces a resolution or latency regression against the incumbent, the cutover does not complete** — the milestone holds open on criterion 7, which is the designed failure mode rather than a schedule slip.
