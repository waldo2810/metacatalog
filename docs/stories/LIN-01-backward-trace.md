# LIN-01 — Trace backward: where does this target field come from

**Epic:** Lineage queries · **Priority:** P0 · **Depends on:** MOD-02

## Story

**As a** data engineer or analyst
**I want** to ask where a warehouse or mart field comes from and see the full path back to source columns
**So that** the question that currently takes twenty minutes of scrolling across spreadsheet tabs takes one command.

## Acceptance criteria

**AC1 — direct upstream**
Given `mc show dwh://core/DimCustomer#FullName`
When it runs
Then it prints the upstream source columns with their system, table and column.

**AC2 — transforms shown**
Given the mapping carries a transform
When the trace prints
Then the transform text appears alongside the edge.

**AC3 — conditions shown**
Given a column mapped by conditional rules (MOD-03)
When the trace prints
Then each branch is shown with its condition, grouped by rule.

**AC4 — depth control**
Given `--up 1`
When it runs
Then only immediate parents are shown; `--up` defaults to unlimited.

**AC5 — table-level input**
Given a URN naming a table rather than a column
When it runs
Then every column of that table is traced, grouped by column.

**AC6 — soft-deleted upstream marked**
Given an upstream column was soft-deleted
When it appears in the trace
Then it is marked `(dropped, run 42)` rather than silently omitted.

**AC7 — unknown URN**
Given a URN that matches nothing
When it runs
Then the error suggests near matches, reusing VAL-01's suggestion helper.

**AC8 — machine-readable output**
Given `--format json`
When it runs
Then the same trace is emitted as JSON for scripting.

## Implementation notes

Target output:

```
dwh://core/DimCustomer#FullName  nvarchar(200)
└── rule: default — CONCAT(FirstName, ' ', LastName)
    ├── mssql://vmprod01/SalesDB/dbo/Customer#FirstName   nvarchar(100)
    └── mssql://vmprod01/SalesDB/dbo/Customer#LastName    nvarchar(100)  (dropped, run 42)
```

Recursive CTE, walking edges backward:

```sql
WITH RECURSIVE up(column_id, depth) AS (
  SELECT :start_id, 0
  UNION
  SELECT e.source_column_id, up.depth + 1
  FROM lineage_edge e
  JOIN up ON e.target_column_id = up.column_id
  WHERE up.depth < :max_depth
)
SELECT * FROM up;
```

`UNION` rather than `UNION ALL` is deliberate: it terminates on a cycle instead of looping forever. VAL-03 rejects cycles, but a query that hangs when data is bad is a bad query — traversal must be safe independently of validation having been run.

Fetch the ring of ids with the CTE, then fetch node and rule detail in one follow-up query keyed by those ids. Building the tree in application code from two flat result sets is simpler and faster than trying to shape the CTE output directly.

Group by rule when printing (AC3); a flat parent list loses which branch a source belongs to, which is exactly the information a conditional mapping exists to record.

## Verification

Trace a two-source concatenation → both parents, transform shown. Trace a conditional column → branches with conditions. `--up 1` on a mart column → stops at the DWH. Trace a table URN → every column grouped. `--format json` → parses, same content.
