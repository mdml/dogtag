# Abstractions

> **Status: pre-build draft (2026-07-14).** The key concepts Dogtag is invested in — more likely to change than the [architecture](architecture.md), but foundational to how everything works. One of a six-doc set; reading order in the [README](../README.md).

- **Vault** — a directory plus committed config; the unit of git, schema, and sharing.
- **Note** — frontmatter properties + markdown body; the document model every surface agrees on. In mixed-ownership notes, inline AI fences are the default (and configurable) markup attributing which prose is the agent's.
- **Type schema** — the Portent-default (or customized) catalog of types, properties, relationships, and lifecycle, plus each type's *ownership policy* (human-only / ai-only / mixed — who may write); one committed file the SDK validates against.
- **Templates** — per-type scaffolds the agent stamps on create.
- **Typed wikilinks** — relationships-as-wikilinks with a small predicate vocabulary; backlinks and inverses derive from them.
- **Search** — a first-class index over notes, properties, and links; derived and rebuildable. Not a convenience: agents can't hold a vault in context, so search is how they work it — without it, the human is back to curating the agent's context by hand.
- **External references** — a sidecar note *about* a file the vault doesn't hold: a URI (`file://`, `s3://`, a cloud path) plus optional integrity metadata (size, checksum). The core never fetches; resolution is the agent's job. The vault records that a thing exists and what it means without holding the bytes — which is also the privacy pattern for keeping sensitive binaries out entirely.
- **Presentation hints** — declarative per-type render hints in the schema (color, icon, lifecycle flow) so every surface — TUI, Tolaria, a wiki — renders the same semantics without hardcoding types. This is how the Tolaria niceties (wikilinks colored by target type, the lifecycle board) become portable config.
- **Target configuration** — the dialect profile (wikilinks vs. markdown links, frontmatter shape, folder conventions), chosen per machine; see [architecture.md](architecture.md) for canonical form vs. materialized dialects.
