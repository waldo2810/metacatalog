# Story index

Requirements and design decisions: [../requirements.md](../requirements.md).

One file per story. Each carries a user story, Given/When/Then acceptance criteria, implementation notes, and dependencies.

Priority: **P0** = the tool is not useful without it. **P1** = needed for Phase 1 to be considered done. **P2** = valuable, cut first if time runs out.

## ING — Ingestion

| ID | Story | Priority |
|---|---|---|
| [ING-01](ING-01-connect-sqlserver.md) | Connect to a SQL Server instance and extract schemas, tables, columns | P0 |
| [ING-02](ING-02-idempotent-reingest.md) | Re-ingest idempotently with run history and soft deletes | P0 |
| [ING-03](ING-03-scope-and-credentials.md) | Scope ingestion by database/schema; credentials from env vars only | P1 |

## MOD — Modeling the warehouse

| ID | Story | Priority |
|---|---|---|
| [MOD-01](MOD-01-declare-dwh-table.md) | Declare a DWH table and its columns in YAML | P0 |
| [MOD-02](MOD-02-map-target-to-sources.md) | Map a target column to one or more ingested source columns | P0 |
| [MOD-03](MOD-03-conditional-rules.md) | Express conditional mappings as rule blocks with `when:` | P1 |
| [MOD-04](MOD-04-constant-columns.md) | Declare constant / system-generated target columns | P1 |
| [MOD-05](MOD-05-marts-layer-chaining.md) | Declare marts that map to DWH columns | P1 |

## VAL — Validation

| ID | Story | Priority |
|---|---|---|
| [VAL-01](VAL-01-unresolved-refs.md) | Fail on unresolved column refs, with file, line and suggestions | P0 |
| [VAL-02](VAL-02-upstream-drift.md) | Fail when a mapped source column was dropped or retyped upstream | P0 |
| [VAL-03](VAL-03-warnings-and-exit-codes.md) | Warn on orphans, type mismatches, cycles; severity-split exit codes | P1 |

## LIN — Lineage queries

| ID | Story | Priority |
|---|---|---|
| [LIN-01](LIN-01-backward-trace.md) | Trace backward: where does this target field come from | P0 |
| [LIN-02](LIN-02-forward-impact.md) | Trace forward: what breaks if I change this source column | P0 |
| [LIN-03](LIN-03-multi-hop.md) | Traverse source → DWH → mart in one query | P1 |

## USE — Process usage

| ID | Story | Priority |
|---|---|---|
| [USE-01](USE-01-declare-processes.md) | Declare processes with kind business/report/app | P1 |
| [USE-02](USE-02-link-process-columns.md) | Link a process to the columns it consumes | P1 |
| [USE-03](USE-03-usage-queries.md) | Answer "who uses this field" and "what does this process need" | P1 |

## COV — Coverage / backlog

| ID | Story | Priority |
|---|---|---|
| [COV-01](COV-01-coverage-report.md) | Per source column: mapped yes/no and process count | P1 |

## EXP — Export

| ID | Story | Priority |
|---|---|---|
| [EXP-01](EXP-01-csv-export.md) | CSV export, one row per mapping edge | P0 |
| [EXP-02](EXP-02-mermaid-render.md) | Mermaid render with `--focus` and `--depth` | P2 |

## OPS — Project ops

| ID | Story | Priority |
|---|---|---|
| [OPS-01](OPS-01-init-scaffold.md) | `mc init` scaffolds a catalog repo with worked examples | P1 |
| [OPS-02](OPS-02-ci-gate.md) | `validate` usable as a CI gate | P2 |

## Suggested order

`OPS-01` → `ING-01..03` → `MOD-01..05` → `VAL-01..03` → `LIN-01..03` → `USE-01..03` → `COV-01` → `EXP-01/02` → `OPS-02`

Each epic is independently demoable. The first genuinely useful checkpoint is the end of LIN.
