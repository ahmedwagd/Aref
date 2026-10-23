# Proposal

## Why

`AGENTS.md` §2 ranks `docs/design/DESIGN.md` as a source of truth for visual decisions, and §10 fixes what it must contain: the freellmapi dashboard adapted to Aref and to RTL, expressed as derived tokens (colours, spacing, radii, motion). The file does not exist. Every UI-bearing change therefore either invents visual values locally — which is exactly the drift §10 forbids ("do not invent new ones") — or blocks. The first UI change (`add-providers-security`) is already being written and needs the token contract to cite.

## What Changes

- Add `docs/design/DESIGN.md` — the visual baseline: token families (colour, spacing, radius, motion, typography), light-theme values, RTL rules, the layout skeleton (sidebar, cards, status pills, charts), window sizing, and an explicit list of what is deliberately not defined yet.
- Fix the provenance of every token family: the upstream freellmapi UI source it is derived from, and the `AGENTS.md` section that governs the adaptation.
- Record the Aref-specific extensions that the upstream palette does not provide (detector score bands, sidebar geometry, semantic series colours) as labelled proposals with acceptance checks, not as settled values.

No product behavior changes. No requirement is added, modified, or withdrawn, and no code, dependency, IPC contract, or build configuration is touched.

## Capabilities

### New Capabilities

None — this is a documentation-only change (`skip_specs: true`). Visual tokens are design inputs to behavior, not behavior; the UI capabilities they serve get their own spec deltas when implemented.

### Modified Capabilities

None.

## Impact

- **Files:** new `docs/design/DESIGN.md`. Nothing else.
- **Unblocks:** any change that renders UI — first `add-providers-security` (app shell, provider/key cards, status pills, usage surfaces), then the detector and humanizer screens.
- **Precedence:** the new document slots into `AGENTS.md` §2 rank 4, below PRD/SRS and above existing code conventions. It cannot override `AGENTS.md` §3 or §10; where the upstream repository disagrees with §10, §10 wins and the disagreement is recorded in the document itself.
- **Review gate:** this is a design artifact. It is written and validated, then returned for user review before any UI change consumes it.
