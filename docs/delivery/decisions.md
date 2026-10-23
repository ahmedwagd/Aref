# Delivery decisions — Aref

Operational record of delivery-shape and sequencing decisions. This file is **not** a
specification. It records what was decided, by whom, and what it constrains.

**Precedence.** `AGENTS.md` §2 applies unchanged: `AGENTS.md` → OpenSpec → PRD/SRS →
`DESIGN.md` → code conventions. A decision recorded here may change *how* the backlog is
delivered; it may not change approved behavior, scope, or a requirement. If a delivery
decision would change behavior, it must go through the owning OpenSpec change — a note in
this folder is not enough.

**Scope of this file.** Decisions and readiness findings only. No task is checked here, no
requirement is promoted, reworded, or restatused, and nothing here substitutes for QA
evidence or an OpenSpec status.

| ID | Date | Change | Subject | Status |
|---|---|---|---|---|
| `DLV-2026-10-23-01` | 2026-10-23 | `add-providers-security` | One change, four PR slices | Decided |

---

## `DLV-2026-10-23-01` — `add-providers-security` stays one change with four PR slices

| Field | Value |
|---|---|
| **Date** | 2026-10-23 |
| **Approver** | Product owner — owner of scope, priority, and sequence |
| **Change** | `add-providers-security` (created 2026-10-22; 0 of 62 tasks complete) |
| **Status** | **Decided.** Binding on sequencing until the owner reverses it in a later entry. |
| **Artifacts changed by this record** | This file only. No file under `openspec/`, `docs/design/`, `docs/PRD.md`, `docs/SRS.md`, or `docs/ar/` was edited, and no requirement status was touched. |

### Decision (verbatim)

The stop condition *"do not create a change that spans more than PR 1"* was raised as a
delivery objection. Asked to choose, the product owner answered:

> **"Keep one change, 4 PR slices"**

**Recorded meaning.** `add-providers-security` remains a **single** OpenSpec change covering
all four PR slices. The four task groups in `tasks.md` are the PR boundaries *inside* that
one change. The three capability specs (`app-shell`, `providers`, `key-security`) land
together, only when slice 4 completes and the change is archived.

### The objection this overrides

| | |
|---|---|
| **Raised by** | Delivery (Scrum Master), as a WIP and reviewability control |
| **Original rule** | Do not open a change that cannot be delivered inside a single PR slice |
| **Why it was raised** | 62 tasks, 26 requirements, three new capabilities, and the first application code in an unbuilt repository — with no ability to validate any slice against a running app until slice 1 lands |
| **Outcome** | **Overridden by the owner.** The owner accepted a larger change in exchange for one coherent capability boundary. The cost is carried as a delivery risk, not argued again. |

### Rejected alternatives

1. **Split into four OpenSpec changes** — one per slice (`add-app-shell`, `add-provider-storage`,
   `add-provider-client`, `add-providers-ui`), archived one at a time.
   *Rejected.* It fragments a contract the four slices share: one `AppError` envelope, one
   keyring trait, one retry/selection seam. Slices 2–4 would each open against a capability
   spec that does not exist yet, so each would either restate its neighbors' requirements or
   carry cross-change dependencies — and three of the four would ship no user-visible value
   on their own.
2. **Trim the change to slice 1** — scaffold only (`1.1`–`1.11`), with providers and keys moved
   to later changes.
   *Rejected.* Slice 1 alone leaves the product unable to send one request or store one key,
   so the scaffold's own contracts (error envelope, IPC boundary, i18n) would be validated
   against nothing but an empty shell. `AGENTS.md` §12 also orders this step as one unit.
3. **Keep one change but land the three capability specs incrementally** (sync `app-shell`
   after slice 1, `key-security` after slice 2, `providers` after slice 3).
   *Rejected by consequence of the chosen option.* See below — this is what the decision
   forecloses.

### Rationale (inferred, not stated by the owner)

The owner gave the choice, not a written memo. The following is **inference** from the
change's own artifacts, and should be read as such:

- `proposal.md` line 21: *"The four slices are PR boundaries inside one approved change, not
  four changes: a PR may land part of a change, and the capability specs are only whole once
  all four have landed."* The owner's choice matches this text; the artifacts did not need
  editing, which is itself evidence that the option was the intended shape.
- `tasks.md` line 145 states the same rule in the workflow section.
- The change is the vehicle that creates `openspec/specs/` — it is empty today. One change,
  one archive, one spec-writing step keeps the first three capability specs internally
  consistent, which matters because later changes inherit them as boundaries.

### Consequence — the part that binds day-to-day delivery

1. **`openspec/specs/` stays empty until all four slices land and archive is approved.**
   Archiving this change writes three capabilities in **one** step:
   `app-shell` (9 requirements), `providers` (13), `key-security` (4) — **26 requirements**.
   There is no intermediate state in which one of the three exists as a living spec.
2. **No `openspec-sync-specs` before slice 4.** Syncing early would create the partial-spec
   state the decision rejected. Any request to sync a single capability is a scope change,
   not a routine step, and goes back to the owner.
3. **Archive is one binary gate.** It cannot be partly done, and it cannot be declared by a
   builder: it needs QA evidence, then the owner's approval, then `release-operator`.
4. **This change is the entire WIP budget.** My standing limit is one or two active changes;
   at 62 tasks it consumes both. No other implementation change starts in parallel unless the
   owner explicitly accepts the parallelism. The evaluation-corpus track
   (`NFR-303`–`NFR-305`) is the only exception already approved by `AGENTS.md` §12, and it
   touches no file this change touches.
5. **A pause leaves a visible stale change.** If delivery stops after slice 2, the change sits
   `in-progress` with two of three capabilities unbuilt and `openspec/specs/` still empty.
   That is the accepted cost of the choice; it is a reporting fact, not a defect.
6. **Slice sequencing is unchanged:** 1 → 2 → 3 → 4, each starting only when the previous
   group is fully checked (`tasks.md` line 3). One task group per PR.

### PR boundaries inside the change

| Slice | `tasks.md` group | Task range | Tasks | Entry evidence |
|---|---|---|---|---|
| **1** | 1. Scaffold, error envelope, locales | `1.1`–`1.11` | 11 | `npm run typecheck` exits 0 on the empty shell; one lockfile |
| **2** | 2. Storage and keys | `2.1`–`2.14` | 14 | group 1 fully checked; no key material on disk |
| **3** | 3. Provider client | `3.1`–`3.20` | 20 | group 2 fully checked; zero probe attempts while idle |
| **4** | 4. Providers UI | `4.1`–`4.7` | 7 | group 3 fully checked; design tokens approved (`D-01`, `D-02`, `D-03`, `D-06`, `D-08`) |
| — | 5. Verification | `5.1`–`5.10` | 10 | aggregate gates; green only after slice 4 |
| | | | **62** | |

**Assumption, flagged:** `tasks.md` line 3 assigns groups 1–4 to the four PRs and calls group 5
*"one integration group"* without naming its PR. I read `5.1`–`5.10` as landing in slice 4's
PR, because `5.1`–`5.7` are build gates that cannot pass before slice 4 exists. If the owner
wants a separate verification PR, that is a fifth PR boundary inside the same change — it
does not reopen this decision. Not a blocker for slice 1.

---

## Readiness

### R-1 — Is `add-design-baseline` (25/25 complete, unarchived) a blocker for slice 1, for slice 4, or for neither?

**Answer: neither.** It is housekeeping, with a cheap recommendation attached.

| Evidence | Finding |
|---|---|
| `openspec/changes/add-design-baseline/.openspec.yaml` → `skip_specs: true` | Archive of that change writes **nothing** to `openspec/specs/`. It cannot collide with this change's three capabilities, and it creates no ordering constraint against `add-providers-security`. |
| Its `tasks.md`: 25 checked boxes, including `6.1` (`openspec validate` exits 0) and `6.2` (the owner's 2026-10-22 approval recorded, `D-01`/`D-02`/`D-03`/`D-08` marked `[Decision]`) | The artifact is finished and its approval is recorded. "Unarchived" is a bookkeeping state, not missing work. |
| `docs/design/DESIGN.md` exists in the tree | The token source `AGENTS.md` §10 requires is present for slice 4. |
| `tasks.md` lines 75–77 (group 4) | The stated gate for slice 4 is the **approved document** — `D-01`, `D-02`, `D-03`, `D-06`, `D-08` — not the archive of the change that produced it. Group 4 is already marked unblocked and scheduled fourth. |
| Slice 1 task IDs: `NFR-203`, `FR-301`, `NFR-402`, `NFR-307`, `NFR-308`, `NFR-205`, `NFR-306`, `NFR-401`, `NFR-403`, `NFR-202` | None consumes a design **value**. `1.2` (Tailwind wiring), `1.8` (logical properties) and `1.9` (direction) apply structural rules from `AGENTS.md` §10; no colour, spacing, radius, or motion token appears. Slice 1 cannot be blocked by a design-baseline archive either. |
| `openspec/specs/` contains no files (glob verified) | Nothing depends on that change having been archived first. |

**Residual risk (low, reporting only):** `openspec list` shows `add-design-baseline` as
`complete` indefinitely, which reads as unfinished work to anyone scanning status. No delivery
work depends on clearing it.

**Recommendation (hygiene, not a gate):** archive `add-design-baseline` **before** slice 1's PR
opens, not inside it. It is documentation-only, it is 25/25 with its approval recorded, and
`AGENTS.md` §12 requires one concern per PR — so it must not be folded into the scaffold PR.
Archive is a `release-operator` action on exact commands the owner approves (`AGENTS.md` §16.3);
it is **not** approved by this record.

### R-2 — What is the entry gate for task `1.1`, given a clean, unbuilt tree?

The gate for `1.1` is **not** OpenSpec status and **not** requirement promotion. It is change
approval plus a path-scoped tree plus a working toolchain.

| # | Gate | Current state | Evidence |
|---|---|---|---|
| **G-1** | The change (or slice 1 explicitly) is approved by the product owner | **OPEN** | `design.md` line 15: `Status: Draft — a design, not an approval.` No approval of the change as a whole is recorded in any artifact. Group 4's 2026-10-22 unblocking approved a *design document*, not this change. |
| **G-2** | Repository is genuinely unbuilt — nothing to reconcile | **SATISFIED** | Glob finds no `src/**`, no `src-tauri/**`, no `package.json`, `package-lock.json`, `Cargo.toml`, `tsconfig.json`, `vite.config.ts`, or `tauri.conf.json`. `1.1` starts from zero. |
| **G-3** | Slice 1's PR can be path-scoped to the scaffold only | **OPEN — needs a commit first** | `git status --short` (parent session; shell unavailable to me here, so this is inherited, not re-run): three untracked paths — `docs/design/`, `openspec/changes/add-design-baseline/`, `openspec/changes/add-providers-security/` — at HEAD `955fa38`. `docs/PRD.md`, `docs/SRS.md`, and `docs/ar/` do not appear as untracked, so they are already in HEAD. The three planning paths must be committed in their own commit before slice 1 opens, or the scaffold PR would mix planning artifacts with application code. |
| **G-4** | Toolchain present on the Windows host: Node/npm (for `npm run typecheck`), a Rust toolchain with MSVC build tools, WebView2 (Tauri 2) | **UNVERIFIED** | `1.1`'s binary check is `npm run typecheck` exits 0 and `package-lock.json` is the only lockfile. Shell is blocked in this session, so presence of `node`, `npm`, `rustc`, and `cargo` is an **assumption to verify at kickoff**, not a finding. |
| **G-5** | Requirements may stay `Draft` | **SATISFIED — not a gate** | `proposal.md` line 45: *"no requirement is promoted, reworded, or withdrawn. `FR-301`–`FR-313` stay `Draft`; promotion is a separate decision."* Implementation is authorized by an approved change, not by promoting SRS rows. |
| **G-6** | No upstream dependency inside the change | **SATISFIED** | `1.1` is the first task of group 1; group 1 has no predecessor. `2.13`'s design dependency is already settled (security constant in `design.md` §6.5). |

**Minimum to start `1.1`:** `G-1` (owner approves the change or slice 1) and `G-3` (planning
artifacts committed separately), with `G-4` confirmed in the first minutes of the task. Nothing
else stands in the way, and no artifact outside `docs/delivery/` needs to change.

---

## Next gate

**Product owner approval of `add-providers-security` as a change — or an explicit, recorded
authorization of slice 1 only.** Until then, `1.1` does not start, because `design.md` still
carries `Status: Draft — a design, not an approval` and no approval of this change is on record.

Approving the change would **not** promote any requirement, close any decision, or authorize a
commit or archive.

## Open questions and assumptions

| # | Item | Type | Owner |
|---|---|---|---|
| 1 | Does group 5 (`5.1`–`5.10`) land in slice 4's PR or its own? | Assumption, flagged above | Product owner |
| 2 | Is `add-design-baseline` archived before slice 1 opens, or left until later? | Hygiene choice, not a blocker | Product owner |
| 3 | Is the local toolchain (Node, Rust/MSVC, WebView2) ready for `1.1`? | Unverified fact | Software Engineer at kickoff |
| 4 | Does the owner want slice 1's approval recorded as change-level approval, or scoped to slice 1? | Decision, see next gate | Product owner |
