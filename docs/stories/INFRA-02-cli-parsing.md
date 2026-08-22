# INFRA-02 — Shared CLI argument parsing across commands

**Epic:** Infrastructure · **Priority:** P0 · **Depends on:** —

## Story

**As a** data engineer running `mc` from the shell or from CI
**I want** every subcommand's flags parsed the same way, with consistent errors for bad input
**So that** `--format`, `--strict` and similar flags behave identically everywhere, and a typo'd flag gets a clear message instead of being silently ignored or misparsed.

## Why this is its own story

The original Python-stack design used stdlib `argparse`, which gives subcommands, typed flags, defaults and error messages for free. Under the zero-dependency policy there is no `clap` (see [Stack](../requirements.md#stack)), so `src/cli.rs` has to parse `env::args()` by hand. The current stub (`src/cli.rs`) only recognizes one bare positional (`init`) — it has no flag support at all. Nearly every later story assumes flags exist (`--source`, `--up N`, `--format json`, `--strict`, `--unmapped`, `--min-processes N`, `--with-examples`, `--force`, `--check`, `--transitive`, `--focus`, `--depth`, `--offline`, `--layer`, `--sort`, `--mapped-multiple`, `--include-deleted`). Specifying the shared parsing behavior once, instead of per-command, avoids ten slightly-different hand-rolled parsers and ten slightly-different sets of edge-case bugs.

## Acceptance criteria

**AC1 — subcommand dispatch**
Given `mc <command> [args...]`
When invoked
Then the first positional argument selects the command, and an unknown command prints the list of valid commands and exits non-zero.

**AC2 — flag forms**
Given a command accepting flags
When invoked with `--flag value`, `--flag=value`, or a bare `--flag` (boolean)
Then all three forms are accepted consistently across every command.

**AC3 — required vs. optional, with defaults**
Given a command with an optional flag that has a documented default (e.g. `--up` defaults to unlimited, `--min-processes` defaults to 0)
When the flag is omitted
Then the documented default applies, and this is true the same way for every command — no command silently requires a flag another command defaults.

**AC4 — typed values fail clearly**
Given a flag documented as taking an integer (e.g. `--up N`, `--depth N`)
When given a non-integer value
Then the command fails with a message naming the flag and the value it rejected, not a panic or a silent 0.

**AC5 — unknown flag is an error**
Given a flag not recognized by the command
When invoked
Then it fails naming the unrecognized flag, rather than being ignored — a typo'd `--strcit` must not silently behave like `--strict` was never passed.

**AC6 — `--help` on every command**
Given `mc <command> --help` or `mc --help`
When invoked
Then usage text is printed listing the command's flags, their defaults, and one line of purpose — generated from the same flag definitions used to parse, so help text cannot drift from actual behavior.

**AC7 — positional + flag mixing**
Given a command with both a positional argument and flags (e.g. `mc show <urn> --up 1 --format json`)
When flags and the positional appear in any order
Then parsing succeeds the same way regardless of order.

## Implementation notes

A small shared parser lives in `src/cli.rs` (or a new `src/args.rs` if `cli.rs` grows past dispatch): each command declares its flags as a list of `(name, kind, default)` — `kind` being string/int/bool — and the shared parser walks `env::args()` once against that declaration, producing a typed result or an `Err` with the offending flag/value. Commands then read out of that typed result rather than each re-walking the raw argument list.

This replaces the current stub's fragile `args.get(len-1)` (which reads the *last* argument, not the first positional after the binary name, and breaks the moment any flag is added) with an explicit walk from index 1.

`--help` generation (AC6) is worth building against the same flag-declaration list from the start — bolting it on later tends to produce help text that lists flags the parser no longer accepts, or omits ones it does.

Keep this deliberately small: a hand-rolled `clap` is out of scope. No subcommand aliases, no flag abbreviation/prefix-matching, no `-h`/`--help` short-flag distinction beyond the two literal forms — add these only if a later story's acceptance criteria actually needs them.

## Out of scope

Flag abbreviation/prefix matching, shell completion generation, colored help output, config-file-as-flags-source.

## Verification

`mc show <urn> --up 1` and `mc show --up 1 <urn>` → identical result. `mc show <urn> --up abc` → clear error naming `--up` and `abc`. `mc show <urn> --strcit` (typo of a flag that exists on another command) → clear "unrecognized flag" error, not silently accepted. `mc --help` and `mc show --help` → usage text lists real flags with real defaults.
