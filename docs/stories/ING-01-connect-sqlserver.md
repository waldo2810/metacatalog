# ING-01 — Connect to a SQL Server instance and extract schemas, tables, columns

**Epic:** Ingestion · **Priority:** P0 · **Depends on:** OPS-01

## Story

**As a** data engineer
**I want** the tool to connect to an Azure VM SQL Server and pull its databases, schemas, tables, views and columns
**So that** the source side of the lineage graph is ground truth rather than something typed by hand into a spreadsheet.

## Acceptance criteria

**AC1 — extraction**
Given a reachable SQL Server with a `SalesDB` database
When I run `mc ingest --source vmprod01`
Then every table, view and column in the configured scope is stored, each with a URN of the form `mssql://vmprod01/SalesDB/dbo/Customer#CustomerId`
And each column records `name`, `ordinal`, `data_type` (including length/precision) and `nullable`.

**AC2 — assets carry kind**
Given the database contains both tables and views
When ingestion completes
Then each asset records `kind` as `table` or `view`.

**AC3 — driver independence**
Given the connector module
When it is read
Then it depends on no driver type directly; it obtains a connection from `ingest/drivers.rs`.

**AC4 — driver selection**
Given a source configured with `driver: tiberius` (or whichever driver names are eventually offered)
When I run `mc ingest`
Then that driver is used, and a driver that cannot be reached produces a clear error rather than a raw panic.

Database driver/client crates are allowed under the project's dependency policy (zero-dependency otherwise), so a SQL Server driver crate — e.g. `tiberius` — can be added to `Cargo.toml` without a separate exception request. Specific crate choice is still open.

**AC5 — progress and summary**
Given ingestion of several databases
When it runs
Then per-database progress is printed and a final summary reports assets and columns seen.

**AC6 — failure is not partial**
Given the connection drops mid-ingest
When the command fails
Then the run row is marked `failed` with the error, and no half-written state is committed for the failed database.

## Implementation notes

Extraction queries (`INFORMATION_SCHEMA` is portable; `sys.*` gives more):

```sql
-- tables and views
SELECT s.name AS schema_name, o.name AS object_name, o.type_desc
FROM sys.objects o
JOIN sys.schemas s ON s.schema_id = o.schema_id
WHERE o.type IN ('U', 'V');

-- columns with resolved types
SELECT s.name, o.name, c.name, c.column_id, t.name AS type_name,
       c.max_length, c.precision, c.scale, c.is_nullable
FROM sys.columns c
JOIN sys.objects o ON o.object_id = c.object_id
JOIN sys.schemas s ON s.schema_id = o.schema_id
JOIN sys.types  t ON t.user_type_id = c.user_type_id
WHERE o.type IN ('U', 'V');
```

Connector shape — keep discovery pure so it can be tested against fixtures with no database:

```rust
trait Connector {
    fn discover(&self) -> impl Iterator<Item = AssetRecord>;
}
```

`AssetRecord` is a plain struct of URN + metadata + columns. Persistence lives in `store/repo.rs`, never in the connector.

Normalize `data_type` to a display form at ingest time (`nvarchar(200)`, `decimal(18,2)`) so exports and type-mismatch warnings compare strings, not tuples.

One connection per database; wrap each database's writes in a single transaction.

## Out of scope

Foreign keys, indexes, stored procedures, view SQL bodies. Column lineage parsed from view definitions is Phase 2.

## Verification

Against `docker run mcr.microsoft.com/mssql/server:2022-latest` seeded with a ~20-table fixture: assert asset and column counts, spot-check a URN and a `decimal(18,2)` type round-trip.
