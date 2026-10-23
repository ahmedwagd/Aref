# Design

## Context

See `proposal.md` — Why. `AGENTS.md` §2 ranks `docs/design/DESIGN.md` fourth in the source-of-truth order and §10 fixes its content and its reference implementation. The document is missing, while the first UI change (`add-providers-security`) is already in flight.

Constraints that shape the approach:

- The styling system is locked to Tailwind CSS v4 via `@tailwindcss/vite`, CSS-first config, no `tailwind.config.js` unless a plugin requires it (`AGENTS.md` §4). Tokens must therefore be expressible as CSS custom properties in `@theme`, not as a JS config object.
- v1 is Windows-only. The macOS-specific visual branches in the upstream source (window vibrancy, traffic-light padding) are not applicable and must not be copied.
- Arabic-first with full RTL (`AGENTS.md` §1, §10). A token set derived from a Latin-only upstream needs an explicit RTL and Arabic-typography position, or it silently bakes LTR assumptions into every screen.
- Upstream is an external, moving repository. Whatever is derived from it must record the file it came from, so a future re-check is possible.

## Goals / Non-Goals

**Goals:**

- One token contract that a UI change can cite instead of inventing values.
- Every token family traceable to a named upstream file and a governing `AGENTS.md` section.
- A layout skeleton expressed as composition rules (surface, measure, radius, density), not as pixel mockups.
- An explicit, short list of what the baseline does *not* define, so absence reads as deferred rather than forgotten.

**Non-Goals:**

- No component library, no per-screen layout, no mockups, no icon set choice.
- No dark-theme commitment (light must stay functional per §10; dark is recorded but not promised).
- No detector or humanizer UI behavior — thresholds, bands and copy belong to their own changes.
- No new dependency, no font package added to the repo.

## Decisions

### 1. Derive from source files, not from screenshots

Tokens are taken from the upstream UI source — `client/src/index.css` (theme block), `client/src/components/token-usage-bar.tsx` (cards, chart, legend), `client/src/components/ui/badge.tsx` (pills, focus ring), `client/src/App.tsx` (shell, header, content measure) — rather than from the README images. Reason: a screenshot yields approximate colours and guessed spacing; the source yields exact `oklch()` values, exact radius multipliers and exact utility classes. Every derived family in `DESIGN.md` cites its file.

Alternative considered: eyeball the hero image. Rejected — it cannot be verified, and "approximately this grey" is the drift §10 forbids.

### 2. Upstream overrides only colour, radius and font; spacing stays Tailwind default

`index.css` overrides `--radius`, the font stacks and the colour tokens. It defines no `--spacing` scale. So the spacing family is Tailwind v4's default 4 px grid, and `DESIGN.md` records it as inherited-with-no-override rather than inventing a scale. The same reasoning applies to motion: upstream defines no duration tokens, but uses `duration-300 ease-in-out` for expand/collapse and `duration-300` for icon rotation, and relies on default `transition-colors` elsewhere. Those observed values become the motion family.

Alternative considered: define a bespoke spacing scale for Aref. Rejected — it would fight Tailwind's defaults for no product gain, and §4 forbids inventing a parallel system.

### 3. The sidebar is an Aref adaptation; the upstream palette already carries it

`AGENTS.md` §10 names "sidebar navigation" as part of the baseline. The upstream repository at `main` renders a horizontal top navbar (`client/src/App.tsx` → `Navbar`, `sticky top-0 z-40 border-b backdrop-blur`), and has no sidebar component (`client/src/components/ui/sidebar.tsx` returns 404).

This is a conflict between the highest-ranked source (`AGENTS.md`) and the referenced implementation. Precedence resolves it: §10 wins, so Aref uses sidebar navigation. The conflict is recorded in `DESIGN.md` rather than smoothed over, because the next reader will fetch the same repository and see a navbar.

Crucially, the sidebar is not invented: `client/src/index.css` already defines a full `--sidebar-*` token set (`--sidebar`, `--sidebar-foreground`, `--sidebar-border`, `--sidebar-accent`, `--sidebar-accent-foreground`, `--sidebar-primary`, `--sidebar-ring`) that the navbar never consumes. Aref consumes those tokens and supplies the geometry the upstream lacks. Geometry values are labelled as proposed in the document and require user approval.

Alternative considered: follow the upstream navbar and treat §10's "sidebar" as loose prose. Rejected — that silently overrides the ranked source on a point it states explicitly.

### 4. Aref extensions are labelled, not absorbed

Three things the product needs have no upstream value: detector score bands (yellow/red per §7.1), sidebar geometry, and a semantic series ramp for charts. Each is introduced as an `--aref-*` token, kept out of the inherited block, labelled with its provenance, and given an acceptance check. Score-band hues stay `[Open]` with a derivation rule anchored on the upstream `--destructive` construction, because choosing a hue is a visual decision the user owns, not a derivation.

Alternative considered: quietly add amber/green to the inherited palette. Rejected — it would make an invented value indistinguishable from a verified one.

### 5. Arabic typography is a stated gap, not a silent fallback

Upstream imports `@fontsource-variable/geist` and `@fontsource-variable/geist-mono` and applies `font-feature-settings: 'ss01','cv11'` to `html`. Geist carries no Arabic coverage. An Arabic-first product cannot inherit that stack unchanged, and the baseline may not pick a face on the user's behalf. `DESIGN.md` therefore splits the typography family into a verified Latin face, an `[Open]` Arabic face with a recommendation and a hard constraint (the stack must never resolve to a Latin-only face; Geist-specific feature settings must not be applied to Arabic runs), and a rule that score and count readouts stay `tabular-nums` / `font-mono` in both languages.

Alternative considered: leave the stack as-is and let `system-ui` fall through. Rejected — on Windows that produces inconsistent Arabic rendering across machines, and it hides a real decision behind a fallback.

### 6. The document is the artifact; the change is its paper trail

`DESIGN.md` is the deliverable; `proposal.md`, this file and `tasks.md` exist so the work is reviewable and traceable under §12. No `specs/` delta is created (`skip_specs: true`) because no behavior changes — a visual token is an input to behavior, and the behaviors it serves will get their own deltas.

## Risks / Trade-offs

- **Upstream drift** → every family cites a file path; the document records the commit-agnostic path and the date it was read, so a re-check is a diff, not an archaeology exercise.
- **Token sprawl** → the inherited block is quoted as-is; Aref additions are namespaced `--aref-*` and capped at three families.
- **Score-band hues unapproved** → the bands are specified as *semantics* (order, label, icon, threshold) with hue deferred; no screen may rely on hue alone, which also satisfies the accessibility baseline in §10.
- **Arabic face undecided** → treated as a blocker for the first screen that ships Arabic type, surfaced in the open-questions list rather than discovered during implementation.
- **Over-specification** → the baseline stops at composition rules and tokens; no component API, no per-screen grid, so UI changes keep design freedom where it is cheap to keep.
- **A reader trusting the upstream navbar** → the conflict is called out in the layout section with both quotes, so the deviation is visible at the point of use.

## Migration Plan

Not applicable — one additive documentation file. Rollback is deleting `docs/design/DESIGN.md`; nothing references it at runtime, and no requirement changes.
