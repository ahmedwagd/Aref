# Tasks

## 1. Product Requirements (PRD)

- [x] 1.1 Write `docs/PRD.md` with Summary, Problem, Target Users & Personas, Positioning, v1 Scope, User Journeys, Feature Set, Success Metrics, Non-Goals, Roadmap, Decision Log — verify every section exists and no placeholders (`TBD`, `TODO`) remain.
- [x] 1.2 Add the feature→requirement traceability table mapping each PRD feature to `FR-xxx` IDs — verify every PRD feature has at least one ID and that each ID resolves in `docs/SRS.md`.

## 2. Software Requirements (SRS)

- [x] 2.1 Write the functional requirements `FR-1xx` (detection), `FR-2xx` (humanization), `FR-3xx` (providers & keys), `FR-4xx` (storage/settings/history), `FR-5xx` (UI & i18n) using RFC 2119 keywords — verify every requirement has a unique ID, an atomic statement, and at least one acceptance criterion.
- [x] 2.2 Write non-functional requirements `NFR-1xx` (performance), `NFR-2xx` (security/privacy), `NFR-3xx` (reliability/offline), `NFR-4xx` (accessibility/i18n) — verify they cite the concrete targets from `AGENTS.md` (offline detection, FPR ≤ 5% EN / ≤ 10% AR, thresholds 40/60, no telemetry, keys in OS keyring only).
- [x] 2.3 Add the SRS sections Data Model, IPC Commands & Events, and Error Codes — verify each IPC entry mirrors the `AGENTS.md` §7–§9 flows and each error code has a translation key and a UI state.
- [x] 2.4 Add the Acceptance Criteria Index and confirm the `AGENTS.md` §12 build-order IDs (`FR-201..207`, `FR-301..304`) resolve to real functional requirements — verify with `Select-String -Pattern "FR-20[1-7]|FR-30[1-4]" docs/SRS.md` (verified in this session with the repository grep tool; the PowerShell command is not runnable in this environment).

## 3. Arabic Summaries

- [x] 3.1 Write `docs/ar/PRD.ar.md` (brief summary: vision, users, v1 scope, features, metrics) — verify it opens with the non-authoritative banner and links to the English PRD.
- [x] 3.2 Write `docs/ar/SRS.ar.md` (brief summary: requirement groups, key NFR targets, error behavior) — verify it opens with the non-authoritative banner and links to the English SRS.

## 4. Verification & Commit

- [x] 4.1 Cross-check every non-negotiable and data-flow rule in `AGENTS.md` §3, §7, §8, §9 against the SRS — verify a written mapping exists (in the PR description or the SRS traceability table) for each rule.
- [x] 4.2 Run `openspec validate add-prd-srs` and confirm it passes — fix any validation error before committing. Evidence recorded in the approval pass on 2026-10-21: `Change 'add-prd-srs' is valid`.
- [x] 4.3 Verify no secrets, real API keys, or user text appear in the four new files — grep the docs for key-like patterns and confirm zero matches.
- [x] 4.4 Commit with `docs: add PRD and SRS for Aref v1` — verify `git log --oneline -1` shows the commit and `git status` is clean. Evidence: commit `561474f` (`docs: add PRD and SRS for Aref v1`), 6 files; `openspec validate add-prd-srs` → `Change 'add-prd-srs' is valid` (exit 0); `git status --short` clean after the commit.

## Workflow follow-up

- Archive the change after the user reviews and approves the PRD/SRS content.
- Verify the archived specs/changes layout after archiving.
