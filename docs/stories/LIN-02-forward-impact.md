# LIN-02 — Trace forward: what breaks if I change this source column

**Epic:** Lineage queries · **Priority:** P0 · **Depends on:** MOD-02

## Story

**As a** data engineer
**I want** a flat list of everything downstream of a source column
**So that** I can assess the blast radius of a change before making it, rather than after someone's report breaks.

## Acceptance criteria

**AC1 — transitive downstream**
Given `mc impact mssql://vmprod01/SalesDB/dbo/Customer#CustomerId`
When it runs
Then every downstream column across all layers is listed, transitively, not just direct children.

**AC2 — flat and sorted**
Given the result
When it prints
Then it is a flat list — not a tree — sorted by layer then URN, so it can be pasted into a change ticket.

**AC3 — hop distance**
Given each affected column
When it prints
Then the number of hops from the origin is shown.

**AC4 — process consumers included**
Given downstream columns consumed by processes (USE-02)
When impact runs
Then affected processes are listed in their own section — the business-facing half of the blast radius.

**AC5 — table-level input**
Given a table URN
When it runs
Then the impact of dropping that whole table is reported, deduplicated across its columns.

**AC6 — count summary**
Given any run
When it finishes
Then it prints totals: affected columns per layer, and affected processes.

**AC7 — empty result is explicit**
Given a column nothing depends on
When it runs
Then it says so plainly, and exits 0 — "nothing depends on this" is a useful answer, not an error.

**AC8 — machine-readable**
Given `--format csv`
When it runs
Then the list is emitted as CSV for pasting into a ticket or sheet.

## Implementation notes

Mirror image of LIN-01's CTE, walking `source_column_id → target_column_id`, same `UNION` cycle safety, same two-query shape.

Target output:

```
impact of mssql://vmprod01/SalesDB/dbo/Customer#CustomerId

hops  layer  column
1     dwh    dwh://core/DimCustomer#CustomerKey
2     mart   mart://finance/FactRevenue#CustomerKey
2     mart   mart://sales/DimCustomerSlim#CustomerKey

processes affected (2)
  report    Monthly billing        via dwh://core/DimCustomer#CustomerKey
  business  Customer onboarding    via mart://finance/FactRevenue#CustomerKey

3 columns (1 dwh, 2 mart) · 2 processes
```

Where a column is reachable by several paths, report the **shortest** hop count and deduplicate — an impact list is about what is affected, not how many ways.

AC4 is what makes this command usable in a change conversation. "Three columns change" is abstract; "monthly billing is affected" is not. It depends on USE-02, so build the column half first and add the process section with that story.

Conditions are ignored: a conditionally-mapped downstream column is still potentially affected. Impact analysis must over-approximate — a false positive costs a glance, a false negative costs an outage.

## Verification

Impact from a source column feeding a DWH column that feeds two marts → three rows with hop counts 1, 2, 2. Add a process on the DWH column → appears in the process section. Impact of an unused column → explicit empty result, exit 0. `--format csv` → parses.
