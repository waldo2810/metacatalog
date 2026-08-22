# MOD-03 — Express conditional mappings as rule blocks with `when:`

**Epic:** Modeling · **Priority:** P1 · **Depends on:** MOD-02

## Story

**As a** data engineer
**I want** a target column to draw from different sources under different conditions
**So that** real cases like "Amount comes from SalesDB for EU, otherwise from OpsDB" are recorded accurately instead of flattened into a footnote.

## Acceptance criteria

**AC1 — rule blocks**
Given a target column with a `rules` list, each rule having `when`, `sources` and `transform`
When I run `mc load`
Then edges are created for every rule, each edge linked to its own rule row.

**AC2 — condition is preserved per rule**
Given two rules with different `when` values
When lineage is exported
Then each row carries the condition belonging to its rule, not a merged one.

**AC3 — default rule**
Given a rule with `when: default` (or `when` omitted)
When it is loaded
Then it is stored with `is_default = 1` and `when_expr = NULL`.

**AC4 — at most one default**
Given two rules on the same column are marked default
When I run `mc load`
Then it fails, naming the file and the line of the second one.

**AC5 — condition text is opaque**
Given `when: region = 'EU'`
When it is loaded
Then the text is stored verbatim and never parsed or evaluated.

**AC6 — forms are mutually exclusive**
Given a column carries both `sources` and `rules`
When I run `mc load`
Then it fails, telling the author to pick one form.

**AC7 — same source in several rules**
Given the same source column appears in two rules of one target
When I run `mc load`
Then two distinct edges exist, one per rule, and neither is deduplicated away.

## Implementation notes

```yaml
columns:
  - name: Amount
    type: decimal(18,2)
    rules:
      - when: region = 'EU'
        sources: [mssql://vm01/SalesDB/dbo/Order#Amt]
        transform: as-is
      - when: default
        sources: [mssql://vm02/OpsDB/dbo/Txn#Value]
        transform: convert to EUR
```

AC7 is why `lineage_edge` is unique on `(rule_id, source_column_id, target_column_id)` rather than on the column pair. A uniqueness constraint on just the pair would silently collapse a conditional mapping into one edge and lose a branch — a data-loss bug that looks like a dedup optimization.

Store `rule.spec_file` and `rule.line` at load time. Later validation and drift errors then point at the exact rule, not just the file.

Rules are ordered; keep an `ordinal` so exports and `show` list them in authored order, with the default rendered last regardless of where it was written.

Graph traversal ignores conditions entirely — a conditional edge is still an edge. Conditions are documentation carried along for humans, surfacing in `show` and in the CSV.

## Verification

Two-rule column → two edges with distinct rules and conditions. Two defaults → error at the second line. Both `sources` and `rules` → error. Same source in two rules → two edges survive.
