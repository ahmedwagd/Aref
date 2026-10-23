# Tasks

Every task is verifiable on its own. Checks are binary: a section either contains the named content or it does not.

## 1. Proposal and method

- [x] 1.1 Write `proposal.md` with Why, What Changes, Capabilities (none), Impact — verify all four headings exist and the Capabilities section states `skip_specs: true` and "None" for both new and modified capabilities.
- [x] 1.2 Write `design.md` recording the six sourcing decisions and the Risks section — verify each decision names the alternative it rejected, and the sidebar conflict (§10 vs. upstream navbar) is stated as resolved by precedence rather than smoothed.
- [x] 1.3 Set `skip_specs: true` in `.openspec.yaml` — verify `openspec status --change add-design-baseline --json` reports the specs artifact as `skipped` and creates no `specs/` directory.

## 2. Sources and provenance

- [x] 2.1 Record the upstream repository, branch and read date, and list every file used — verify each file in the list was actually fetched in this session and that no value in the document cites a file absent from the list.
- [x] 2.2 Define the provenance labels (`[Inherited]`, `[Derived]`, `[Adapted]`, `[Proposed]`, `[Open]`) — verify the label table exists before the first token table, and that every token table row carries exactly one label or is covered by the section's label.
- [x] 2.3 State the two places where copying upstream would be wrong (no sidebar, Latin-only fonts) — verify both are stated in the Sources section, not only in the sections that fix them.

## 3. Token families

- [x] 3.1 Colour: quote the full light-theme `:root` block with exact `oklch()` values — verify every token name in the upstream block appears, and no value is rounded or approximated.
- [x] 3.2 Colour: record the dark block as inherited-but-not-promised — verify the section says v1 ships light only (`D-08`, approved 2026-10-22), exposes no theme switch, and keeps the `.dark` values only as the starting point for a later dark-theme change.
- [x] 3.3 Colour: define the score bands (`AGENTS.md` §7.1: yellow ≥ 40, red ≥ 60) — verify `--aref-score-red` reuses `--destructive`, the amber value is `[Open]` with a derivation rule and a contrast acceptance check, and a never-hue-alone constraint is stated.
- [x] 3.4 Typography: quote the inherited stacks and base-layer feature settings — verify the Arabic face is recorded as `[Decision]` (IBM Plex Sans Arabic, `D-06`, approved 2026-10-22) with the constraints it must meet, and that Geist-specific `'ss01'`/`'cv11'` are explicitly excluded from Arabic runs.
- [x] 3.5 Spacing: record that upstream defines no spacing scale, so the Tailwind v4 4 px grid applies — verify the claim names the file it was checked against and the observed density values are labelled as reference, not tokens.
- [x] 3.6 Radius: quote the seven-step scale and its multipliers — verify each step's px value matches `--radius: 0.5rem` × its multiplier, and each role assignment cites the component that uses it.
- [x] 3.7 Motion: record the observed durations and easings — verify every duration cites the upstream expression it came from, and the reduced-motion rule is present.

## 4. Layout, RTL and sizing

- [x] 4.1 Layout: record the shell, header band and content measure — verify the content measure is `max-w-6xl` with `px-6 py-8`, and the macOS-only header branch is explicitly excluded because v1 is Windows-only.
- [x] 4.2 Layout: record the sidebar as a documented deviation — verify the section quotes `AGENTS.md` §10, states the upstream navbar fact with the 404 evidence, resolves by precedence, and labels the geometry `[Proposed]`.
- [x] 4.3 Layout: record cards, pills and charts as composition rules — verify each of the three names the upstream file it was derived from, and the charts section states that *which* data is charted is out of scope.
- [x] 4.4 RTL: write §7.6 with the logical-property rule, the sidebar side, mirrored versus non-mirrored icons, `dir="auto"` for unknown-direction content, chart direction and script-aware leading — verify each rule traces to `AGENTS.md` §10 or to a named upstream file, and that upstream's physical utilities (`pl-20`, `pr-1.5`) are called out as not to be copied.
- [x] 4.5 Window sizing: record 1240×820 default, 1000×680 minimum, centred — verify the derived consequences state the usable content width at the minimum window and match the sidebar width in §7.2.

## 5. Boundaries and review

- [x] 5.1 Write the "deliberately not defined" list — verify it holds the six items still deferred (score-band hue, icon set, empty/loading/error visuals, window chrome, brand assets, per-screen layouts) and no longer lists the Arabic typeface (`D-06`) or the dark-theme scope (`D-08`), both decided in §10.
- [x] 5.2 Write the decision log — verify every `[Proposed]` and `[Open]` item in the document appears in the log with an owner; `D-01`, `D-02`, `D-03`, `D-06` and `D-08` are recorded as `[Decision]` with the approver (product owner) and the 2026-10-22 date, and no other row is marked approved.
- [x] 5.3 Write the change-control section — verify it states that post-approval edits need their own OpenSpec change and that UI changes may not add local constants.
- [x] 5.4 Verify no requirement changed — verify no file under `docs/PRD.md`, `docs/SRS.md`, `docs/ar/`, `openspec/specs/`, or `AGENTS.md` changed. The one file outside this change's own paths that the approval touched is `openspec/changes/add-providers-security/tasks.md` (2026-10-22: group 4 unblocked, the short-secret rule added to 2.6 and 2.13); no requirement text in it changed, so `AGENTS.md` §2's requirement set is intact.
- [x] 5.5 Verify no invented identity — grep `docs/design/DESIGN.md` for hex colour literals and for font-family names — verify zero hex literals and that the only families named as values are Geist (`[Inherited]`) and IBM Plex Sans Arabic (`D-06`, approved 2026-10-22), both licensed and cited; no third family is invented.

## 6. Validation

- [x] 6.1 Run `openspec validate add-design-baseline` — verify it exits 0 and prints that the change is valid; record the exact output in the change notes.
- [x] 6.2 Record the product owner's approval of 2026-10-22 — verify the status line names the approver, the date and "as written"; §10 marks `D-01`, `D-02`, `D-03` and `D-08` as `[Decision]` with that date; and §9 no longer lists the Arabic typeface or the dark-theme scope.

## Workflow follow-up

- `docs/design/DESIGN.md` was approved by the product owner on 2026-10-22, as written. `D-01`, `D-02`, `D-03`, `D-06` and `D-08` are binding; `D-04`, `D-05` and `D-09` remain `[Proposed]`, and `D-07` and the amber value in `D-03` remain `[Open]`.
- The approval unblocked group 4 of `add-providers-security` (Providers UI), now scheduled as that change's fourth slice; its tasks file records the unblock on the same date.
- Archive only when the user asks: `AGENTS.md` §16.3 reserves `openspec archive` to `release-operator`, on an exact approved command.
