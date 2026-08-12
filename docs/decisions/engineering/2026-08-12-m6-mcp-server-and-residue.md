# The M6 MCP server, deployment, and inherited residue

- Status: accepted
- Date: 2026-08-12

## Context

M6's dynamic workflow is an SDK-backed MCP server deployed beside the maintainers' existing connector infrastructure. It inherits the M4 connector residue — the incumbent server-side FTS index and `pkm tags` — and M5's capture obligation. Packet-time verification establishes the mechanisms rather than inferring them from workflows: the current claude.ai connector serves search, show, and tag vocabulary from the vault repository's TypeScript MCP server and persistent SQLite index; direct `dogtag capture` has not become a real workflow beyond release verification.

The server is a beta workflow and an SDK consumer, not the promotion of MCP into the product's general public surface. That boundary permits a deliberately bounded tool set even though users may eventually expect the full SDK over MCP.

## Decision

### Four tools, one configured vault

M6 exposes exactly four model-controlled tools: **`search`, `show`, `capture`, and `tags`**. `tags` is the fourth because M4 explicitly sent the incumbent vocabulary/count authority here; omitting it would preserve `pkm tags` without a new destination. No general dispatcher and no early mirror of `doctor`, `check`, `list`, `find`, contract administration, index administration, or agent-contract generation ships at M6. A later packet may broaden the surface after the bounded workflow supplies evidence about tool selection and schemas.

Each server instance resolves and verifies one vault at startup and holds that opaque handle. Calls carry no vault name or arbitrary path. This confines ambient authority, prevents registry disclosure, and makes concurrent requests share a handle without creating process-global semantic state.

Tool results are MCP-native structured results whose semantic payloads come directly from public SDK values. They preserve diagnostic identifiers and severities, compatibility facts, paths, capture plans and results, warnings, and recovery. They do not wrap the CLI's top-level JSON document, carry CLI exit codes, or invent server-owned semantic models.

`capture` accepts `preview`; otherwise it plans and applies in one call, preserving M5's one-act transaction. It remains never gated on Dogtag configuration. The server supplies provenance `agent` and a configured actor where one exists; an absent actor produces the existing unattributed warning and the write still lands. HTTP authentication controls access to the configured vault and is not reinterpreted as authorship.

### Two transports and one authentication boundary

The server supports stdio for local use and Streamable HTTP for deployment. The hosted Dogtag MCP reuses the incumbent infrastructure's authentication boundary: **Clerk OAuth with Dynamic Client Registration, restricted to the one `CLERK_ALLOWED_USER_ID`; Caddy terminates TLS and is the only public route to the server.** A valid Clerk principal for any other user is unauthorized. Missing, malformed, invalid, expired, or wrong-user credentials receive HTTP `401` with the OAuth protected-resource challenge and no MCP dispatch. A static bearer token is permitted only when Clerk is absent in local development and tests; it is never a hosted fallback. Stdio relies on the local process's authority.

The deployed instance runs beside the existing infrastructure and initially serves the founder vault. Authentication and vault selection come from deployment configuration and encrypted environment state; neither may define vault semantics. Protocol tests cover the OAuth validation seam, the allowed-user gate, the local static-bearer mode, the `401` challenge, and the rule that an unauthorized request never reaches a tool.

### `tags` reports observed vocabulary and counts

`tags` returns the tags observed on notes in the configured corpus, each as `{ tag, count }`, where `count` is the number of notes carrying that exact tag. A repeated identical tag within one note counts once. An optional `prefix` retains tags whose stored string begins with that exact prefix; an optional positive `limit` caps the result after ordering. Results order by count descending, then tag ascending as the deterministic tie-break. The default limit is fixed with the tool schema during implementation and becomes contract at that point.

Declared-but-unused contract values do not appear. The tool answers “what vocabulary is live in this corpus,” preserving the incumbent connector's behavior; `contract explain` remains the surface for what the contract permits. The Rust SDK owns collection, counting, filtering, and ordering, and TypeScript and MCP only adapt the resulting values.

### Cutovers and receipts

The connector's `search`, `show`, and `tags` calls move to the SDK-backed server. Packet-time verification has established the mechanism being displaced, so the receipt can name it precisely. On completion the incumbent connector handlers, server-side FTS database, and its reindex scheduling retire. `pkm tags` retires because the new `tags` tool preserves its live-vocabulary/count consumer.

The M5 capture obligation closes differently. There is no incumbent capture mechanism: Max had not used direct `dogtag capture`, and the published-binary exercises were release proofs rather than a daily workflow. M6 therefore establishes MCP capture as a new path and records real-use evidence; it does not call adoption a cutover or invent a parallel lane. Binary attachment handling remains with the incumbent and keeps its M9 destination.

The interactive `just search` fzf picker remains named residue and moves to the TUI milestone. It is a human UI consumer, not connector infrastructure.

### No persistent Dogtag index at M6

Dogtag search remains a corpus scan. The accepted trigger is a real-vault median above one second; the measured 650/714 ms medians do not fire it. Being resident does not itself justify staleness and invalidation semantics. The server initially holds no cross-call semantic cache. Local and deployed MCP latency are measured in the M6 receipt, and a median above the trigger opens the index decision then.

### Alternatives considered

- **The whole current SDK as MCP tools.** Rejected at M6: it expands stable remote schemas, authentication review, conformance, and model tool-selection cost beyond the workflow the milestone exists to prove.
- **Only the original three tools.** Rejected: it leaves M4's `pkm tags` residue alive at the milestone assigned to retire it.
- **A general operation dispatcher.** Rejected: it obscures capability discovery and creates one unbounded schema instead of four reviewable contracts.
- **Vault per call.** Rejected: it broadens ambient filesystem and registry authority and defeats the long-lived-handle evidence M6 is meant to produce.
- **CLI JSON documents as MCP results.** Rejected: CLI rendering and exit semantics are not SDK semantics.
- **Unauthenticated HTTP or network placement alone.** Rejected: the server exposes private reads and mutation.
- **A hosted static bearer token.** Rejected: the existing OAuth installation and single-user principal gate already provide revocation, client registration, and an identity the hosted workflow understands; a second production auth mode broadens the security boundary.
- **Declared tag vocabulary instead of observed counts.** Rejected: it changes the incumbent tool's question, hides open namespaces' live values, and cannot report actual usage. Combining declared and observed values in one list is also rejected because a zero count would conflate “allowed but unused” with corpus evidence.
- **A persistent index or in-memory cache now.** Rejected: the recorded trigger has not fired, and either option creates freshness behavior with no measured need.

## Consequences

- The MCP surface is intentionally smaller than the TypeScript SDK. “SDK-backed” means shared semantics, not identical surface area.
- Authentication can refuse a request before Dogtag sees it; once authorized, capture's never-gated product stance remains intact.
- Retiring the incumbent index also retires its richer ranking and filters for the connector. Those differences are accepted only to the extent already adjudicated by the M4 cutover; the M6 receipt must not claim feature parity.
