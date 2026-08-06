# Reference

Exhaustive lists. Nothing here teaches anything; if you want to be taught, start at
[guides/README.md](guides/README.md).

## Pages

- [configuration](reference/configuration.md) — every key the gateway reads, its default, and
  whether a reload picks it up.
- [reference/limits.md](reference/limits.md) — the deployment-wide limits, and the sizing
  constants.
- [reference/cli/overview.md](reference/cli/overview.md) — the command-line tools.
- [reference/api/README.md](reference/api/README.md) — the HTTP and streaming API.

## Two pages called `limits.md`

There are two, deliberately. [reference/limits.md](reference/limits.md) is what the deployment
enforces — disk, retention, segment size. [reference/api/v2/limits.md](reference/api/v2/limits.md)
is what a *caller* is allowed, per collector and per connection. They move independently and
merging them has been proposed twice and rejected twice.

Because both exist, always link them with enough path to tell them apart. The same goes for the
several pages named `overview.md` and for every `README.md` in this tree.

## Stability

Reference pages describe the current minor version. Where behaviour changed, the release note
says so and this page does not carry the history; see
[releases/2026-07-quillon-3-1.md](releases/2026-07-quillon-3-1.md).
