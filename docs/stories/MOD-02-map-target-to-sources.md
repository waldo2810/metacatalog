# MOD-02 — Map a target column to one or more ingested source columns

**Epic:** Modeling · **Priority:** P0 · **Depends on:** MOD-01, ING-01

## Story

**As a** data engineer
**I want** to declare which source columns feed each warehouse column, and how
**So that** lineage is captured once, in a reviewable file, instead of in XLOOKUP formulas across 50 tabs.

## Acceptance criteria

**AC1 — single source**
Given a target column with one entry under `sources`
When I run `mc load`
Then one lineage edge exists from that source column to the target column.

**AC2 — multiple sources**
Given `FullName` lists both `Customer#FirstName` and `Customer#LastName`
When I run `mc load`
Then two edges exist, both carrying the same transform text.

**AC3 — cross-source mapping**
Given a target maps to columns from two different SQL Server instances
When I run `mc load`
Then both edges are created — nothing assumes one target draws from a single system.

**AC4 — transform text is preserved**
Given `transform: CONCAT(FirstName, ' ', LastName)`
When lineage is queried or exported
Then that text appears verbatim, unparsed and uninterpreted.

**AC5 — rebuild, not accumulate**
Given a source ref is removed from the YAML
When I re-run `mc load`
Then its edge is gone — declared edges are rebuilt from the files each load.

**AC6 — refs resolve to ingested columns**
Given a source ref
When it is resolved
Then it must match an ingested column URN exactly, including case-insensitive object name comparison consistent with SQL Server collation
And failure is handled by VAL-01.

## Implementation notes

Simple form — a bare `sources` list is sugar for a single default rule (MOD-03):

```yaml
columns:
  - name: FullName
    type: nvarchar(200)
    sources:
      - mssql://vmprod01/SalesDB/dbo/Customer#FirstName
      - mssql://vmprod01/SalesDB/dbo/Customer#LastName
    transform: CONCAT(FirstName, ' ', LastName)
```

Normalize both forms to the same internal structure — a target column owns a list of rules, each owning a list of source refs — in the Pydantic model via a validator. Downstream code then never branches on which form the author used. Getting this normalization in early is what keeps the resolver, exporter and renderer free of special cases.

Every edge is born from a rule, so `lineage_edge.rule_id` is `NOT NULL`. The sugar form creates an implicit rule with `is_default = 1` and `when_expr = NULL`.

Resolution is a URN → `column.id` lookup against a dict built once per load, not a query per ref. At 10k columns that is the difference between a second and a minute.

Case handling: SQL Server object names are usually case-insensitive. Store URNs in the case reported by the server, resolve case-insensitively, and report the canonical spelling in errors.

## Verification

Map a two-source concatenation → two edges, one transform. Remove a ref and reload → one edge. Map across two instances → both resolve.
