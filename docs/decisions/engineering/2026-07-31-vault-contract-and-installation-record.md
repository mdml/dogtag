# The committed vault contract and the local installation record

- Status: accepted (amended 2026-08-01 — see [Amendments](#amendments))
- Date: 2026-07-31

## Context

M2 is the milestone that decides the committed vault-contract format ([beta.md](../../beta.md#milestones)), and everything else at M2 waits on it: the `dense` and `starter` fixture corpora are written in it, `dogtag contract explain` renders it, and `dogtag doctor` validates it. [architecture.md](../../architecture.md) fixes the shape from above — *two assets carry this in the beta and no more*, one committed vault contract and one local installation record — and the [settings PDR](../product/settings.md) fixes the model: settings are scoped by who must agree, scopes are authorities rather than an unrestricted precedence stack, and product defaults materialize a complete contract rather than remaining a live inheritance layer.

Two documents disagreed about how many committed assets there are. [README.md](../../../README.md)'s vault sketch showed `.dogtag/schema.toml` alongside `.dogtag/target.toml`, while `architecture.md` places the dialect *inside* the committed contract and permits no second committed asset. The README is written readme-first and loses; this record's decision is what it now describes.

This is an engineering record rather than a product one because the settings PDR routes it here by name: *"the concrete file layout, serialization format, merge behavior, and generated language types belong to the SDK's architecture … and the repository's ADR trail, not this PDR"* ([settings](../product/settings.md)). The product model — scopes as authorities, secrets as references, changes classified before application — is that PDR's; the spelling is this one's.

## Decision

### The two assets

- **The committed vault contract is `.dogtag/contract.toml`**, relative to the vault root. Its presence is also the vault-root sentinel ([vault discovery and selection](2026-07-31-vault-discovery-and-selection.md)).
- **The local installation record is `$XDG_CONFIG_HOME/dogtag/installation.toml`**, defaulting to `~/.config/dogtag/installation.toml` on both macOS and Linux. It is never committed, never required to exist, and **M2 reads it but never writes it**.

A dotted directory keeps the contract out of the note space — [note-types](../product/note-types.md) indexes every Markdown file under the root and gives folders no taxonomic meaning — and gives the discovery sentinel a precise target rather than a bare filename. `contract` rather than `schema` because the file carries dialect, write policy, and a compatibility version as well as a schema, and a filename must be self-describing without its directory ([documentation architecture](2026-07-30-documentation-architecture.md)). XDG on both platforms rather than platform-native paths, matching `install.sh`'s existing `$XDG_BIN_HOME` posture: one documented location, one code path.

### Format, encoding, and version

- **TOML.** The repository already speaks it in `tools.toml`, `coverage-baseline.toml`, `security-exceptions.toml`, and both conformance fixture schemas. Note carefully what this does *not* claim: `serde` and `toml` are workspace dependencies today but are **not in the shipped binary's dependency closure**, because the SDK crate has an empty `[dependencies]`. Parsing the contract moves them into that closure, which fires the SBOM trigger in [the supply-chain policy](2026-07-30-supply-chain-and-vulnerability-policy.md); see [the release record](2026-07-31-m2-release-and-cutover.md).
- **Encoding:** UTF-8 without a BOM, LF line endings, a trailing newline. On read, a BOM, CRLF line endings, or invalid UTF-8 each produce their own diagnostic rather than being silently normalized — spans are measured in Unicode scalar values and byte offsets, so silent rewriting would make every span a lie. When the SDK later *emits* a contract, emission order is the schema's declared order rather than alphabetical.
- **`contract_version` is a single monotonically increasing integer** in the domain 0 and above, `1` at M2. `0` is a legal value that is below the supported floor, which is what makes the below-floor branch derivable from a real fixture rather than only from a unit test. The SDK declares a contiguous supported range; the classification rules are in [diagnostics and compatibility](2026-07-31-diagnostics-and-compatibility.md).
- **Unknown keys are always fatal**, on both assets. A misspelled key is this format's worst failure mode — a typo'd `requred` silently demoting a required property, or a typo'd `capabilties` silently removing a closed-write policy — and no convenience is worth making it survivable.
- **Key legality is scoped to the declared version, and parsing is two-pass.** A key is legal exactly when the version the contract *declares* defines it — not when the reading tool happens to know it. So a file declaring version 1 with a version-2 key is refused identically by every tool, rather than loading on a newer build and producing a misleading unknown-key error on an older one. The parser therefore extracts `contract_version` first and validates the body against that version's schema second.
- **Version classification precedes and suppresses structural validation.** A contract outside the supported range yields exactly one compatibility diagnostic naming the version found and the range supported — never a pile of unknown-key errors that misdiagnose a newer format as a typo.

Fatal, version-scoped unknown keys are what make a single integer sufficient. A `major.minor` version's only machine meaning would be "an older tool may read the subset of a newer minor it understands," and strict parsing forecloses that. The supported *range* carries every distinction the version needs to make.

### Composition, authority, and provenance

**Every setting has exactly one authorized scope**, so the two assets never overlap and there is nothing to override. The committed contract owns types, properties, relationships, capabilities, the lifecycle declaration, write policy, dialect, and `contract_version`. The installation record owns the vault registry and actor identity. No invocation input — flag or environment variable — may supply a contract-owned setting; the partition is structural rather than policed.

The installation record deliberately carries **less than `architecture.md` assigns to machine-local configuration**, which names "editor dialect, paths, integrations, and presentation" and, elsewhere, index placement. None of those has a reader at M2: no index exists until M4, and `beta.md`'s Deferred list rules out working-tree dialect materialization for the whole beta, so there is no materialized editor dialect for M2 to select. These are deferrals, not removals — the per-machine editor dialect that [markdown-flavor](../product/markdown-flavor.md), [abstractions.md](../../abstractions.md), and [product.md](../../product.md) all promise remains promised, and lands in this record's file when a surface reads it. A field nothing reads is a guess waiting to be defended.

Actor identity is the one field kept without an M2 reader, and the asymmetry is deliberate rather than overlooked: it is the field a vault's *owner* sets once, so its absence is what the M2 `doctor` report is for — telling you now that provenance will be unattributed later, rather than at the first write.

**Provenance has three sources**, reported per resolved leaf value:

- `contract` — written explicitly in `.dogtag/contract.toml`, with the file path and span.
- `installation` — written explicitly in the installation record, with the file path and span.
- `default` — the format's declared default for an omitted optional field, attributed to the contract version that defines it.

The third source is the one that needed reconciling. The settings PDR rejects *live inheritance from current product defaults*, because an unchanged vault must not acquire new semantics when its SDK changes. A **format** default is not that: it is a property of `contract_version = 1`, and changing one requires a version bump that an unchanged vault does not have.

That holds only if resolution — not merely attribution — is version-scoped, so the rule is stated rather than implied: **the resolved model is shaped by the version the contract declares.** The SDK carries the declared defaults of every version in its supported range and resolves an omission against the declared version's table, never against the newest one it knows. A field that only a later version defines is **absent from** a model resolved at an earlier version, never defaulted into it. Without this, a tool supporting versions 1 and 2 would resolve a version-1 vault's omissions against version 2's defaults, change that vault's semantics on upgrade, and report provenance asserting it had not — worse than plain live inheritance, because the provenance would lie.

The M0 scenario `contract-loads-with-provenance` named a slightly different triple: *"the committed contract, an initialization default, or the local installation record."* That is not satisfied as written, and the earlier claim that it was is withdrawn. An *initialization* default is materialized into the contract at init time, so at read time its provenance is `contract` — it is not a third source. And the authority partition makes `installation` unreachable as the source of a contract-owned setting by construction. The scenario is corrected to name the three sources this record actually defines, which is a correction to acceptance material rather than a weakening of it.

Provenance is reported **per leaf value**, addressed by a stable dotted key path, because the question worth answering mechanically is *"is this property optional because the author decided so, or because nobody said?"* — and the omitted field is precisely the one a per-node model cannot describe.

### Invocation inputs

None of them semantic. Selection: `--vault` and `DOGTAG_VAULT` ([vault discovery and selection](2026-07-31-vault-discovery-and-selection.md)). Location: `XDG_CONFIG_HOME` locates the installation record, which also gives the conformance harness hermetic runs without a dogtag-specific variable. Rendering: `--format`, the `NO_COLOR` convention, and `--provenance` on `contract explain`. Exit behavior: `--strict`.

`--strict` promotes warning-severity diagnostics to a nonzero exit. It was initially deferred "until a real workflow asks for one" and then reinstated in the same packet, because the workflow that asks for one is this milestone's own cutover: an unattended scheduled health check whose only automatic signal is the exit code. Without it, a nested vault appearing nearer than the intended one resolves a *different corpus*, reports a warning, and exits `0` — the check reports healthy while inspecting the wrong vault, which is the outcome [the discovery record](2026-07-31-vault-discovery-and-selection.md) calls the worst available.

`XDG_CONFIG_HOME` deserves one caution it did not originally get: it is an attacker-reachable input, not merely a test convenience. Whoever controls an invocation's environment — a CI definition, a wrapper script, an agent harness — redirects the registry without needing any filesystem write. The installation record is trusted exactly as far as the environment that locates it, and that becomes a write-target question at the mutation milestone.

### The minimum M2 declarations

**Types.** `name` (the discriminator value) and `capabilities`, a subset of `identity-bearing` (0..n), `catch-all` (exactly one), and `closed-write` (0..n). An unknown capability name is a load error. Cardinality is validated when the contract loads, reasoning only over declarations and never over a type's name.

**Properties.** `name`, `kind`, and `required`, declared per type. The closed lattice of value kinds is eight: `string`, `integer`, `float`, `boolean`, `date`, `datetime`, `enum` (with `values`), and `list` (with `of`, naming any scalar kind — `list` may not nest). Three are forced by prior decisions: `enum`, because the lifecycle axis is a closed set of values; `boolean`, because flags are boolean properties; `list`, because [note-types](../product/note-types.md)' flavor-as-tags model has no other representation. The remaining five are judgement, and are named as such rather than dressed as forced.

`date` and `datetime` carry their lexical form here rather than leaving it to the milestone that parses notes: **`date` is an RFC 3339 `full-date` and `datetime` an RFC 3339 `date-time` with a mandatory offset.** Fixing this now is not scope creep but its opposite — their entire meaning *is* a lexical form, and leaving it undecided would hand the answer to whatever coercion the frontmatter parser happens to perform, which is the silent reinterpretation this record rejects YAML for. For the same reason `integer` and `float` are distinct on the wire: `1` is not a `float` and `1.0` is not an `integer`.

There are **no value constraints** — no `pattern`, no bounds, no `format` hint. A corpus that needs "this string is a URL" writes a linter over the SDK's public API, which is the product's stated answer to wanting more than the kernel does; a plugin system is a standing non-goal in [product.md](../../product.md). A `format` hint the kernel records but never enforces was rejected outright: in a contract whose premise is that declarations are enforced, a declared constraint nothing checks misleads every agent that reads it, and quietly reintroduces caller-owned reinterpretation.

**Relationships.** `predicate` and `required`, and nothing else. Relationships are declared as their own construct rather than as a property of some reference kind, because `architecture.md` makes note, type, property, and relationship four distinct kernel concepts. `required = true` means at least one edge with that predicate must be present; the maximum is what remains undecided, so cardinality as a whole is deferred rather than half-stated.

A `targets` key constraining which types may be the far end was decided and then removed. The stated justification — that the [relationships PDR](../product/relationships.md) fixed its shape — was simply false: that PDR enumerates its config-points exhaustively as the predicate vocabulary, which predicates each class requires, optional reification, and derived lenses, and a type-range constraint appears in none of them. Its only endpoint invariant is that a typed link resolves to a real note, which is a resolution check rather than a type restriction. With nothing fixing its shape and no scenario at any milestone asserting that it constrains anything, `targets` would have been a declared constraint the tool never keeps — rendered by `contract explain` into agent-facing instructions as though it did — which is precisely the defect this record rejects two paragraphs above for unenforced `format` hints. Links, typed links, resolution, and backlinks are all unaffected by its absence.

**Lifecycle.** A `[lifecycle]` table is **mandatory**, and declares either an axis or its explicit absence. See [the lifecycle declaration](2026-07-31-lifecycle-declaration-and-the-seam.md).

**Write policy.** The `closed-write` capability, and nothing else. [note-types](../product/note-types.md) fixes that the type is the key edit policy binds to, and the [authorship PDR](../product/authorship.md) owns the semantics; the richer vocabulary (human-only, ai-only, mixed, named authors) waits for the milestone that performs mutations.

**Dialect.** `[dialect]` with one key, `links`, valued `wikilink` or `markdown`. M2 parses, validates, and explains it; M3 consumes it.

The dividing line between what is declared now and what waits: **declare what an existing decision already fixes the shape of, defer what nothing has decided.** `dialect` qualifies because `architecture.md` already places it inside the committed contract, and M2 is the milestone deciding that contract's format. Relationship cardinality, `targets`, and the full write-policy vocabulary are fixed by nothing, and inventing them now is the freeze-a-guess failure [the conformance harness ADR](2026-07-30-conformance-harness-shape.md) rejected for fixture corpora.

### Validity rules enforced when the contract loads

All of these are checkable without reading a single note, and all of them are the difference between a contract that loads and a contract that is *correct*:

- Type names are unique; property names are unique within a type; predicates are unique within a type.
- A capability name is one of the three declared; catch-all cardinality is exactly one; the other two admit any number.
- `enum` declares a non-empty list of unique values; `list` declares an `of` naming a scalar kind.
- A property name used on more than one type declares the same kind on each, so a corpus-wide question about a property has one answer.
- The lifecycle rules in [the lifecycle record](2026-07-31-lifecycle-declaration-and-the-seam.md).

Without these, a contract with two identically named types, or an empty `enum`, loads with zero diagnostics and is then rendered by `contract explain` to an agent as the vault's rules. Adding a rule later is a tightening that some existing contract may fail, so the rules that are cheap to state are stated now rather than accumulated during implementation.

### What the two assets look like

The declarations above fix an inventory; this fixes the spelling, so that the fixtures, the parser, and the unknown-key allow-list are not each invented separately.

```toml
# .dogtag/contract.toml
contract_version = 1

[dialect]
links = "wikilink"          # or "markdown"

[lifecycle]
axis = "status"
ordinary = { absent = true }   # or { value = "current" }

[[flag]]
property = "leaned_on"

[[type]]
name = "person"
capabilities = ["identity-bearing"]

  [[type.property]]
  name = "full_name"
  kind = "string"
  required = true

  [[type.property]]
  name = "status"
  kind = "enum"
  values = ["draft", "archived", "superseded"]
  required = false

  [[type.property]]
  name = "leaned_on"
  kind = "boolean"
  required = false

  [[type.relationship]]
  predicate = "works-at"
  required = false

[[type]]
name = "capture"
capabilities = ["catch-all"]
```

```toml
# ~/.config/dogtag/installation.toml
installation_version = 1

[actor]
name = "A Maintainer"

[[vault]]
name = "work"
path = "/data/vaults/work"
```

**`installation_version` is mandatory from the first release that reads the file**, which is M2. This is not symmetry for its own sake: unknown keys are fatal on this asset too, so a version field cannot be retrofitted later — every already-installed build would reject the very key that announces the new format. It is classified against its own supported range with its own diagnostic identifiers, exactly as the contract is.

Registry entries carry a kebab-case `name` and an **absolute** `path`. No tilde expansion, no environment expansion, no relative paths: a relative registry path would resolve against the current directory and reintroduce the cwd-dependence the no-fallback selection rule exists to eliminate. Duplicate names are a load error, which is also what forecloses a last-wins shadowing of an existing name by an appended entry.

### Alternatives considered

- **`dogtag.toml` at the vault root.** Visible and discoverable, and the root marker would be the sentinel directly. Rejected: it sits in the note space, and any later committed asset needs either another root file or a directory added late — the dotted directory costs one ergonomic point now and is the answer to a question that will be asked again.
- **`.dogtag/schema.toml`, the README's name.** Rejected: it under-describes a file carrying dialect, write policy, and a version, and the README needed correcting regardless because it also promises a `target.toml` that `architecture.md` forbids.
- **YAML.** It would match note frontmatter, so an agent would speak one language. Rejected: implicit type coercion and anchor semantics are exactly the silent reinterpretation the determinism obligation forbids, and it would add a dependency. The cost accepted is real — the contract's language differs from frontmatter's.
- **JSON.** Rejected: no comments, and hostile to the hand-editing the README promises remains possible.
- **`major.minor` versioning.** Rejected as reasoning above: with fatal unknown keys the minor component carries no machine meaning.
- **Tolerating unknown keys for forward compatibility.** Rejected: it makes a typo silently drop a `required = true` or a `closed-write`, which is the format's worst failure made routine.
- **No format defaults; every field written explicitly.** Genuinely attractive — the file becomes readable in complete isolation. Rejected because it puts `required = false` and `capabilities = []` on every declaration in every contract. (The original rejection also leaned on a golden scenario being satisfiable without amendment, which turned out to be untrue; that half of the argument is withdrawn and the scenario is corrected.)
- **Keeping `targets` with an honest justification, or keeping it explicitly unenforced.** Rejected in favor of removing it: the first would add endpoint-type validation to a milestone that did not scope it, for a construct no fixture profile asked for; the second knowingly ships the misleading declaration this record rejects elsewhere.
- **Per-vault configurable defaults, a `[defaults]` table.** Rejected: no declaration could then be read in isolation, and a one-line edit would silently change the meaning of every type at once. Large blast radius for a few saved keystrokes.
- **Defaults supplied by the installed SDK.** Rejected: this is precisely the live inheritance the settings PDR ruled out.
- **Provenance per declaration node.** Rejected: it cannot say where an omitted field's value came from, and the omitted field is the one worth asking about.
- **A dogtag-specific variable for the installation record's location.** Rejected: `XDG_CONFIG_HOME` already does it, and a second way to say the same thing is a second thing to document, test, and keep consistent.
- **M2 writing the installation record.** Rejected: `doctor` stays read-only, and nothing at M2 — no scenario, no cutover — needs registration to happen through the tool.
- **Platform-native configuration paths on macOS.** Rejected: two code paths and two documented locations, diverging from the installer's existing posture, for no benefit a CLI user asked for.

## Consequences

- **The README's vault sketch is corrected by this record**, to one committed asset at `.dogtag/contract.toml` with the dialect inside it.
- **A hidden directory is invisible to a browsing human and inside Obsidian.** `dogtag doctor` and `dogtag contract explain` are what pay that back, and they are the two surfaces M2 ships — but a user who never runs them has no on-disk cue that the vault is configured.
- **Every additive format change costs `contract_version = 2`, and a hand migration for any vault that wants the new construct.** Stated precisely, because the first draft of this bullet overstated it: an unmigrated version-1 vault keeps loading, since the supported range covers it. Migration is the price of *using* a new construct, not of continuing to work. With no migration tooling in the beta, that price is a hand edit, paid by two fixtures and two founder vaults. The genuinely growing cost is on the SDK side — a per-version default table and a per-version key set, for every version the range keeps. The known deferrals are relationship cardinality, `targets`, the write-policy vocabulary, and any value constraints.
- **The authority partition means there is no override mechanism to explain**, and also none to reach for. A future setting that genuinely wants two scopes will need this record superseded rather than extended.
- **Index placement, editor dialect, paths, integrations, and presentation are all absent from the installation record**, so M4 and later either add them under a new `installation_version` or discover they belong elsewhere. Anyone comparing this record against `architecture.md`'s machine-local list will find it short, deliberately.
- **`float` is in the lattice with no consumer outside the fixtures.** A numeric property is ordinary in any corpus and the kind costs nothing beyond the one it already shares with `integer` — but note that the fixture floors do not mandate exercising every kind, precisely so that a kind nobody reaches for is visible as unused rather than kept alive by a coverage rule. Evidence that the lattice can shrink has to come from a real corpus.
- **A corpus wanting enforced value formats has no contract-level route.** The linter-over-the-SDK answer is real but unproven — no consumer has written one yet, which makes the [architecture obligation](../../architecture.md) about consumer-authored validation a claim this milestone asserts rather than demonstrates.

## Amendments

The Decision above stands as written; these later records change parts of it, and the original text is left intact so the change is legible.

- **2026-08-01 — version-scoped key legality is a promise, not yet a mechanism.** The Decision says key legality is scoped to the declared version and that the SDK carries every supported version's default table. The implementation carries neither dimension: the key sets are per-table constants and the two defaults are literals, and the declared version reaches only the diagnostic text and the provenance attribution. That is indistinguishable from correct while the supported range is `1..=1`, which is why it survived review twice. It stops being indistinguishable at the first bump, when a version-1 contract would be judged against the union key set and an omitted key would take version 2's value while its provenance said `contract_version = 1` — the lying provenance this record already names as worse than plain inheritance. **A per-version key set and default table are due before the supported range widens, and widening it without them is the regression.** Recorded as an explicit deferral rather than fixed now: at one supported version the mechanism has no reachable second branch, and the coverage floor bans unreachable branches.

- **2026-08-01 — a declaration's name is never empty and never holds a `.`, refused at load under `contract.declaration-name-invalid`.** Provenance is addressed by a dotted key path built by joining the corpus's own declaration names, and this record fixed a lexical form for the registry's `name` and for nothing else. A type named `t.property.p` therefore produced the same key as type `t`'s property `p`, and `Provenance::insert` replaces on collision — so `contract explain --provenance` could print, for one declaration's value, a source location that points at the line declaring another's. An empty name addressed nothing at all, yielding keys like `type..capabilities`, and rendered as an empty heading in the generated agent contract. The rule is checked at the single place a declaration's name is read, which is why one identifier covers type names, property names, relationship predicates, and flag properties rather than four. **The smallest rule that restores injectivity was chosen deliberately.** The obvious alternative — the kebab-case form this record already fixes for a registry entry's `name` — would reject `full_name`, which both fixture corpora use throughout, and would make the lexical shape of a corpus's own vocabulary the kernel's business, which is the opposite of every other rule here. Neither committed fixture holds a dotted or an empty name, so nothing in the tree migrates. One consequence reaches past provenance: a kernel diagnostic identifier is always dotted, so a declaration name can no longer spell one, and the headline-forgery vector that rendering-side folding answers for every other quoted value is closed outright for names. The two fixtures that forged a headline through a type name now plant a dotless one, so they still exercise the fold.

- **2026-08-01 — `[dialect]`'s mandatoriness and version 1's default table were inferred, not decided.** The Decision states `[lifecycle]` is mandatory and says only what `[dialect]` contains, never whether it may be omitted. The parser inferred that absence is a load error and minted `contract.missing-dialect` for it — a permanent public identifier derived from an inference. The two implemented defaults (`required = false`, `capabilities = []`) appear in this record only inside a rejected alternative, never as the version-1 table the SDK is obliged to carry. Both readings are reasonable; neither is decided here, and a later decision that `links` defaults to `wikilink` would change every existing v1 vault's meaning with no version bump available to carry it.

- **2026-08-01 — four enforced rules this record does not state.** `contract.duplicate-flag`, `contract.no-types`, and the two `contract.lifecycle-ordinary-invalid` refusals (not exactly one of `value`/`absent`; `absent = false` and `none = false`) are all enforced and all absent from the validity list. Each is a reasonable reading and each mints a permanent identifier, so the record cannot currently be used to check the implementation. **A fifth joined them on the same date and is recorded in the next bullet**, `contract.duplicate-capability`, which differs from the four above in that the record does speak to it — ambiguously.

- **2026-08-01 — one type may not declare the same capability twice, refused at load under `contract.duplicate-capability`.** The validity list says a capability name is one of the three declared, that catch-all cardinality is exactly one, and that "the other two admit any number" — a sentence about how many *types* may carry a capability, which the parser also read as licensing the same name twice within one type's list. `capabilities = ["catch-all", "catch-all"]` therefore loaded clean, and the two surfaces that show it then disagreed: the generated agent contract rendered `### \`capture\` — catch-all, catch-all`, because the heading joins the declared list, while `doctor` listed the type once under that capability, because it filters types. A repeat is now a load error pointing at both occurrences with `first declared here`, which is what `contract.duplicate-type`, `contract.duplicate-property`, `contract.duplicate-predicate` and `contract.duplicate-flag` all already are — a repeat needs two spans, and the resolved model keeps one declaration per name. A repeated *unknown* capability is still reported as unknown once per occurrence: nothing is claimed for a name the format does not define, so the narrower fault is never swallowed by the wider one. Like the four above, this mints a permanent public identifier from a reading rather than from a stated rule.

- **2026-08-01 — a version too large for a `u32` is classified, not refused by the parser. Fixed on both assets.** This record puts `contract_version` in the domain 0 and above and makes classification total over it, sending anything above the supported range to `compat.contract-too-new`; `installation_version` is fixed symmetrically. `contract_version = 4294967296` instead produced `contract.version-invalid`, and `installation_version = 4294967296` produced `installation.version-invalid` — the inversion the next paragraph forbids, on both assets. Both now classify: a literal that is not negative and does not fit a `u32` is *above every range this SDK can support, whatever the range is*, so it takes `compat.contract-too-new` or `compat.installation-too-new`. **Both versions stay `u32` deliberately.** No fixed-width type makes classification total, because the domain the format declares has no upper bound and TOML keeps the literal rather than converting it — widening to `u64` or `i128` would only move the same defect further out, so the version above the range travels as *the file's own bytes* instead. That is also what makes the message honest about radix and digit separators: `contract_version = 0xFFFF_FFFF_F` now renders as written, where the previous refusal restated it as `0xFFFFFFFFF` on the contract and as `FFFFFFFFF` on the record — two different lies about one file. Consequences: the identifier changes on both assets and each refusal now carries the compatibility diagnostic's help; the contract refusal moves from the value's span to the whole file, which is where every other too-new contract already pointed, while the record refusal keeps its span and gains a help line it did not have. `doctor`'s contract version section reads `beyond \`0..=4294967295\` (too new; supported 1..=1)` where it previously read `not declared`. The exit code is unaffected. The out-of-domain message for a *negative* version is reworded on both assets to say "a whole number 0 or above" rather than naming a `u32` range that is no longer the domain. `SCHEMA_VERSION` is deliberately **not** bumped: no field name or type changed, and `version.found` was already `null` on a contract that declares no version.
