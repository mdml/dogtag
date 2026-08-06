# Configuration

Every key the gateway reads. A key not listed here is not read; an unknown key in the file is a
refusal to start rather than a warning, which is what makes a downgrade after
[upgrading](guides/upgrading.md) require the old file.

## `[ledger]`

| Key | Default | Reload | Notes |
| --- | --- | --- | --- |
| `path` | none | no | Required. A restart is needed to change it. |
| `segment_bytes` | 134217728 | no | Sealed at this size or `segment_age`, whichever first. |
| `segment_age` | `1h` | no | See [compaction](internals/ledger/compaction.md). |
| `retention` | `30d` | yes | A duration, never a size. |

## `[listen]`

| Key | Default | Reload | Notes |
| --- | --- | --- | --- |
| `ingest` | none | no | Required. |
| `admin` | `127.0.0.1:9701` | no | The CLI talks to this. |
| `tls_cert` | none | yes | Reread by `fleet reload`. |
| `tls_key` | none | yes | Reread by `fleet reload`. |

## `[accept]`

| Key | Default | Reload | Notes |
| --- | --- | --- | --- |
| `queue_depth` | 4096 | yes | Not durable. Raising it hides backpressure rather than fixing it. |
| `clock_skew` | `5m` | yes | Envelopes outside this are refused. |

## `[region]`

| Key | Default | Reload | Notes |
| --- | --- | --- | --- |
| `name` | none | no | Required. Written into the ledger generation and never changed afterwards. |

## Environment

Every key above has an environment spelling: uppercase, table and key joined by an underscore,
prefixed `QUILLON_`. `QUILLON_LEDGER_PATH` is `[ledger] path`. Environment wins over the file,
which is what makes the container image in [installation](guides/installation.md) work with an
empty file.

## Vocabulary

Segment, retention and region are used here exactly as [the glossary](glossary.md) defines
them. The reloadable keys are the ones marked so [above](#listen).
