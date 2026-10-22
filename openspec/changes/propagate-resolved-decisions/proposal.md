# Proposal

## Why

`resolve-open-decisions` settled five rows of `docs/SRS.md` §9 — `G-09`, `G-13`, `G-19`, `G-20`, `G-27` — but the two derivative documents still describe them the old way. `docs/ar/SRS.ar.md` §5 lists all five under مفتوحة (open) while line 40 lists only the six previously settled IDs, and the `docs/PRD.md` feature table cites the same five IDs as live constraints with no status word. That is the one contradiction `resolve-open-decisions` task 6.5 found and deliberately did not fix, because that change's write scope is `docs/SRS.md` only.

The SRS is now internally consistent; its summaries are not. A reader of the Arabic summary or the PRD would conclude five settled decisions are still pending. The correction is small and the wording is copied from the approved SRS, not decided here.

## What Changes

- `docs/ar/SRS.ar.md` §5: move `G-09`, `G-13`, `G-19`, `G-20`, `G-27` out of the مفتوحة sentence and into the محسومة sentence, each with a one-clause descriptor drawn from its settled §9 row.
- `docs/PRD.md` §7 feature table, cells 171, 175, 176, 178: replace each trailing "remaining" note with a short resolved clause naming the settled rule and the `G-xx` ID.
- `openspec/changes/resolve-open-decisions/tasks.md` task 6.5: record the check result and the approved handoff to this change, then tick it.
- Inspect the `docs/PRD.md` §11 decision log and change it only if it asserts any of the five is open.
- Add no requirement, change no requirement status, and change no product behavior.

## Capabilities

### New Capabilities

None — this is a documentation-only change (`skip_specs: true`). No requirement text, status, or acceptance criterion changes, so no spec-level behavior changes and no delta spec exists to write.

### Modified Capabilities

None.

## Impact

- **Files:** `docs/ar/SRS.ar.md` (§5, lines 40–42), `docs/PRD.md` (§7 feature table cells 171, 175, 176, 178; §11 decision log inspected), and `openspec/changes/resolve-open-decisions/tasks.md` (task 6.5 only). No source code, dependencies, IPC contracts, migrations, or build configuration are touched.
- **Affected requirement IDs:** `FR-301`, `FR-307`, `FR-308`, `FR-309`, `FR-310`, `FR-312`, `FR-313` are cited by the settled rows. All remain `Draft`; this change promotes none of them and rewords no requirement.
- **Source of truth:** `docs/SRS.md` is not edited. Every clause written here is derived from a settled §9 row at lines 1106 (`G-09`), 1110 (`G-13`), 1116 (`G-19`), 1117 (`G-20`), 1124 (`G-27`) or from the normative cell each row points at (367, 380, 491, 848, 850, 861).
- **Closes:** the single contradiction reported by `resolve-open-decisions` task 6.5 — the Arabic summary listing five settled IDs as open. The PRD cells were stale rather than false; they are refreshed for the same traceability reason.
- **Non-negotiables:** untouched. Keyring-only key storage, loopback-only discovery, offline detection, no telemetry, and no user text in logs remain exactly as written in `AGENTS.md` §3.
