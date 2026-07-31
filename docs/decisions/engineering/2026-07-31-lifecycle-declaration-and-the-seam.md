# The lifecycle declaration, and resolving the golden-scenario assumption

- Status: accepted
- Date: 2026-07-31

## Context

The M0 golden scenario `list-filters-by-declared-lifecycle-axis` opens *"Given a contract that names its lifecycle axis and declares how the ordinary state is encoded."* Every scenario runs against every fixture profile and there are no waivers, so that clause asserts something about all four profiles — and `docs` is a repository documentation tree in which most files carry no frontmatter at all. A corpus of that shape plausibly has no life axis to name.

That is the seam under test, not an incidental wording problem. The [lifecycle PDR](../product/lifecycle.md)'s 2026-07-30 amendment says the substrate binds to *a corpus's declaration* of which property is its life axis, never to a vocabulary it knows, precisely so that a differently-shaped corpus is not translated at the boundary. A scenario that cannot run against a corpus without a lifecycle would be testing the opposite of that. [strategy.md](../../strategy.md) names this exact situation as a leading signal to change course: *"Conformance scenarios cannot be expressed against more than one fixture profile without special-casing."*

So the assumption has to be resolved through the configuration model. A profile waiver is not available and, per [the conformance harness ADR](2026-07-30-conformance-harness-shape.md), is not expressible.

## Decision

**A `[lifecycle]` table is mandatory in every contract, and the axis it declares is optional.** The table declares either an axis or the explicit absence of one; omitting the table entirely is a load error.

```toml
# A corpus whose ordinary state is the absence of a value
[lifecycle]
axis = "status"
ordinary = { absent = true }

# A corpus whose ordinary state is a named value
[lifecycle]
axis = "stage"
ordinary = { value = "current" }

# A corpus with no life axis
[lifecycle]
none = true
```

**The values come from the axis property's own `enum` declaration**, never restated inside `[lifecycle]`. One source of truth, and drift between the two is not merely caught but impossible.

**Load-time consistency checks**, all of which reason over declarations and none of which knows a vocabulary word:

- `axis` names a property declared on at least one type, and that property's kind is `enum`.
- `ordinary = { value = … }` requires the named value to be a member of that enum, and requires the property to be `required = true` on every type that declares it — the ordinary state cannot be a named value that notes are allowed to omit.
- `ordinary = { absent = true }` requires the property to be `required = false` — absence cannot be the ordinary state of a property every note must carry.
- `none = true` is exclusive with `axis` and `ordinary`.

**Flags are declared alongside the axis.** Each `[[flag]]` names a declared property whose kind is `boolean`. Orthogonality is structural rather than checked: a flag is a separate property from the axis, so it cannot be a point on it.

**Filtering by lifecycle against a contract that declares no axis is an error diagnostic**, not a silently empty or silently full result. Returning everything would claim the whole corpus is in the ordinary state; returning nothing would claim none of it is. Both are lies a caller cannot detect.

### What this does to the scenario

`list-filters-by-declared-lifecycle-axis` becomes expressible against all four profiles without a waiver and without naming a vocabulary word. Two profiles filter by a declared axis in opposite encodings — `dense` with the ordinary state absent, `starter` with it a named value — and any profile declaring `none` receives a stable diagnostic derived entirely from its own declaration.

The result is a stronger seam test than the original scenario could give. The original proved the core does not know *which* lifecycle words a corpus uses. This one additionally proves the core does not assume a corpus **has** a lifecycle at all — which is the assumption most likely to be one author's convention wearing an invariant's clothes, since every corpus the maintainers have ever built has one.

### Alternatives considered

- **A mandatory lifecycle axis in every contract.** The scenario would run everywhere unchanged, with no new construct. Rejected: `docs` would have to declare a lifecycle it does not have, so the fixture would carry a declaration that exists to satisfy the tool rather than to describe the corpus. That is a product convention imposed on a corpus — the exact failure the no-waiver rule exists to detect — and it would make the matrix green by making the seam fake.
- **Optional by omission: no `[lifecycle]` table means no axis.** Least ceremony, and no redundant declaration in corpora that have no lifecycle. Rejected: it makes silence a decision. A forgotten table and a deliberately absent one are indistinguishable, in a format where every other omission of a required construct is fatal, and where the entire unknown-key policy exists because silent omission is the worst failure available. The cost of rejecting it is one three-word table in corpora that have nothing to say.
- **Restating the axis values inside `[lifecycle]`.** More readable in one place. Rejected: two copies of one closed set, with nothing preventing them from disagreeing.
- **A magic string sentinel, `ordinary = "absent"`.** Flatter than a table. Rejected: it reserves a vocabulary word. A corpus whose enum genuinely contains a value named `absent` could not express itself, and reserving vocabulary is what the capability model exists to avoid.
- **Presence or absence of an `ordinary_value` key.** Fewest characters. Rejected: it reintroduces omission-as-decision inside the very table made mandatory to remove it.
- **Deferring flags to the milestone that filters by them.** Strictest scope discipline. Rejected: the lifecycle PDR treats one axis plus orthogonal flags as a single model, so splitting it across two contract versions is arbitrary, and the shape is already fixed — a flag is a boolean property. Both fixtures would otherwise describe a lifecycle model their contracts could only half state.
- **Declaring per-type flag eligibility now.** The lifecycle PDR notes the pause flag belongs only to types with work-progress. Rejected: eligibility is enforcement nothing performs until notes are read, and the PDR calls it config without fixing its shape.
- **Returning an empty result when filtering a corpus with no axis.** Rejected: a lie the caller cannot detect.

## Consequences

- **Every contract carries a `[lifecycle]` table, including corpora with nothing to say.** `[lifecycle] none = true` is three words of ceremony that buys the guarantee that no lifecycle state is ever inferred from silence.
- **`none = true` is a real declaration with real consequences**, and a corpus that adopts a lifecycle later changes it deliberately rather than by adding a table nobody notices.
- **If lifecycle ever becomes genuinely mandatory, that is a `contract_version` bump plus a corpus decision for everyone who declared `none`.** This is the low-reversibility half of the decision; the syntax is cheap to change, the model is not.
- **The consistency checks give M2 real validation work that touches no note**, which is what keeps the milestone's contract-versus-corpus line clean while still producing something worth running.
- **`docs` and `records` will most likely declare `none`**, which means the profiles carrying the closed-write and repeated-basename axes contribute nothing to the lifecycle seam. The seam's *filtering* evidence arrives at M3, since `list-filters-by-declared-lifecycle-axis` is an M3 scenario, and rests on `dense` and `starter` alone; M2 contributes only the load-time declaration check. That is deliberate — [beta.md](../../beta.md#fixture-profiles) pairs the two profiles for exactly this — but worth stating plainly rather than letting a four-profile matrix imply four-profile evidence.
- **This record moves a line the [lifecycle PDR](../product/lifecycle.md) drew as an invariant**, so that PDR is amended rather than left to contradict it. "Whether a corpus has a life axis at all" was not among its config-points; it is now, and the PDR's header says so, because turning an invariant into a config-point is a deliberate and visible act in that trail. The product argument lives there; the TOML spelling lives here.
