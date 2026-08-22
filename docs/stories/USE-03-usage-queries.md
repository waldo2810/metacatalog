# USE-03 — Answer "who uses this field" and "what does this process need"

**Epic:** Process usage · **Priority:** P1 · **Depends on:** USE-02

## Story

**As a** data engineer planning the warehouse
**I want** to ask which processes consume a field, and which fields a process needs
**So that** warehouse scope is driven by actual demand rather than by guesswork.

## Acceptance criteria

**AC1 — consumers of a column**
Given `mc uses mssql://vmprod01/SalesDB/dbo/Customer#CustomerId`
When it runs
Then every process consuming it is listed with kind, owner and criticality.

**AC2 — indirect consumers**
Given a process using a DWH column that is fed by the queried source column
When `mc uses --transitive` runs
Then that process is listed too, marked as indirect with the path length.

**AC3 — table rollup**
Given a table URN
When `mc uses` runs
Then processes are grouped by the column they consume, plus a section for table-level usage.

**AC4 — process requirements**
Given `mc process process://business/monthly-billing`
When it runs
Then every column that process uses is listed, grouped by layer and by source system.

**AC5 — unserved fields flagged**
Given a process using source columns not yet mapped into any DWH or mart column
When `mc process` runs
Then those are flagged as not yet served by the warehouse — the direct input to the build backlog.

**AC6 — empty results**
Given a column no process uses
When `mc uses` runs
Then it says so plainly and exits 0.

**AC7 — machine-readable**
Given `--format csv`
When either command runs
Then output is CSV.

## Implementation notes

Target output:

```
$ mc process process://business/monthly-billing
Monthly billing run  (business, owner finance-ops, criticality high)

source — vmprod01/SalesDB
  dbo.Customer#CustomerId          served by dwh://core/DimCustomer#CustomerKey
  dbo.Invoice#Amount               NOT SERVED
dwh
  core.DimCustomer#CustomerKey

1 of 2 source columns not yet served by the warehouse
```

AC5 is the point of the whole epic. "This high-criticality process reads two source fields the warehouse does not carry" is a prioritised piece of work; a usage list on its own is trivia.

"Served" means: the source column has at least one outgoing edge to a `dwh://` or `mart://` column that is not soft-deleted. One query, reused by COV-01 — factor it into `graph/coverage.rs` from the start rather than writing it twice.

AC2's transitive mode reuses LIN-02's downstream CTE, then joins `process_usage` on the reachable set. Default to direct-only: the transitive answer is broader and slower, and the direct answer is what people usually mean.

## Verification

Two processes on one column → both listed with metadata. Process using an unmapped source column → NOT SERVED flag and the summary count. `--transitive` picks up the process attached to the downstream DWH column. Unused column → explicit empty result.
