# OPS-03 — Test suite structure: unit vs. integration, and what CI runs

**Epic:** Project ops · **Priority:** P1 · **Depends on:** INFRA-01, INFRA-02, OPS-01

## Story

**As a** data engineer contributing to `mc` itself
**I want** a clear split between fast unit tests and slower integration tests, with CI running the right ones at the right gate
**So that** a PR gets fast feedback on the code, without every commit needing a live SQL Server to pass, and without that speed costing real coverage of the parts that matter (soft-delete detection, drift severity, upsert idempotency).

## Why this is its own story

`docs/requirements.md` names `cargo test` as the tool but says nothing about what must be tested or how. Every other story's "Verification" section is an informal, story-scoped acceptance check — useful for confirming a feature works, but none of them say whether it runs in CI, on every PR, or only by hand. OPS-02 gates PRs on `mc validate --offline` against **catalog YAML content**, which says nothing about whether the Rust code that implements `validate` is itself tested. That gap is worth closing explicitly, the same way INFRA-01/02 closed the parser/CLI gap: without it, "done" for a story is whatever the implementer happened to check by hand.

## Acceptance criteria

**AC1 — unit tests run on every PR**
Given any pull request touching `src/**`
When CI runs
Then `cargo test --lib` (or equivalent unit-only invocation) executes and must pass — no external services, no network, no Docker.

**AC2 — unit test scope**
Given a module with logic that can be wrong in a way a human reviewer might miss (URN parsing/formatting, YAML parsing and line tracking, the type-narrowing severity table from VAL-02, suggestion ranking from VAL-01, the layer-ordering allowed-pairs check from MOD-05, CSV formula-injection prefixing from EXP-01)
When that module is written
Then it ships with unit tests covering its documented edge cases, in the same PR — not deferred to a follow-up.

**AC3 — integration tests use a real (temp) SQLite store**
Given a test exercising ingestion, load, validation or lineage traversal end-to-end
When it runs
Then it opens a real `rusqlite` connection against a temp-file or in-memory database and runs real SQL — no mocked database layer. This is deliberate: ING-02's soft-delete/resurrection logic and the recursive-CTE traversal in LIN-01/LIN-02 are exactly the kind of thing a mocked DB would let pass while the real upsert or the real CTE is wrong.

**AC4 — SQL Server integration tests are separated and not PR-blocking**
Given a test requiring a live SQL Server (ING-01's Docker-fixture verification, ING-03's connection tests)
When CI is configured
Then those tests are tagged or located separately (e.g. `tests/sqlserver_integration.rs`, or a Cargo feature gate) so a PR from a contributor without Docker/SQL Server access still gets a green `cargo test --lib` run, and the SQL Server suite runs on a schedule or as an explicit opt-in CI job instead.

**AC5 — the OPS-01 scaffold is a regression fixture**
Given `mc init --with-examples`'s generated catalog (OPS-01 AC2)
When the YAML contract changes in a way that breaks it
Then a test running `mc init --with-examples && mc validate --offline` against a temp dir fails the build — this was already called out in OPS-01's implementation notes; this story is where it becomes an actual CI-enforced test rather than an intention.

**AC6 — no coverage percentage gate**
Given the project's small team and solo-to-small delivery model
When CI is configured
Then there is no enforced line/branch coverage percentage threshold — AC2's "ships with tests for documented edge cases" is the bar, not a number, because a coverage percentage is easy to game and does not target the invariants (URN identity, origin isolation, soft-delete-never-hard-delete) that actually matter to this project.

**AC7 — test fixtures live under `tests/fixtures/`**
Given multiple stories need YAML or CSV fixtures (COV-01, EXP-01, ING-02)
When those fixtures are created
Then they live in one shared `tests/fixtures/` directory rather than being duplicated per-test-file, so a fixture used by both `mc coverage` and `mc process` tests (USE-03's note about sharing the "served" query) can't drift into two slightly different versions of the same data.

## Implementation notes

Directory shape (indicative):

```
src/                          # unit tests live inline: #[cfg(test)] mod tests
tests/
  integration.rs              # SQLite-backed end-to-end tests (AC3)
  sqlserver_integration.rs    # gated separately (AC4), not run by default `cargo test`
  fixtures/
    catalog/                  # sample YAML catalogs (shared with OPS-01 scaffold, AC5/AC7)
    csv/                      # expected export output for EXP-01
```

Gate `sqlserver_integration.rs` behind a Cargo feature (e.g. `--features sqlserver-tests`) or an environment variable check that skips with a clear message when unset, rather than failing — a contributor running plain `cargo test` locally should never be blocked by a missing SQL Server.

AC3's temp-SQLite approach reuses whatever `OPS-01` AC7 already builds for store bootstrap (open + migrate on first use) — integration tests get a fresh store the same way a new user's first command does, which doubles as a regression check on the migration runner itself.

CI wiring: extend OPS-02's `validate.yml` workflow (or add a sibling `test.yml`) running `cargo test --lib` and the AC3 SQLite integration tests on every PR; leave the SQL Server suite as a separate scheduled or manually-triggered workflow, consistent with OPS-02's existing offline/online split rationale.

## Out of scope

Coverage percentage tooling (`cargo llvm-cov`/`tarpaulin`) unless a future story asks for it. Property-based/fuzz testing — worth revisiting once the hand-rolled YAML parser (INFRA-01) exists, but not required for Phase 1. Performance/load testing at the 10k-column scale target — that is a manual verification step (see `docs/requirements.md` scale target), not an automated gate.

## Verification

`cargo test --lib` on a clean checkout with no Docker running → passes, no network calls attempted. Break the type-narrowing severity table (VAL-02) → a unit test catches it before the PR verification steps would. Break the OPS-01 scaffold YAML → the AC5 test fails the build. Run without SQL Server reachable → SQL Server suite skips with a clear message rather than failing the whole run.
