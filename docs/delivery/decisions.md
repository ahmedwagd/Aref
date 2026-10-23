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

**Live gate state.** The newest entry governs what may start. Earlier gate tables are annotated
in place with a dated marker when their state changes, not deleted, so the log stays an audit
trail.

| ID | Date | Change | Subject | Status |
|---|---|---|---|---|
| `DLV-2026-10-23-01` | 2026-10-23 | `add-providers-security` | One change, four PR slices | Decided |
| `DLV-2026-10-23-02` | 2026-10-23 | `add-providers-security` | Change-level approval; slice 1 (group 1) unblocked | Decided |

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
| **G-1** | The change (or slice 1 explicitly) is approved by the product owner | **CLOSED 2026-10-23** — change-level approval, `DLV-2026-10-23-02` | At the time of this finding: `design.md` line 15 read `Status: Draft — a design, not an approval`, and no approval of the change as a whole was recorded in any artifact. Group 4's 2026-10-22 unblocking approved a *design document*, not this change. Closed by the owner's answer "Approve the whole change". |
| **G-2** | Repository is genuinely unbuilt — nothing to reconcile | **SATISFIED** | Glob finds no `src/**`, no `src-tauri/**`, no `package.json`, `package-lock.json`, `Cargo.toml`, `tsconfig.json`, `vite.config.ts`, or `tauri.conf.json`. `1.1` starts from zero. |
| **G-3** | Slice 1's PR can be path-scoped to the scaffold only | **SATISFIED 2026-10-23** at HEAD `183c663` | At the time of this finding: three untracked paths — `docs/design/`, `openspec/changes/add-design-baseline/`, `openspec/changes/add-providers-security/` — at HEAD `955fa38`, i.e. planning artifacts not yet committed. They are now committed at `183c663` with a clean tree, so the scaffold PR starts from a scaffold-only diff. Evidence inherited from the orchestrator; shell was unavailable in this session. |
| **G-4** | Toolchain present on the Windows host: Node/npm (for `npm run typecheck`), a Rust toolchain with MSVC build tools, WebView2 (Tauri 2) | **UNVERIFIED** | `1.1`'s binary check is `npm run typecheck` exits 0 and `package-lock.json` is the only lockfile. Shell is blocked in this session, so presence of `node`, `npm`, `rustc`, and `cargo` is an **assumption to verify at kickoff**, not a finding. |
| **G-5** | Requirements may stay `Draft` | **SATISFIED — not a gate** | `proposal.md` line 45: *"no requirement is promoted, reworded, or withdrawn. `FR-301`–`FR-313` stay `Draft`; promotion is a separate decision."* Implementation is authorized by an approved change, not by promoting SRS rows. |
| **G-6** | No upstream dependency inside the change | **SATISFIED** | `1.1` is the first task of group 1; group 1 has no predecessor. `2.13`'s design dependency is already settled (security constant in `design.md` §6.5). |

**Table note (2026-10-23).** `G-1` and `G-3` rows were annotated when their state changed; each
keeps its original finding in the evidence cell. `G-4` is still an unverified fact, not a gate.

**Minimum to start `1.1`:** `G-1` (owner approves the change or slice 1) and `G-3` (planning
artifacts committed separately), with `G-4` confirmed in the first minutes of the task. Nothing
else stands in the way, and no artifact outside `docs/delivery/` needs to change.

---

## Next gate

> **Superseded 2026-10-23 by `DLV-2026-10-23-02`.** That approval was given, change-level;
> `1.1` is unblocked. The text below is kept as the state at time of writing. The gate that now
> blocks the next slice is `G-S2` — group 1 (`1.1`–`1.11`) fully checked. The one decision the
> approval did settle is a delivery question, not a technical one: change-level rather than
> slice-scoped. Every open technical decision — `G-12` above all — is untouched.

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
| 4 | Does the owner want slice 1's approval recorded as change-level approval, or scoped to slice 1? | **Answered 2026-10-23: change-level** — `DLV-2026-10-23-02` | Product owner |

---

## `DLV-2026-10-23-02` — `add-providers-security` approved as a change; group 1 (slice 1) unblocked

| Field | Value |
|---|---|
| **Date** | 2026-10-23 |
| **Approver** | Product owner — owner of scope, priority, risk, and release |
| **Change** | `add-providers-security` — one change, four PR slices (`DLV-2026-10-23-01`); 62 tasks, 0 checked when this approval was recorded |
| **Status** | **Decided and binding.** Change-level approval, not slice-scoped. Binding on delivery until the owner reverses it in a later entry. |
| **Approval instrument** | **This entry.** Not `openspec/changes/add-providers-security/design.md`, not a checked task, not a PR. |
| **Provenance** | The question and the answer were put to the owner in the 2026-10-23 delivery session and relayed verbatim to this record by the orchestrator. I did not speak to the owner directly; the quoted answer is the approval. |
| **Repository state when recorded** | HEAD `183c663`, working tree clean; the change is committed there with `docs/design/DESIGN.md` and this file. `openspec validate add-providers-security --strict` passed and `openspec list` showed `in-progress`, 0/62 tasks. **Inherited from the orchestrator** — shell was unavailable in this session, so these were not re-run here. |
| **Artifacts changed by this record** | This file only. No file under `openspec/`, `docs/design/`, `docs/PRD.md`, `docs/SRS.md`, or `docs/ar/` was edited. No requirement was promoted, reworded, or restatused. |

**Note on the ID.** The approved option says "against `DLV-2026-10-23-01`". That ID is used here
as the reference: entry 01 is the sequencing decision, and its `R-2` gate table held `G-1` open.
A closed gate is a new dated fact, so it is recorded as entry 02 rather than by rewriting entry
01; entry 01's `G-1` row is annotated in place with a pointer. The approved intent — record the
change-level approval in this file — is honoured.

### The approval (verbatim)

**Question put to the owner** — one decision, no bundle:

> "The change is valid but unapproved, which blocks task 1.1. Approve it so slice 1 can start?"

**Owner's answer:**

> **"Approve the whole change (Recommended)"**

**The option's description, quoted because the answer alone does not carry its scope:**

> "scrum-master records change-level approval against DLV-2026-10-23-01 in docs/delivery/decisions.md, then software-engineer implements slice 1 (tasks 1.1-1.11) inside it. Consistent with your 'one change, four PR slices' ruling. Slices 2-4 still stop at their own entry gates per tasks.md line 3, so this does not authorize the other 51 tasks."

### What this authorizes

| # | Authorized | Limit that still applies |
|---|---|---|
| 1 | **The change `add-providers-security` is approved as a whole** — its 26 requirements across `app-shell` (9), `providers` (13), and `key-security` (4) are the approved implementation scope. | The requirements are *served*, not *promoted*. `Draft` status is untouched. |
| 2 | **Task group 1 (`1.1`–`1.11`) may start now**, as one PR slice, with `software-engineer` as owner and each task's own binary check as the bar. | Starting is not done. A task is done when its check passes (`tasks.md` line 3) and the `AGENTS.md` §12 definition of done holds. |
| 3 | **Slices 2–4 need no further owner *approval*.** The change they belong to is approved. | They still stop at their **entry gates**: group 2 after group 1 is fully checked, group 3 after group 2, group 4 after group 3. These are dependency gates, not approval gates. |
| 4 | The four-slice delivery shape of `DLV-2026-10-23-01` stands as the shape of the approved scope. | Running slices in parallel still needs an explicit owner decision (`DLV-2026-10-23-01` consequence 4). |

**The reading that keeps both rules true:** the approval blocker is gone; the sequencing blocker
is not. An agent may start `1.1` today and may **not** start `2.1` until group 1 is fully
checked, even though `2.1` sits inside an approved change.

### What this does NOT authorize

| # | Not authorized | Rule it would otherwise break |
|---|---|---|
| 1 | **Tasks `2.1`–`5.10` — the other 51 tasks.** Starting any group before its predecessor is fully checked, including "just scaffolding" work that belongs to slice 2. | `tasks.md` line 3; `DLV-2026-10-23-01` consequence 6. |
| 2 | **Any promotion, reword, or restatus of a requirement.** `FR-301`–`FR-313`, `NFR-201`, `NFR-202`, `NFR-205`, `NFR-206`, `NFR-306` stay `Draft`; so do the skeleton's `NFR-203`, `NFR-307`, `NFR-308`, `NFR-401`, `NFR-402`, `NFR-403`. | `proposal.md` line 45; `design.md` line 15; `AGENTS.md` §2. Implementation authority comes from an approved change, not from SRS row status. |
| 3 | **Any commit, push, merge, archive, tag, or release** — including committing slice 1's 11 tasks. Those run only as the exact commands the owner approves, executed by `release-operator`. | `AGENTS.md` §16.3. Archive additionally needs QA evidence and the owner's approval. |
| 4 | **Any deviation from the change's scope**: a provider SDK, a telemetry or crash-reporting package, a cloud key reachable from the UI, a network call on the Detect path, a `tailwind.config.js` by default, a non-`npm` lockfile, signing steps, a macOS/Linux code path, a key or user text in a log line, a plaintext key fallback. | `AGENTS.md` §3, §4, §10; `NFR-201`–`NFR-203`, `NFR-307`, `NFR-308`. None is negotiable for schedule. |
| 5 | **A change to the four-slice sequencing**, an early `openspec-sync-specs`, an archive before slice 4, or a split of the change. | `DLV-2026-10-23-01` consequences 2 and 6. Each is a scope change and returns to the owner. |
| 6 | **A quality-gate waiver, or a "done" claim from the builder.** QA's independent gate is unchanged by this entry, and no checkbox here substitutes for test evidence. | `AGENTS.md` §12; this file's preamble. |
| 7 | **A decision on group 5's placement** (`5.1`–`5.10` in slice 4's PR or its own). Carried forward below, not assumed. | Still open; see carry-forward 1. |
| 8 | **A settlement of `design.md` §13 items 3, 5, 6.** None blocks slices 1–3. | `design.md` §13; the only settled decisions are `D-1`–`D-5` (`tasks.md` line 11). |

### Gate status after this entry

| Gate | State | Evidence |
|---|---|---|
| **`G-1`** — the change (or slice 1) is approved by the owner | **CLOSED 2026-10-23** | This entry. Was `OPEN` in `DLV-2026-10-23-01` §R-2. |
| **`G-2`** — tree genuinely unbuilt, nothing to reconcile | **SATISFIED** | No `src/**`, `src-tauri/**`, or manifests at the last check; `1.1` starts from zero. |
| **`G-3`** — planning artifacts committed in their own commit | **SATISFIED 2026-10-23** | Change, `docs/design/DESIGN.md` and this file are committed at `183c663`; tree clean. Inherited evidence, not re-run here. |
| **`G-4`** — toolchain (Node/npm, Rust + MSVC, WebView2) on the Windows host | **OPEN — unverified fact** | First-minute check at kickoff. Not an approval gate; `software-engineer` confirms it before claiming `1.1`'s check. |
| **`G-5`** — requirements may stay `Draft` | **SATISFIED — not a gate** | See *What this does NOT authorize* row 2. |
| **`G-6`** — no upstream dependency inside the change | **SATISFIED** | `1.1` is group 1's first task. |
| **`G-S2`** — **new: slice 2 entry** | **OPEN — blocks `2.1`–`2.14`** | Group 1 (`1.1`–`1.11`) fully checked, each box backed by its own check, plus `G-4` confirmed. Same shape for `G-S3` (group 2 checked) and `G-S4` (group 3 checked). |

**The gate that now blocks slice 2 is `G-S2`: group 1 fully checked.** Not owner approval, not
OpenSpec status. It closes only when all 11 tasks in group 1 are checked *and* the evidence behind
each is verified — a checked box is a claim, not proof.

### The `design.md` line 15 question — position

`openspec/changes/add-providers-security/design.md` line 15 still reads:

> Status: `Draft` — a design, not an approval. Requirement statuses are untouched; `FR-301`–`FR-313` stay `Draft`.

**Position: the line is accurate, not stale, and it must not be read as evidence that the change
is unapproved. No file under `openspec/` needs to change for this approval to take effect or for
slice 1 to start.**

Three reasons, in the order a reader should apply them:

1. **The line's subject is the document, not the change.** "A design, not an approval" says this
   document confers no approval. That is still exactly true: this entry is the instrument, and the
   document confers nothing.
2. **Its second clause is still literally true, and deliberately so.** No requirement status moved.
   The approval was given on the understanding that it would not promote anything (`G-5`).
3. **Authority runs the other way.** `AGENTS.md` §2 ranks OpenSpec above delivery records. Editing
   an OpenSpec artifact to say "Approved" because a delivery decision said so would assert a
   delivery fact inside a spec-authority file — the precedence problem, not the fix. Approval state
   belongs in `docs/delivery/`; design content belongs in `openspec/`.

**Binding reading rule for later agents.** Change-approval state is read from this file, never
inferred from a `Draft` status in `design.md`. `Draft` there describes the document's own standing;
it is not a statement about the change. Conversely, this entry approves the **change**, not the
design's open items — §13 items 3, 5, 6 stay open.

**Recommendation, non-blocking, outside my write path.** One pointer clause after line 15 would
remove the ambiguity permanently, e.g.: *"Change-level delivery approval: `docs/delivery/decisions.md`
`DLV-2026-10-23-02`; requirement statuses above remain untouched."* That clarifies standing without
changing a status, and `design.md` belongs to `software-architect`. I did not edit it. Until it
lands, this entry governs, and the missing pointer is **not** a gate — no agent may block slice 1
on it.

### Carry-forward — still open, not assumed

| # | Item | State after this entry | Owner |
|---|---|---|---|
| 1 | Group 5 placement: `5.1`–`5.10` in slice 4's PR, or a fifth verification PR inside the same change? | **Still unanswered.** The approval puts group 5 in scope; it does not choose which PR carries it. `DLV-2026-10-23-01` reads it as slice 4's PR and flags that as an assumption. Not a blocker for slices 1–3. | Product owner |
| 2 | Archive `add-design-baseline` (25/25, unarchived) before slice 1 opens? | **Still open.** Hygiene, not a gate, and it must not be folded into slice 1's PR (one concern per PR, `AGENTS.md` §12). | Product owner / `release-operator` |
| 3 | Toolchain readiness for `1.1` (`G-4`) | **Unverified fact.** Confirm at kickoff, before claiming `1.1`'s check. | `software-engineer` |
| 4 | Whether `design.md` line 15 gains the pointer clause | **Recommendation only**, non-blocking, not required for slice 1. | `software-architect`, via `orchestrator` |

**Closed by this entry:** open question 4 of `DLV-2026-10-23-01` — the approval is **change-level**,
not slice-scoped.

### Consequence for the next move

`1.1` may start. The next obligation is the first *evidence* gate, not another approval: 11 checks
green, then `G-S2`. No commit happens until the owner approves the exact commands.

