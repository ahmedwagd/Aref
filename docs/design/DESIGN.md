# Aref — Visual Baseline

**Status:** approved by the product owner on 2026-10-22, as written, under the OpenSpec change `add-design-baseline`. Five decisions are binding as of that date: `D-01`, `D-02`, `D-03` and `D-08` (§10), joining `D-06`, approved the same day. `D-04`, `D-05` and `D-09` remain `[Proposed]`, and `D-07` and the amber value inside `D-03` remain `[Open]`; those are not approved and must not be presented as settled. A UI change may cite the approved tokens and the approved layout grammar from this document. A value it does not define is still a gap to raise, not a local constant to add.

**Rank:** `AGENTS.md` §2, position 4 — below `AGENTS.md`, OpenSpec, `docs/PRD.md` and `docs/SRS.md`; above existing code conventions. Nothing here may override `AGENTS.md` §3 (non-negotiables) or §10 (frontend rules). Where this document and `AGENTS.md` disagree, `AGENTS.md` wins and this document is wrong.

**What this is:** the token contract and layout grammar that UI changes cite instead of inventing values. **What this is not:** a component library, a set of mockups, or a per-screen design.

## 0. How to read the provenance labels

Every value carries one:

| Label | Meaning |
|---|---|
| `[Inherited]` | Taken verbatim from the upstream freellmapi UI source. Value is exact, not approximated. |
| `[Derived]` | Follows from an upstream file by a stated rule (e.g. "upstream overrides no spacing scale, so Tailwind's default applies"). |
| `[Adapted]` | An upstream pattern re-pointed at an Aref need. The shape is upstream; the target is ours. |
| `[Proposed]` | An Aref decision introduced by this document. **Requires user approval.** |
| `[Decision]` | An Aref decision the product owner has approved, with its date and approver recorded in §10. Not open to reinterpretation during implementation. |
| `[Open]` | Deliberately unresolved. Do not fill it in during implementation; raise it. |

## 1. Sources

Upstream: **`tashfeenahmed/freellmapi`** (MIT), read from branch `main` on 2026-10-22. Files used:

| File | What it establishes |
|---|---|
| `client/src/index.css` | The theme block: colour tokens, radius scale, font stacks, base-layer typography. The single source for all inherited values. |
| `client/src/components/token-usage-bar.tsx` | Card surface and padding, chart track and legend, numeric typography. |
| `client/src/components/ui/badge.tsx` | Status pills, their variants, and the focus ring. |
| `client/src/App.tsx` | App shell, header band, content measure and gutters, navigation item states. |
| `client/src/theme.tsx` | Light/dark/system resolution and `color-scheme`. |

Governing sections of `AGENTS.md`: §3 (non-negotiables), §4 (stack), §7.1 (score thresholds 40/60), §10 (frontend rules, RTL, window sizing, accessibility baseline).

Two upstream facts matter up front, because both are places where copying would be wrong:

1. **The upstream dashboard has no sidebar.** `client/src/App.tsx` renders a horizontal top navbar (`sticky top-0 z-40 border-b backdrop-blur`), and `client/src/components/ui/sidebar.tsx` does not exist (HTTP 404). `AGENTS.md` §10 nonetheless names "sidebar navigation" as part of the baseline. §10 wins; see §7.2.
2. **The upstream stack is Latin-only.** It imports `@fontsource-variable/geist` and `@fontsource-variable/geist-mono`. Aref is Arabic-first, so the typography family cannot be inherited unchanged; the face that replaces it is decided in §3.1.

## 2. Colour

### 2.1 Light theme — inherited block

`[Inherited]` verbatim from `client/src/index.css` `:root`. `AGENTS.md` §10 requires the light theme to stay functional, so this block is the required theme, not a fallback. `[Decision]` (`D-01`, approved by the product owner 2026-10-22): these values are Aref's theme exactly as written; no value is substituted, rounded, or re-derived.

| Token | Value | Role |
|---|---|---|
| `--background` | `oklch(0.995 0 0)` | App surface |
| `--foreground` | `oklch(0.18 0 0)` | Primary text |
| `--card` | `oklch(1 0 0)` | Card surface (one step above background) |
| `--card-foreground` | `oklch(0.18 0 0)` | Text on cards |
| `--popover` | `oklch(1 0 0)` | Menus, dialogs |
| `--popover-foreground` | `oklch(0.18 0 0)` | Text in popovers |
| `--primary` | `oklch(0.22 0 0)` | Primary action fill |
| `--primary-foreground` | `oklch(0.98 0 0)` | Text on primary |
| `--secondary` | `oklch(0.97 0 0)` | Secondary surface |
| `--secondary-foreground` | `oklch(0.22 0 0)` | Text on secondary |
| `--muted` | `oklch(0.97 0 0)` | Inert surfaces, chart tracks |
| `--muted-foreground` | `oklch(0.52 0 0)` | Secondary text, metadata |
| `--accent` | `oklch(0.96 0 0)` | Hover / active fill |
| `--accent-foreground` | `oklch(0.22 0 0)` | Text on accent |
| `--destructive` | `oklch(0.577 0.245 27.325)` | Destructive action, **and the red score band** |
| `--border` | `oklch(0.92 0 0)` | Hairlines, card borders |
| `--input` | `oklch(0.92 0 0)` | Input borders |
| `--ring` | `oklch(0.55 0 0)` | Focus ring |
| `--chart-1` … `--chart-5` | `oklch(0.28 0 0)`, `0.45`, `0.6`, `0.75`, `0.87` | Neutral series ramp, dark → light |
| `--sidebar` | `oklch(0.98 0 0)` | Sidebar surface |
| `--sidebar-foreground` | `oklch(0.18 0 0)` | Sidebar text |
| `--sidebar-primary` | `oklch(0.22 0 0)` | Sidebar primary action |
| `--sidebar-primary-foreground` | `oklch(0.98 0 0)` | Text on sidebar primary |
| `--sidebar-accent` | `oklch(0.96 0 0)` | Sidebar active / hover fill |
| `--sidebar-accent-foreground` | `oklch(0.22 0 0)` | Text on sidebar accent |
| `--sidebar-border` | `oklch(0.92 0 0)` | Sidebar edge |
| `--sidebar-ring` | `oklch(0.55 0 0)` | Sidebar focus ring |

Observations that constrain usage:

- The palette is **achromatic except `--destructive`**. There is no amber, no green, no brand hue. Any coloured element in Aref is therefore either the destructive red or an Aref extension (§2.3). `[Derived]`
- `--background` and `--card` differ by ~0.005 in lightness, so a card reads as elevated through its `--border` hairline, not through fill contrast. Cards must keep their border. `[Derived]`
- The `--sidebar-*` set exists upstream but no upstream component consumes it (the navbar does not). Aref consumes it; see §7.2. `[Fact]`

### 2.2 Dark theme — recorded, not promised

`[Inherited]` the `.dark` block exists upstream with a full parallel set (`--background: oklch(0.135 0 0)`, `--card: oklch(0.175 0 0)`, `--border: oklch(1 0 0 / 10%)`, and so on), and `client/src/theme.tsx` resolves `light | dark | system` with `color-scheme` and persistence.

Aref v1 **ships light only**. `[Decision]` — `D-08`, approved by the product owner 2026-10-22. Dark is neither required by `AGENTS.md` nor promised by this baseline, and v1 exposes no theme switch: there is no second theme to switch to. The upstream `.dark` values are recorded only so that a later dark-theme change starts from them instead of re-deriving.

### 2.3 Aref extensions — score bands

The detector produces a 0–100 score with yellow ≥ 40 and red ≥ 60 (`AGENTS.md` §7.1). The inherited palette can express only the red band. This is the **only** place where Aref must add colour.

| Token | Value | Band | Status |
|---|---|---|---|
| `--aref-score-clear` | `var(--foreground)` | `< 40` | `[Decision]` (`D-03`, 2026-10-22) — no new hue; the calm state stays neutral |
| `--aref-score-yellow` | `[Open]` | `≥ 40` | `[Open]` — see derivation rule below |
| `--aref-score-red` | `var(--destructive)` | `≥ 60` | `[Adapted]` — the inherited red, reused as the top band |

Derivation rule for `--aref-score-yellow` `[Derived]`: same construction as `--destructive` — hold lightness and chroma within ±0.06 of `0.577 / 0.245`, set hue in the 80–95° range, then verify. Acceptance check: ≥ 4.5:1 against `--background` when used as text, ≥ 3:1 when used as a graphical object (bar, dot, border).

Hard constraint, independent of the value chosen `[Decision]` (`D-03`, approved by the product owner 2026-10-22): **a band is never conveyed by hue alone.** Every score surface shows the number, and every banded surface carries a text label and a distinct icon or shape. This keeps the bands legible in greyscale, under colour-vision deficiency, and in the RTL/print cases, and satisfies the accessibility baseline in `AGENTS.md` §10. Consequence: the exact hue is cosmetic, which is why it can stay `[Open]` without blocking layout work.

## 3. Typography

| Family | Upstream source | Governing section | Status |
|---|---|---|---|
| Latin sans + mono, feature settings | `client/src/index.css` | §4, §10 | `[Inherited]` |
| Arabic face | — (upstream has none) | §1, §10, `D-06` | `[Decision]` — IBM Plex Sans Arabic (§3.1) |

`[Inherited]` stacks, upstream:

```css
--font-heading: var(--font-sans);
--font-sans: 'Geist Variable', ui-sans-serif, system-ui, -apple-system, sans-serif;
--font-mono: 'Geist Mono Variable', ui-monospace, SFMono-Regular, Menlo, monospace;
```

`[Decision]` Aref changes exactly one declaration: `--font-sans` gains a leading Arabic entry (§3.1). `--font-heading` keeps aliasing it, and `--font-mono` is untouched.

`[Inherited]` base layer: `html` gets `font-sans antialiased`, `font-feature-settings: 'ss01', 'cv11'`, `text-rendering: optimizeLegibility`; `body` adds `'tnum'`; `code, pre, kbd, samp` use `--font-mono` with `'ss02', 'ss03'`; `.tabular-nums` sets `font-variant-numeric: tabular-nums`.

Rules that follow:

- **Numeric readouts are tabular.** Scores, percentages, counts and timings use `.tabular-nums`, and inline figures inside prose use `font-mono` — the upstream pattern (`<span className="font-mono text-muted-foreground">`) — so digits do not jitter as they update. `[Inherited]`
- **`'ss01'` and `'cv11'` are Geist-specific** stylistic sets. They must not be applied to Arabic runs, where they are meaningless at best and can alter shaping at worst. `[Derived]` Because the inherited base layer sets them on `html`, they must be scoped to a Latin-only selector — or dropped from the global rule — now that the Arabic face leads the stack (§3.1); a global declaration reaches Arabic text by default.
- **`'tnum'` is applied globally upstream.** For Arabic this is a decision, not an inheritance: whether Arabic body text keeps tabular figures is `[Open]`, and it is **not tracked as a decision row** — §10 has no row for it (`D-07` covers which digits scores use, not tabular figures in Arabic body text) and §9 does not list it as deferred — so it stays a question to raise under the `[Open]` rule in §0, not a value this document settles. Western digits `0–9` are recommended for scores in both locales, because `AGENTS.md` §7.1 states the thresholds as 40 and 60.

### 3.1 Arabic face — `[Decision]` (`D-06`)

**Approved by the product owner on 2026-10-22.** The gap the previous draft recorded — an Arabic-first product cannot ship a Latin-only stack, and the baseline may not choose a face on the user's behalf — is closed here, and §9 no longer lists it as deferred.

**Face:** **IBM Plex Sans Arabic**, the Arabic companion of IBM Plex Sans, designed by Mike Abbink, Bold Monday, Khajag Apelian and Wael Morcos. `[Fact]`

**Licence:** SIL Open Font License 1.1, `Copyright © 2017 IBM Corp. with Reserved Font Name "Plex"`. Verified on 2026-10-22 against three public sources: `IBM/plex` `LICENSE.txt`, the Google Fonts family metadata (`license: "OFL"`), and the `@fontsource/ibm-plex-sans-arabic` package metadata (`OFL-1.1`). OFL permits bundling and redistributing the font inside an application provided the copyright notice and licence travel with it, and forbids selling the font by itself; neither restriction affects Aref. `[Fact]`

The Reserved Font Name has one consequence `[Derived]`: an unmodified copy may ship under the name "IBM Plex Sans Arabic", but a subset or otherwise modified file becomes a Modified Version that may not use the "Plex" name (OFL condition 3). **v1 therefore ships the files unmodified.**

**Weights shipped in v1:** regular `400`, medium `500`, semibold `600` — exactly the three the baseline uses (constraint 3 below). `[Decision]` The family is distributed as seven static weights, Thin 100 to Bold 700, with no variable axis in the Google Fonts metadata as read on 2026-10-22, so this is three static files rather than one variable file. `[Fact]` Bold `700` is not shipped: nothing in this baseline sets it, and a screen needing it has left the type scale.

**Bundling, and the no-network consequence:** the three files ship inside the app bundle as local assets. No CDN, no `@import` from a font host, no runtime download. **Consequence: no font is ever fetched over the network — not on first launch, not after an update, and not in a fully offline session.** That is not a performance preference; it follows from `AGENTS.md` §3.1 (detection works with zero network access) and §3.2 (no telemetry, no remote fetch), which a runtime webfont request would violate. `[Decision]`

Delivery constraint `[Decision]`: the `@ibm/plex-sans-arabic` npm package is **excluded**. Its README states that the package "uses IBM Telemetry to collect de-identified and anonymized metrics data" and that installing it constitutes agreement to that collection — incompatible with `AGENTS.md` §3.2. The files must arrive by a route that carries no telemetry: vendored from the IBM release archive, or through a plain files-and-CSS package such as `@fontsource/ibm-plex-sans-arabic` (OFL-1.1, no telemetry, the same mechanism the inherited `@fontsource-variable/geist` already uses). Either route is a dependency decision under `AGENTS.md` §4 and needs its purpose, licence and maintenance statement in the PR.

**Stack:**

```css
--font-sans: 'IBM Plex Sans Arabic', 'Geist Variable', ui-sans-serif, system-ui, -apple-system, sans-serif;
```

`[Decision]` The Arabic face leads; Geist follows and keeps Latin and Western numerals. `--font-heading` still aliases `--font-sans`; `--font-mono` is unchanged.

`[Derived]` **The ordering alone does not deliver that split, and this is the trap in the decision.** IBM Plex Sans Arabic covers `arabic`, `latin`, `latin-ext` and `cyrillic-ext` (Google Fonts `subsets`, `primary_script: "Arab"`), so as the first family in the list it also claims Latin letters and Western digits, and Geist is never reached. The Arabic face is therefore declared with a `unicode-range` restricted to the Arabic blocks (Arabic, Arabic Supplement, Arabic Extended-A, Arabic Presentation Forms-A and -B), so Arabic characters resolve to it while Latin characters and `0–9` fall through to Geist. A stack with the ordering but without that scoping silently replaces the Latin face this document inherits, and is a defect rather than a simplification. `[Fact]` on the coverage, `[Derived]` on the remedy.

**Fallback chain:** `ui-sans-serif, system-ui, -apple-system, sans-serif` — the inherited tail, unchanged. `[Decision]` It must never resolve an Arabic run to a Latin-only face (constraint 1): a silent fallback is a defect, not a degradation. Because the files ship with the app, a missing face is a packaging or checksum failure caught before release, never a runtime network fallback.

**Interaction with `D-07`:** this decision does not settle the digits question. With the scoping above, Western digits render from Geist and Arabic-Indic digits from IBM Plex Sans Arabic, so both answers to `D-07` stay available. `[Derived]`

**Out of scope:** `--font-mono` stays Geist Mono; Arabic has no monospace role in v1. `[Decision]`

The constraints that governed the choice, now binding as acceptance criteria:

1. The stack must never resolve to a face without Arabic coverage. A silent fall to a Latin-only family is a defect, not a degradation.
2. It must be available offline on Windows — bundled with the app, not fetched — because detection is offline by default (`AGENTS.md` §3.1) and the UI must not depend on a network fetch for its own type.
3. It must support the weights the baseline uses (regular / medium / semibold) and match Geist's optical size closely enough that a mixed Arabic–English paragraph does not visibly change rhythm mid-line. **Met on the first half:** 400 / 500 / 600 ship, and the `unicode-range` split keeps the two faces adjacent on the same line. Whether the rhythm actually holds is a review check on the first Arabic screen, not an assumption made here.
4. Adding the font files is a dependency decision under `AGENTS.md` §4 and needs its purpose, licence and maintenance status stated in the PR.

## 4. Spacing

`[Derived]` `client/src/index.css` overrides colour, radius and fonts, and defines **no** spacing scale. Aref therefore uses Tailwind CSS v4's default spacing unit, `--spacing: 0.25rem` (4 px), on a 4 px grid. No custom spacing tokens are introduced.

Observed upstream usage, to be treated as the density reference rather than as hard tokens `[Inherited]`:

| Context | Utility | px |
|---|---|---|
| Card padding | `p-5` | 20 |
| Page gutters | `px-6` | 24 |
| Page vertical padding | `py-8` | 32 |
| Section gap | `gap-6` | 24 |
| Inline cluster gap | `gap-2` | 8 |
| Legend column gap | `gap-x-5`, `gap-y-1.5` | 20 / 6 |
| Header-to-body | `mb-3`, `mt-3` | 12 |
| Chart-to-legend | `mt-4` | 16 |

`[Proposed]` Aref-specific density rules: cards are the primary grouping unit, so spacing between sibling cards is 24 px (`gap-6`) and spacing inside a card is 20 px (`p-5`) — the upstream ratio, kept so a screen composed of cards reads as one system.

## 5. Radius

`[Inherited]` from `client/src/index.css`, with the scale documented in its own comment. Base `--radius: 0.5rem`, all steps are multipliers of it:

```css
--radius-sm: calc(var(--radius) * 0.5);   /*  4px */
--radius-md: calc(var(--radius) * 0.75);  /*  6px */
--radius-lg: var(--radius);               /*  8px */
--radius-xl: calc(var(--radius) * 1.5);   /* 12px */
--radius-2xl: calc(var(--radius) * 2);    /* 16px */
--radius-3xl: calc(var(--radius) * 3);    /* 24px */
--radius-4xl: calc(var(--radius) * 4);    /* 32px */
```

Role assignment `[Inherited]`, as used by the upstream components:

| Role | Token | px |
|---|---|---|
| Buttons, inputs | `lg` | 8 |
| Tables, lists, menus | `2xl` | 16 |
| **Cards** | `3xl` | 24 |
| **Status pills** | `4xl` (fully rounded at pill height) | 32 |
| Chart legend swatch | `sm` | 4 |
| Chart track | `full` | — |

Because every step is a multiple of one base, changing `--radius` rescales the whole UI coherently. That is the reason not to add ad-hoc radii: a single off-scale value breaks the property.

## 6. Motion

`[Derived]` upstream defines no duration or easing tokens. Observed values in `client/src/components/token-usage-bar.tsx` and `client/src/components/ui/badge.tsx` are adopted as the family:

| Purpose | Value | Evidence |
|---|---|---|
| Colour / hover transitions | `transition-colors`, default duration (150 ms) | nav items, legend button, badge |
| Expand / collapse | `duration-300 ease-in-out` on `max-height` | legend disclosure |
| Directional affordance | `duration-300` on `transform` (`rotate-180` on chevron) | legend disclosure |
| Opacity affordance | `transition-opacity` (150 ms) | brand link hover |
| Enter / exit | provided by `tw-animate-css`, imported in `index.css` | `@import "tw-animate-css"` |

`[Proposed]` rules:

- Nothing animates longer than 300 ms. Progress feedback for a long-running job is a progress indicator, not an animation.
- `prefers-reduced-motion: reduce` disables transform and size transitions; colour and opacity changes may remain. Required by the accessibility baseline in `AGENTS.md` §10.
- No motion may imply progress that has not happened. Humanizer progress reflects real per-attempt events (`AGENTS.md` §7.2 step 8), never an indeterminate shimmer over a real counter.

## 7. Layout skeleton

### 7.1 App shell

`[Inherited]` from `client/src/App.tsx`:

- Shell is a column: `flex flex-col min-h-screen bg-background`.
- Content column: `mx-auto w-full max-w-6xl px-6 py-8` — a 72 rem measure, centred, with 24 px gutters. The `w-full` is load-bearing upstream (flex cross-axis sizing); keep it.
- Header band: `sticky top-0 z-40 border-b backdrop-blur bg-background/80`.
- Full-bleed routes opt out of the measure and fill the remaining height. Upstream does this only for its playground; Aref's analogue is the Humanize workspace. `[Adapted]`

Windows note: the upstream header has a macOS-only branch (`bg-background/45` over native vibrancy, `pl-20` for traffic lights). Aref is Windows-only (`AGENTS.md` §3.8), so that branch does not exist here and must not be ported — the 80 % opaque wash is the only header treatment.

### 7.2 Sidebar — the documented deviation

`AGENTS.md` §10 states the baseline includes "sidebar navigation". The upstream repository renders a top navbar and has no sidebar component. The two sources disagree; `AGENTS.md` §2 ranks `AGENTS.md` above the reference implementation, so **Aref uses a sidebar**, and the disagreement is recorded here rather than resolved silently.

The sidebar is not invented from nothing: its colours are the inherited `--sidebar-*` set (§2.1), which upstream defines but never uses.

`[Decision]` geometry and treatment (`D-02`, approved by the product owner 2026-10-22):

| Property | Value | Note |
|---|---|---|
| Width | `15rem` (240 px), fixed | At the 1000 px minimum window it leaves ~712 px of content after gutters — enough for a two-pane detector view. Collapsing is out of scope for v1. |
| Side | `inline-start` | Right in `ar`, left in `en`. Never `left`. |
| Surface | `bg-sidebar`, `text-sidebar-foreground` | |
| Edge | `border-inline-end border-sidebar-border` | Logical, so the hairline follows the writing direction |
| Nav item | 40 px tall, `rounded-lg`, `px-3`, icon + label, `gap-3` | |
| Nav item active | `bg-sidebar-accent text-sidebar-accent-foreground` | Upstream's navbar marks active state with a 1 px underline; a sidebar needs a fill, and `--sidebar-accent` is the token designed for it |
| Nav item hover | `bg-sidebar-accent/60`, `transition-colors` | |
| Focus | `ring-[3px] ring-sidebar-ring/50` | Matches the pill focus ring |

### 7.3 Cards

`[Inherited]` from `token-usage-bar.tsx`: `rounded-3xl border bg-card p-5`.

`[Proposed]` composition rules:

- Card title: `text-sm font-medium`; card meta: `text-xs text-muted-foreground`.
- A card never nests a card. Grouping inside a card is done with spacing and `--border` dividers.
- Cards are the unit that provider/key management is built from (`AGENTS.md` §10: "card-based provider/key management"): one card per provider, one row per key, status pill in the card header.

### 7.4 Status pills

`[Inherited]` from `components/ui/badge.tsx`: `inline-flex h-5 w-fit items-center gap-1 rounded-4xl border border-transparent px-2 py-0.5 text-xs font-medium whitespace-nowrap`.

Variants as defined upstream: `default` (`bg-primary text-primary-foreground`), `secondary` (`bg-secondary text-secondary-foreground`), `destructive` (`bg-destructive/10 text-destructive`), `outline` (`border-border text-foreground`), `ghost`, `link`.

`[Proposed]` mapping to Aref states:

| State | Variant |
|---|---|
| Provider healthy / key valid | `outline` with a neutral dot |
| Key invalid (401) | `destructive` |
| Rate limited (429) | `secondary` with a countdown label |
| Local provider reachable | `outline` with a filled dot |
| Running / in progress | `secondary` |

Pills always carry text; the dot is decoration, never the only signal. This is the same rule as §2.3.

### 7.5 Charts

`[Inherited]` grammar from `token-usage-bar.tsx`:

- Track: `flex h-2.5 rounded-full overflow-hidden bg-muted` — 10 px tall, fully rounded, muted.
- Segments: inline `backgroundColor` per series, widths as percentages of a total.
- Legend: `grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-x-5 gap-y-1.5 text-xs tabular-nums`, each entry a `size-2 rounded-sm` swatch, a truncated label, and an end-aligned figure.
- Figures in the legend use `font-mono text-muted-foreground`.

`[Adapted]` for Aref: neutral series use `--chart-1` … `--chart-5`; alert series use the score bands (§2.3). **Which data is charted is not a visual decision** and is not specified here — it belongs to the feature change that renders it. What this baseline fixes is only the grammar: a single stacked or segmented track, a flat legend beneath it, tabular figures, and no provider-coloured chaos (Aref has one subject, not thirty providers).

### 7.6 RTL and bidi

Arabic is the primary quality bar (`AGENTS.md` §1), not a translation target, so direction is a first-class rule rather than a stylesheet variant.

| Rule | Detail | Source |
|---|---|---|
| Direction lives on the root | `dir="rtl"` for `ar`, `dir="ltr"` for `en`, set on `<html>` from the active locale. No component sets its own direction except `dir="auto"` for unknown content. | §10 + `[Derived]` |
| Logical properties only | `ms-`/`me-`, `ps-`/`pe-`, `start-`/`end-`, `border-inline-*`, `inset-inline-*`, `rounded-s-*`/`rounded-e-*`, `text-start`/`text-end`. No `left`, `right`, `ml-`, `mr-`, `pl-`, `pr-`, `text-left`, `text-right`. | §10 |
| Sidebar side | `inline-start`: right in `ar`, left in `en`. Its hairline is `border-inline-end`. | §7.2 |
| Icon direction | Mirror glyphs that mean *along the reading direction* (forward/back chevrons, next/previous, indent/outdent, send). Never mirror glyphs that mean *a thing* (search, play, check, plus, brand and provider marks). Vertical glyphs (expand/collapse chevrons) never mirror in either direction. | `[Proposed]` |
| Chart direction | The track fills from `inline-start`; the legend swatch precedes its label in DOM order, so it lands on the correct side without a direction branch; figures align `end`. | `[Derived]` from §7.5 |
| Unknown-direction content | Any user-supplied or mixed-script string renders inside `dir="auto"`. A number inside an RTL sentence gets its own `dir="ltr"` element rather than forcing the sentence. | §10 |
| Script-aware leading | Line-height is set per script, not globally. Starting values `[Proposed]`: Latin 1.5, Arabic 1.75 — to be confirmed against IBM Plex Sans Arabic (§3.1), the face now chosen, since Arabic ascenders and diacritics need more leading at the same size. | `[Proposed]` |

Upstream is only *partially* logical, and this is a place not to copy it: `client/src/App.tsx` uses `ms-10` and `ms-auto` (logical) but also `pl-20` for macOS traffic lights, and `client/src/components/ui/badge.tsx` uses `pr-1.5`/`pl-1.5`. `[Fact]` Aref ports neither.

Verification is part of done, not a later pass: every screen is checked in both directions for sidebar side, focus-ring clipping, ellipsis side, scrollbar side, dropdown and popover alignment, and toast placement (`AGENTS.md` §10, §12).

## 8. Window sizing

`[Inherited]` from `AGENTS.md` §10:

| Property | Value |
|---|---|
| Default | 1240 × 820 |
| Minimum | 1000 × 680 |
| Position | centred on first launch |
| Resizable | yes, above the minimum |

Consequences for layout `[Derived]`:

- The 72 rem (`max-w-6xl`) content cap is never reached at the default width, so the cap only matters on large monitors; the layout must look complete at 1240 px, not merely survive it.
- At the 1000 px minimum with a 240 px sidebar and 48 px of gutters, the content column has ~712 px. Any two-pane view must remain usable at that width; anything that needs more must scroll or stack, not clip.
- Nothing may depend on a hover state to be discoverable, since a resized or maximised window is still the same target.

## 9. Deliberately not defined

Recorded so that absence reads as deferred, not forgotten. Each of these is a decision for a later change, and inventing one now would be the drift `AGENTS.md` §10 forbids:

- **Score-band hue** (§2.3) — cosmetic, non-blocking, because bands never rely on hue alone.
- **Icon set** — upstream uses `lucide-react`; Aref has not chosen, and choosing one is a dependency decision under §4.
- **Empty, loading and error-state visuals** — `AGENTS.md` §8 and §9 require every failure to have a designed state; what that state *looks like* is defined with the screen that owns it, using the card/pill/typography grammar above.
- **Window chrome** — native title bar versus custom, and how the sidebar meets it.
- **Illustrations, logo and brand marks** — no asset exists and none is invented here.
- **Per-screen layouts and component APIs** — this is a baseline, not a specification.

Two entries an earlier draft carried have left this list because the product owner decided them on 2026-10-22: the Arabic typeface (`D-06`, §3.1) and dark-theme scope (`D-08`, §2.2). The six above are what remains deferred.

## 10. Decision log

Decisions in this document. Five were approved by the product owner on 2026-10-22 — `D-01`, `D-02`, `D-03`, `D-06` and `D-08` — and are recorded as `[Decision]` with their date and approver. The rest are not approved: a `[Proposed]` row becomes `[Decision]` only when the product owner says so, with a date, and an `[Open]` row has no position at all.

| # | Decision | Status | Owner |
|---|---|---|---|
| D-01 | Adopt the upstream light palette verbatim as Aref's required theme | `[Decision]` — approved 2026-10-22 | product owner |
| D-02 | Sidebar navigation with inherited `--sidebar-*` colours and the geometry in §7.2 | `[Decision]` — approved 2026-10-22 | product owner |
| D-03 | Score bands: neutral / amber / `--destructive`, never hue-only | `[Decision]` — approved 2026-10-22; amber value `[Open]` | product owner |
| D-04 | Spacing = Tailwind v4 default grid, no custom scale | `[Proposed]` | product owner |
| D-05 | Motion capped at 300 ms, reduced-motion honoured | `[Proposed]` | product owner |
| D-06 | Arabic face: **IBM Plex Sans Arabic** (OFL 1.1), shipped unmodified as local files in weights 400/500/600, leading `--font-sans` ahead of Geist with a `unicode-range` scoping so Latin and Western digits stay Geist; no runtime font fetch (§3.1) | `[Decision]` — approved 2026-10-22 | product owner |
| D-07 | Western digits for scores in both locales | `[Open]` — recommendation only | product owner |
| D-08 | v1 ships light theme; dark deferred | `[Decision]` — approved 2026-10-22 | product owner |
| D-09 | RTL icon-mirroring rule and script-aware leading (§7.6) | `[Proposed]` | product owner |

## 11. Change control

This document is approved (2026-10-22). From here it changes only through its own OpenSpec change, like `docs/PRD.md` and `docs/SRS.md` (`AGENTS.md` §2). A UI change that needs a value this document does not define must either use an existing token or raise the gap — not add a local constant.

Re-checking upstream: the derivation above is tied to the files listed in §1 as read on 2026-10-22. If upstream changes, re-read those files and diff against §2, §5 and §7 before assuming this document is stale.
