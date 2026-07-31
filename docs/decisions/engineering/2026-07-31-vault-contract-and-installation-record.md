# The committed vault contract and the local installation record

- Status: accepted
- Date: 2026-07-31

## Context

M2 is the milestone that decides the committed vault-contract format ([beta.md](../../beta.md#milestones)), and everything else at M2 waits on it: the `dense` and `starter` fixture corpora are written in it, `dogtag contract explain` renders it, and `dogtag doctor` validates it. [architecture.md](../../architecture.md) fixes the shape from above — *two assets carry this in the beta and no more*, one committed vault contract and one local installation record — and the [settings PDR](../product/settings.md) fixes the model: settings are scoped by who must agree, scopes are authorities rather than an unrestricted precedence stack, and product defaults materialize a complete contract rather than remaining a live inheritance layer.

Two documents disagreed about how many committed assets there are. [README.md](../../../README.md)'s vault sketch showed `.dogtag/schema.toml` alongside `.dogtag/target.toml`, while `architecture.md` places the dialect *inside* the committed contract and permits no second committed asset. The README is written readme-first and loses; this record's decision is what it now describes.

## Decision

### The two assets

- **The committed vault contract is `.dogtag/contract.toml`**, relative to the vault root. Its presence is also the vault-root sentinel ([vault discovery and selection](2026-07-31-vault-discovery-and-selection.md)).
- **The local installation record is `$XDG_CONFIG_HOME/dogtag/installation.toml`**, defaulting to `~/.config/dogtag/installation.toml` on both macOS and Linux. It is never committed, never required to exist, and **M2 reads it but never writes it**.

A dotted directory keeps the contract out of the note space — [note-types](../product/note-types.md) indexes every Markdown file under the root and gives folders no taxonomic meaning — and gives the discovery sentinel a precise target rather than a bare filename. `contract` rather than `schema` because the file carries dialect, write policy, and a compatibility version as well as a schema, and a filename must be self-describing without its directory ([documentation architecture](2026-07-30-documentation-architecture.md)). XDG on both platforms rather than platform-native paths, matching `install.sh`'s existing `$XDG_BIN_HOME` posture: one documented location, one code path.

### Format, encoding, and version

- **TOML.** The repository already speaks it in `tools.toml`, `coverage-baseline.toml`, `security-exceptions.toml`, and both conformance fixture schemas, and `serde` + `toml` are already workspace dependencies, so the format adds nothing to the dependency graph.
- **Encoding:** UTF-8 without a BOM, LF line endings, a trailing newline. Reading is byte-agnostic; when the SDK later *emits* a contract, emission order is the schema's declared order rather than alphabetical, so a generated contract is stable across runs and reviewable as a diff.
- **`contract_version` is a single monotonically increasing integer**, `1` at M2. The SDK declares a contiguous supported range; the classification rules are in [diagnostics and compatibility](2026-07-31-diagnostics-and-compatibility.md).
- **Unknown keys are always fatal**, on both assets. A misspelled key is this format's worst failure mode — a typo'd `requred` silently demoting a required property, or a typo'd `capabilties` silently removing a closed-write policy — and no convenience is worth making it survivable.

Fatal unknown keys are what make a single integer sufficient. A `major.minor` version's only machine meaning would be "an older tool may read the subset of a newer minor it understands," and strict parsing forecloses that, so the minor component would communicate severity to humans and nothing to code. The supported *range* carries every distinction the version needs to make.

### Composition, authority, and provenance

**Every setting has exactly one authorized scope**, so the two assets never overlap and there is nothing to override. The committed contract owns types, properties, relationships, capabilities, the lifecycle declaration, write policy, dialect, and `contract_version`. The installation record owns the vault registry and actor identity. No invocation input — flag or environment variable — may supply a contract-owned setting; the partition is structural rather than policed.

The installation record deliberately does **not** carry index placement at M2, although `architecture.md` lists it among the record's beta contents: no index exists until M4, and a field nothing reads is a guess waiting to be defended.

**Provenance has three sources**, reported per resolved leaf value:

- `contract` — written explicitly in `.dogtag/contract.toml`, with the file path and span.
- `installation` — written explicitly in the installation record, with the file path and span.
- `default` — the format's declared default for an omitted optional field, attributed to the contract version that defines it.

The third source is the one that needed reconciling. The settings PDR rejects *live inheritance from current product defaults*, because an unchanged vault must not acquire new semantics when its SDK changes. A **format** default is not that: it is a property of `contract_version = 1`, and changing one requires a version bump that an unchanged vault does not have. So a vault's meaning is fixed by the file plus the version the file names, and never by which build of dogtag happens to be installed. This also makes the M0 scenario `contract-loads-with-provenance` true as written, with its three named sources intact.

Provenance is reported **per leaf value**, addressed by a stable dotted key path, because the question worth answering mechanically is *"is this property optional because the author decided so, or because nobody said?"* — and the omitted field is precisely the one a per-node model cannot describe.

### Invocation inputs

Exactly four, none of them semantic:

- `--vault` and `DOGTAG_VAULT` select which vault ([vault discovery and selection](2026-07-31-vault-discovery-and-selection.md)).
- `XDG_CONFIG_HOME` locates the installation record, which also gives the conformance harness hermetic runs without a dogtag-specific variable.
- `--format` and the `NO_COLOR` convention select rendering.

A `--strict` flag promoting warnings to a nonzero exit is deferred until a real workflow asks for one.

### The minimum M2 declarations

**Types.** `name` (the discriminator value) and `capabilities`, a subset of `identity-bearing` (0..n), `catch-all` (exactly one), and `closed-write` (0..n). An unknown capability name is a load error. Cardinality is validated when the contract loads, reasoning only over declarations and never over a type's name.

**Properties.** `name`, `kind`, and `required`. The closed lattice of value kinds is eight: `string`, `integer`, `float`, `boolean`, `date`, `datetime`, `enum` (with `values`), and `list` (with `of`). `enum` is forced — the lifecycle axis is a closed set of values; `boolean` is forced — flags are boolean properties; `list` is forced — [note-types](../product/note-types.md)' flavor-as-tags model has no other representation.

There are **no value constraints** — no `pattern`, no bounds, no `format` hint. A corpus that needs "this string is a URL" writes a linter over the SDK's public API, which is the product's stated answer to wanting more than the kernel does; a plugin system is a standing non-goal in [product.md](../../product.md). A `format` hint the kernel records but never enforces was rejected outright: in a contract whose premise is that declarations are enforced, a declared constraint nothing checks misleads every agent that reads it, and quietly reintroduces caller-owned reinterpretation.

**Relationships.** `predicate`, `required`, and `targets` — the declared types that may be the far end, with omission meaning any declared type. Relationships are declared as their own construct rather than as a property of some reference kind, because `architecture.md` makes note, type, property, and relationship four distinct kernel concepts. Cardinality is deferred: nothing has decided whether it is a `min`/`max` pair, an exactly-one marker, or a bare many flag.

**Lifecycle.** A `[lifecycle]` table is **mandatory**, and declares either an axis or its explicit absence. See [the lifecycle declaration](2026-07-31-lifecycle-declaration-and-the-seam.md).

**Write policy.** The `closed-write` capability, and nothing else. [note-types](../product/note-types.md) fixes that the type is the key edit policy binds to, and the [authorship PDR](../product/authorship.md) owns the semantics; the richer vocabulary (human-only, ai-only, mixed, named authors) waits for the milestone that performs mutations.

**Dialect.** `[dialect]` with one key, `links`, valued `wikilink` or `markdown`. M2 parses, validates, and explains it; M3 consumes it.

The dividing line between what is declared now and what waits: **declare what an existing decision already fixes the shape of, defer what nothing has decided.** `targets` and `dialect` are fixed by the relationships PDR and by the `dense`/`docs` profile axes respectively, and the single-integer version makes deferring them cost a migration for every vault stamped at M2. Relationship cardinality and the full write-policy vocabulary are fixed by nothing, and inventing them now is the freeze-a-guess failure [the conformance harness ADR](2026-07-30-conformance-harness-shape.md) rejected for fixture corpora.

### Alternatives considered

- **`dogtag.toml` at the vault root.** Visible and discoverable, and the root marker would be the sentinel directly. Rejected: it sits in the note space, and any later committed asset needs either another root file or a directory added late — the dotted directory costs one ergonomic point now and is the answer to a question that will be asked again.
- **`.dogtag/schema.toml`, the README's name.** Rejected: it under-describes a file carrying dialect, write policy, and a version, and the README needed correcting regardless because it also promises a `target.toml` that `architecture.md` forbids.
- **YAML.** It would match note frontmatter, so an agent would speak one language. Rejected: implicit type coercion and anchor semantics are exactly the silent reinterpretation the determinism obligation forbids, and it would add a dependency. The cost accepted is real — the contract's language differs from frontmatter's.
- **JSON.** Rejected: no comments, and hostile to the hand-editing the README promises remains possible.
- **`major.minor` versioning.** Rejected as reasoning above: with fatal unknown keys the minor component carries no machine meaning.
- **Tolerating unknown keys for forward compatibility.** Rejected: it makes a typo silently drop a `required = true` or a `closed-write`, which is the format's worst failure made routine.
- **No format defaults; every field written explicitly.** Genuinely attractive — the file becomes readable in complete isolation. Rejected because it puts `required = false` and `capabilities = []` on every declaration in every contract, and it would require amending a golden scenario whose three-source prose is satisfiable without amendment.
- **Per-vault configurable defaults, a `[defaults]` table.** Rejected: no declaration could then be read in isolation, and a one-line edit would silently change the meaning of every type at once. Large blast radius for a few saved keystrokes.
- **Defaults supplied by the installed SDK.** Rejected: this is precisely the live inheritance the settings PDR ruled out.
- **Provenance per declaration node.** Rejected: it cannot say where an omitted field's value came from, and the omitted field is the one worth asking about.
- **A dogtag-specific variable for the installation record's location.** Rejected: `XDG_CONFIG_HOME` already does it, and a second way to say the same thing is a second thing to document, test, and keep consistent.
- **M2 writing the installation record.** Rejected: `doctor` stays read-only, and nothing at M2 — no scenario, no cutover — needs registration to happen through the tool.
- **Platform-native configuration paths on macOS.** Rejected: two code paths and two documented locations, diverging from the installer's existing posture, for no benefit a CLI user asked for.

## Consequences

- **The README's vault sketch is corrected by this record**, to one committed asset at `.dogtag/contract.toml` with the dialect inside it.
- **A hidden directory is invisible to a browsing human and inside Obsidian.** `dogtag doctor` and `dogtag contract explain` are what pay that back, and they are the two surfaces M2 ships — but a user who never runs them has no on-disk cue that the vault is configured.
- **Every additive format change costs `contract_version = 2` and a migration for every existing vault.** That price is deliberate and is cheapest now, while only two fixtures and two founder vaults exist. It will not stay cheap, and the known deferrals — relationship cardinality, the write-policy vocabulary, any value constraints — are the ones expected to trigger it.
- **The authority partition means there is no override mechanism to explain**, and also none to reach for. A future setting that genuinely wants two scopes will need this record superseded rather than extended.
- **Index placement is absent from the installation record**, so M4 either adds it under a new `installation_version` or discovers it belongs elsewhere.
- **`float` is in the lattice with no consumer at M2.** It is there because a numeric property is ordinary in any corpus and the kind costs nothing beyond the one it already shares with `integer`; if nothing uses it by the beta verdict, that is evidence the lattice can shrink.
- **A corpus wanting enforced value formats has no contract-level route.** The linter-over-the-SDK answer is real but unproven — no consumer has written one yet, which makes the [architecture obligation](../../architecture.md) about consumer-authored validation a claim this milestone asserts rather than demonstrates.
