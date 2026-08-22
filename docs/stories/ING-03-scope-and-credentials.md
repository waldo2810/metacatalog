# ING-03 — Scope ingestion by database/schema; credentials from env vars only

**Epic:** Ingestion · **Priority:** P1 · **Depends on:** ING-01, INFRA-01

## Story

**As a** data engineer
**I want** to declare which databases and schemas to ingest, with credentials supplied by the environment
**So that** ingestion stays fast and relevant, and the catalog repo is safe to commit and share.

## Acceptance criteria

**AC1 — source declaration**
Given `catalog/sources.yml` declaring a source with `databases`, `include_schemas` and `exclude_schemas`
When I run `mc ingest`
Then only matching objects are ingested.

**AC2 — no credentials in YAML**
Given any file under `catalog/`
When it is loaded
Then a `password`, `pwd` or `connection_string` key is a validation **error** naming the file and line, not a warning.

**AC3 — env var indirection**
Given a source with `connection_env: MC_VMPROD01_DSN`
When I run `mc ingest` with that variable set
Then the connection uses it
And when it is unset, the command fails with a message naming the missing variable and the source it belongs to.

**AC4 — selective ingest**
Given several declared sources
When I run `mc ingest --source vmprod01`
Then only that source is contacted, and other sources' rows are untouched — in particular, not soft-deleted.

**AC5 — object name filters**
Given `exclude_tables: ["tmp_*", "_bak_*"]`
When I ingest
Then matching objects are skipped, using glob semantics.

**AC6 — connection check**
Given a configured source
When I run `mc ingest --check`
Then the tool connects, reports server version and reachable databases, and writes nothing.

## Implementation notes

```yaml
# catalog/sources.yml
sources:
  - name: vmprod01
    kind: sqlserver
    host: vmprod01.internal
    driver: tiberius           # or another driver crate — allowed under the database-driver exception
    connection_env: MC_VMPROD01_DSN
    databases: [SalesDB, OpsDB]
    include_schemas: [dbo, sales]
    exclude_tables: ["tmp_*", "_bak_*"]
```

AC2 is the one to get right early — it is far cheaper to make the shape impossible than to scrub a credential out of git history later. Enforce it in the hand-rolled YAML validator by rejecting unknown keys outright, plus an explicit check for the forbidden key names, so a typo'd key is caught by the same mechanism.

AC4 matters more than it looks: soft-delete detection (ING-02) filters by `last_seen < run_ts`. Scoping that query by `source_id` is what stops a single-source ingest from marking every other source's columns as deleted.

`--check` shares the driver-connect path with `ingest` so a green check genuinely means ingest can connect.

## Verification

Ingest with `include_schemas: [dbo]` → nothing from other schemas. Unset the env var → clear failure naming the variable. Ingest source A only → source B's `last_seen` and `deleted_at` unchanged. Add `password:` to a YAML file → validation error with file and line.
