# Markdown flavor: plain files, a small typed dialect

- **Status:** adopted (interviewed and confirmed 2026-07-22; lived daily by the incumbent corpus); amended 2026-07-30 — identity is the path, the name is a resolution shorthand (see Design)
- **Decided / updated:** 2026-07-22; amended 2026-07-30 (identity)
- **Layer:** substrate
- **Config vs. invariant:** *Invariants* — notes are plain markdown files; the dialect stays small, and every extension carries machine-enforceable meaning; a writing surface never alters bytes it didn't semantically touch; identity is the path and a bare name is a resolution shorthand whose ambiguity is reported against the link. *Config-points* — the dialect's concrete inventory (canonical home: each vault's committed contract, validated by the SDK's contract types (`crates/dogtag`, forthcoming — see [Architecture](../../architecture.md)), never restated here), the per-machine editor dialect rendered at the boundary of one canonical stored form, and whether a corpus adopts name-uniqueness as authoring discipline.

## 👁️ Intent

Write and accumulate a lifetime of notes in a form that any tool, any editor, any agent — and future-me with no tooling at all — can read, diff, debug, and repair. Two properties are the ones the system would defend last:

- **Separation of concerns.** The store is not coupled to any application. Storage, editing, indexing, and presentation are independent layers; any one of them can be replaced without touching the notes. A note's meaning must never depend on which tool wrote it.
- **Legibility under failure.** When something goes wrong — a bad write, a broken link, a mangled merge — the artifact itself is debuggable with bare hands: open the file, read it, fix it. No export step, no schema migration, no application required to understand your own knowledge.

Portability and git-diffability follow from the same choice: plain text is the only format every version-control, search, and diff tool already speaks, and the only one with a multi-decade survival record.

## 🎨 Design

- **Notes are plain markdown files; the file is the whole artifact.** Prose and metadata travel together — copy the file and you have copied the note, its data, and its relationships.
- **A small typed dialect over a plain base.** The base grammar is the common markdown baseline any mainstream renderer already speaks — CommonMark plus the ubiquitous extensions popularized by GFM (tables, strikethrough) — chosen on the same grounds as plain text itself: universality. The dialect is the handful of extensions *beyond* that baseline: a frontmatter metadata block, wikilinks for addressing other notes, and in-band authorship fences. The admission test below governs only the dialect, not the baseline. The concrete inventory and syntax are the contract's to define (the canonical home above); this PDR fixes only the *shape*: small, typed, machine-checked.
- **The admission test.** An extension enters the dialect only when it carries **machine-enforceable meaning** — something a linter validates or an index consumes. Presentation sugar never earns syntax. The eviction trigger is the same test failing later: when nothing enforces or consumes a construct any more, it leaves the dialect rather than lingering as decoration. *(Receipt: the incumbent corpus evicted embedded query blocks on exactly this ground.)*
- **The metadata plane.** Frontmatter is for what has a **schema and gets indexed**; the body is *writing* — reachable by full-text search, never schema'd. That is the whole line between data and prose: if a machine must validate or query it, it goes in frontmatter; if a human reads it, it goes in the body. Structured relationships therefore live in frontmatter (syntax here; vocabulary and inverse semantics in [`relationships`](relationships.md)).
- **Identity: the path is the identity, the name is a shorthand; the title is the opening H1.** *(Amended 2026-07-30 — this bullet previously required names to be vault-unique.)* A note's stable identity is its location in the corpus, and the first H1 carries the display title, free to change without breaking the link graph. A link written as a bare name resolves **if and only if that name is unambiguous**; where two notes share a name, the reference must qualify itself enough to pick one. The consequence that matters: **ambiguity is a defect of the link, not of the corpus.** Two notes may legitimately share a name — daily notes filed by year, a per-directory index — and a system that refuses to open such a corpus, or that demands a rename sweep before the first read, is imposing a convention rather than reading what is there. Renaming a *title* is an edit; changing an *identity* is a link-integrity operation the substrate must mediate. Whether a given corpus additionally *chooses* name-uniqueness is discipline, and a valuable one; it is not a precondition for being read.
- **The round-trip contract.** A surface that writes must **preserve every byte it didn't semantically touch**. The one sanctioned exception is a *dialect boundary*: a surface may edit in its own on-disk dialect provided the store keeps **one canonical form** and the conversion is explicit, deterministic, and owned by the substrate — never an editor's incidental serializer. An editor that re-serializes whole files on save is the failure this contract exists to make impossible by construction.

## ⚖️ Tradeoffs

- **Rejected: a database as the store** (the app-owned-workspace model). Couples the knowledge to an application's lifetime and export goodwill; loses bare-hands debuggability, version-control diffing, and the ability to point any new tool at the corpus. Databases are the right *derived* layer (see the index) — never the source of truth.
- **Rejected: rich-text / proprietary formats.** Same coupling, plus opacity: the artifact can't be read, diffed, or repaired without the owning application, and agents can't write it natively.
- **Rejected: sidecar metadata files.** Splits the artifact in two — the note no longer travels whole, renames must be transacted across pairs, and every consumer needs pairing logic. Violates both load-bearing Intent properties at once.
- **Rejected: inline body metadata** (`key:: value` fields in prose). Blurs the one line the design depends on: the body becomes half-schema'd, machines must parse prose, and humans must write data. Frontmatter keeps the planes separable.
- **Rejected: a large or expressive dialect** (embedded query languages, transclusion, presentation directives). Every construct taxes every future parser, every surface's round-trip, and every reader's understanding — compounding costs for point conveniences. The admission test is the dam.
- **Rejected: vault-unique names as a precondition for reading a corpus.** *(Amendment, 2026-07-30.)* Uniqueness makes link resolution trivially cheap, and a curated corpus benefits from the discipline — but as an *invariant* it inverts the relationship between the system and the notes. Folder-organized corpora produce repeated names as a matter of course, and a system that reports those as corpus-level errors is asking the corpus to change shape before it will read it. Resolving names lazily, and reporting ambiguity where the ambiguity actually is — at the reference that can't be resolved — costs one index lookup and refuses nothing that exists.
- **Rejected: trusting editor serializers with the round-trip.** A GUI that parses-and-reserializes on save will eventually rewrite what it didn't understand; guard-lints can catch it after the fact, but the contract belongs in the product: don't write what you didn't touch. *(Receipt: the incumbent corpus's GUI fallback produced exactly these serializer regressions.)*

## 🖥️ Surfaces

The dialect and the round-trip contract are UI-independent; what differs per surface is which side of the contract it sits on.

- **TUI** — a pure reader: renders the dialect (wikilinks resolved, frontmatter as a clean header, fences folded) and writes nothing, so the contract costs it nothing.
- **CLI / SDK** — the authoring surfaces. A text editor edits bytes directly and preserves what it doesn't understand *by construction*; the substrate's write verbs write schema-validated frontmatter, which is the "conversion owned by the tool" half of the contract.
- **MCP** — reads freely; writes only through substrate verbs, never raw file rewrites.
- **GUI editor (fallback)** — the cautionary case: the incumbent is a parse-and-reserialize editor held to the contract externally (guard-lints, a pinned version) because it can't honor it internally. A conforming GUI would edit through the canonical form + dialect-boundary model instead.

## Notes

- Interview follow-ups resolved 2026-07-22: "separation of concerns" confirmed as *store decoupled from every application layer*; identity (filename-as-slug / H1-as-title) confirmed as this PDR's to own, with `note-types` and `wikilink-ui` consuming it; the base-grammar-vs-dialect line settled as "the admission test governs only what's beyond the common GFM-ish baseline."
- The per-machine-dialect exception is the tool's model (canonical form in the store, per-editor on-disk dialects at the boundary — [Product](../../product.md)); this PDR grounds why it's the *only* sanctioned deviation from byte-preservation.
