# Proposal

## Why

`docs/SRS.md` §9 is the single authoritative record of unresolved decisions: each row carries a status, an owner, its affected requirement IDs, and the gate it blocks, and every requirement that depends on it cites `Pending decision (G-xx)` instead of a settled rule.

Five of those rows gate the providers and security work (`G5`): `G-19` (OpenCode endpoint and port), `G-09` (how a provider's model identifier is determined), `G-27` (the masking rule and the test that proves a masked preview is not reversible), `G-20` (existing keys unreadable at run time), and `G-13` (exponential backoff base and cap).

Those values cannot be asserted anywhere until the record says they are settled. An implementation change that hard-coded an endpoint, a masking rule, or a backoff value would contradict the approved SRS, which still records the question as `Open`. The decisions therefore need a record of their own: a change under which each row is settled, the citations that depend on it are updated, and the rationale is preserved.

Resolution order, approved on 2026-10-22: `G-19` → `G-09` → `G-27` → `G-20` → `G-13`, one user decision at a time. `software-architect` owns all five (approved 2026-10-22). The decisions are deliberately not folded into the skeleton change, so that no implementation change asserts a value the SRS still calls open.

## What Changes

- For each of the five rows, in the approved order and only after the user approves that individual decision: change its `docs/SRS.md` §9 status from `Open` to `Resolved` and replace the question with the settled rule — dated and attributed — in the style of the already-resolved rows (`G-01`, `G-10`, `G-21`, `G-25`, `G-28`, `G-29`).
- Replace each citation of a resolved row so that no requirement, data-model field, or interface note still points at an open decision: `G-13` at line 367; `G-09` at lines 380, 840, 898; `G-27` at lines 455, 851; `G-20` at line 481; `G-19` at lines 472, 838, 898.
- Record one design section per decision in this change's `design.md` — decision, rationale, alternatives considered, approver, date — owned by `software-architect`.
- Add no requirement, change no requirement status, and change no product behavior.

## Capabilities

### New Capabilities

None — this is a documentation-only change (`skip_specs: true`). Every decision concerns requirements that already exist in `docs/SRS.md`, and `openspec/specs/` holds no living specs yet.

### Modified Capabilities

None.

## Impact

- **Files:** `docs/SRS.md` (the citation lines listed above and §9 rows at lines 1096, 1100, 1106, 1107, 1114) and `design.md` inside this change. No source code, dependencies, IPC contracts, migrations, or build configuration are touched.
- **Affected requirement IDs:** `FR-301`, `FR-307`, `FR-308`, `FR-309`, `FR-310`, `FR-312`, `FR-313`. All are `Draft` and stay `Draft`; resolving a decision records the rule, it does not promote a requirement to `Approved`. `FR-309` joins the list because the settled backoff curve governs the wait after a timeout as well as after a network drop and a rate-limited retry.
- **Unblocks:** clearing gate `G5` is what lets the skeleton + Providers & Security change carry `FR-301..313` against settled rules instead of `Pending decision` markers. No other gate (`G2`, `G3`, `G4`, `G6`, `G7`) is affected by these five rows.
- **Derivative references:** the PRD feature table (`docs/PRD.md` lines 171, 175, 176, 178) and `docs/ar/SRS.ar.md` §5 (lines 40–42) cite these rows. This change verifies they do not still assert openness; editing them is not part of it.
- **Non-negotiables:** untouched. Keyring-only key storage, loopback-only discovery, offline detection, no telemetry, and no user text in logs remain exactly as written in `AGENTS.md` §3.
