# Decision records

Two trails, one question each. Both supersede rather than delete. An accepted ADR's Decision is never rewritten; a PDR is a living position that evolves in place.

- **[Product decision records](product/README.md)** (`docs/decisions/product/`) — what Dogtag *is* and how it behaves for its users, written timelessly and for any PKM. Named by stable topic slug, because a product position is a living document that evolves in place.
- **[Architecture decision records](engineering/README.md)** (`docs/decisions/engineering/`) — how *this repository* is built and shipped: layout, toolchain, dependencies, pipelines, policy. Named `YYYY-MM-DD-slug.md`, because a build decision is a dated event.

## Which one to write

**If the decision would still matter to someone reimplementing dogtag from the docs alone, it is a PDR. If it only matters to someone working in this repository, it is an ADR.**

This is the one home of that rule; the two trail READMEs and [AGENTS.md](../../AGENTS.md) point here rather than restating it.

The same test routes documents, not just decisions — which is why the narrative spec set ([product.md](../product.md), [abstractions.md](../abstractions.md), [architecture.md](../architecture.md), [beta.md](../beta.md), [strategy.md](../strategy.md)) sits above this directory rather than inside either trail. See [documentation architecture and roadmap ownership](engineering/2026-07-30-documentation-architecture.md).

Each trail's README carries its own format, conventions, and voice rules.
