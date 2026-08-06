# The API

One version is current: v2. There is no v1 in any supported release, and there will not be a v3
inside 3.x — the envelope is frozen for the major version, which
[decisions/0003-freeze-the-envelope-schema.md](decisions/0003-freeze-the-envelope-schema.md)
argues for at length.

## Resources

- [collectors](reference/api/v2/collectors.md) — register, retire, list.
- [streams](reference/api/v2/streams.md) — open a stream, resume one, read the opening frame.
- [reference/api/v2/limits.md](reference/api/v2/limits.md) — what a caller is allowed.

## Schemas

- [envelope](reference/api/v2/schemas/envelope.md) — the outer frame.
- [reading](reference/api/v2/schemas/reading.md) — one measurement.

## Machine-readable

The same surface is published as an OpenAPI document at
[the API description](https://api.quillon.example/v2/openapi.json). Where it and these pages
disagree, the document is generated from the gateway and these pages are not, so believe the
document and open an issue.

## Errors

Every refusal carries a stable identifier, and the identifiers are listed once in
[errors](reference/api/errors.md). Branch on the identifier; the prose beside it is for people
and changes without notice.
