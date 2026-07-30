# Settings: one resolved configuration model, scoped by who must agree

- **Status:** adopted — interviewed and confirmed 2026-07-29
- **Decided / updated:** 2026-07-29
- **Layer:** substrate — the SDK resolves and validates settings; surfaces and workflows consume the resulting configuration
- **Config vs. invariant:** *Invariant* — configuration is a **first-class, typed, inspectable substrate model** resolved separately for each vault in an installation; settings are scoped by who must agree, not by implementation convenience; product defaults materialize a complete committed vault contract at initialization rather than remaining a live inheritance layer; installation and invocation choices may narrow capabilities but never silently weaken or reinterpret that contract; every SDK consumer reads the same resolved model and may operate multiple isolated vaults simultaneously; shared-contract changes expose compatibility and corpus impact before application. *Config-point* — the schema, lifecycle labels, relationship vocabulary, ownership policy, retrieval tuning, views, workflows, VCS posture, target dialect, installation bindings, and presentation choices inventoried below.

> **Boundary.** The other PDRs own the meaning of each configurable concept; this PDR owns how those config-points become one coherent settings surface. It inventories and links rather than redefining them. [Architecture](../../ARCHITECTURE.md) owns the implementation boundary; the SDK's public configuration types and file format are build-time decisions downstream of this product contract.

## 👁️ Intent

A configurable knowledge system must let people run meaningfully different vaults without forking the product, while still giving every agent, SDK consumer, and interface one trustworthy answer to "how does this vault work?" One application may also operate several vaults whose schemas and identities must remain independent. Settings scattered across application preferences, prompts, scripts, and undocumented conventions make the operational layer impossible to reproduce or verify. The user needs configuration to be portable where collaboration requires agreement, local where installations genuinely differ, and explicit enough that an agent can safely inspect and change it.

## 🎨 Design

**Settings are part of the substrate.** The SDK loads, resolves, validates, and explains configuration before any surface interprets it. A CLI, MCP server, Python loop, TypeScript webhook, and custom UI operating the same vault receive the same semantic configuration. A surface may add presentation choices, but it may not privately redefine what a type, relationship, lifecycle state, ownership rule, or saved view means.

**Configuration is layered by who must agree.**

- **Initialization profiles** make a new vault useful before its owner designs a schema. They materialize a complete vault contract and then leave the runtime resolution chain; an updated profile reaches an existing vault only through an explicit proposed change.
- **Vault configuration** contains corpus semantics and named operational definitions every collaborator and automation must share: schema, relationships, lifecycle roles, ownership policy, views, and vault-carried workflows. It is committed and versioned with the corpus.
- **Installation configuration** contains facts that legitimately vary per checkout or runtime: the vault registry, local paths, actor identity, editor dialect, available integrations, secret references, index placement, workflow triggers, and surface presentation. It is local and must not change the meaning of committed content.
- **Invocation input** selects an operation or temporarily changes non-semantic execution behavior; it cannot silently override vault invariants. A durable preference belongs in one of the layers above.

There is no user-level scope in the initial model. If repeated configuration across installations creates real friction, a user scope may be added for settings explicitly declared user-eligible; it cannot override committed vault semantics. Cross-installation preference syncing is otherwise the concern of the user's configuration-management tool.

**Scopes are authorities, not an unrestricted precedence stack.** Every setting declares which scopes may supply it. Schema types are vault-only; cache paths are installation-only; actor identity may come from the installation or invocation. The effective configuration has one answer with provenance for each setting, but a lower scope cannot override a value it has no authority to own.

**Secrets are references, never settings values.** Configuration may name a credential source or integration, but tokens, private keys, and passwords live in the host's secret mechanism. A committed vault must remain safe to clone; installation configuration must remain safe to inspect and diagnose.

**One model may have many explicitly owned assets.** A schema, saved view, workflow, authorship policy, and versioning policy need not occupy one monolithic file. The SDK resolves them into one typed model, every value has one canonical owner, file division does not create inheritance, and diagnostics identify the source asset responsible for a value. Named views and workflows are first-class vault assets; their schedules, credentials, and external bindings remain installation configuration.

**The SDK is multi-vault by construction.** An installation registry may name several vaults and their local bindings. Applications can hold several vault handles simultaneously, run operations independently, and aggregate results that retain vault provenance. Configuration, indexes, diagnostics, events, and mutations are vault-scoped; one invalid or unavailable vault does not prevent others from opening. The initial model does not merge identity namespaces, resolve cross-vault links, or provide atomic cross-vault writes.

**Changes follow the scope of what they can affect.**

- A **vault-contract change** is compared with the active contract, classified for compatibility, inspected for corpus impact, and represented as an explicit plan before application. Configuration and any required document migration apply together or not at all, with recovery information preserved.
- An **installation change** is typed, validated, and applied atomically. It normally requires no corpus migration, but reports operational effects such as an index rebuild or service restart.
- **Invocation input** is validated with the operation and is not persisted as configuration.

The SDK always makes a shared-contract plan available and rejects invalid or unauthorized changes. Whether a valid plan requires interactive human confirmation, delegated agent authority, or automated policy belongs to the caller; Dogtag does not mandate an approval ceremony.

**Local choices may narrow, never silently weaken.** An installation may disable networking, enter read-only mode, or lack an optional model. The resulting capability is unavailable or explicitly degraded; local configuration cannot turn off ownership enforcement, reinterpret lifecycle roles, relax identity rules, or claim a contract guarantee it cannot provide.

**Settings evolve under compatibility rules.** Configuration declares the format or capability version it expects. A newer SDK can explain incompatibility, preview migration, and preserve a recovery path. An upgrade never silently changes corpus semantics merely because a default changed.

**The PDR inventory is the settings map.**

| Owning PDR | Committed vault contract | Installation binding or surface state |
| --- | --- | --- |
| [markdown-flavor](markdown-flavor.md) | Canonical dialect inventory | Materialized editor dialect |
| [note-types](note-types.md) | Type taxonomy, capability declarations, required properties, edit-policy hooks | Type presentation |
| [relationships](relationships.md) | Predicate vocabulary, required relationships, optional reification and derived lenses | Relationship presentation |
| [lifecycle](lifecycle.md) | State and flag labels, the declared axis and flag bindings, eligible types | State presentation and filters |
| [search-and-filter](search-and-filter.md) | Indexed fields and ranking, semantic model, saved views | Interaction idiom and local index placement |
| [wikilink-ui](wikilink-ui.md) | Alias policy, candidate ranking, external-resolution policy | Display name, color, glyphs, modes, and gestures |
| [inbox-workflow](inbox-workflow.md) | Birth states, composite workflows, named routing slots | Capture bindings, routing integrations, triggers, and cadence |
| [git-integration](git-integration.md) | VCS requirements, attribution grammar, gates, shared remote policy | Concrete checkout, credentials, automation, and surface affordances |
| [authorship](authorship.md) | Actor/provenance vocabulary, ownership assignments, carve-out and in-band policy | Actor identity for this installation and authorship presentation |

The table names ownership, not storage keys. Concrete vocabularies remain in the schema and SDK configuration types rather than being copied into this record.

## ⚖️ Tradeoffs

- **Rejected: settings owned independently by each surface.** It is convenient for a UI to keep its own type colors, saved views, or lifecycle interpretation, but semantic settings then drift and an agent receives a different vault depending on which door it enters. Surfaces may own presentation; the SDK owns meaning.
- **Rejected: everything committed.** Editor paths, local integration availability, credentials, and presentation preferences do not belong to every collaborator. Committing them creates churn, leaks machine assumptions, and tempts secrets into the corpus.
- **Rejected: everything local.** A schema or ownership rule that does not travel with the notes is not a corpus contract. A fresh clone would contain data without the rules required to interpret or maintain it.
- **Rejected: live inheritance from current product defaults.** An unchanged vault must not acquire new semantics because its installed SDK changed. Starter profiles generate an explicit contract; later profile improvements arrive as proposed, inspectable changes.
- **Rejected: a singleton current vault.** Current-directory discovery is a useful CLI convenience but a poor SDK foundation. It prevents multi-vault interfaces, invites global-state leakage, and makes provenance ambiguous when results are aggregated.
- **Rejected: one monolithic settings file.** Views, workflows, schema, and policy evolve at different rates and deserve distinct canonical assets. One resolved model provides consistency without forcing unrelated definitions into one document.
- **Rejected: a first-class user scope before demonstrated need.** It adds storage, synchronization, precedence, and non-human-runtime questions before the beta needs them. The authority model leaves a deliberate extension point for user-eligible settings later.
- **Rejected: unrestricted precedence through environment variables and flags.** Unlimited overrides make effective behavior difficult to explain and allow an invocation to violate the contract collaborators thought they shared. Invocation inputs select and tune operations; they do not secretly redefine vault semantics.
- **Rejected: configuration as arbitrary text for an agent to rewrite.** Human-readable files are valuable, but readability alone provides no safety. Typed parsing, impact planning, validation, and migrations are what make conversational customization trustworthy.
- **Rejected: local weakening of the vault contract.** A restricted installation is useful; an installation that silently reports success while skipping shared guarantees is corrosive. Missing capabilities degrade explicitly, while local policy may only narrow what is permitted.

## 🖥️ Surfaces

- **SDKs.** Rust, Python, TypeScript, and later bindings expose the same resolved configuration model, diagnostics, provenance, capability state, and plan/apply operations in language-idiomatic forms. They support multiple simultaneous vault handles with no process-global semantic state.
- **CLI.** Current-directory discovery selects a vault as a convenience over the same explicit open operation. `dogtag config show`, `check`, `plan`, and `migrate` make the effective model and its provenance inspectable; flags cannot bypass semantic validation.
- **MCP.** Agents inspect configuration and propose typed changes through tools backed by the SDK, rather than inferring policy from prompt prose.
- **TUI and custom interfaces.** May register, open, switch among, and aggregate labeled results from multiple vaults. They read each vault's semantics from its SDK handle and layer local presentation without redefining it.
- **Bare files.** Human-readable committed configuration keeps the vault understandable and recoverable without a particular surface; the SDK remains the authority for resolution and validation.

## Notes

- The first conformance test is two independently configured vaults operated simultaneously by the same released SDK. A difference that requires a product fork is evidence that either the configuration model is incomplete or the supposed invariant is personal.
- Multi-vault aggregation does not create a federated corpus: results retain vault identity, links resolve within their originating vault, and mutations target exactly one vault.
- The concrete file layout, serialization format, merge behavior, and generated language types belong to the SDK's architecture ([ARCHITECTURE](../../ARCHITECTURE.md), [BETA](../../BETA.md)) and the repository's ADR trail ([docs/adr](../adr/README.md)), not this PDR.
