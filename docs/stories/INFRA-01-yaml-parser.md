# INFRA-01 — Parse a YAML subset with line-number tracking

**Epic:** Infrastructure · **Priority:** P0 · **Depends on:** —

## Story

**As a** data engineer maintaining the catalog
**I want** the tool's YAML parser to track the source line of every value it reads
**So that** validation errors (VAL-01, MOD-01, ING-03) can point at the exact line of a bad key or ref, and mistakes are fixable without hunting through the file.

## Why this is its own story

The original Python-stack design (`docs/requirements.md`) got parsing, schema validation and line tracking for free from `ruamel.yaml` (round-trip mode) plus Pydantic. Under the project's zero-dependency policy (`std` only; database driver crates are the sole blanket exception — see [Stack](../requirements.md#stack)), there is no YAML crate to lean on. A hand-rolled parser with line tracking is real, freestanding engineering, and every other story that touches `catalog/**` (MOD-01, MOD-02, MOD-05, ING-03, VAL-01, OPS-01's scaffold) depends on it existing first. It earns a story instead of staying an implementation-note aside.

## Acceptance criteria

**AC1 — supported subset**
Given a file using block-style mappings, block-style sequences, plain and quoted scalars, and nesting by indentation
When it is parsed
Then a tree of nodes is produced with no data loss.

**AC2 — explicitly unsupported constructs are rejected, not silently misparsed**
Given a file using flow style (`{a: 1}`, `[1, 2]`), anchors/aliases (`&x`, `*x`), multi-document streams (`---`), or tags (`!!str`)
When it is parsed
Then parsing fails with a clear "unsupported YAML feature" error naming the file and line, rather than producing a wrong or partial tree.

**AC3 — line number on every node**
Given any scalar, mapping key, or sequence item
When the parse tree is inspected
Then it carries the 1-based source line it appeared on, independent of how deeply nested it is.

**AC4 — comments tolerated**
Given a file with `#`-prefixed comments, including inline documentation on a key (per OPS-01 AC5)
When it is parsed
Then comments are ignored and do not affect the resulting tree or its line numbers.

**AC5 — scalar types**
Given plain scalars that look like strings, integers, decimals, booleans, or `null`
When parsed
Then each is typed accordingly, matching YAML 1.1-ish core-schema behavior closely enough that `type: nvarchar(200)` and similar values round-trip as strings, not something else.

**AC6 — malformed input fails clearly**
Given a file with inconsistent indentation, an unterminated quote, or a duplicate key in the same mapping
When it is parsed
Then it fails naming the file and line, not a stack trace or a panic.

**AC7 — no round-trip / re-serialization requirement**
Given the tool never programmatically rewrites a `catalog/**` file
When the parser is designed
Then it only needs to parse forward into a line-tracked tree — no "write this tree back out as YAML" capability is required, unlike `ruamel.yaml`'s round-trip mode.

## Implementation notes

Shape: a recursive-descent or line-oriented parser producing a small `enum Node { Scalar(String, Line), Seq(Vec<Node>, Line), Map(Vec<(String, Node)>, Line) }`-shaped tree (naming indicative, not prescriptive). Every node's `Line` field is what VAL-01's error reporting and INFRA-01 consumers key off of.

Downstream schema validation (MOD-01 AC5, ING-03 AC2, VAL-01) walks this tree directly rather than a typed struct — keep the tree alive alongside any validated struct for the whole load, matching the note already in VAL-01: discarding it after parsing is what makes line reporting impossible to retrofit later.

Indentation-sensitive parsing is the fiddly part. Budget real test time for: tabs vs. spaces (reject tabs, matching YAML spec practice, with a clear error), sibling keys at inconsistent indentation, and block scalars (`|`, `>`) if the catalog format ends up needing multi-line `description` fields — confirm with MOD-01 before building; if not needed, drop from scope and note it as out of scope explicitly rather than half-supporting it.

Duplicate-key detection (AC6) also serves MOD-01 AC4 (duplicate column declared twice in one file) — one check, two consumers.

## Out of scope

Flow style, anchors/aliases, multi-document streams, YAML tags, round-trip re-serialization, block scalars (`|`/`>`) unless a later story requires multi-line values.

## Verification

Parse a sample catalog file (from OPS-01's scaffold) → tree matches expected structure, every node's line number matches the file. Feed it flow-style YAML → clear rejection, not silent misparse. Duplicate key in one mapping → error names file and line. Tabs for indentation → clear error, not a silent misparse.
