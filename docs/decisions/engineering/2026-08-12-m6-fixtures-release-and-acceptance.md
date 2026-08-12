# M6 fixtures, release, and acceptance

- Status: accepted
- Date: 2026-08-12

## Context

M6 must prove one Rust implementation through Rust and TypeScript, exercise a deployed MCP workflow, generate the agent contract, publish both native and npm artifacts, and discharge inherited connector residue. It consumes existing vault configuration and therefore owes an explicit contract-version decision. The committed fixtures currently straddle versions 2 and 3; that preserved floor evidence but makes the current corpus state harder to read as milestones accumulate.

## Decision

### Version and fixtures

`contract_version` stays **3**. M6 adds no vault-shared configuration seat: binding, transport, authentication, package distribution, and deployment are installation or build concerns, while agent-contract generation consumes the already resolved contract.

All committed fixture corpora move to version 3 for one legible current state. Compatibility evidence does not disappear: the harness derives version-1 and version-2 variants and runs every applicable language-neutral scenario through them in both Rust and TypeScript. These transformations are a required, printed matrix dimension rather than profile waivers.

The downgrade transformations are deterministic and checked. They may change the `contract_version` declaration and remove only contract seats that do not exist at the target version, including the version-3 capture and birth-state seats and the version-2 tag-vocabulary and record-kind seats where applicable. They may not change note bytes, paths, or any contract value legal at both source and target versions. The harness asserts that invariant before running scenarios and prints the transformed version as a matrix dimension. A scenario whose required construct genuinely does not exist at the target version is derived into the equivalent target-version condition by the scenario's existing transformation machinery, never skipped and never satisfied by deleting unrelated semantics. A version floor remains supported only while these derived variants execute and pass.

MCP protocol scenarios are a separate integration layer over the same `dense`, `starter`, and `docs` corpora. They cover tool discovery, argument refusal, structured results, fixed-vault isolation, HTTP authentication, concurrent requests, capture preview and apply, and transport parity where applicable. Every applicable scenario runs against every profile with no waiver. No fourth MCP-specific corpus is added because the protocol introduces no new corpus shape.

### Report schema and release identity

The CLI's JSON report schema takes its one M6 tick, 4 to 5, when `contract generate-agents` lands. MCP result schemas do not consume additional CLI schema ticks. Rust crates, CLI archives, the root npm package, and native npm packages share the same prerelease version, nominally `0.1.0-beta.5`.

### Acceptance criteria

M6 is complete when:

1. The napi-rs binding and published package satisfy the binding record, with clean installs and runtime tests on every claimed platform/runtime pair.
2. Every applicable semantic scenario passes through Rust and TypeScript over the three current corpora and their derived version-1 and version-2 variants, with no waiver.
3. The four-tool MCP server satisfies the server record over stdio and authenticated HTTP, including concurrent-request and fixed-vault tests.
4. `contract generate-agents` satisfies the generation record; committed fixture outputs and the founder-vault non-drift gate are green.
5. The CLI report schema moves exactly once to 5; `contract_version` remains 3 and the supported range remains `1..=3`.
6. The scripted smoke sequence covers an npm install, direct TypeScript search/show/capture preview, agent-contract generation, stdio MCP, and authenticated HTTP MCP, and is green before the tag.
7. `just gate` passes 20/20 in an independent worktree, `just smoke` passes, every changed CodeScene-supported file scores 10.0, and any coverage-baseline movement follows the marked proposed-and-ratified path.
8. `0.1.0-beta.5` is published from a passing tag; CLI archives and npm packages are installed and verified on macOS and Linux, including attestations and native-package selection.
9. The existing claude.ai connector is repointed to the SDK-backed server; its search/show/tags handlers, persistent FTS database, reindex scheduling, and `pkm tags` retire with a receipt. The fzf picker remains named TUI residue.
10. MCP capture has real-use evidence and M5's rolled-forward obligation closes with the honest receipt that no incumbent capture mechanism existed. Attachment handling remains M9 residue.
11. Local and deployed MCP search medians are recorded; a median above one second opens the persistent-index decision before closure.

### Cutover evidence is goal-shaped, not calendar-shaped

The inherited seven-day periods were heuristics intended to prevent a single successful invocation from masquerading as adoption. M4 demonstrated that elapsed time alone was not the evidence: the useful receipt was mechanism verification, real queries, measured latency, classified behavioral differences, and explicit residue. M6 therefore does not prescribe a calendar minimum. Its connector cutover closes when the acceptance evidence above exists and no unresolved defect or fallback remains. A behavior-changing fix invalidates the affected evidence and requires it to be rerun; it does not restart an arbitrary clock.

### Alternatives considered

- **Bump contract version 4.** Rejected: no new contract seat has a consumer.
- **Keep committed corpora split across versions.** Rejected: the rationale becomes progressively less legible; derived variants preserve the compatibility proof mechanically.
- **Unconstrained downgrade fixtures.** Rejected: a transformation could make an old-version corpus pass by simplifying the very semantics under test; the byte- and shared-seat invariants make that visible.
- **Use only TypeScript smoke tests.** Rejected: the binding promise is shared behavior, which requires the common scenario set at the language boundary.
- **Treat raw MCP exchanges as SDK conformance.** Rejected: transport mechanics and semantic behavior are separate contracts.
- **Independent npm versioning.** Rejected: one implementation released as mismatched versions makes provenance and defect reports ambiguous.
- **Retain a seven-day minimum.** Rejected: no evidence established seven as a meaningful threshold; acceptance names the observations the period was meant to obtain.

## Consequences

- The fixture matrix grows substantially because versions become a derived dimension and TypeScript becomes a second adapter. That is the cost of simplifying committed state without losing floor evidence.
- M6 may close sooner or later than seven days. The receipt must show the named evidence; elapsed time cannot substitute for it.
- Implementation begins only after this packet and the amendments it requires merge. Packet-before-implementation remains unchanged.
