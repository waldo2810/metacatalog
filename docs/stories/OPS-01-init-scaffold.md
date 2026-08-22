# OPS-01 — `mc init` scaffolds a catalog repo with worked examples

**Epic:** Project ops · **Priority:** P1 · **Depends on:** INFRA-01, INFRA-02

## Story

**As a** data engineer
**I want** one command that creates a working catalog directory with realistic examples
**So that** the YAML contract is learned by editing something that already validates, not by reading docs.

## Acceptance criteria

**AC1 — scaffold**
Given `mc init` in an empty directory
When it runs
Then it creates `catalog/sources.yml`, `catalog/warehouse/`, `catalog/marts/`, `catalog/processes/`, `.metacatalog.yml`, `.gitignore` and `README.md`.

**AC2 — examples validate**
Given `mc init --with-examples`
When followed by `mc validate --offline`
Then the samples pass, exercising a multi-source mapping, a conditional rule, a generated column, a mart over the DWH, and a process with usage links.

**AC3 — store is ignored**
Given the generated `.gitignore`
When inspected
Then the SQLite store and any exports are ignored — the YAML is source of truth, the store is a derived cache.

**AC4 — non-destructive**
Given a directory that already has a `catalog/`
When `mc init` runs
Then it refuses rather than overwriting, unless `--force` is given.

**AC5 — inline documentation**
Given the generated YAML
When read
Then comments explain each key, including that credentials belong in env vars only.

**AC6 — offline validation**
Given no ingest has run
When `mc validate --offline` runs
Then YAML syntax, schema and internal consistency are checked while ref resolution against ingested columns is skipped with a notice.

**AC7 — store bootstrap**
Given no store file
When any command runs
Then the store is created and migrated automatically, with no separate init step.

## Implementation notes

```
catalog/
  sources.yml
  warehouse/dim_customer.yml
  marts/finance_revenue.yml
  processes/billing.yml
.metacatalog.yml          # store path, severity overrides, defaults
.gitignore                # metacatalog.db, exports/
README.md                 # how to run ingest / validate / export
```

AC6 is the story's quiet payoff: the examples reference `mssql://` URNs that cannot resolve until someone ingests a real database. Without an offline mode, `mc init --with-examples && mc validate` fails on first contact — the worst possible first impression. Offline mode also gives CI something to run on a repo with no database access (OPS-02).

The examples double as fixtures. Point the test suite at the same files so a change to the YAML contract that breaks the scaffold fails the build immediately, instead of shipping a scaffold that no longer validates.

AC7 folds migrations into normal startup: open the store, read `PRAGMA user_version`, apply numbered `.sql` files above it. Fifty lines, and no user ever runs a migrate command.

Keep the scaffold small. A scaffold with thirty example files gets deleted wholesale; one with four gets edited.

## Verification

`mc init --with-examples` in a temp dir → tree created, `mc validate --offline` exits 0. Run again → refuses without `--force`. Delete the store, run any command → recreated and migrated. Break the example YAML → test suite fails.
