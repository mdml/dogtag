# House style

Short, and enforced in review rather than by a tool.

## Sentences

Say the thing, then say why. A page that opens with three paragraphs of context is a page whose
first sentence has not been written yet.

Present tense for what the software does. Past tense for what was decided — the decision records
under [decisions/README.md](decisions/README.md) are the only pages that talk about the past.

## Links

Link the first mention on a page, not every mention. Link with enough path to be unambiguous:
this tree has five files called `README.md`, four called `overview.md`, and two called
`limits.md`, so a bare name is a coin flip and a path is not.

Use the page's own title as the link text where it reads naturally. Do not write "click here",
and do not write the raw path as the text unless the path is the point — as it is in
[reference/README.md](reference/README.md), where two same-named pages are being distinguished
on purpose.

## Headings

One `#` per page, matching the title. Anything below `###` is a sign the page is two pages.

Where this style says nothing, follow
[the house writing guide](https://handbook.quillon.example/writing) — but this page wins where
the two disagree.

## Code

Indented blocks for commands, with no shell prompt character. A reader copying a line should not
have to delete anything.

## Terms

Use the glossary's word, or add yours to [the glossary](glossary.md) in the same change. The
worst thing in this tree is two words for one concept.

## Before you submit

[contributing/review-checklist.md](contributing/review-checklist.md).

The tree this style describes starts at [the top-level README](README.md).
