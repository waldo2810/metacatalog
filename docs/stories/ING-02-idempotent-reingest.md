# ING-02 — Re-ingest idempotently with run history and soft deletes

**Epic:** Ingestion · **Priority:** P0 · **Depends on:** ING-01

## Story

**As a** data engineer
**I want** repeated ingestion to converge rather than duplicate, and to record what changed
**So that** I can re-run it freely and later prove *when* a source column disappeared.

## Acceptance criteria

**AC1 — idempotent**
Given a source already ingested
When I run `mc ingest` again with nothing changed upstream
Then asset and column row counts are unchanged, no duplicate URNs exist, and `last_seen` advances to the new run.

**AC2 — additions**
Given a new column appears upstream
When I re-ingest
Then it is inserted, and the run summary reports it under added columns.

**AC3 — soft delete, never hard delete**
Given a previously ingested column no longer exists upstream
When I re-ingest
Then its row is retained with `deleted_at` set to the run timestamp
And it is excluded from lineage queries and exports by default
And the run summary reports it under removed columns.

**AC4 — resurrection**
Given a soft-deleted column reappears upstream
When I re-ingest
Then `deleted_at` is cleared and the same row (same id, same URN) is reused, so existing mappings to it keep working.

**AC5 — origin is respected**
Given declared DWH rows exist in the store
When ingestion runs
Then no row with `origin = 'declared'` is inserted, updated or soft-deleted.

**AC6 — run history**
Given several ingests have run
When I query the `run` table
Then each has `started_at`, `finished_at`, `status`, and counts for assets seen, columns seen, added and removed.

## Implementation notes

Upsert on the URN unique index. SQLAlchemy Core with the SQLite dialect's `on_conflict_do_update`; the Postgres dialect offers the same call, which is the portability the Core choice buys.

Soft-delete pass, scoped hard to this run's source **and** to ingested rows:

```sql
UPDATE "column" SET deleted_at = :run_ts
WHERE origin = 'ingested'
  AND deleted_at IS NULL
  AND last_seen < :run_ts
  AND asset_id IN (SELECT id FROM asset WHERE source_id = :source_id);
```

`last_seen < :run_ts` is what makes "not seen this run" mean deleted. Every touched row must have `last_seen` written, including unchanged ones — an upsert that skips no-op rows breaks the delete detection. This is the easiest bug to introduce in the whole project.

Resurrection falls out of upserting on URN and clearing `deleted_at` in the update branch. Do not delete-and-reinsert: it changes the id and orphans mappings.

Asset-level soft delete follows the same pattern; soft-deleting an asset soft-deletes its columns.

## Verification

Ingest twice against an unchanged fixture → identical counts. Drop a column, re-ingest → `deleted_at` set, summary reports it. Re-add the column, re-ingest → same row id, `deleted_at` null.
