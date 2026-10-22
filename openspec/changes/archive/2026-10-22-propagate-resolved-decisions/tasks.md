# Tasks

Docs-only propagation. One task per file touched, plus one inspection that may produce no edit. No task edits `docs/SRS.md`, changes a requirement status, or introduces a new decision: every clause written here is copied from a settled `docs/SRS.md` §9 row or the normative cell it points at.

## 1. Arabic summary

- [x] 1.1 `docs/ar/SRS.ar.md` §5: move `G-09`, `G-13`, `G-19`, `G-20`, `G-27` out of the مفتوحة sentence (line 42) into the محسومة sentence (line 40), each with the one-clause descriptor from `design.md` decision 3, in ascending ID order and in the file's existing sentence style; keep `1 s` and `30 s` exactly as `docs/SRS.md` writes them and add no directional formatting character.
  - Verify: in `docs/ar/SRS.ar.md`, each of the five IDs occurs on the محسومة line and nowhere else, the مفتوحة line contains none of the five, and a search for U+200E, U+200F, U+202A–U+202E, and U+2066–U+2069 in the file returns zero matches.
  - **Evidence (2026-10-22):** the محسومة line is now 40 and holds eleven IDs in ascending order (`G-01`, `G-09`, `G-10`, `G-13`, `G-19`, `G-20`, `G-21`, `G-25`, `G-27`, `G-28`, `G-29`); line 42 lists the remaining fifteen. Content search for `G-09|G-13|G-19|G-20|G-27` over the whole file returns **one line, 40** — the five appear nowhere else. Search for `[\x{200E}\x{200F}\x{202A}-\x{202E}\x{2066}-\x{2069}]` over the file returns **0 matches**. Descriptors are abbreviations of SRS rows 1106, 1110, 1116, 1117, 1124 respectively; `1 s` and `30 s` are byte-identical to SRS line 367. No other ID moved: `G-12`, absent from the original مفتوحة list, was left absent because that list is introduced by "أهمها" and is not exhaustive.
  - Note: shell was available for `openspec` and `git` only, so the content checks above used the file-search tool and reads, not `grep`/`git`; the commands actually run are recorded under task 4.1.

## 2. PRD

- [x] 2.1 `docs/PRD.md` §7 feature table: replace the trailing "remaining" note in cells 171, 175, 176, and 178 with the settled clause and the `G-xx` ID, leaving every other cell of those rows untouched.
  - Verify: cells 171, 175, 176, 178 each read a settled rule plus a `G-xx` marker and contain no bare "handling `G-09`", "base and cap `G-13`", "rule `G-27`", "behavior `G-20`", or "endpoint `G-19`" phrasing; the still-open IDs in the same rows (`G-15` in 175) are unchanged; the feature table still has thirty feature rows.
  - **Evidence (2026-10-22):** the four cells now read: 171 "model_id is required, user-supplied, and authoritative; no model is chosen for the user (`G-09`, resolved)"; 175 "Backoff base 1 s and cap 30 s per wait (`G-13`, resolved); wrong-language definition `G-15`"; 176 "Last four characters behind a fixed-width mask (`G-27`, resolved); an unreadable key is unusable but not invalid (`G-20`, resolved)"; 178 "Order: OpenCode, Ollama, LM Studio, then cloud; OpenCode is user-configured, keyless, off by default, never probed (`G-19`, resolved)". The open `G-15` citation in 175 and the row-176 `FR-310`, `FR-313` cell are unchanged. The table still runs lines 158–187, **thirty feature rows**, matching the completeness check at line 189. Every other line of the PRD is untouched: `git diff -U0 -- docs/PRD.md` reports only 171, 175, 176, and 178.
- [x] 2.2 `docs/PRD.md` §11 decision log: inspect every row for these five IDs and correct it only if it asserts one of them is open; otherwise record that no edit is required and why.
  - Verify: the check is stated with the row count examined and the exact reading of any row that mentions `G-09`, `G-13`, `G-19`, `G-20`, or `G-27`; if no row mentions them, the task records zero matching rows and `docs/PRD.md` §11 is byte-identical.
  - **Evidence (2026-10-22): inspected, no edit required.** §11 runs lines 242–255: header, separator, and **twelve** decision rows. A search for the five IDs over the whole PRD returns only the four feature-table cells edited in 2.1, so **zero** §11 rows mention any of the five; the log never asserts they are open. The 2026-10-21 document-approval row (line 255) says document approval "does not resolve any open `G-xx` decision", which is still true — these five were settled by `resolve-open-decisions` on 2026-10-22, not by document approval — so it is a dated record of what that approval did, not a claim about current row status. Extending the log with the five decisions would duplicate the authoritative SRS §9 record (`design.md` decision 6), so §11 was left byte-identical.

## 3. Upstream task record

- [x] 3.1 `openspec/changes/resolve-open-decisions/tasks.md` task 6.5: reword it so it records the executed check, the contradiction it found, and the approved handoff to this change, then tick it; leave every other task in that file unchanged.
  - Verify: 6.5 is ticked, its text names the five IDs and `propagate-resolved-decisions` as the approved destination, and no other task line in the file was reworded (checked by reading the file; the change directory is untracked, so `git diff` cannot display it).
  - **Evidence (2026-10-22):** 6.5 now reads as an executed check: it records that line 42 of the Arabic summary listed the five as مفتوحة while all five read `Resolved` in SRS §9, that the PRD cells were stale rather than false, that the product owner approved `propagate-resolved-decisions` on 2026-10-22 to carry the correction, and that its executed branch was "report for a separate approved edit" — so the box is ticked. The other nine tracked tasks and the two "Workflow follow-up" bullets were read after the edit and are unchanged; the file has **10 checkboxes, all ticked**. Shell access was limited to `openspec` and `git`, so this check was a read, and no `git diff` output is claimed for the untracked change directory.

## 4. Verification

- [x] 4.1 Run both validators and confirm the working tree contains only the intended files.
  - Verify: `openspec validate propagate-resolved-decisions` and `openspec validate resolve-open-decisions` each report the change valid; `git status --short` lists the two modified derivative docs, the pre-existing `docs/SRS.md` modification from `resolve-open-decisions`, and both untracked change directories; `git diff -U0` on the two derivative docs touches only PRD lines 171, 175, 176, 178 and Arabic lines 40, 42.
  - **Evidence (2026-10-22), exact output:**
    - `openspec validate propagate-resolved-decisions` → `Change 'propagate-resolved-decisions' is valid` / `ℹ [INFO] file: skip_specs is set in .openspec.yaml: change declares no spec-level behavior changes, zero deltas accepted`.
    - `openspec validate resolve-open-decisions` → `Change 'resolve-open-decisions' is valid` / same INFO line.
    - `git status --short` →
      ```
       M docs/PRD.md
       M docs/SRS.md
       M docs/ar/SRS.ar.md
      ?? openspec/changes/propagate-resolved-decisions/
      ?? openspec/changes/resolve-open-decisions/
      ```
    - `git diff --stat` (stdout; git also printed `warning: in the working copy of ... LF will be replaced by CRLF` for the three files on stderr) →
      ```
       docs/PRD.md       |  8 ++++----
       docs/SRS.md       | 38 ++++++++++++++++++++++++--------------
       docs/ar/SRS.ar.md |  4 ++--
       3 files changed, 30 insertions(+), 20 deletions(-)
      ```
    - `git diff -U0 -- docs/PRD.md docs/ar/SRS.ar.md` → hunks at PRD `@@ -171 +171 @@`, `@@ -175,2 +175,2 @@`, `@@ -178 +178 @@` and Arabic `@@ -40 +40 @@`, `@@ -42 +42 @@` — no other line in either file.
    - `docs/SRS.md`'s 38-line diff is the pre-existing `resolve-open-decisions` work, not this change: this change's only two file diffs are the ones above, and no command here wrote to `docs/SRS.md`.

## Workflow follow-up

- Archive this change only after the product owner approves the result; archiving is not approved by this change.
- Keep implementation out of this change: the settled values are consumed by the skeleton + Providers & Security change.
