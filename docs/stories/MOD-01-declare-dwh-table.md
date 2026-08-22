# MOD-01 — Declare a DWH table and its columns in YAML

**Epic:** Modeling · **Priority:** P0 · **Depends on:** OPS-01, INFRA-01

## Story

**As a** data engineer
**I want** to define a warehouse table and its fields in a YAML file
**So that** the warehouse can be designed and reviewed before it exists, with the design under version control.

## Acceptance criteria

**AC1 — declaration**
Given `catalog/warehouse/dim_customer.yml` declaring `layer: dwh`, `namespace: core`, `table: DimCustomer` and a list of columns
When I run `mc load`
Then a declared asset `dwh://core/DimCustomer` exists with one declared column per entry
And each column URN is `dwh://core/DimCustomer#<ColumnName>`.

**AC2 — origin isolation**
Given the load completes
When rows are inspected
Then every row it wrote has `origin = 'declared'` and a `spec_file` pointing at the source YAML
And no row with `origin = 'ingested'` was modified.

**AC3 — file is source of truth**
Given a column is deleted from the YAML
When I re-run `mc load`
Then that declared column and its edges are removed from the store — declared state mirrors the files exactly.

**AC4 — duplicate detection**
Given two files declare the same table, or one file declares the same column name twice
When I run `mc load`
Then it fails naming both files, or the file and both line numbers.

**AC5 — required fields**
Given a file missing `layer`, `namespace`, `table` or `columns`
When I run `mc load`
Then it fails with the file, the line and the missing field named.

**AC6 — descriptions**
Given a table or column carries `description`
When it is loaded
Then the text is stored and appears in `show` output and CSV exports.

## Implementation notes

```yaml
# catalog/warehouse/dim_customer.yml
layer: dwh
namespace: core
table: DimCustomer
description: Conformed customer dimension
owner: data-platform
columns:
  - name: CustomerKey
    type: int
    description: Surrogate key
  - name: FullName
    type: nvarchar(200)
```

Mappings arrive in MOD-02; this story is the container.

AC3 argues for delete-then-insert of all declared rows within one transaction — simple and correct, and at 10k columns fast enough. But declared column **ids** must survive, since process usage links (USE-02) point at them; so upsert declared columns on URN and delete only those absent from the files.

Layer, namespace and table together produce the URN. Validate that namespace and table are safe identifiers (`[A-Za-z_][A-Za-z0-9_]*`) so URNs never need escaping — a rule that is nearly free now and expensive to retrofit.

File location does not affect identity: `layer` and `namespace` in the document are what count. Directory layout stays a human convenience.

## Verification

Load a two-table sample → assets and columns present with correct URNs. Delete a column, reload → gone. Declare a duplicate → error names both locations.
