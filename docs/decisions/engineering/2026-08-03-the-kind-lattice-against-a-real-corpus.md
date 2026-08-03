# The value-kind lattice against a real corpus

- Status: accepted
- Date: 2026-08-03

## Context

[The vault-contract record](2026-07-31-vault-contract-and-installation-record.md) fixes a closed lattice of eight value kinds — `string`, `integer`, `float`, `boolean`, `date`, `datetime`, `enum`, and `list`, the last naming any scalar kind and forbidden from nesting. It closes with a consequence that frames the lattice's open question in one direction only: `float` ships with no consumer outside the fixtures, the fixture floors deliberately do not mandate exercising every kind so that an unused kind stays visible, and *"evidence that the lattice can shrink has to come from a real corpus."*

A real corpus has now spoken, and it said the opposite thing.

M2's cutover requires each founder vault's contract to be hand-authored and to load with zero diagnostics, which is the first evidence step and a private act this trail can only cite. One of the two has now been authored. It loads clean — types in the tens, all three capabilities in use, an axis whose ordinary state is absence, the wikilink dialect — but reaching that state required leaving several of the corpus's real properties **undeclared**, because the format has no way to say what they are. That is a different finding from an unused kind, and it arrived through the one channel the record said it would have to.

Nothing here is a defect in the implementation. The parser enforces exactly what the format specifies; the format specifies a lattice that this corpus exceeds.

## Decision

**The two gaps are recorded now, with the shape of the fix named, and the fix itself deferred.** The deferral is forced rather than chosen: every additive format change costs `contract_version = 2`, and [the compatibility record](2026-07-31-diagnostics-and-compatibility.md) freezes the supported floor for the whole beta. There is no version to spend inside this ladder. What is decided is that the evidence is written down while it is fresh, and that the fix is named specifically enough that a later version cannot reach for a weaker one and call it done.

Per [the privacy gate](2026-07-31-m2-fixtures-and-the-privacy-gate.md), only shape crosses from a founder vault into this repository. No property name, type name, or lifecycle word from that corpus appears below; the counts are orders of magnitude rather than a census; and the illustrative names in the sketches are invented for this record.

### Gap one: the corpus holds records, and `list` may not nest

Roughly half a dozen properties on a single identity-bearing type hold either a **record** — a small, fixed set of named sub-fields — or a **list of records**. The canonical instance is a labeled multi-value channel, the shape any contact model reaches for when one node has several addresses of the same kind and each needs a label alongside its value. A second instance is a structured personal name, whose parts must stay addressable rather than being re-split out of a display string.

Neither is exotic and neither is a corpus eccentricity. They are what an identity-bearing type looks like once it has to round-trip against an external contact system, which is precisely the convergence [note-types](../product/note-types.md) says the identity-bearing capability exists to serve.

The properties are therefore **omitted from that contract rather than approximated**, and a comment in the file names each omission and why. Declaring them `string` would have been the tempting move and is the wrong one: `contract explain` renders every declaration into agent-facing instructions, so a `string` there does not degrade gracefully — it actively instructs an agent that a record is a scalar. That is the same defect this trail already rejected for unenforced `format` hints, arriving from the other side.

**The fix, when a version can carry it, is a `record` kind that declares its own fields, plus permitting `list` to name it in `of`.** Sketched with invented names:

```toml
  [[type.property]]
  name = "waypoints"
  kind = "list"
  of = "record"

    [[type.property.field]]
    name = "caption"
    kind = "string"
    required = true

    [[type.property.field]]
    name = "reached_on"
    kind = "date"
```

One level of nesting, with fields drawn from the existing scalar lattice and no recursion. That bound is the point: it covers both observed shapes exactly, and it keeps a declaration readable in isolation, which is the property the rejected `[defaults]` table was rejected for destroying.

### Gap two: a reserved prefix is a pattern, and the format declares names

An editor in that vault's toolchain writes its own system properties into note frontmatter under a reserved single-character prefix, on essentially every note in the corpus. The corpus's own schema handles this as a **prefix rule** — any key matching the prefix is permitted, none is enumerated — because the set is the editor's to grow and enumerating it would mean chasing a third party's releases.

The contract format declares names. A prefix has no expression in it, and inventing one would be the format taking a position on a foreign tool's namespace.

**No fix is proposed, deliberately.** Both available shapes are worse than the gap. A wildcard property would punch a hole straight through the rule that makes unknown keys fatal — the rule this format's worst failure mode depends on. A general pattern facility would be a constraint language, which is the direction [the vault-contract record](2026-07-31-vault-contract-and-installation-record.md) already refused when it removed value constraints and told a corpus that wants more than the kernel does to write a linter over the SDK's public API. The honest statement is that **a contract cannot describe frontmatter a foreign tool owns**, and that a corpus in that position has keys its contract is silent about. Recording that is worth more than papering it.

### Alternatives considered

- **An opaque or JSON-blob kind.** One kind, and every unexpressible shape becomes expressible immediately. Rejected: it says "there is structure here that I decline to describe" about exactly the data an agent most needs described, and `contract explain` would render that opacity into instructions as though it were the author's intent. It also removes the pressure that produces a real answer — a blob absorbs the second and third findings silently, so the lattice would stop learning from corpora at the moment it started.
- **Declaring the record-valued properties as `string`.** Keeps the keys visible in the contract. Rejected as above: a declaration that is wrong about a shape is worse than a declaration that is absent, because absence is legible and error is not.
- **Declaring the list-of-record properties as `list` of `string`.** Correct about the container, wrong about the element. Rejected for the same reason, with the added cost that it is *nearly* right, which is the kind of wrong a reader stops checking.
- **A recursive record kind.** More general, and no harder to specify. Rejected: nothing observed needs it, generality here buys an unbounded rendering problem for `contract explain`, and a second level can be added by a later version far more easily than it can be removed.
- **Widening the lattice inside the beta.** Rejected: it is not available. The supported range is frozen at `1..=1` for the ladder's duration, and the amendment already on the vault-contract record warns that widening the range before a per-version key set and default table exist is itself the regression.
- **Saying nothing until a version is available to spend.** Rejected: the evidence is a founder vault's one-time authoring pass, and the reasoning behind each omission is at its sharpest on the day the omission was made. A record written later would be reconstructing it.

## Consequences

- **One founder vault's contract is knowingly incomplete**, and its own comments say so. Every consequence of that lands at the milestone that first validates notes against a contract — not at M2, which reads no note. At that milestone the omitted keys become undeclared-key findings on a corpus that is otherwise clean, and that must be understood as this record's deferral surfacing rather than as a corpus defect.
- **The lattice's open question is now two-sided.** `float` is still a kind with no consumer, and the fixture floors still leave it visible as unused. But shrink is no longer the only move the evidence supports, and any future pass at the lattice has to weigh a removal against an addition that a real corpus has already asked for.
- **The record kind is scoped before it is scheduled**, which inverts the usual risk. The freeze-a-guess failure this trail warns about is fixing a shape nothing has asked for; here a shape has been asked for and the cost of naming it now is that a later milestone may find a third instance that does not fit the one-level bound.
- **`starter` and `dense` remain within the v1 lattice**, so neither fixture demonstrates this gap and the conformance matrix will not surface it. The evidence lives in this record and in a private vault, which is the weakest form of evidence this repository accepts — and it is accepted here for the same reason the privacy gate has no mechanical enforcement.
- **The prefix gap has no scheduled resolution and may never get one.** A corpus whose editor owns part of its frontmatter has a contract that is silent about that part, indefinitely. That is a real limit on the claim that a contract describes a vault, and it is stated rather than filed.
