# Review checklist

Answer these before asking for review. A reviewer will ask them anyway, more slowly.

## Placement

- Is the page in the folder a reader would look in? The folder is the classification; see
  [CONTRIBUTING.md](CONTRIBUTING.md).
- Is it linked from the nearest index — the `README.md` or `overview.md` beside it?

## Content

- Does the first sentence say what the page is for?
- Does it duplicate a reference page? Guides link to
  [reference/README.md](reference/README.md); they do not restate it.
- If it is a runbook, does it match the shape in
  [writing-a-runbook.md](guides/operations/runbooks/writing-a-runbook.md)?

## Links

- Does every link name a page that exists?
- Is every link to a repeated name path-qualified? Five `README.md`, four `overview.md`, two
  `limits.md` — see the note in [reference/README.md](reference/README.md).

## Terms

- Every term either already in [the glossary](glossary.md) or added by this change.

## Release-time

At each minor release, every runbook is reread and every reference page checked against the
release notes — most recently
[releases/2026-07-quillon-3-1.md](releases/2026-07-quillon-3-1.md). A page that survives two
releases without being reread should be assumed stale.
