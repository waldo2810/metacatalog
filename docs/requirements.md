# metacatalog — Phase 1 requirements

## Problem

Some organization's data lineage live in 50-tab Excel workbook wired together with `XLOOKUP`/`XMATCH`. It is unreviewable, uncollaborative, and breaks silently when a source schema changes. It is already strained at two source systems, and more are coming.

Commercial catalogs (Collate, Atlan) were rejected on cost: orgs need lineage, not a platform.

A second need sits alongside lineage: **"which processes use this field?"** — meaning consumers (business processes, reports, applications), not ETL jobs. Combined with mapping coverage, this tells us what is actually worth building in the warehouse.

## Scope

| | |
|---|---|
| **Sources** | Ingested. SQL Server instances on Azure VMs. Schemas, tables, columns extracted automatically. Ground truth. |
| **DWH and marts** | Declared. They do not exist as physical databases yet. Their fields are authored in YAML and mapped to source fields. |
| **Interface** | CLI + a git-tracked `catalog/` directory of YAML. No web UI in Phase 1. |
| **Output** | CSV exports for analysts; Mermaid diagrams for engineers' docs and PRs. |

Collaboration story: the catalog is a git repo. Mappings change via pull request. `validate` is the review gate.

## Users

| Persona | Does |
|---|---|
| Data engineer | Runs the CLI, authors YAML mappings and process declarations, reviews PRs |
| Data analyst / BI | Reads exported CSV to answer lineage questions. Does not author YAML |

## Questions the tool must answer on day one

1. Where does this DWH/mart field come from? (backward trace to source columns, with transforms)
2. What breaks if I change this source column? (forward impact, transitive)
3. Which source fields are unused — and which are wanted but not yet built?

## Decisions

| Topic | Decision |
|---|---|
| Scale target | 50–300 tables, 1k–10k columns. Focus+depth views mandatory; SQLite sufficient |
| Ingest cadence | Manual now. CLI shaped so a CI/cron wrapper is trivial later |
| Connectivity | Undecided (laptop vs jump box). Connectors environment-agnostic; connection strings from env vars; **no credentials in YAML, ever** |
| Mapping shape | Rule blocks per target column: each rule has `when:`, its own `sources:` and its own `transform:`. Plus constant/system-generated targets with no source |
| Processes | One entity with `kind: business \| report \| app`. Plain `process -> column` usage links. Declared in YAML |
| "Unused" | Report mapped yes/no and process count **separately**, so "wanted but not built" is sortable |
| Enforcement | `validate` exits non-zero on errors. Severity split: errors fail, warnings do not |
| Analyst output | CSV, one row per mapping edge |

## Design invariants

These three carry the whole design. Violating any of them costs a rewrite.

### 1. URN is identity

```
mssql://vmprod01/SalesDB/dbo/Customer#CustomerId
dwh://core/DimCustomer#CustomerKey
mart://finance/FactRevenue#Amount
process://report/monthly-billing
```

YAML references URNs. Every join is on URN, never on autoincrement id. `urn.rs` is built first and its format is frozen.

### 2. `origin` on every row: `ingested` | `declared`

Ingestion may only write `ingested` rows. The spec loader may only write `declared` rows. Neither can touch the other's rows.

Without this, the next `ingest` run silently destroys hand-authored mapping work.

### 3. Soft delete via `last_seen` / `deleted_at`

A column absent from an ingest run is marked, never hard-deleted. This lets validation report *"mapped source column vanished in run 42"* instead of throwing a foreign-key error. **Stale is worse than absent.**

## Stack

Rust — hard zero-dependency policy: `Cargo.toml` `[dependencies]` stays empty except for database driver/client crates, which are allowed by policy. Never add a *non-driver* crate without asking first; solve with `std` or hand-written code instead. (Supersedes the project's original Python-based stack — Phase 1 was scoped assuming Python 3.12 for `sqlglot`, but the language was switched to Rust and that rationale no longer applies.)

| Concern | Choice |
|---|---|
| CLI | hand-rolled arg parsing (`src/cli.rs`), no `clap` |
| YAML parsing | hand-rolled subset parser that retains line numbers on every node, no `serde_yaml`/`ruamel` equivalent |
| YAML schema | hand-rolled validation over the parsed tree, no `serde`/Pydantic equivalent |
| Error line mapping | validation reads the line straight off the parse-tree node it is checking — no separate mapping step, since the parser already carries line numbers |
| DB | `rusqlite` (bundled SQLite), approved 2026-08-22 during OPS-01 because the store is specified as SQLite with `PRAGMA user_version` migrations |
| Migrations | `PRAGMA user_version` runner over numbered `.sql` files |
| SQL Server | a database driver crate (e.g. `tiberius`), allowed under the database-driver exception — specific crate not yet pinned |
| Traversal | Hand-written recursive CTE, executed as raw SQL through `rusqlite` |
| Export / render | `std::io`/`String` for CSV; Mermaid by string building |
| Tests / packaging | `cargo test`; `cargo build` |

Runtime dependencies are `rusqlite` plus whatever database driver crates ingestion needs — all database driver/client crates are allowed by policy; every other crate still needs sign-off.

## Data model

```
data_source(id, name, kind, host, connection_env, last_run_id)
asset(id, urn UNIQUE, source_id NULL, layer, database, schema, name, kind,
      origin, spec_file NULL, first_seen, last_seen, deleted_at NULL)
column(id, asset_id, urn UNIQUE, name, ordinal, data_type, nullable,
       description, origin, last_seen, deleted_at NULL)
rule(id, target_column_id, when_expr NULL, transform NULL, is_default,
     spec_file, line)
lineage_edge(id, rule_id, source_column_id, target_column_id,
             source_asset_id, target_asset_id, origin,
             UNIQUE(rule_id, source_column_id, target_column_id))
process(id, urn UNIQUE, name, kind, description, owner, spec_file)
process_usage(id, process_id, column_id, UNIQUE(process_id, column_id))
run(id, source_id, started_at, finished_at, status, assets_seen,
    columns_seen, columns_added, columns_removed, error)
```

Asset-level lineage is a **view** over `lineage_edge` grouped by asset pair — never stored twice.

## CLI surface

| Command | Purpose |
|---|---|
| `mc init` | Scaffold a `catalog/` directory |
| `mc ingest [--source NAME]` | Discover schemas, upsert `ingested` rows, record a run |
| `mc load` | Parse `catalog/**`, upsert `declared` rows, rebuild declared edges |
| `mc validate` | Load + resolve without writing; non-zero exit on errors |
| `mc show <urn> [--up N] [--down N]` | Text lineage tree around a node |
| `mc impact <urn>` | Flat downstream list |
| `mc uses <urn>` | Processes consuming a column |
| `mc process <urn>` | Columns a process needs |
| `mc coverage [--unmapped] [--min-processes N]` | Build-backlog report |
| `mc export --format csv -o FILE` | One row per mapping edge |
| `mc render --format mermaid --focus URN --depth N` | Diagram for docs/PRs |

## Out of scope (Phase 1)

Web UI · authentication · static HTML viewer · SQL parsing / auto-lineage from views and procs · ADF/SSIS pipeline connectors · standalone `diff` command (run history is stored; the report waits) · business glossary · tags · data quality.

The connector interface (ING-01) and invariant #2 keep all of these cheap to add later.

See [stories/README.md](stories/README.md) for the story index.
