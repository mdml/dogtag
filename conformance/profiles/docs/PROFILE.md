# Fixture profile: `docs`

**Stands for:** the dev team.
**Corpus:** scheduled, built at M4 (lexical retrieval), where a folder-organized markdown-link corpus stresses retrieval hardest.

## Distinguishing axes

- **Repository documentation shapes.** The corpus looks like a software repository's docs tree, not a curated vault: guides, references, decision records, per-directory indexes.
- **Folder organization.** Meaning lives in the directory structure rather than in per-note metadata; the contract must express that without the core learning any folder's name.
- **Markdown-link dialect.** References are standard markdown links, frequently path-qualified and relative. The dialect axis's other side from `dense`'s wikilinks.
- **Most files carrying no frontmatter.** The majority of notes have a body and nothing else, stressing how typing and required-property validation behave when the discriminator must come from declared defaults rather than explicit frontmatter.
- **Repeated basenames.** `README.md` and `index.md` recur in nearly every directory — repeated names as the *normal* case, not an edge case — stressing bare-name ambiguity reporting and path-qualified resolution.

## What the fixture is

A corpus authored fresh for the fixture, with its shape derived from real repository documentation trees (the product repository's own docs among them). No existing file is copied; the shape is what carries the test value.

## Why it is not built yet

Beyond the committed vault-contract format being an M2 decision, this profile earns its keep at M4: lexical retrieval over a folder-organized, frontmatter-sparse, repeated-basename corpus is the hypothesis it exists to stress. It is specified now so no earlier scenario can quietly assume its axes away.
