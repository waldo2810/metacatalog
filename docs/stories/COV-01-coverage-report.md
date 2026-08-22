# COV-01 — Per source column: mapped yes/no and process count

**Epic:** Coverage · **Priority:** P1 · **Depends on:** MOD-02, USE-02

## Story

**As a** data engineer planning the warehouse
**I want** one report showing, for every source column, whether it is mapped and how many processes want it
**So that** "wanted but not built" sorts to the top and becomes the build backlog.

## Acceptance criteria

**AC1 — one row per source column**
Given `mc coverage`
When it runs
Then every non-deleted ingested column appears once with: URN, data type, mapped yes/no, downstream target count, process count.

**AC2 — the two signals stay separate**
Given the report
When it prints
Then mapped-status and process-count are distinct columns and are never combined into one "used" flag — a field wanted by three processes and mapped nowhere is the most interesting row in the file, and merging the signals hides it.

**AC3 — backlog filter**
Given `mc coverage --unmapped --min-processes 1`
When it runs
Then only unmapped columns with at least one consuming process are listed, sorted by process count descending.

**AC4 — criticality weighting**
Given processes carry `criticality` (USE-01)
When the report is produced
Then it includes the highest criticality among consuming processes, and `--sort criticality` orders by it.

**AC5 — scope filters**
Given `--source`, `--database` or `--schema`
When supplied
Then the report is scoped accordingly.

**AC6 — over-mapped detection**
Given `--mapped-multiple`
When it runs
Then columns feeding more than N targets are listed — candidates for a conformed dimension rather than repeated point mappings.

**AC7 — summary line**
Given any run
When it finishes
Then it prints totals: columns, mapped, unmapped, unmapped-but-wanted.

**AC8 — CSV**
Given `--format csv`
When it runs
Then the report is CSV with the same columns, for handing to analysts.

## Implementation notes

Target output:

```
$ mc coverage --unmapped --min-processes 1 --sort criticality
column                                             type            procs  crit
mssql://vmprod01/SalesDB/dbo/Invoice#Amount        decimal(18,2)   3      high
mssql://vmprod01/SalesDB/dbo/Invoice#DueDate       date            2      high
mssql://vmprod01/OpsDB/dbo/Ticket#Priority         int             1      medium

412 columns · 180 mapped · 232 unmapped · 27 unmapped but wanted
```

That last number is the one the client will act on.

Left join ingested columns against an aggregate of outgoing edges and an aggregate of `process_usage`. Aggregate in subqueries before joining — joining both directly to columns multiplies rows and inflates both counts, a classic fan-out bug that produces plausible-looking wrong numbers.

Table-level process usage (USE-02 AC3) must count toward every column of that table, or fields in a table someone flagged wholesale will look unwanted. Do this as a UNION in the usage aggregate.

Reuse the "served" query from USE-03 (`graph/coverage.rs`) so `mc process` and `mc coverage` can never disagree about whether a column is mapped.

Exclude soft-deleted columns by default; `--include-deleted` shows them, which doubles as a cleanup list for mappings pointing at dead fields.

## Verification

Fixture with a column used by 3 processes and mapped nowhere → tops `--unmapped --min-processes 1`. Verify counts against hand-computed values on a small fixture — particularly with a column feeding two targets and used by two processes, to catch fan-out. Table-level usage → all its columns show a process count. CSV opens in Excel.
