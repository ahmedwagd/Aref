# Proposal

## Why

The repository has an engineering contract (`AGENTS.md`) but no product or software requirements documents. `AGENTS.md` already references `docs/PRD.md`, `docs/SRS.md`, and requirement IDs (e.g. `FR-201`), and the planned build order depends on those IDs — yet none of them exist. Before any code is written, Aref needs one authoritative, traceable definition of what v1 must do, so that every OpenSpec change and every test can cite a requirement.

## What Changes

- Add `docs/PRD.md` — product requirements: vision, target users, problem, positioning, v1 scope, user journeys, feature list, success metrics, explicit non-goals.
- Add `docs/SRS.md` — software requirements: functional requirements with stable IDs (`FR-1xx` detector, `FR-2xx` humanizer, `FR-3xx` providers & keys, `FR-4xx` storage/settings/history, `FR-5xx` UI & i18n), non-functional requirements (`NFR-xxx`), data model, interfaces, and acceptance criteria per requirement.
- Add brief Arabic summaries in `docs/ar/PRD.ar.md` and `docs/ar/SRS.ar.md` (summaries only; the English documents remain authoritative).
- Establish the requirement-ID scheme that future OpenSpec changes and test names must cite.

No product behavior is introduced by this change: the documents describe the already-approved product decisions from `AGENTS.md` and the brainstorming session.

## Capabilities

### New Capabilities

None — this is a documentation-only change (`skip_specs: true`). The product capabilities it specifies will get their own OpenSpec spec deltas when they are implemented.

### Modified Capabilities

None.

## Impact

- **Files:** new `docs/PRD.md`, `docs/SRS.md`, `docs/ar/PRD.ar.md`, `docs/ar/SRS.ar.md`. No source code, dependencies, IPC contracts, or build configuration are touched.
- **Unblocks:** the first implementation change (project skeleton), which will cite the requirement IDs defined here.
- **Precedence:** the new documents slot into the source-of-truth order defined in `AGENTS.md` §2, below OpenSpec and above design docs.
