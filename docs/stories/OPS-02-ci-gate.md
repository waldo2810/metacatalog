# OPS-02 — `validate` usable as a CI gate

**Epic:** Project ops · **Priority:** P2 · **Depends on:** VAL-03, OPS-01

## Story

**As a** data engineer
**I want** mapping changes checked automatically on every pull request
**So that** review catches broken lineage before merge — the collaboration property the spreadsheet could never have.

## Acceptance criteria

**AC1 — offline PR check**
Given a PR touching `catalog/**`
When CI runs `mc validate --offline`
Then schema errors, duplicates, layer inversions and credential leaks fail the build without any database access.

**AC2 — full check with a store**
Given CI can restore a store artifact or reach the databases
When `mc validate` runs
Then ref resolution and drift checks (VAL-01/02) run too.

**AC3 — annotated output**
Given `--format github`
When run in CI
Then findings are emitted as `::error file=...,line=...::message` workflow commands, so they appear inline on the PR diff.

**AC4 — exit codes honoured**
Given warnings only
When CI runs without `--strict`
Then the build passes; with `--strict` it fails.

**AC5 — example workflow shipped**
Given `mc init`
When it scaffolds
Then it writes `.github/workflows/validate.yml` running the offline check on PRs.

**AC6 — scheduled ingest is documented**
Given the README
When read
Then it shows the wrapper for a scheduled ingest job — the CLI takes no changes to support it, and that is the point of building it as a CLI.

**AC7 — usable exit summary**
Given a CI run
When it finishes
Then the last line is the counts summary (VAL-03 AC8), so a truncated log still shows the verdict.

## Implementation notes

```yaml
# .github/workflows/validate.yml
name: validate catalog
on:
  pull_request:
    paths: ["catalog/**"]
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: astral-sh/setup-uv@v5
      - run: uv run mc validate --offline --format github
```

Offline is the default for PR checks deliberately. The runner will not reach an internal Azure VM without extra plumbing, and a gate that needs a VPN is a gate that gets disabled within a month. The offline check still catches the majority of authoring mistakes.

AC2's fuller check belongs on a schedule, not on PRs — run it wherever ingest runs, after ingest, so drift is reported against fresh data. That job wants `--strict` off and its findings reported to a channel rather than failing a build nobody is watching.

`--format github` is a different reporter over the same `Finding` records from VAL-03. If findings were formatted as strings at the point of detection rather than kept as records, this story would mean rewriting every check — which is why VAL-03 specifies the record shape.

No custom GitHub Action, no container. A workflow file calling the CLI is portable to Azure Pipelines, which is likelier where this client ends up.

## Verification

Open a PR with a typo'd URN → build fails with an inline annotation on the right line. Warning-only change → passes; same with `--strict` → fails. Run the workflow on a repo with no database access → offline check completes.
