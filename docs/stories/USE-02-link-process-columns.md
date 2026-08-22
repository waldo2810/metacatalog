# USE-02 — Link a process to the columns it consumes

**Epic:** Process usage · **Priority:** P1 · **Depends on:** USE-01, ING-01

## Story

**As a** data engineer
**I want** to record which columns each process reads
**So that** I can see demand for a field independently of whether the warehouse serves it yet.

## Acceptance criteria

**AC1 — column usage**
Given a process declaring `uses` with column URNs
When I run `mc load`
Then a `process_usage` row exists per column.

**AC2 — any layer**
Given `uses` entries pointing at source, DWH and mart columns
When they are loaded
Then all are accepted — a process may read straight from source today and from the mart tomorrow.

**AC3 — table-level usage**
Given a `uses` entry naming a table rather than a column
When it is loaded
Then it is recorded as usage of that table, for the common case where the consuming detail is unknown.

**AC4 — refs must resolve**
Given a `uses` entry matching nothing
When I run `mc validate`
Then VAL-01 reports it with file, line and suggestions, exactly as for mapping refs.

**AC5 — rebuild on load**
Given a `uses` entry is removed from YAML
When I reload
Then the usage row is gone — usage mirrors the files.

**AC6 — deduplicated**
Given the same column listed twice for one process
When it is loaded
Then one usage row exists, and a warning notes the duplicate.

**AC7 — dropped column**
Given a process uses a column that was soft-deleted upstream
When I run `mc validate`
Then it is reported — a consumer pointing at a vanished field is exactly the breakage worth knowing about.

## Implementation notes

```yaml
processes:
  - slug: monthly-billing
    name: Monthly billing run
    kind: business
    uses:
      - mssql://vmprod01/SalesDB/dbo/Customer#CustomerId
      - mssql://vmprod01/SalesDB/dbo/Invoice#Amount
      - dwh://core/DimCustomer#CustomerKey
      - mssql://vmprod01/OpsDB/dbo/Ticket        # whole table (AC3)
```

Usage links are deliberately plain — no read/write flag, no frequency, no criticality on the link. Criticality lives on the process (USE-01). The question being answered is "who wants this field", and a plain link answers it. Attributes can be added later without migrating existing data; removing them cannot.

AC3 needs `process_usage` to reference either a column or an asset. Use two nullable FK columns with a check constraint that exactly one is set, rather than a polymorphic id — the queries stay readable and the database keeps the invariant.

Reuse the resolver and the URN→id index from MOD-02 verbatim. If resolution logic gets duplicated here, the two copies will drift and error messages will stop matching.

AC7 shares VAL-02's dropped-column query with a different join; keep one function taking the edge source, so both mappings and usages report drift identically.

## Verification

Declare a process using four columns across layers → four rows. Remove one, reload → three. List one twice → one row plus warning. Table-level usage → asset-linked row. Drop a used column upstream, re-ingest → validate reports it.
