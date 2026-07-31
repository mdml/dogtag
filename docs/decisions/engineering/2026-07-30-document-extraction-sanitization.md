# Document extraction and sanitization

- Status: accepted (amended 2026-07-31 — see [Amendments](#amendments))
- Date: 2026-07-30

## Context

The six product documents and the ten product decision records were drafted inside the maintainers' private repository, interleaved with personal infrastructure details, references to private planning records, other people's names, and working-process scratch. Moving them here required a defined transformation: the public copies must read standalone to someone with no access to the source repository, carry no private material, and still preserve the decision substance — the reasoning and its dates are what make the documents trustworthy.

## Decision

The documents were moved in one sanitization pass governed by these rules:

1. **Decision substance is kept.** Intents, designs, trade-offs, rejected alternatives, the configuration-versus-invariant seam, persona and fixture tables, decision dates, generic illustrative examples, and references to public products at their real URLs all move intact.
2. **Private identities are removed.** Names of private individuals and the identities of prospective beta participants do not appear; participants are described by role or persona only. Public attribution of published patterns (an author's published format, a cited open-source project) is retained — public authorship is credit, not leakage.
3. **Private infrastructure is generalized.** Hostnames, hosting providers, and self-hosted services are replaced with role descriptions ("a self-hosted forge"), and only where the decision's argument actually needs the role; otherwise the reference is dropped.
4. **Links are repointed by rule, not case by case.** Cross-references among the moved documents become repository-relative links. Pointers to a canonical schema definition that lived in private tooling are repointed at their new canonical home — each vault's committed contract, validated by the SDK's forthcoming contract types. Links into private decision records are dropped; where such a record materially corroborated an argument, a single linkless prose clause preserves the corroboration, with no slugs or dates. References to private planning documents become linkless prose ("the maintainers' private planning record") or pointers to this repository's own ADR trail where sequencing is what was meant.
5. **Working-process archaeology is deleted.** Command transcripts from personal tooling, migration histories, corpus statistics, drafting punch lists, and process-scratch sections are removed rather than generalized — they are evidence of process, not decision substance, and their absence loses nothing a public reader needs.
6. **The acceptance test is standalone readability.** Every moved document must make complete sense to a reader with no access to the source repository; any sentence that fails that test is rewritten or removed.

### Alternatives considered

- **Rewriting the documents from scratch for publication.** Rejected: a fresh rewrite would launder away the recorded reasoning, the dates, and the rejected alternatives — precisely the properties that distinguish a decision record from marketing copy.
- **Publishing with visible redaction markers.** Rejected: inline `[redacted]` scars draw attention to what was removed, read poorly, and invite speculation; clean transformation by rule produces documents that stand on their own.
- **Keeping the private copies canonical and mirroring sanitized copies here.** Rejected: dual canonical homes drift, and it would contradict the doc set's own one-canonical-home rule. The public copies are canonical from this commit forward; the private repository remains the process record only.

## Consequences

- The documents keep honest dates and decision history while carrying no private content; a dedicated privacy review pass verifies the result mechanically (pattern searches for the removed categories) before release.
- A few corroborating claims now stand without their receipts. Readers must take them on the record's word; that trade was accepted deliberately in favor of participant and infrastructure privacy.
- Future edits to the six documents and PDRs happen here, under these same rules — anything drafted privately first must pass the same transformation before it lands.
- The same gate binds future fixture work: a derived fixture contract (the `dense` profile's, mechanically derived from a private corpus's schema) publishes that corpus's type names, property vocabulary, and lifecycle words, and therefore requires its own dedicated privacy pass — renaming or pruning personal vocabulary — before it lands.
- This ADR is itself written under rule 6: it describes the transformation without reproducing anything the transformation removed.

## Amendments

The Decision above stands as written; this later record changes part of it, and the original text is left intact so the change is legible.

- **2026-07-31 — the `dense` fixture is derived numerically, so the anticipated vocabulary pass does not apply to it.** The fourth Consequence above says a derived fixture contract publishes the source corpus's type names, property vocabulary, and lifecycle words, and therefore requires a dedicated privacy pass renaming or pruning personal vocabulary before it lands. [The M2 fixture and privacy record](2026-07-31-m2-fixtures-and-the-privacy-gate.md) takes a different route: only the corpus's *shape* crosses the boundary, as counts and axis facts, and every name in the public fixture is authored fiction. No vocabulary is published, so there is nothing to rename or prune. The gate is not removed but replaced, by a check that the artifact crossing the boundary contains no strings at all — and it runs before the commit rather than before the push, because a public history is not rewritten. The six transformation rules and the standalone-readability test are unchanged and continue to govern every document here.
