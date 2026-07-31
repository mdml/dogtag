# The M2 surfaces: `doctor`, `contract explain`, and where rendering lives

- Status: accepted
- Date: 2026-07-31

## Context

M2 ships two commands, `dogtag doctor` and `dogtag contract explain` ([beta.md](../../beta.md#milestones)). The second has an unusual obligation attached: [architecture.md](../../architecture.md) makes the vault's agent contract **generated rather than written**, because *"hand-maintained agent instructions always eventually lie, and the cost of that lie compounds as agents do more of the writing,"* and [beta.md](../../beta.md#in-scope) describes `contract explain` as *"rendering the resolved contract as agent-readable instructions so they cannot drift from it."* That makes its Markdown output an artifact whose entire purpose is non-drift, not a convenience view.

Two earlier decisions constrain the shape. `doctor` must report discovery and installation facts even when the contract is unusable ([diagnostics and compatibility](2026-07-31-diagnostics-and-compatibility.md)), and the settings PDR rejects process-global vault state.

## Decision

### `dogtag doctor`

`dogtag doctor [--vault <name-or-path>] [--format text|json] [--strict]`.

**It opens exactly two files: `.dogtag/contract.toml` and the installation record.** No note is read and no directory under the vault root is enumerated. It reports the resolved root and how it was resolved, contract presence, parse, and version classification, capability enumeration and cardinality, the lifecycle declaration's consistency, the declared dialect, and the installation record's presence and validity. It writes nothing, anywhere. `--strict` promotes warnings for exit purposes only, which is what lets the scheduled cutover check detect a wrong-vault resolution from its exit code alone.

**It reports the registry entry for the resolved vault, not the whole registry.** The full inventory sits behind an explicit flag. A registry enumerates every vault a user has registered, by user-chosen name and absolute path — and dogtag is agent-facing by design, so `doctor --format json` would otherwise pull a complete vault inventory, and the home-directory layout with it, into an agent's context and its provider's logs in answer to a question about one vault. The same output is what gets pasted into a bug report during the seven-day parallel run.

Not enumerating notes is where the M2/M3 line actually lives. The moment `doctor` counts notes it needs a traversal policy — what counts as a note, which directories are skipped, how a derived index directory and the version layer's own directory are treated, whether symlinks are followed — and every one of those is an M3 decision that would get frozen by a guess made to populate a progress counter.

The consequence worth stating plainly: **what moves onto installed dogtag at M2 is the vault's configuration health check, not corpus linting.** Corpus checks move at M3 with `check`.

### `dogtag contract explain`

`dogtag contract explain [--vault <name-or-path>] [--format markdown|json] [--provenance]`.

- **`markdown` is the default** and is the generated agent contract — the rendering `architecture.md` requires not to drift from the resolved contract. It reads well enough for a human that no third rendering is needed. Its preamble names the resolved vault root, so an agent consuming piped output receives the provenance along with the instructions; printing the root to a terminal protects only a human who is watching.
- **`json`** is the complete resolved model, always carrying per-leaf provenance.
- **`--provenance`** annotates the Markdown with each value's source and location. It is opt-in because the Markdown's job is instructing an agent, and a source annotation on every line makes it materially worse at that.

**When the contract does not resolve, `contract explain` refuses** with the diagnostics and an exit code of 1, and points at `doctor`. Explaining a contract that did not resolve would be a fiction handed to an agent as the vault's rules.

At M2 `contract explain` writes to standard output only. Generating the vault's `AGENTS.md` on disk is a write, and belongs to the milestone that performs writes.

### Rendering ownership

**The SDK owns the Markdown rendering, the JSON serialization, and a plain-text diagnostic renderer. The CLI owns argument parsing, environment and current-directory resolution, registry-name resolution, colour, stream routing, and the mapping from severity to exit code.**

This is the same boundary as *"the CLI consumes only the public API,"* applied to the artifact where drift would be most costly. If the CLI owned the Markdown rendering, the MCP server and the TypeScript binding would each grow their own, and an agent would receive a different vault contract depending on which door it entered — which is exactly the drift generation exists to prevent. The cost is real: the SDK's public surface grows rendering, which is not obviously a kernel concern until you ask what happens when there are three consumers.

### The SDK's vault API

**Discovery and contract resolution are separate operations.**

```
discover(start)                   -> Discovered { root, diagnostics }
root_at(path)                     -> Result<VaultRoot, Diagnostic>
resolve_registered(name, record)  -> Result<VaultRoot, Diagnostic>
open(root, installation)          -> Opened { root, installation, contract: Result<…>, diagnostics }
```

The three entry points and the opaque `VaultRoot` are [the discovery record](2026-07-31-vault-discovery-and-selection.md)'s; the split matters here because `--vault <path>` must be verified rather than searched, and because discovery emits diagnostics on success.

`Opened` always carries the root, the installation record's state, and the diagnostic list; the resolved contract is a `Result` inside it. Semantic operations take a resolved contract and cannot be reached without one.

A single atomic `open() -> Result<Vault>` cannot produce the report `doctor` is required to produce when the contract version is out of range — the root and registry facts would be lost inside the error. An infallible `open()` carrying diagnostics has the opposite defect: nothing in the type system would stop a caller acting on an unresolved contract, which is caller-owned semantic reinterpretation arrived at by omission rather than by decision.

### Alternatives considered

- **Three `contract explain` formats — text, markdown, json.** A compact human summary distinct from the agent rendering. Rejected: three renderings of one model, and the human and agent versions drift apart precisely because nothing forces them to agree.
- **JSON only at M2, with the Markdown rendering deferred.** Rejected: `beta.md` names the agent-readable rendering as the command's reason for existing, so deferring it ships the command without its purpose.
- **Always showing provenance in the Markdown.** One rendering to keep correct, provenance never hidden. Rejected: it degrades the artifact whose job is instructing an agent.
- **Explaining partially when the contract fails to resolve.** An agent wants as much as it can get. Rejected: a partially-resolved contract presented as the vault's rules is more dangerous than no answer, and `doctor` already exists for broken vaults.
- **The SDK owning the model and the CLI rendering everything.** Keeps the SDK surface minimal. Rejected: three consumers, three agent contracts.
- **The SDK owning JSON while Markdown lives in the CLI.** Rejected: it shares the half that does not need sharing and keeps the half that does.
- **`doctor` counting notes and reporting corpus size.** The first thing anyone asks for. Rejected: it decides M3's traversal policy by accident, for a counter.
- **One atomic `open()`**, or **one infallible `open()`.** Rejected as reasoning above.

## Consequences

- **`doctor` cannot answer "is my corpus healthy?" at M2** — only "is my vault configured coherently?" The cutover criteria say which of the two moves, and anyone reading "the daily vault health check moved" without that qualifier will overestimate what shipped.
- **The SDK's public surface now includes rendering**, which will look like a layering mistake to anyone who arrives without the non-drift argument. This record is the answer to that review comment.
- **The `Opened` shape makes the partial-state case ordinary rather than exceptional**, which is right for a diagnostic tool and slightly verbose for every other caller, who must unwrap a `Result` inside a struct.
- **Two renderings must stay semantically equal** — every declaration in the Markdown must appear in the JSON and vice versa. That is an acceptance criterion, not a hope, and the `contract-explain-renders-every-declaration` scenario is what holds it up.
- **`contract explain` writing to standard output only** means the generated-agent-contract obligation is demonstrated but not yet delivered to a vault; the file that must not drift does not exist on disk until a later milestone writes it.
