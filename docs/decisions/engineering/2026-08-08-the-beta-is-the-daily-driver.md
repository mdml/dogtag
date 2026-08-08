# The beta is the daily driver

- Status: proposed
- Date: 2026-08-08

## Context

[beta.md](../../beta.md) promises "the complete product lifecycle with a deliberately small operational kernel," and its In scope line reads "one plan/validate/apply mutation, **initially** `capture`." That word promised a sequence the ladder never continues: M6 delivers bindings and an MCP server, M7 second-vault onboarding, M8 upgrade and the verdict. No rung mints a typed note, edits one, or imports an attachment.

Nor are those deferred. The Deferred list consciously excludes Python bindings, TUI and MCP as product surfaces, saved views, semantic search, general import, dialect materialization, and merge UX — and says nothing about typed authoring or editing. So the largest remaining surface is neither scoped nor excluded; it fell between the two, and "initially" is what papers over the gap.

M5 made the gap concrete. Its cutover named the maintainers' authoring skills as the incumbent and [was rolled forward](2026-08-07-m5-release-and-cutover.md#amendments) when they turned out to delegate to a **typed-note stamper** — filing, not capture. The two verbs that stamper backs, `pkm note new` and the `edit-note` path, are the maintainers' most-used vault operations by a wide margin, and under the current ladder they stay on the incumbent tooling for the entire beta.

That is the real cost, and it is not about a missing feature. The beta's [ship test](../../beta.md#ship-test) asks that "both complete a real CLI write workflow" and that "every workflow moved onto Dogtag by the per-milestone cutover rule is still there." `capture` satisfies the first literally and thinly, and the cutover rule never reaches the operations that carry daily use — so **the beta can pass its own test while the maintainers still live in the incumbent tooling.** A test that can be satisfied without the product being used is the failure mode the cutover rule exists to prevent, one rung up.

## Decision

**The beta is the daily driver.** Adjudicated by the maintainer 2026-08-08: the beta's purpose is not to prove a kernel and hand daily work back to the incumbent tooling, but to take that work over. The Promise's "deliberately small operational kernel" described the M0–M5 rungs honestly and stops being the whole story here; the small kernel is the *foundation* the authoring surfaces are built on, not the boundary of the beta.

### Three surfaces stand between `capture` and daily driving

Each is a `pkm` verb in constant use, and each is a distinct packet:

1. **Typed creation.** `pkm note new` stamps fileClass and subtype, frontmatter, required tags, a body template, a humanized `# Title`, and a born status. `capture` deliberately does none of that — [the inbox-workflow PDR](../product/inbox-workflow.md) holds capture and filing apart, and this is the filing half.
2. **Editing.** In-place mutation under [the markdown-flavor PDR](../product/markdown-flavor.md)'s rule that a writing surface never alters bytes it did not semantically touch, plus lifecycle transitions.
3. **Attachment import.** The residue [M5 named homeless for the third time](2026-08-07-m5-release-and-cutover.md), with twenty-one items waiting in the maintainer's inbox store.

### The ladder gains three rungs

| Rung | Delivers | Cutover |
| --- | --- | --- |
| **M6** | TypeScript SDK and the MCP server — **unchanged** | `capture`, per [the M5 amendment](2026-08-07-m5-release-and-cutover.md#amendments) |
| **M7** | Typed creation and triage: `new`, and filing what `capture` leaves unfiled | `pkm note new` |
| **M8** | Editing: `edit`, and lifecycle transitions | the `edit-note` path |
| **M9** | Attachments: `import` | `pkm attachment import` |
| **M10** | Second-vault onboarding — was M7 | |
| **M11** | Upgrade and beta verdict — was M8 | |

**M6 does not move**, so the capture cutover scheduled there survives this change untouched. **Creation precedes editing** because M5 deliberately left plan scope, collision, and atomicity unmade: editing is the first mutation that *must* decide them, and creation is the cheaper packet that proves the shared machinery first. **Import comes last** because it is the most self-contained of the three and its residue is already documented as waiting.

### What this obliges

- The Promise's kernel sentence and its six-step lifecycle gain the authoring step; In scope's one-mutation line becomes the mutation sequence the ladder actually walks.
- The ship test's write clause names the authoring workflows, so it can no longer be satisfied by `capture` alone.
- Each new rung carries the ordinary cutover obligation, and for the first time in this beta those cutovers retire verbs the maintainers use every day — which is the point, and also the risk.

### Alternatives considered

- **Defer typed authoring to v1 and say so in Deferred.** The cheapest honest fix, and rejected on the adjudication: it makes the beta a kernel proof whose ship test passes while the incumbent keeps the daily work, which is the failure this record names.
- **One authoring rung covering create, edit, and import.** Rejected: they share mutation machinery but not their hard problems — creation's is templates and per-type stamping, editing's is byte preservation, import's is binary movement and layout. One packet would decide all three badly.
- **Insert authoring before M6.** Rejected: it would strand the capture cutover just scheduled at M6, and the MCP server is itself part of daily driving, since agents are how these vaults are operated.
- **Leave "initially" to carry the sequence.** Rejected as the status quo this record exists to end: a promise with no rung is not a plan.

## Consequences

- **The beta gets substantially longer** — twelve rungs where there were nine, and the three new ones are mutation packets, the most expensive kind this repository writes.
- **The verdict moves further out**, so the E0 graduation decision [strategy.md](../../strategy.md) frames arrives later, with more evidence behind it.
- **Three cutovers now retire load-bearing daily verbs.** A fidelity regression in any of them stops that milestone, and unlike the read cutovers, a bad week here costs the maintainers real authoring friction rather than a fallback command.
- **`pkm` retires incrementally rather than at once**, and the intermediate states — dogtag creating, `pkm` editing — must both work against one vault throughout.
- **This record does not design the three surfaces.** Each needs its own packet, and the hard questions M5 parked land in M8's.
