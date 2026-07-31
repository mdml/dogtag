# Dogtag — product

> **Status: beta product draft, started 2026-07-03; spin-off activated 2026-07-29.** The product case for **Dogtag-the-tool** — the shareable SDK a stranger can install, with the maintainers' incumbent vault demoted to one configuration of it. Working doc for maintainers and agents, published as it stands: the `>` blocks mark product questions the maintainers have deliberately not yet answered — they are the open edges of the case, kept visible rather than polished away. Companions: [README.md](../README.md), [abstractions.md](abstractions.md), [architecture.md](architecture.md), [beta.md](beta.md), and [strategy.md](strategy.md). **Target artifact:** a standalone shareable doc set for beta users, collaborators, SDK consumers, and build partners. The decisions pass closed in the maintainers' private planning record; extraction and release sequencing lives in [beta.md](beta.md) and the product repository's ADR trail (`docs/decisions/engineering/`).

## Thesis

Dogtag is a personal knowledge management SDK. Unlike existing apps, Dogtag is designed for AI agents: they configure it, co-author the notes, and keep the vault maintained. That means Dogtag can be used by anyone who uses AI agents, whether for "traditional" personal knowledge management (meeting notes, journaling), research projects, "LLM wikis", or any project that requires writing, indexing, and summarizing text and binary files. Software expertise is not required — an SDK normally implies a human developer, but here the developer is the agent, and it works from natural language instructions.

## Why

We're in an exciting new era where AI agents can cheaply and quickly process more text than any human has seen in a lifetime, but existing knowledge management tools are impeding adoption.

Historically, the barrier to knowledge management has been operational: the best practices of and software for categorizing, tagging, and linking notes have existed for a long time, but it takes too much time and effort to follow these practices for most use cases. That barrier is now gone.

The labor agents provide comes in three flavors. They draft notes, fixing typos, cleaning transcripts, finding wikilinks and formatting aliases, and applying tags. They ingest data, syncing and structuring the inputs to the knowledge base (from photos to movie watching history). They synthesize across notes, making the knowledge base legible at different levels of granularity, from all notes related to a project or person, to trends over time. Only drafting requires a human in the loop; ingestion and synthesis can be automated and run on a schedule.

At the same time, it's not as simple as pointing an agent at a folder and asking for a knowledge base (in Dogtag, like Obsidian, a "vault"). Agents have limited context and memory, so can't keep the structure in their heads. They need a contract to maintain and enforce.

The root problem with existing tools is where the operations layer lives — the templates, validation, structure semantics, and indexes that turn text files into a knowledge base. Database-like tools (Notion, Roam, Craft) expose both the data layer (text, binary files) and the operations layer only through a minimal API. Markdown tools (Obsidian, [Tolaria](https://github.com/refactoringhq/tolaria)) use the file system for the data layer, but keep operations in the app: templates are app config, validation is a plugin or absent, and link/type semantics execute only in the app's runtime. Either way, the operations layer is only fully available via the GUI.

The new agentic AI era has led to a massive increase of interest in Markdown-based knowledge management, but the use cases are limited by coupling the data and operations layers. Drafting with an agent requires a follow-up pass to insert wikilinks and structure properly; ingestion and synthesis are out of reach. Deployment is blocked the same way: a wiki site, an MCP endpoint, or simply sharing a vault over Git all need the operations layer without the GUI. To compensate, users write scripts or write documentation to configure the apps — that is, when they don't just skip building a knowledge base at all.

For Dogtag, the knowledge base contract is the product. The categorization, templates, lint, and index are headless, so an agent, a server, a site renderer, or a teammate's checkout are all first class. Perhaps just as exciting: once the contract is the product, UIs are cheap to build and can be tailored to the user or use-case — a TUI for devs, a wiki for a team, a dashboard for a research project. A thousand UI flowers can bloom.

## Who it's for

| Persona | Already has | Wants from Dogtag |
| --- | --- | --- |
| PKM enthusiast | years of notes (Obsidian, Notion, Roam), strong opinions, an AI subscription | agent labor over a corpus they already curate — without surrendering their structure |
| Researcher | a corpus problem — papers, experiments, drafts — and an AI subscription | project memory and synthesis: what did I read, decide, and try, and why |
| Dev team | a repo, a git host, agentic CLIs already in the daily workflow | a knowledge base that lives next to the code — decisions, onboarding, runbooks that agents keep true |
| Exec team | meetings, decisions, and documents scattered across tools | org memory: a decision log and a browsable wiki that someone (an agent) actually maintains |
| Decision maker | a high-stakes, multi-party project and no appetite for tooling | a maintained record of who said what, when, and what was decided — set up *for* them, not *by* them |

Two examples anchor the decision-maker row: an estate executor tracking obligations across lawyers, banks, and beneficiaries; and an adult child coordinating an aging parent's doctor's appointments and prescriptions. In both, the beneficiary is not the installer — someone technical stands the vault up, and the agent does the upkeep.

The floor, in comfort and spend: *guided* terminal use (install an agentic CLI, paste the commands the agent suggests, report back — the agent operates the tool) and any agentic-CLI subscription (nothing is Claude-specific — but without *some* subscription the agent labor that is the product doesn't exist). The decision-maker row bends the comfort floor further: setup needs a technical sponsor, and from then on their surface is conversation. Permanently not served: the AI-averse, for whom an agent writing into their notes is the premise failing, not a feature missing; and anyone unwilling to pay for the agent labor. Everything else — phone-first capture, team multi-writer conveniences — is staging, not exclusion; the Roadmap carries the order.

> User stories — one short vignette each, concrete (what they say/type, what the agent does, what exists a week later):
> - The PKM enthusiast: migrating years of Obsidian notes without surrendering their structure; week one of agent upkeep.
> - The researcher: pointing a fresh vault at a live project; the payoff moment of "what did I already conclude about X?"
> - The decision maker: a vault they never installed — capture arrives by voice and forwarded email, the agent files and cross-links it, they ask questions in plain language.
> - A team story (pick dev or exec): shared vault on the org's git host, each member on their favored editor; the wiki as the read surface — and the slot where the deferred agent-synthesized wiki layer ("LLM wiki") lands later.

## The product — a tour

The first hour — install, init or import, customize the schema, connect a remote — is the [README's Getting started](../README.md#getting-started); it belongs to the doc a stranger reads first. The tour here picks up at steady state.

Day to day, the agent is the primary operator — capture, triage, ingestion, and synthesis all run through it — and the human touches the vault directly in exactly two places: the shipped minimal TUI for browse and search, and their own editor for writing, with the on-disk dialect configured per machine for that editor (Obsidian and [Tolaria](https://github.com/refactoringhq/tolaria) at launch) — so teammates sharing a vault can each keep their favored editor. The MCP server is the surface for software: other agents, remote clients, future phone capture. Scheduled labor is the user's to compose — Dogtag ships the verbs a schedule would call, and your agent can build the routine around them; the schedulability is the product, the scheduler is not.

Just as deliberate is what's absent. No query builder — faceted browse and search over the authored structure replace it. No sync service — git is the store. No bundled scheduler, no embedded editor, no plugin system: the contract makes all of these cheap to build yourself, and keeping them out keeps the product small enough to trust.

How it's built is [architecture.md](architecture.md); the concepts it's built from are [abstractions.md](abstractions.md); what a vault looks like on disk is in the [README](../README.md).

## What ships in the box

> - The deliverable, literally: v1 of "the box" is taking shape as CLI + MCP server + minimal TUI + the Portent default schema — what else, if anything: template repo, skill pack?
> - What does the user bring (editor, git host, AI subscription) — and which of those does the product help stand up?
> - Migration: what's the story for an existing Obsidian/Notion/plain-markdown corpus — a backfill pipeline like the maintainers', or something gentler? (The first hour's `import` is the shallow end of this story; the deep end lives here.)
> - The update channel: once installed, how do schema changes, new skills, and lint rules reach an existing vault without breaking it?

## Invariants vs. config

> - The load-bearing section — the spin-off *is* this conversion. Which of today's personal invariants become config-points: first-person voice? the privacy-tier model? the type taxonomy? the tag vocabulary? commit discipline?
> - The Portent default sharpens the test: an invariant is what the maintainers' config and the Portent config *share* (typed objects, explicit relationships, a lifecycle, plain files, an agent-maintained contract); where they differ (type count, predicate vocabulary, lifecycle encoding) is config by demonstration.
> - Resolved (2026-07-14): per-type *ownership policy* is core — the invariant is that the substrate always knows who may write where; inline AI fences are the shipped default markup for mixed notes, and are configurable. `Source` ships in the default schema; removing it is config, but its immutability contract is core-enforced wherever the type exists.
> - What default config does a fresh install get, and where does it live (the settings PDR's surface)?
> - Per row: point at the owning PDR's seam line rather than restating it — no-snapshot rule.

## Non-goals

> - What is this not, even at success — hosted sync, realtime collaboration, mobile-first app, a general agent framework, a plugin ecosystem (recipes stay simple)?
> - Keep the boundary with the tour's closing what's-absent paragraph: that's what v1 doesn't ship; this is what the product never becomes.
> - Deferred, not dropped (named triggers): the agent-synthesized wiki layer (returns behind a demonstrated-value test); frontmatter-aware merge + team conflict UX (returns when a real multi-writer vault exists).
> - For each remaining item: dropped forever, or deferred behind a named trigger?

## Roadmap

> - Smallest shareable artifact that tests the thesis — a template repo? one guided friend-onboarding?
> - Staging: which personas v1 serves (the maintainers → technical friends → set-up-for-someone-else vaults), and what graduates later (starter kits + schema interview, phone-first capture, team multi-writer conveniences).
> - Stage gates: what must be true to onboard the first beta user, to go public? (The extraction gate itself is resolved: the PDR set closed and the product repository exists.)
> - Extraction deliverable is a pair: this spec + an executable conformance suite (the round-trip properties and golden vaults from Architecture's ship-daily pillar). The suite is the machine-readable spec — what a coding agent builds against in a loop, and what makes a one-shot agent build plausible; the prose stays for humans.
> - What signal would kill the spin-off — and is "stays a private tool forever" an acceptable terminal state?

## Open questions

> - Park what's genuinely undecided, dated: the name (dogtag-the-repo vs. dogtag-the-tool)? support-burden tolerance?
> - Distribution asset resolved 2026-07-29: `dogtag.dev` is already owned for the install script, SDK docs, compatibility guidance, and product site; an existing logo is the beta's starting brand asset.
> - License resolved 2026-07-30: **Apache-2.0** from a public repository, deliberately foreclosing the paid branch before E1/E2 produce evidence — an SDK whose premise is that others embed it isn't served by a license its audience must think about. If a paid question returns, it returns as something built beside the SDK, not as a retraction of it. Rationale and rejected alternatives are recorded in the maintainers' private planning record.
> - Platform floor: macOS + Linux at v1 is the assumption; when (whether) Windows.

## Pointers

- [README.md](../README.md) · [abstractions.md](abstractions.md) · [architecture.md](architecture.md) · [beta.md](beta.md) · [strategy.md](strategy.md) — the rest of the doc set
- [docs/decisions/product/](decisions/product/README.md) — the product decision records (PDRs); each owns its config/invariant seam line
- [docs/decisions/engineering/](decisions/engineering/README.md) — the product repository's own ADR trail: build decisions, honestly dated
