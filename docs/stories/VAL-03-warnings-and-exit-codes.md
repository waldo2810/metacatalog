# VAL-03 — Warn on orphans, type mismatches and cycles; severity-split exit codes

**Epic:** Validation · **Priority:** P1 · **Depends on:** VAL-01

## Story

**As a** data engineer
**I want** incomplete work to warn while genuinely broken references fail
**So that** half-finished mapping can be committed and reviewed without either disabling the gate or drowning in noise.

## Acceptance criteria

**AC1 — exit codes**
Given a validate run
When it completes
Then exit 0 = clean, 1 = at least one error, 2 = warnings only, and 3 = the tool itself failed (bad config, unreadable store).

**AC2 — orphan target warns**
Given a declared target column with no rules and no `generated`
When I run `mc validate`
Then a warning names it as unmapped — and exit stays 0 or 2, never 1.

**AC3 — type mismatch warns**
Given a target declared `int` mapped to a source `nvarchar(200)`
When I run `mc validate`
Then a warning reports the mismatch with both types and the transform text, since a transform may well make it legitimate.

**AC4 — cycles are errors**
Given a cycle among column edges
When I run `mc validate`
Then it is an error listing the URNs on the cycle in order.

**AC5 — same-asset derivation is not a cycle**
Given MOD-04 `derived_from` edges within one table
When cycle detection runs
Then they are not flagged, unless the columns genuinely form a column-level cycle.

**AC6 — severity override**
Given `.metacatalog.yml` sets `severity: { type_mismatch: error, orphan_target: ignore }`
When I run `mc validate`
Then those checks use the configured severity, and `ignore` suppresses them entirely.

**AC7 — strict mode**
Given `mc validate --strict`
When warnings exist
Then it exits 1 — one flag turns the CI gate from permissive to total.

**AC8 — summary**
Given any run
When it finishes
Then it prints a count per check (`3 errors, 11 warnings across 5 files`) after the detail, so the tail of the output is readable.

## Implementation notes

Checks and default severities:

| Check | Default |
|---|---|
| `unresolved_ref` (VAL-01) | error |
| `dropped_source` (VAL-02) | error |
| `type_change_narrowing` (VAL-02) | error |
| `type_change_widening` (VAL-02) | warning |
| `layer_inversion` (MOD-05) | error |
| `duplicate_default_rule` (MOD-03) | error |
| `cycle` | error |
| `orphan_target` | warning |
| `type_mismatch` | warning |
| `stale_source` | warning |
| `credential_in_yaml` (ING-03) | error, not overridable |

Represent every finding as one `Finding(check, severity, message, spec_file, line, urns)` record. Severity is applied at the end from config, so adding a check is one function and one table row — and the reporter, exit-code logic and `--strict` never change.

Cycle detection: iterative DFS with a colour map over the column edge graph, not recursion; at 10k columns a recursive walk can blow the stack, and it fails as a crash rather than a finding.

AC6's `ignore` matters for adoption. Mid-migration, the client will have hundreds of orphan targets; a team that cannot silence a check will instead stop reading the output entirely.

`credential_in_yaml` is deliberately not overridable — a check that can be configured away is not a safety check.

## Verification

Orphan column → warning, exit 2. Same run with `--strict` → exit 1. Set `orphan_target: ignore` → silent, exit 0. Hand-build a cycle → error listing the ring. `derived_from` chain → no cycle reported.
