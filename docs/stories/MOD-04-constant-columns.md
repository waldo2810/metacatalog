# MOD-04 — Declare constant / system-generated target columns

**Epic:** Modeling · **Priority:** P1 · **Depends on:** MOD-02

## Story

**As a** data engineer
**I want** to declare target columns that legitimately have no upstream source
**So that** surrogate keys, load timestamps and literals are documented as deliberate rather than flagged as unfinished work.

## Acceptance criteria

**AC1 — generated columns**
Given a column with `generated: surrogate-key` (or `load-timestamp`, `constant`, `derived`) and no sources
When I run `mc load`
Then it loads without error and no lineage edge is created.

**AC2 — not an orphan**
Given a generated column
When `mc validate` runs
Then no orphan warning is raised for it (contrast VAL-03, where a plain sourceless column does warn).

**AC3 — constant value**
Given `generated: constant` with `value: "EUR"`
When it is loaded
Then the value is stored and appears in `show` output and CSV exports.

**AC4 — derived from siblings**
Given `generated: derived` with `derived_from: [Amount, TaxRate]` naming columns of the same table
When I run `mc load`
Then intra-table edges are created between those sibling columns and the target
And each sibling name must resolve within the same table, or loading fails naming the unknown one.

**AC5 — mutually exclusive**
Given a column carries both `generated` and `sources`/`rules`
When I run `mc load`
Then it fails, telling the author to pick one.

**AC6 — visible as a leaf**
Given a backward trace reaching a generated column
When it is displayed
Then it is shown as a terminal node labelled with its generation kind, not as a dead end with a missing parent.

## Implementation notes

```yaml
columns:
  - name: CustomerKey
    type: int
    generated: surrogate-key
    description: Identity assigned at load

  - name: LoadedAt
    type: datetime2
    generated: load-timestamp

  - name: CurrencyCode
    type: char(3)
    generated: constant
    value: EUR

  - name: NetAmount
    type: decimal(18,2)
    generated: derived
    derived_from: [Amount, TaxRate]
    transform: Amount * (1 - TaxRate)
```

`generated` is an enum on the column: `surrogate-key | load-timestamp | constant | derived`. Store it on the column row; a free-text kind would defeat the AC2 check.

AC4 introduces the graph's only same-asset edges. They are ordinary `lineage_edge` rows, so traversal needs no special case — but the cycle check (VAL-03) must therefore tolerate self-referential *assets* while still rejecting cycles between *columns*.

This story is small but load-bearing: without it, every warehouse table produces orphan warnings for its keys and audit columns, and the team learns to ignore the warning output. A warning channel nobody reads is worse than no warning channel.

## Verification

Declare one column of each kind → loads clean, `validate` silent. Constant value appears in the CSV. `derived_from` naming an unknown sibling → error. `generated` plus `sources` → error.
