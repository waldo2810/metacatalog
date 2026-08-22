# LIN-03 — Traverse source → DWH → mart in one query

**Epic:** Lineage queries · **Priority:** P1 · **Depends on:** LIN-01, LIN-02, MOD-05

## Story

**As an** analyst
**I want** a mart field traced all the way back to its originating source columns in one step
**So that** I never have to manually chain "mart comes from DWH" and "DWH comes from source" — the exact chore the spreadsheet's cross-tab XMATCHes exist to do.

## Acceptance criteria

**AC1 — full chain**
Given source → DWH → mart mappings
When I run `mc show mart://finance/FactRevenue#Amount`
Then the trace reaches the originating `mssql://` columns through the intervening DWH columns.

**AC2 — intermediates visible**
Given a multi-hop trace
When it prints
Then each intermediate column is shown with its layer and its transform, so the chain of transformations reads top to bottom.

**AC3 — ultimate sources only**
Given `--roots-only`
When it runs
Then only the terminal upstream columns are listed, flat, with no intermediates — the "just tell me where it really comes from" view.

**AC4 — mixed depth**
Given a mart column drawing from both a DWH column and a source column directly (MOD-05 AC6)
When it is traced
Then both paths appear with their differing depths.

**AC5 — diamonds**
Given two DWH columns fed by one source column, both feeding one mart column
When it is traced
Then the shared source appears once per distinct path, with the paths distinguishable, and traversal does not blow up combinatorially.

**AC6 — depth guard**
Given `--up` unset
When it runs
Then a default cap (20) applies and a notice prints if it was hit — protection against pathological data.

**AC7 — generated terminals**
Given a path ending in a MOD-04 generated column
When it prints
Then it terminates cleanly labelled with its generation kind.

## Implementation notes

The CTEs from LIN-01/02 already recurse, so this story is mostly about **path** handling rather than reachability. To distinguish paths (AC5) carry the path in the CTE:

```sql
WITH RECURSIVE up(column_id, depth, path) AS (
  SELECT :start_id, 0, CAST(:start_id AS TEXT)
  UNION ALL
  SELECT e.source_column_id, up.depth + 1, up.path || '>' || e.source_column_id
  FROM lineage_edge e
  JOIN up ON e.target_column_id = up.column_id
  WHERE up.depth < :max_depth
    AND instr(up.path, CAST(e.source_column_id AS TEXT)) = 0
)
```

Note this variant uses `UNION ALL` with an explicit `instr` path check for cycle safety, because path enumeration needs duplicate rows that `UNION` would collapse. The string-path trick is portable between SQLite and Postgres; `path` is the cycle guard, so it is not optional here.

`--roots-only` is `WHERE column_id NOT IN (SELECT target_column_id FROM lineage_edge)` applied to the reachable set — cheap, and likely the view analysts use most.

Watch AC5's cost: path enumeration is exponential in the worst case. The depth cap plus a result-row cap (say 10k, with a warning when hit) keeps a pathological graph from hanging the CLI. At the stated scale this will not trigger, but the guard costs three lines.

## Verification

Three-layer sample: trace from mart reaches source through DWH with intermediates and transforms. `--roots-only` lists only source columns. Build a diamond → paths distinguishable, no duplicate explosion. Force depth 20 → notice printed.
