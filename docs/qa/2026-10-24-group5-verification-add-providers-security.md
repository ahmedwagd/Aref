# Group-5 verification record — `add-providers-security`

- **Date:** 2026-10-24
- **Executor:** QA (verification subagent; independent of the implementer)
- **Change:** `openspec/changes/add-providers-security` — group 5 (tasks `5.1`–`5.10`), the last open group
- **Build under test:** working tree at HEAD `8ffc864` ("feat: add provider client, loopback discovery, and provider commands") plus slice-4 uncommitted work: 5 modified files (`tasks.md`, `src/App.tsx`, `src/components/AppShell.tsx`, `src/i18n/ar.json`, `src/i18n/en.json`) and untracked `src/features/**`, `src/components/{Button,ErrorMessage,StatusPill}.tsx`, 7 `tests/providers.*.test.tsx`, `tests/support/providers.ts`, `src-tauri/aref.db`.
- **Environment:** Windows, repository `D:\Workspace\Desktop\Clear`; tool versions as printed by the tools themselves: vitest v5.0.5, vite v8.3.5. `node --version` / `cargo --version` probes were outside the command allow-list and were not run; versions are implied by the passing runs.
- **Working directory for npm commands:** repo root. **Working directory for cargo commands:** `src-tauri` (via the runner's workdir parameter; no `cd` used).

## Verdict

All ten group-5 checks passed and are recorded with their exit statuses below. The change's own tasks are complete pending the owner's commit-and-archive decision. This record makes no release statement beyond this change: `openspec/specs/` is still empty, no CI exists, and later changes (detector, humanizer, storage of history) have their own gates. See "Scope of this verdict".

## Gate table

| # | Check | Command (verbatim) | Exit | Key output |
|---|---|---|---|---|
| 5.1 | Typecheck | `npm run typecheck` | 0 | `tsc --noEmit`, no errors |
| 5.2 | Lint | `npm run lint` | 0 | `eslint .`, no findings |
| 5.3 | Frontend tests | `npm test` | 0 | `Test Files 14 passed (14)`, `Tests 113 passed (113)`; duration 9.01 s |
| 5.4 | Bundle build | `npm run build` | 0 | 41 modules; `dist/index.html` 0.39 kB, `dist/assets/index-JSzZV6AP.css` 22.09 kB, `dist/assets/index-BINBwAOA.js` 250.80 kB |
| 5.5 | Rust formatting | `cargo fmt --check` (in `src-tauri`) | 0 | no output (no diffs) |
| 5.6 | Clippy | `cargo clippy --all-targets -- -D warnings` (in `src-tauri`) | 0 | `Finished 'dev' profile [unoptimized + debuginfo] target(s) in 22.28s`, zero warnings |
| 5.7 | Rust tests | `cargo test` (in `src-tauri`) | 0 | 14 test binaries, **211 passed, 0 failed, 0 ignored** (124 lib + 87 integration + 0 doc); see breakdown |
| 5.8 | Requirement trace audit | manual audit of `tasks.md` lines 104–133 against `docs/SRS.md` + the three delta specs | pass | 26 rows; every named task exists in the file; every cited `FR-`/`NFR-` ID resolves in `docs/SRS.md`; row names match the delta-spec requirement texts verbatim; details below |
| 5.9 | OpenSpec validation | `openspec validate add-providers-security` | 0 | `Change 'add-providers-security' is valid` |
| 5.10 | Secret scan | pattern scans via ripgrep syntax over tracked and untracked source (see below) | pass | zero key-shaped literals; zero fixture values outside fixtures/test modules; details below |

### 5.7 breakdown (cargo test, 0 failures everywhere)

`unittests src\lib.rs` 124 · `unittests src\main.rs` 0 · `composition_inert` 7 · `i18n_catalog` 4 · `key_material` 3 · `loopback_harness` 4 · `mask_constants` 7 · `provider_architecture` 5 · `providers_client` 12 · `providers_config` 8 · `providers_surface` 13 · `redaction_boundary` 5 · `storage` 13 · `tauri_config` 4 · `test_support_gate` 2 · doc-tests 0. Total **211 passed / 0 failed / 0 ignored**.

### 5.3 deviation check

The implementer reported `npm test` as 14 files / 113 tests. This run reproduced **exactly** those counts: 14 files, 113 tests, all green. No suite count changed, no test was skipped.

## 5.8 — requirement trace audit (method and result)

Source of truth: `openspec/changes/add-providers-security/tasks.md` lines 104–133 (the "Requirement coverage" table) audited against `docs/SRS.md` and the three delta specs (`specs/app-shell/spec.md`, `specs/providers/spec.md`, `specs/key-security/spec.md` inside the change).

1. **Row count and task existence.** The table has exactly 26 rows. Every task number named in every row exists as a checkbox in this file: rows reference only `1.1–1.11`, `2.1–2.14`, `3.1–3.20`, `4.1–4.7`, `5.9`, `5.10`, all present. Checked row by row; no dangling reference.
2. **Row names map to delta-spec requirements verbatim.** The `app-shell` delta defines 9 requirements whose exact names are rows 1–9; the `providers` delta defines 13 requirements whose exact names are rows 10–22; the `key-security` delta defines 4 requirements whose exact names are rows 23–26. 9 + 13 + 4 = 26 — the table is a complete, name-exact index of the change's three deltas.
3. **Cited IDs resolve.** A pattern scan of `tasks.md` for `FR-\d+|NFR-\d+` yields 30 unique IDs: `FR-107`, `FR-212`, `FR-509`, `FR-301`–`FR-313`, `FR-404`, `NFR-201`–`NFR-206`, `NFR-207`, `NFR-305`–`NFR-308`, `NFR-401`–`NFR-403`. Every one was located in `docs/SRS.md` (e.g. `FR-301` SRS line 373, `FR-404` line 515, `NFR-207` line 709, `NFR-305` line 747). No task cites an ID absent from `docs/SRS.md` or from the three delta specs. `G-09`/`G-13`/`G-19`/`G-20`/`G-27` and `D-1`–`D-5` are governance/decision IDs resolved in SRS §9 and in this file's "Settled decisions" section respectively; they are citations, not requirement IDs.

Known, documented non-coverage (stated in the table itself, lines 135–141, and verified here): requirement 17's "completed chunks in the partial report" belongs to the humanize change (no chunking exists in this change; task 3.14 covers the part that exists). Requirement 18's `refused` marker is settled as `D-5` and stated in the delta. Both are explicit in the file; nothing was silently dropped.

**Result: 5.8 passes.**

## 5.10 — secret and fixture scan (exact patterns and results)

Scan tooling: ripgrep-syntax pattern search over the working tree (build/vendor directories `node_modules`, `dist`, `target` are gitignored — confirmed by their absence from `git status --short`; they were not searched). The native `rg` binary is not on this shell's PATH; the identical patterns were executed through the session's grep tool. No match was weakened, excluded, or reinterpreted to make a gate pass — every raw match was read in context and classified below.

| Scan | Pattern | Matches | Classification |
|---|---|---|---|
| OpenAI-style key shape | `sk-[A-Za-z0-9_-]{8,}` | 1 | `src-tauri\tests\provider_architecture.rs:256` — the substring `sk-constant` inside the English word "mask-constant" in a comment. Not a key-shaped literal. |
| Other provider key shapes | `sk-ant-\|AKIA[0-9A-Z]{16}\|ghp_[A-Za-z0-9]{36}\|github_pat_[A-Za-z0-9_]{22,}\|AIza[0-9A-Za-z_-]{35}\|xox[baprs]-[0-9A-Za-z-]{10,}` | 0 | — |
| PEM / certificate blocks | `-----BEGIN [A-Z ]*PRIVATE KEY\|-----BEGIN CERTIFICATE` | 0 | — |
| Hex blobs (≥40 chars) | `[A-Fa-f0-9]{40,}` | 100+ (truncated) | All matches are `Cargo.lock` `checksum = "…"` lines — crates.io dependency content hashes, standard lockfile format, not secrets. The same pattern over `src/`, `src-tauri/src/`, `src-tauri/tests/`, `tests/`, `docs/`, and the change directory returns **zero** matches. |
| Fixture value outside fixtures | `fixture-secret` | 45 | See classification below. |

**`fixture-secret` occurrence classification (all 45):**

- Test-only files (`src-tauri/tests/key_material.rs`, `providers_client.rs`, `mask_constants.rs` (`fixture-secret-9876` variant), `providers_surface.rs`, `redaction_boundary.rs`, `tests/support/providers.ts`): allowed — these are the fixtures.
- `src-tauri/src/**` occurrences (`commands/parse.rs` ×4, `commands/dto.rs` ×1, `security/keyring.rs` ×10, `security/mask.rs` ×2 (`fixture-secret-9876`, `fixture-secret-انتث` — multi-byte fixture), `providers/client.rs` ×5, `security/redact.rs` ×6, `providers/keys.rs` ×3, `providers/pool.rs` ×2): every occurrence sits **inside a `#[cfg(test)] mod tests` block**. Verified structurally: each file's `#[cfg(test)]`/`mod tests {` line precedes its fixture lines, and the two files with late fixture lines (`providers/client.rs`, `providers/pool.rs`) end with the closing brace of the test module immediately after the last fixture test — no production code follows. Production (non-test) code: **zero** occurrences.
- `openspec/changes/add-providers-security/tasks.md` ×2 and `design.md` ×1: self-references — the task text and design prose naming the fixture convention and the scan itself. Not a use of a secret.

**"Real user text":** this change contains no corpus and no history store; every text fixture in the tree is synthetic (`fixture-secret-a`, `fixture-secret-انتث`, fixture model IDs, `.invalid`-TLD endpoints — confirmed by reading `tests/support/providers.ts`). The user-text dimension of `NFR-207` is satisfied vacuously here; the substantive corpus-policy check arrives with the detector/humanizer changes.

**Result: 5.10 passes** (zero secrets, zero key-shaped strings, zero fixture values outside fixtures/test modules).

## Observations (defects and notes, none blocking this group)

1. **`src-tauri/aref.db` — stray untracked artifact (test-isolation defect, severity low).** An SQLite file sits at `src-tauri/aref.db`, untracked, with no `.gitignore` rule covering it. Per the orchestrator's context it was produced by an earlier `cargo test` run (the DB connection string is `sqlite:aref.db`, relative to the process working directory — consistent with `tests/storage.rs`'s "the connection string names the database file the guard copies" test). It was present before and after this verification run. **It was not staged, modified, or deleted by this run.** Risk: it can be accidentally staged into a commit and it makes `git status` noisy for every future cargo test run. Recommended (for the build agent, not QA): either point the test's DB path at a temp directory or add a `.gitignore` rule; either is a one-line change that does not weaken any test.
2. **Command allow-list gaps (environment, not product):** `rg` is not on the shell PATH (ripgrep syntax ran through the session's grep tool instead); `git check-ignore` and arbitrary PowerShell one-liners were denied. Nothing was concluded from a denial alone; the facts above were established with allowed tools.
3. **Performance note (informational):** `npm test` reports jsdom was created 14 times (45.57 s of tracked time); vitest itself suggests `pool: 'vmThreads'` to keep isolation while sharing the environment. No action required for this change; worth a look when the suite grows.

## Scope of this verdict (what was NOT tested)

- No CI exists; all gates were run locally on one machine, once each. Reproducibility on a clean machine is unproven until CI lands.
- No installer/packaging artifact exists yet, so packaging gates (model bundling, offline restore, installer smoke) are not applicable to this change and were not run.
- No Arabic/English visual RTL screenshots were taken in this verification; RTL coverage in this change is asserted by the component tests in groups 1 and 4 (both directions render, logical-property scans) and passed as part of 5.3.
- False-positive-rate gates (NFR-303/304) belong to the detector/corpus track, not this change.
- The frontend gates ran against the full current tree including slice 4's uncommitted files; the Rust gates ran against the crate as committed plus `src-tauri` untracked test files. Both reflect the exact tree a commit would capture.

## Recommendation

Group 5 is complete and honestly evidenced. The change `add-providers-security` has all 62/62 tasks checked once the ten `5.x` boxes are flipped. **Ready for the owner's commit-and-archive decision**, subject to: (a) the owner reviewing this record, (b) the release operator staging explicit paths only (never `aref.db`), and (c) a follow-up one-line fix for the stray-DB artifact either before commit or as the first task of the next change.
