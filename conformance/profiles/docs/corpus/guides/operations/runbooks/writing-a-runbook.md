# Writing a runbook

A runbook is followed by someone who did not write it, under time pressure, on a page they have
not read before. Everything below follows from that.

## Shape

- A severity in the first two lines: routine, urgent, or emergency.
- What the reader is about to do, before the first command.
- Numbered steps with the command inline, not in an appendix.
- An "afterwards" section, because half of all incidents are made worse by the cleanup.

## Rules

**No branching in the steps.** If the procedure genuinely forks, that is two runbooks and a
sentence at the top pointing at both.

**Say what is safe.** [queue-drain.md](guides/operations/runbooks/queue-drain.md) says outright
that quiescing loses nothing, because the reader's actual question at that moment is whether
they are about to make it worse.

**Name the destructive step.** If a step cannot be undone, it says so in the step, not in a
preamble the reader skipped.

## Review

Runbooks go stale silently — the command changes and nobody reads the page until the next
incident. Every runbook is reread at each minor release; the checklist for that is in
[contributing/review-checklist.md](contributing/review-checklist.md).

The placement rule for a new runbook is the same as for every other page:
[CONTRIBUTING.md](CONTRIBUTING.md).
