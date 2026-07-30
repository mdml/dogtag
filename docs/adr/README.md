# Architecture decision records

This trail records **build decisions** for this repository: layout, toolchain, dependency policy, test structure, release pipeline — the choices about how dogtag is built and shipped, as opposed to how it behaves.

**Product stances live elsewhere.** A decision about how dogtag behaves for its users — for any PKM, on any vault — is a product decision record (PDR) and belongs in [docs/decisions/](../decisions/README.md). A decision about this repository's own machinery belongs here. Rule of thumb: if it would still matter to someone reimplementing dogtag from the docs alone, it is a PDR; if it only matters to someone working in this repository, it is an ADR.

## Format

One file per decision, named `YYYY-MM-DD-slug.md` — the date the decision was made, then a kebab-case slug:

```markdown
# <Title>

- Status: proposed | accepted | superseded by [<slug>](<YYYY-MM-DD-slug.md>)
- Date: YYYY-MM-DD

## Context
## Decision
## Consequences
```

## Conventions

- **Honest rationale.** Record the real reasons, including the unglamorous ones — cost, time, uncertainty, taste. An ADR that flatters its decision is worthless on the day the decision needs revisiting.
- **Alternatives considered.** Every ADR names what was rejected and why, inside the Decision section. Most of an ADR's future value lives there.
- **Consequences include the bad ones.** Accepted risks and trade-offs go in Consequences, not in a drawer.
- **Supersede, don't delete.** When a decision changes, write a new ADR and flip the old one's Status to `superseded by …` with a link. Never rewrite an accepted ADR's Decision after the fact; history stays legible.
