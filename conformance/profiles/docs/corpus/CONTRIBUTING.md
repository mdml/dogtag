# Contributing

Two rules cover most of it: put the page where a reader would look for it, and link it from the
nearest index.

## Where a page goes

The folder is the classification. A page under `guides/` is a guide because of where it sits,
not because it says so at the top; the same is true of a reference page, a runbook and a
decision record. If you cannot decide which folder a page belongs in, it is usually two pages.

- `guides/` — followed start to finish.
- `reference/` — looked things up in.
- `decisions/` — one decision each, numbered, never edited after they are accepted.
- `internals/` — how it works, for people changing it.
- `releases/` — what shipped.

## Before you open a change

- Run the docs build. It fails on a link that names no page.
- Read [contributing/review-checklist.md](contributing/review-checklist.md) and answer it
  honestly; a reviewer will ask the same questions more slowly.
- If you added a term, add it to the [glossary](glossary.md) in the same change.

## Style

The house style is in [contributing/docs-style.md](contributing/docs-style.md). It is short and
it is enforced in review rather than by a tool.
