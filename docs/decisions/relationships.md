# Relationships: directed typed links, derived inverses

- **Status:** adopted (interviewed, drafted, and confirmed 2026-07-22; the derived lenses lead the substrate — see Notes)
- **Decided / updated:** 2026-07-22
- **Layer:** both — the substrate stores, validates, and indexes links; surfaces navigate and analyze them
- **Config vs. invariant:** *Invariants* — links are directed and typed; a relationship is authored in exactly one direction and its inverse is always derived, never hand-maintained; a typed link must resolve to a real note; structured relationships live in the metadata plane ([markdown-flavor](markdown-flavor.md)). *Config-points* — the predicate vocabulary (the system supports arbitrary predicates; the curated set is per-vault config — canonical home: each vault's committed contract, validated by the SDK's contract types (`crates/dogtag`, forthcoming — see [ARCHITECTURE](../../ARCHITECTURE.md))), which predicates each class requires, whether relationship reification is enabled (and for what), and which derived lenses (entity multigraph, co-occurrence weighting) are built.

## 👁️ Intent

The relationship system records **how notes influence one another**. Three jobs follow, for any knowledge base and any author:

- **The dependency closure of a note.** The most important job: a note is not self-contained — to fully understand it you must know what it draws on. The link system is what makes "what do I need to read to understand this" a navigable, answerable question rather than archaeology.
- **Influence and time.** Links are directed, so the graph is a record of what the authors were thinking about, and when — which notes fed which, how attention moved. Direction is what lets the system reason about influence and timing, not just adjacency.
- **Apples-to-apples analysis.** Only links with semantics can be compared, aggregated, and queried as like-with-like. A pile of untyped references answers "connected to what?"; typed links answer "authored by whom, referencing what, attended by whom" — questions with structure.

## 🎨 Design

- **Directed, typed links between notes.** Every structured relationship is an edge with a direction and a predicate, authored from the note being written toward what it references or involves. Untyped body wikilinks remain the prose-level reference; structured relationships live in the metadata plane (syntax split: [markdown-flavor](markdown-flavor.md)).
- **Authored once; the inverse is a view.** A relationship is written in exactly one direction. The inverse — backlinks, the neighborhood — is derived by the index, always available, never hand-maintained. The bare inverse suffices: a backlink labeled with its forward predicate ("linked from X, via `authors`") answers navigation and influence questions without a second vocabulary of named inverses.
- **Arbitrary predicates; small vocabulary as discipline.** The system supports any predicate type — there is nothing magical about a fixed set, and real semantics must never be squeezed into a predicate that doesn't mean it. A *small* curated vocabulary is best practice, not a system limit: simple systems win, and good vocabularies tend to stay small. The vocabulary is config with one canonical home; this PDR never enumerates it.
- **Links resolve.** A typed link must point at a real note — an edge with a dangling endpoint is not a relationship, it's a string. When the target doesn't exist yet, the reference belongs in prose until it does.
- **Two graphs, one substrate.** The authored graph is note-as-node — the digraph the jobs above read directly. Complementing it (not replacing it) is the **entity lens**: a derived multigraph whose nodes are the identity-bearing notes the base is about ([note-types](note-types.md) — entities, and potentially other node-like classes such as topics), and whose edges are *evidenced by notes* — a note referencing two entities is typed edge evidence between them. The lens is what makes a synthesized wiki coherent: a node's page is generated from its edge evidence, with ownership fencing letting synthesized and authored prose coexist. Marked as a pursued exploration: the note-node graph is settled; the entity lens's full use-case surface is deliberately still open.
- **Implicit relations have standing.** Co-occurrence — two entities referenced in the same note with no typed link — is a derived, weighted signal (PageRank-shaped), never an authored fact. Its two jobs: *disambiguation context* (the Alex mentioned alongside a workplace is probably the Alex who works there — without anyone authoring that inference) and *suggestion* — surfacing missing explicit relationships for a human or agent to confirm. Weight strength is a tuning question; the standing is the product position.
- **Opt-in reification.** A relationship may graduate to its own note — a relationship sidecar — when the relation itself carries structure or content: its own dates, its own type, its own history worth writing about. The reified note is also the edge's synthesis anchor (the wiki page for a marriage, a mentorship, a partnership). Whether a vault enables a reified relationship class, and for what domain, is schema config; the capability is the product design.

## ⚖️ Tradeoffs

- **Rejected: untyped links only.** Backlinks alone answer "connected," never "how." The meaning stays trapped in prose, every consumer re-infers it, and no analysis can treat links as like-with-like. Typing is what turns a link pile into a graph.
- **Rejected: an enforced-small closed vocabulary.** Enforcement doesn't make semantics smaller — it pushes real meaning into overloaded predicates or back into prose, both of which corrupt the apples-to-apples property that typing exists for. Keep the *system* open and the *vocabulary* disciplined.
- **Rejected: hand-maintained bidirectional links.** Two authored copies of one fact is a standing consistency liability every editor and agent must service. The inverse is a view over the forward direction — derived, it can never drift.
- **Rejected: named inverse predicates.** A vocabulary of inverses (`authored-by`/`wrote`) doubles what users and consumers must know, for no query power the bare derived inverse doesn't already provide.
- **Rejected: promoting co-occurrence to authored fact.** Auto-asserting inferred edges pollutes the authored graph with claims nobody made. The derived layer stays derived; the product move is *suggestion*, with a human or agent confirming what becomes explicit.
- **Rejected: reifying every relationship.** Everything-as-a-node taxes the common case — most edges carry no data of their own — with ceremony that discourages linking at all. Reification is the exception a relation earns, not the default it pays.
- **Tension carried, not resolved: how much the implicit layer matters.** The intuition is second-tier — very important, especially as knowledge bases gain authors and entities — but its true weight is unknowable without more usage; the investment is sized by lived demand, not built ahead of it.

## 🖥️ Surfaces

The graph model, the one-direction invariant, and the derived-inverse contract are UI-independent; surfaces differ in how they author and traverse.

- **CLI / SDK (authoring).** Predicates are stamped and validated at note creation; entity targets resolve via name lookup; the lint resolves every typed-link value.
- **TUI (traversal).** Backlinks and neighborhood as a consumption view — the dependency closure and inverse, navigable keyboard-first.
- **MCP.** `backlinks` as a first-class tool; typed-link fields returned on `show`; the same graph, programmatic and remote.
- **Tolaria (GUI fallback).** Renders labeled typed inverses from the frontmatter arrays — the capability that motivated the metadata-plane form.

## Notes

- Receipts (corroborating, not the argument): the incumbent corpus runs one configuration of the arbitrary-predicate model — a small curated set of linted predicates plus unlinted per-class soft predicates, small by discipline — with the metadata-plane frontmatter form and a lint that resolves every typed-link value, soft predicates included.
- The genealogy `relationship` class is the existing instance of opt-in reification: kinship relations that carry their own type, parties, and dates. One domain's config, not the capability's boundary.
- The entity lens and co-occurrence weighting lead the substrate: no index projection exists yet. The named first experiment: project the existing wikilink index into an entity-pair × note table and examine what falls out. The wiki-generation connection is the product-level ground a future synthesis layer would build on (receipt: the incumbent corpus's prior wiki layer was retired for lack of exactly this substrate).
- Honesty note on strength: the Intent's influence-framing is held at medium strength by the product owner; the dependency-closure job is the firm core. Recorded so a future revision knows which part was load-bearing.
