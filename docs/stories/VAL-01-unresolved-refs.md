# VAL-01 — Fail on unresolved column refs, with file, line and suggestions

**Epic:** Validation · **Priority:** P0 · **Depends on:** MOD-02, INFRA-01

## Story

**As a** data engineer
**I want** a mistyped or non-existent column reference to fail loudly, pointing at the exact line and suggesting the right name
**So that** a broken mapping is caught at review time instead of surfacing as a silent `#N/A` months later.

## Acceptance criteria

**AC1 — unresolved ref is an error**
Given a source ref matching no ingested or declared column
When I run `mc validate`
Then it reports an error and exits non-zero.

**AC2 — file and line**
Given the failing ref
When the error prints
Then it names the YAML file and the 1-based line of that specific ref, not of the document or the column.

**AC3 — suggestions**
Given a ref to `Customer#CustmerId` where `Customer#CustomerId` exists
When validation fails
Then up to three nearest candidates are printed, ranked by similarity.

**AC4 — narrowed suggestions**
Given the table part of the URN resolves but the column does not
When suggestions are computed
Then they are drawn from that table's columns only.

**AC5 — unknown table**
Given the table itself does not exist
When validation fails
Then the message says so explicitly and suggests table names, so a wrong-schema typo is not mistaken for a wrong-column one.

**AC6 — malformed URN**
Given a ref that is not a valid URN at all
When validation runs
Then the error explains the expected shape with an example of the right scheme.

**AC7 — report everything**
Given twelve bad refs across four files
When I run `mc validate`
Then all twelve are reported in one run, grouped by file and ordered by line — validation never stops at the first error.

## Implementation notes

This story is the direct replacement for the workbook's failed `XLOOKUP`, and it is the single most valuable thing the tool does. Message quality is the feature, not a nicety.

Target output:

```
catalog/warehouse/dim_customer.yml:17
  error: unresolved source ref
    mssql://vmprod01/SalesDB/dbo/Customer#CustmerId
  table mssql://vmprod01/SalesDB/dbo/Customer exists; column does not.
  did you mean:
    CustomerId      (ingested, run 42)
    CustomerNo
    CustomerTypeId
```

**Line numbers.** The hand-rolled YAML parser must attach a line number to every node it produces, since there is no `ruamel`-equivalent crate to lean on. Validation walks the parsed tree directly and reads the line off the node it is checking — keep that tree (not just the validated struct) alive for the whole load; discarding it after parsing is what makes line reporting impossible to retrofit.

Suggestions use a hand-rolled nearest-match ranking (e.g. Levenshtein distance, top 3, some similarity cutoff) against the candidate set from AC4/AC5 — no fuzzy-match crate; this is small enough to write directly against the zero-dependency policy.

Resolve against a URN → id dict built once per load, and keep a parallel index keyed by table URN so AC4's narrowing costs nothing extra.

Refs to soft-deleted columns are *not* this story — they resolve, and VAL-02 handles them with a much more specific message.

## Verification

Typo a column → error names file, line and the right column. Typo a schema → table-level message. Break twelve refs across four files → all twelve reported, grouped and ordered.
