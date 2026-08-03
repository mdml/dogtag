# The public document model: membership, frontmatter, typing, and links

- Status: accepted
- Date: 2026-08-03

## Context

M3 ships the public document model plus `check`, `list`, and `show` ([beta.md](../../beta.md#milestones)). Most of the model is already fixed by standing records and is consumed here rather than decided: notes are plain Markdown files whose frontmatter is the schema'd plane and whose body is unschema'd prose; identity is the path and a bare name is a per-reference resolution shorthand ([markdown-flavor](../product/markdown-flavor.md)); every note carries exactly one type and flavor lives in tags ([note-types](../product/note-types.md)); typed links live in frontmatter, must resolve, and their inverses are derived, while an untyped prose reference may legitimately point at a note that does not exist yet ([relationships](../product/relationships.md)); property kinds carry fixed lexical forms — RFC 3339 dates, `integer` and `float` distinct on the wire ([the vault-contract record](2026-07-31-vault-contract-and-installation-record.md)); and `show` returns one shared shape across every profile (the `show-returns-document-model` scenario).

What was deliberately left to this milestone is recorded in [the M2 surfaces record](2026-07-31-m2-surfaces-and-the-sdk-boundary.md): the traversal policy — what counts as a note, which directories are skipped, how symlinks are treated — was named an M3 decision that M2 refused to freeze by accident. Two later findings shape the rest. [The kind-lattice record](2026-08-03-the-kind-lattice-against-a-real-corpus.md) establishes that one founder corpus carries frontmatter keys a foreign editor owns, on essentially every note, that no contract can declare — so note-side unknown keys cannot be treated the way contract-side unknown keys are. And the `docs` profile specification commits to a corpus in which most files carry no frontmatter at all, with the discriminator coming "from declared defaults rather than explicit frontmatter."

## Decision

### Membership and traversal

**A note is any file with the `.md` extension under the vault root, found by a traversal that skips every directory whose name begins with `.` and does not follow symlinks.** There is no configurable ignore list.

The dot rule is one rule doing three jobs: it excludes `.dogtag/` (the contract is not a note — [the vault-contract record](2026-07-31-vault-contract-and-installation-record.md) put it in a dotted directory precisely to keep it out of the note space), and it excludes `.git/`, `.obsidian/`, and every other tool's private directory without the format learning any tool's name. Non-Markdown files are ignored silently — they are not notes and their presence is not a finding. A `.md` file that cannot be read (permissions, invalid UTF-8) is a diagnostic against that file, never a traversal abort: one unreadable note must not make the corpus unreadable.

Symlinks are not followed because identity is the path: a note reachable through a symlink would hold two identities, every resolution answer would depend on which one the traversal met first, and a cycle would need detection machinery whose only job is coping with a shape the identity model already rules out. A symlinked directory inside a vault is therefore invisible to dogtag, which is a real limit stated here rather than discovered.

Traversal order never reaches output: results and diagnostics are sorted by the deterministic total order [the diagnostics record](2026-07-31-diagnostics-and-compatibility.md) already fixes, so the filesystem's enumeration order is unobservable.

### Frontmatter: a strict YAML subset, kind-driven

**Frontmatter is parsed as a restricted YAML subset: plain scalars, sequences, and mappings nested at most one level below the top (the depth the `record` kind needs). Anchors, aliases, tags, multi-document streams, non-string keys, and duplicate keys are refused, each with its own diagnostic.** The vault-contract record rejected YAML for the contract because implicit coercion and anchor semantics are silent reinterpretation; frontmatter is where the corpus already lives, so the language is kept but the dangerous half is fenced off.

**Every scalar is read as its bytes and validated against the declared kind's lexical form — nothing is coerced.** `1` satisfies `integer` and not `float`; `1.0` the reverse; `date` and `datetime` are exactly the RFC 3339 forms the contract record fixed; an `enum` value must be a member; a `boolean` is `true` or `false`; a `string` property accepts any scalar's bytes as they were written. YAML's implicit typing never runs: `NO` is a string, not a boolean, because the declared kind — not the parser's guess — decides what a value means. A value that fails its declared kind's lexical form is a diagnostic naming the kind and the span. Values of undeclared keys are not validated, because there is no declaration to validate against.

Encoding faults diverge from the contract's strict trio deliberately: **invalid UTF-8 is an error** (the file cannot be honestly read), while **a BOM or CRLF line endings on a note are warnings.** The contract is dogtag's own asset and can be held to one encoding; a corpus is decades of files written by whatever wrote them, and refusing to read a CRLF note fails the markdown-flavor obligation to read what is there. Nothing is silently normalized — spans are measured over the file's actual bytes.

### Typing and the catch-all binding

**The discriminator is the frontmatter key `type`, whose value must be a scalar naming a declared type.** This is the format's one reserved frontmatter word, and it is consumed from [note-types](../product/note-types.md) rather than minted here: that record's invariant is one class per note, its surfaces section enforces the `type:` field, and its receipts record the single `type:` key as the incumbent's sole on-disk discriminator. A configurable discriminator key would be a new construct nothing has asked for.

**A note with no frontmatter, or frontmatter without a `type` key, is a member of the catch-all type.** This is what the capability means: [architecture.md](../../architecture.md) defines catch-all as the bottom type that accepts anything so capture never blocks on classification, and the `docs` profile already depends on the discriminator coming from declared defaults. The binding is derived from the capability declaration alone — no new contract key, and it works identically for a version-1 contract.

A `type` value that is not a scalar is a diagnostic; a scalar naming no declared type is the `unknown-type` diagnostic the M0 scenario fixes. Neither falls back to the catch-all: the catch-all binds *absence*, never *error* — a note that says what it is and is wrong must be reported as wrong, not silently reclassified.

The binding makes one contract shape incoherent — a catch-all that declares `required = true` properties would render "accepts anything" beside requirements every untyped note instantly fails. [The contract-version-2 record](2026-08-03-contract-version-2.md) adds the load-time rule forbidding it, scoped to version 2, because tightening version 1's validity would stop a previously-loading vault from loading, which breaks the upgrade promise. In a version-1 corpus whose catch-all requires properties, untyped notes simply collect missing-required findings.

### Undeclared keys are `info`

**A frontmatter key that is neither a declared property, a declared relationship predicate, the discriminator, nor the declared tag property is reported at `info` severity, per key, against its span.** It is never an error, never a warning, and `--strict` does not touch it (`--strict` promotes warnings only, per [the diagnostics record](2026-07-31-diagnostics-and-compatibility.md)).

Contract-side unknown keys are fatal because both sides of that file belong to dogtag. Note-side they cannot be: the kind-lattice record's prefix gap is a foreign editor writing its own keys on essentially every note of a real corpus, permanently, and no contract can describe them. An error would make that corpus unopenable; a warning would fire on every note forever, which is precisely the failure mode the diagnostics record created `info` to prevent — a warning level that recurs benignly trains its reader to ignore warnings. `info` keeps the finding visible in `check`'s report (the kind-lattice record expects these findings and names them "the deferral surfacing rather than a corpus defect") without poisoning the exit code the cutover's scheduled run depends on. The cost is stated plainly: a typo'd *optional* property key is also `info`, and a corpus that wants it louder writes a linter over the SDK's public API — the product's standing answer to wanting more than the kernel does.

### Properties, relationships, and links

A declared property's value is validated as above. A declared relationship appears in frontmatter as a key named by its predicate, valued a link or a sequence of links, written in the contract's declared dialect (`wikilink` or `markdown` — M2 parsed and explained `[dialect]`; this milestone is its first consumer). **A typed link must resolve; one that does not is the `dangling-typed-link` error.** A `required = true` predicate with no edge is a missing-required finding.

**Untyped body wikilinks are parsed and resolved for the surfaces that want them, but a dangling untyped reference is not a finding** — the relationships record is explicit that a prose reference belongs in prose until its target exists, so danglingness is only a defect where a relationship was claimed.

**Name resolution:** a note's name is its filename without the `.md` extension. A reference containing no `/` is a bare name and resolves iff exactly one note in the corpus bears that name; a reference containing `/` is path-qualified, resolved against the vault root, with `.md` appended when absent. Ambiguity is an error against the reference, carrying every candidate path as related evidence — the markdown-flavor model ("ambiguity is a defect of the link, not of the corpus") made mechanical. Two notes sharing a name, with nothing referencing the bare name, is not a finding of any severity.

### The document-model shape and its diagnostics

`show`'s result — the one shared shape the M0 scenario demands — is: the note's identity (its vault-relative path), its declared type and whether it was bound by declaration or by catch-all, its properties as declared-kind values, its relationships as resolved edges, its tags (the declared tag property's values, when the contract declares one), and its body as uninterpreted text. The title is the first H1, carried as display metadata and never as identity.

M3 mints two diagnostic areas, per [the M3 surfaces record](2026-08-03-m3-surfaces-check-list-show.md): **`note`** for a single note's own structure and **`link`** for reference resolution. The core identifiers this record's rules produce: `note.unknown-type`, `note.type-invalid`, `note.missing-required-property`, `note.property-kind-invalid`, `note.undeclared-property` (info), `note.frontmatter-unsupported` (refused YAML constructs), `note.frontmatter-invalid` (parse failure), `note.unreadable`, the encoding pair at warning, the tag-namespace pair defined in [the contract-version-2 record](2026-08-03-contract-version-2.md), and `link.dangling-typed-link`, `link.ambiguous-reference`, `link.target-not-found`. The full enum is called out for review in its own right when it lands, as the M2 identifier set was — identifiers are permanent, and this list is the record's floor, not a cap on the review.

### Alternatives considered

- **Following symlinks.** Rejected: two paths would be two identities for one note, and resolution would depend on traversal order.
- **A contract-level ignore list.** Rejected: a new version-2 key nothing has asked for, and a second membership rule to explain in `contract explain`.
- **Missing `type` as a diagnostic.** Rejected: it contradicts the `docs` profile specification and the catch-all capability's stated purpose; offering it would mean superseding both.
- **An explicit default-type contract key.** Rejected: redundant with the catch-all's meaning, plus cardinality rules for a construct the capability already carries.
- **Undeclared keys as warnings, or unreported.** Rejected as reasoning above: the first blocks the cutover and burns the warning level; the second hides the typo'd-optional-key failure mode entirely and contradicts the kind-lattice record's expectation that the deferral surfaces.
- **Full YAML 1.2 with post-hoc typing.** Rejected: implicit coercion and anchor semantics are the silent reinterpretation the contract record already refused, arriving through the other file.
- **A subset without nested mappings.** Rejected: record-valued properties would be unrepresentable at the milestone that adds the `record` kind.
- **A configurable discriminator key.** Rejected: a new construct with no requester; note-types already fixes the spelling and the invariant.
- **Falling back to the catch-all on an unknown `type` value.** Rejected: reclassifying an error as an answer, silently.

## Consequences

- **One founder corpus will carry permanent `info` findings on essentially every note**, by design, and the `check` report will say so on every run. That is the prefix gap being visible rather than papered, and the exit code is unaffected.
- **A dotted directory is invisible, wholesale.** A user who keeps notes under a dot-directory loses them without a diagnostic; the rule is cheap to state and this is its price.
- **The YAML subset will refuse real files.** A corpus using anchors or merge keys does not load its frontmatter, with a diagnostic naming the construct. That is a deliberate narrowing of what "reads what is there" claims, and it is the same trade the contract made, recorded rather than implied.
- **`type` is the format's one reserved frontmatter word.** A corpus whose vocabulary wants `type` to mean something else cannot have it; every other key name remains the corpus's own.
- **The catch-all binding gives `docs` its viability** and gives capture its no-blocking property, at the cost that a typo'd `type` key's *absence* (the key misspelled entirely, e.g. `tyep:`) binds the note to the catch-all and reports the real key at `info` — the one place the typo story is soft, stated here so nobody rediscovers it as a surprise.
- **Body content is untouched at M3 beyond link extraction.** No heading structure, no fences, no tasks — the body stays prose until a milestone needs otherwise.
