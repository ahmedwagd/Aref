# Design

## Context

See `proposal.md` — Why. `AGENTS.md` is the engineering contract and already fixes the product scope, the tech stack, the non-negotiables, and the first build order. Two reference documents are missing and are cited by `AGENTS.md` §2:

- `docs/PRD.md` — product intent.
- `docs/SRS.md` — precise, testable behavior.

The OpenSpec specs directory is empty today; this change is documentation-only (`skip_specs: true`). The content must be derived from `AGENTS.md` and the approved brainstorming decisions without inventing new product behavior.

## Goals / Non-Goals

**Goals:**

- A single authoritative PRD/SRS pair that every future OpenSpec change, PR, and test can cite.
- A stable requirement-ID scheme that survives reordering and rewrites.
- Testable acceptance criteria for every functional requirement, aligned with `AGENTS.md` §7, §8, §9.
- Brief Arabic summaries for stakeholders, explicitly marked non-authoritative.

**Non-Goals:**

- No implementation, scaffolding, or dependency changes.
- No UI mockups or design tokens (that is `docs/design/DESIGN.md`, a separate change).
- No deep technical design of the detector/humanizer internals (OpenSpec design docs per feature will cover that).
- No website/hybrid — roadmap only in the PRD, out of SRS scope.

## Decisions

### 1. Two documents, two audiences

- **PRD** answers *why, for whom, what, and how we measure success*. It contains vision, personas, problem statement, positioning, v1 scope, user journeys, feature overview, success metrics, and non-goals.
- **SRS** answers *exactly how the system must behave*. It contains functional requirements with IDs, non-functional requirements, data model, IPC surface, error behavior, and acceptance criteria.

Alternative considered: one combined document. Rejected because `AGENTS.md` §2 mandates both files and their audiences differ (product review vs. engineering).

### 2. Requirement-ID scheme

Stable, never reused, grouped by capability:

| Range | Area |
|---|---|
| `FR-1xx` | Detection (local engine, scoring, explanation, Deep Scan) |
| `FR-2xx` | Humanization pipeline (chunking, retries, protection, reports) |
| `FR-3xx` | Providers, keys, failover, quotas, local-provider detection |
| `FR-4xx` | Storage, settings, history, migrations |
| `FR-5xx` | UI shell, i18n/RTL, navigation, input validation |
| `NFR-1xx` | Performance and resource use |
| `NFR-2xx` | Security and privacy |
| `NFR-3xx` | Reliability and offline behavior |
| `NFR-4xx` | Accessibility and internationalization |

Each functional requirement is atomic, uses RFC 2119 keywords (`MUST`, `MUST NOT`, `SHOULD`, `MAY`), and carries at least one acceptance criterion written so it can become a test name (`FR-201: rejects empty input`). IDs are assigned once and never renumbered; removed requirements are marked `Withdrawn` instead of deleted.

Alternative considered: OpenSpec spec IDs only. Rejected because PRD/SRS are contract documents cited outside OpenSpec, and `AGENTS.md` already references `FR-xxx` IDs.

### 3. Traceability

- Each PRD feature maps to one or more SRS `FR-xxx` IDs (a feature→requirement table in the PRD).
- Future OpenSpec changes cite the `FR-xxx` IDs they implement; tests cite the same IDs in their names.
- `AGENTS.md` §12 build order (`FR-201..207`, `FR-301..304`) must resolve to real requirements in this SRS.

### 4. Language policy

English is canonical in `docs/PRD.md` and `docs/SRS.md`. `docs/ar/PRD.ar.md` and `docs/ar/SRS.ar.md` are brief summaries (scope, feature list, key requirements) and are explicitly marked as non-authoritative; on any conflict the English text wins.

Alternative considered: full bilingual duplication. Rejected for now — doubles maintenance cost and drift risk before the product ships.

### 5. Content sourcing and change control

- Every requirement traces back to an existing `AGENTS.md` rule or an approved brainstorm decision; anything new is either left out or recorded as an open question in the SRS.
- Conflicts discovered while writing (e.g. key storage in SQLite vs. OS keyring) are resolved in favor of `AGENTS.md` and noted in the PRD decision log.
- After approval, PRD/SRS changes require their own OpenSpec change, matching `AGENTS.md` §2.

### 6. Structure

PRD sections: Summary, Problem, Target Users & Personas, Positioning, v1 Scope, User Journeys, Feature Set, Success Metrics, Non-Goals, Roadmap, Decision Log.

SRS sections: Introduction & Conventions, System Overview, Functional Requirements (by area), Non-Functional Requirements, Data Model, IPC Commands & Events, Error Codes, Acceptance Criteria Index, Open Questions.

## Risks / Trade-offs

- **Docs drift from `AGENTS.md`** → single precedence order in `AGENTS.md` §2; every behavior-changing PR updates both.
- **Over-specification before implementation** → acceptance criteria stay behavior-level; internal algorithms (Deep Scan mechanism, model architecture) remain open questions.
- **Requirement sprawl** → ID ranges capped per area; each FR must be independently testable, otherwise it is split.
- **Stale Arabic summaries** → summaries are short and carry a non-authoritative banner; they are updated in the same PR as the English text.
- **False traceability** (citing IDs that do not exist) → the SRS acceptance-criteria index is the checklist; `openspec validate` plus a grep for `FR-` in code/tests catches dangling references.

## Migration Plan

Not applicable — additive documentation only. Rollback is deleting the four new files; no runtime behavior is affected.
