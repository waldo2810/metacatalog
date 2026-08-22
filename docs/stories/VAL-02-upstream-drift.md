# VAL-02 — Fail when a mapped source column was dropped or retyped upstream

**Epic:** Validation · **Priority:** P0 · **Depends on:** ING-02, MOD-02

## Story

**As a** data engineer
**I want** validation to fail when a source column I depend on has disappeared or changed type upstream
**So that** schema drift is caught the moment it happens instead of discovered by a broken load or a wrong number in a report.

## Acceptance criteria

**AC1 — dropped column is an error**
Given a mapped source column that was soft-deleted by the latest ingest
When I run `mc validate`
Then it reports an error and exits non-zero.

**AC2 — blame the run**
Given a dropped column
When the error prints
Then it names the run id and timestamp in which the column stopped appearing, and the affected target column
And it names the YAML file and line of the mapping.

**AC3 — retype is reported**
Given a mapped source column whose `data_type` changed between runs
When I run `mc validate`
Then it is reported, at error severity when the change can truncate or lose data (narrowing length, precision, or a type-family change) and at warning severity otherwise.

**AC4 — dropped asset**
Given an entire source table disappeared
When I run `mc validate`
Then it is reported once for the table, listing the affected target columns, rather than once per column.

**AC5 — stale source is called out**
Given a source has not been ingested for more than N days (configurable, default 7)
When I run `mc validate`
Then a warning states the source's last successful run — absence of drift errors must not be mistaken for freshness.

**AC6 — never ingested**
Given a source declared in `sources.yml` that has never been ingested
When mappings reference it
Then the message says the source was never ingested, distinct from the VAL-01 "column not found" case.

**AC7 — resurrection clears it**
Given a dropped column returns and is re-ingested
When I run `mc validate`
Then the error is gone with no edit to the YAML.

## Implementation notes

This is the story that justifies the whole project over a spreadsheet. Excel cannot notice that a column stopped existing; this makes it a failing build.

It rests entirely on invariant #3: columns are soft-deleted, so a mapping's target row still exists to be reported on. With hard deletes, this check degrades to a foreign-key error with no context.

```sql
SELECT c.urn, c.deleted_at, r.id AS run_id, r.finished_at,
       tc.urn AS target_urn, ru.spec_file, ru.line
FROM lineage_edge e
JOIN "column" c  ON c.id = e.source_column_id
JOIN "column" tc ON tc.id = e.target_column_id
JOIN rule ru     ON ru.id = e.rule_id
JOIN asset a     ON a.id = c.asset_id
JOIN run r       ON r.id = a.last_run_id
WHERE c.deleted_at IS NOT NULL;
```

Type-change detection needs the previous type. Rather than a full history table, keep `previous_data_type` and `type_changed_run_id` on the column row, written by ingest when the type differs. That is enough for AC3 and avoids a schema-history feature nobody asked for yet.

Severity for AC3 comes from a small comparison table: same family and widening → warning; narrowing or family change (`nvarchar(200)` → `nvarchar(50)`, `decimal` → `int`, `int` → `nvarchar`) → error. Keep the rules in one function with unit tests; it is the kind of logic that quietly rots.

AC5 exists because a clean validate run is otherwise ambiguous: it could mean "no drift" or "we haven't looked in a month".

## Verification

`ALTER TABLE Customer DROP COLUMN LastName`, re-ingest, validate → non-zero, names `DimCustomer.FullName`, the run, and the YAML line. Widen a column → warning. Narrow it → error. Restore and re-ingest → clean, no YAML edited. Skip ingest for 8 days → staleness warning.
