# EXP-01 — CSV export, one row per mapping edge

**Epic:** Export · **Priority:** P0 · **Depends on:** MOD-02

## Story

**As an** analyst
**I want** the whole mapping set as a flat CSV
**So that** I can filter and pivot it in Excel the way I do today — except it is generated, validated and never hand-maintained.

## Acceptance criteria

**AC1 — one row per edge**
Given `mc export --format csv -o mappings.csv`
When it runs
Then each lineage edge is one row.

**AC2 — columns**
Given the file
When opened
Then it has: `target_urn, target_layer, target_namespace, target_table, target_column, target_type, condition, transform, source_urn, source_system, source_database, source_schema, source_table, source_column, source_type, source_status`
And URN components are split into separate columns as well as being present whole, so filtering needs no text surgery.

**AC3 — conditions per row**
Given a conditionally-mapped column (MOD-03)
When exported
Then one row per rule per source, each carrying its own condition and transform.

**AC4 — generated columns included**
Given MOD-04 generated columns
When exported
Then they appear with empty source fields and `condition` set to their generation kind, so a reader sees the complete target column list.

**AC5 — dropped sources marked**
Given a source column soft-deleted upstream
When exported
Then `source_status` is `dropped` with the run timestamp; default excludes nothing, since a broken mapping is what a reader most needs to see.

**AC6 — filters**
Given `--layer mart` or `--source vmprod01`
When supplied
Then only matching rows are exported.

**AC7 — Excel-safe**
Given the file
When opened in Excel
Then it is UTF-8 with BOM, `\r\n` line endings, and values starting `=`, `+`, `-` or `@` are prefixed so Excel does not interpret them as formulas.

**AC8 — deterministic**
Given no data changed
When exported twice
Then the files are byte-identical, so the export can be committed and diffed.

## Implementation notes

This is the analysts' only interface, so it is P0 despite being the simplest story here.

AC7 is not fussiness. Transform text like `=CONCAT(...)` will absolutely appear in this data, and Excel will evaluate it — corrupting the cell and, in the formula-injection case, worse. Prefix with `'` on export. Test it explicitly.

AC8 means a stable `ORDER BY target_urn, rule.ordinal, source_urn` and no timestamps in the body. A committed, diffable export gives the team change history for free — the review artifact the workbook never had.

Hand-written CSV writer over `std::io::Write`, with a UTF-8 BOM written first and `\r\n` line endings — no CSV crate.

Stream rows rather than materializing; at 10k columns this is small, but streaming costs nothing and removes any ceiling.

Denormalize URN parts in SQL, not in application code — the parts are already stored as columns on `asset`, so re-parsing URNs to split them is both slower and a second source of truth.

## Verification

Export a fixture → row count equals edge count plus generated columns. Conditional column → one row per rule-source pair with distinct conditions. Transform beginning with `=` → prefixed, opens inert in Excel. Export twice → `diff` shows nothing. `--layer mart` → mart rows only.
