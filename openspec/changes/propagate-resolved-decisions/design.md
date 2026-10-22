# Design

## Context

See `proposal.md` — Why. The five settled rows already exist in `docs/SRS.md` §9 and their rules are already spelled out in the normative cells. This change carries those rules into the two derivative documents and closes the check that `resolve-open-decisions` deliberately left failing.

Current state, verified on 2026-10-22:

- `docs/ar/SRS.ar.md` line 40 (محسومة) lists `G-01`, `G-10`, `G-21`, `G-25`, `G-28`, `G-29`; line 42 (مفتوحة) still lists `G-09`, `G-13`, `G-19`, `G-20`, `G-27` among the open rows; line 44 sends the reader to English §9 for full detail.
- `docs/PRD.md` cells 171, 175, 176, 178 cite the same five IDs in their Notes column with no status word, so they are stale rather than false.
- `docs/PRD.md` §11 decision log has no row for any of the five.

## Goals / Non-Goals

**Goals:**

- Make the Arabic summary and the PRD feature table agree with `docs/SRS.md` §9 on the state of these five decisions.
- Keep every ID and every status unchanged, and leave `docs/SRS.md` byte-identical.
- Leave the record in `resolve-open-decisions` truthful and complete, so it can be archived.

**Non-Goals:**

- No new decision, no rewording of a decision, no change to `FR-301`, `FR-307`, `FR-308`, `FR-309`, `FR-310`, `FR-312`, `FR-313`.
- No edit to `docs/ar/PRD.ar.md`, `AGENTS.md`, `.opencode/**`, `resolve-open-decisions/design.md`, or `resolve-open-decisions/proposal.md`.
- No expansion of the PRD's decision log with the five decisions: the SRS §9 row is the decision record, and duplicating it in the PRD would create a second copy that can drift.

## Decisions

**1. Propagate in a separate docs-only change instead of widening `resolve-open-decisions`.** Approved by the product owner on 2026-10-22. The resolving change's write scope is `docs/SRS.md`; reopening it would invalidate its recorded verification and mix two review surfaces. A separate `skip_specs: true` change keeps each diff reviewable against one purpose.

**2. Arabic summary: move the five entries in place, keep one محسومة bullet, keep ascending ID order.** Alternative: a new "recently settled" bullet. Rejected because line 44 already declares the English §9 the only complete table, so a third list would add a category the file does not maintain.

**3. Descriptors are one-clause abbreviations, not full restatements.** Each is derived from the settled row's own sentence and its normative cell:

| ID | SRS row | Normative cell | Clause carried into the summary |
|---|---|---|---|
| `G-09` | 1106 | 380, 850 | required, user-supplied `model_id`; authoritative; no model chosen for the user |
| `G-13` | 1110 | 367 | one curve, base 1 s, cap 30 s per wait, full jitter; `Retry-After` replaces the computed delay |
| `G-19` | 1116 | 479, 848 | user-configured, keyless, disabled by default, never discovered or probed, no default endpoint or port |
| `G-20` | 1117 | 491 | unusable but not `invalid`; fail over, status unchanged; `E_KEYRING_UNAVAILABLE` with `reason` `unreadable` when none is readable |
| `G-27` | 1124 | 861 | last four characters behind a fixed-width mask that does not vary with key length and includes no prefix |

**4. Numeric values are copied as `1 s` and `30 s`, not localized.** The English SRS is binding and the summary is derivative; converting the unit or the digits would create a second rendering of the same constant that could drift from the tested value.

**5. PRD cells keep the `G-xx` ID and gain a `resolved` marker.** The ID preserves the trace to the decision, and the marker distinguishes a settled rule from the still-open citations that sit in neighbouring cells (`G-15`, `G-05`, `G-02`). Alternative: dropping the ID once resolved. Rejected because the PRD would then lose the only pointer from the feature to the decision that fixed its behaviour.

**6. Decision log inspected, not extended.** No §11 row mentions any of the five IDs. The 2026-10-21 document-approval row says document approval "does not resolve any open `G-xx` decision", which remains true: the five were settled by `resolve-open-decisions` on 2026-10-22, not by document approval. No correction is therefore required, and adding rows would duplicate the SRS record.

**7. No directional formatting characters in the Arabic edit.** The summary relies on the renderer's RTL handling; inserting bidi controls would make the line fragile in diffs, search, and copy-paste, and the project supplies none of them today.

## Risks / Trade-offs

- A one-clause descriptor could be read as the complete rule → the clause is an abbreviation by construction, and line 44 continues to point at English §9 for the full row; the normative cells keep the testable wording.
- A reader could mistake a resolved `G-xx` citation in the PRD for a requirement-status change → the cells name the decision only, and no `FR-xxx` status or acceptance criterion is touched.
- The `resolve-open-decisions` task record could overstate what was verified → task 6.5 will state the result, the branch taken, and the handoff without claiming the derivative files were verified before this change ran.
