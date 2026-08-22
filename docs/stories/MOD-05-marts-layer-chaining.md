# MOD-05 — Declare marts that map to DWH columns

**Epic:** Modeling · **Priority:** P1 · **Depends on:** MOD-02

## Story

**As a** data engineer
**I want** to declare data marts whose fields map to warehouse fields
**So that** lineage runs end to end from a mart field back to the originating source columns.

## Acceptance criteria

**AC1 — mart declaration**
Given a file with `layer: mart`, `namespace: finance`, `table: FactRevenue`
When I run `mc load`
Then a declared asset `mart://finance/FactRevenue` exists with column URNs `mart://finance/FactRevenue#Amount`.

**AC2 — mapping to DWH**
Given a mart column whose sources are `dwh://` URNs
When I run `mc load`
Then edges are created from those DWH columns to the mart column, using the same rule machinery as MOD-02/03.

**AC3 — one loader**
Given mart and DWH files
When they are parsed
Then the same Pydantic model, resolver and rule handling process both — layer is data, not a code path.

**AC4 — chained trace**
Given source → DWH → mart mappings
When I trace backward from the mart column
Then the path reaches the original source columns through the DWH (see LIN-03).

**AC5 — layer ordering**
Given a DWH column whose sources are `mart://` URNs
When I run `mc validate`
Then it is an error: allowed directions are source→dwh, source→mart, dwh→mart, and same-asset derivation (MOD-04).

**AC6 — mart straight from source**
Given a mart column mapping directly to an `mssql://` URN, bypassing the DWH
When I run `mc load`
Then it is allowed, since the client has marts fed straight from source.

**AC7 — load order independent**
Given a mart file references a DWH column declared in a file parsed later
When I run `mc load`
Then it resolves — all declared columns are registered before any refs are resolved.

## Implementation notes

```yaml
# catalog/marts/finance_revenue.yml
layer: mart
namespace: finance
table: FactRevenue
columns:
  - name: Amount
    type: decimal(18,2)
    sources: [dwh://core/FactSales#NetAmount]
    transform: as-is
  - name: SourceRef
    type: nvarchar(50)
    sources: [mssql://vm01/SalesDB/dbo/Order#OrderNo]
```

AC7 forces a two-pass load: pass one registers every declared asset and column from every file; pass two resolves refs and builds edges. A single-pass loader appears to work until someone renames a file and the ordering changes — a bug that reads as random.

AC5 is cheap to implement (compare URN schemes against an allowed-pairs set) and prevents the graph acquiring cycles through layer inversion, which would otherwise surface far away as an infinite traversal.

Marts and DWH tables differ only in URN scheme and allowed upstream layers. Resist adding a `MartSpec` class — one `TableSpec` with a `layer` field is the whole difference.

## Verification

Sample with one mart over one DWH table plus one direct-from-source column. Backward trace from the mart column reaches source columns. A `dwh://` column sourcing from `mart://` → validation error. Rename files to change parse order → results identical.
