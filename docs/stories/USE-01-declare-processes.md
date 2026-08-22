# USE-01 — Declare processes with kind business/report/app

**Epic:** Process usage · **Priority:** P1 · **Depends on:** OPS-01

## Story

**As a** data engineer
**I want** to declare the business processes, reports and applications that consume data
**So that** field usage can be attributed to something the business recognises.

## Acceptance criteria

**AC1 — declaration**
Given `catalog/processes/billing.yml` declaring a process with `name` and `kind`
When I run `mc load`
Then a process exists with URN `process://<kind>/<slug>`.

**AC2 — kinds**
Given `kind`
When it is validated
Then it must be one of `business`, `report`, `app`, and any other value fails naming the allowed set.

**AC3 — several per file**
Given a file with a `processes:` list
When it is loaded
Then each entry becomes its own process — reports especially come in batches and should not need a file each.

**AC4 — metadata**
Given `description`, `owner` and `criticality`
When declared
Then they are stored and shown in usage query output.

**AC5 — slug stability**
Given a process whose `name` changes but whose `slug` stays
When I reload
Then the URN is unchanged and its usage links survive.

**AC6 — duplicate URNs**
Given two processes resolving to the same URN
When I run `mc load`
Then it fails naming both files and lines.

**AC7 — declared, like everything else**
Given loaded processes
When rows are inspected
Then `origin = 'declared'` and `spec_file` is set.

## Implementation notes

```yaml
# catalog/processes/billing.yml
processes:
  - slug: monthly-billing
    name: Monthly billing run
    kind: business
    owner: finance-ops
    criticality: high
    description: Generates invoices on the 1st of each month

  - slug: revenue-dashboard
    name: Revenue dashboard
    kind: report
    owner: bi-team
    criticality: medium
```

`slug` is explicit and required rather than derived from `name`. Deriving it means renaming a report silently changes its URN and orphans every usage link — AC5 exists specifically to prevent that. Validate slugs as `[a-z0-9-]+`.

Kind lives in the URN (`process://report/revenue-dashboard`) so the scheme stays parseable and a kind change is visibly a different entity. That is the intended trade: reclassifying a process is a deliberate act with a visible diff.

`criticality` is free-form enum (`high|medium|low`) with no behaviour attached in Phase 1 beyond display and sorting in COV-01 — it is what turns the coverage report into a prioritised backlog rather than a list.

## Verification

Load a file with three processes → three rows with correct URNs. Bad kind → error listing allowed values. Change `name`, keep `slug` → URN and links unchanged. Duplicate slug+kind across files → error naming both.
