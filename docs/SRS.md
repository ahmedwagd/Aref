# Aref — Software Requirements Specification (SRS)

> **Status: Approved — 2026-10-21.**
> This document is approved by the product owner as of 2026-10-21 and is binding from that date. Document approval does not change any requirement's status and does not settle any open `G-xx` decision. Every requirement below carries exactly one of three statuses, defined in §1.5: `Draft`, `Pending decision (G-xx)`, or `Withdrawn`. A `Withdrawn` requirement is retained only so that its ID is never reused; it imposes no obligation and must not be implemented. A `Pending decision (G-xx)` requirement keeps its testable invariant and leaves only the dependent target open, recorded in §9.

## Document control

| Field | Value |
|---|---|
| Document status | Approved — 2026-10-21 |
| Version | 1.0 |
| Approved by | Product owner |
| Approval date | 2026-10-21 |
| Approval scope | Document approval only. Open `G-xx` decisions remain open, and per-requirement statuses are unchanged (§1.5, §9) |
| Governing change | `openspec/changes/add-prd-srs` (documentation-only, `skip_specs: true`) |
| Canonical language | English. Arabic summaries in `docs/ar/` are derivatives and never authoritative. |
| Companion document | `docs/PRD.md` (product intent). The PRD owns *why*; this SRS owns *what the system must do*. |

---

## 1. Introduction & Conventions

### 1.1 Purpose and scope

This SRS is the single authoritative description of Aref v1 behavior. It exists so that every OpenSpec change, every pull request, and every test can cite one stable requirement ID instead of re-deriving intent from prose.

In scope: functional behavior of the Detector, the Humanizer, provider and key handling, local persistence, and the UI shell with its validation and localization obligations; the non-functional targets that constrain them; the data model; the proposed IPC surface; the error catalog; the acceptance-criteria index; and the open decisions that remain.

Out of scope: product positioning, personas, success metrics, and roadmap (owned by the PRD); visual tokens and component styling (a separate change); deep internal design of detector features, prompt construction, and model architecture (each implementation change carries its own OpenSpec design). Nothing in this document derives from the design baseline, which does not exist yet.

### 1.2 Source precedence

When this document and a higher source disagree, the higher source wins and the conflict is escalated rather than resolved in prose. The order is the one fixed in `AGENTS.md` §2:

| Rank | Source | Role for this document |
|---|---|---|
| 1 | `AGENTS.md` | Engineering contract. Never contradicted here. |
| 2 | OpenSpec — `openspec/specs/` and the approved change | Captures the currently approved scope. |
| 3 | `docs/PRD.md`, `docs/SRS.md` (this file) | Product and software requirements. |
| 4 | Design baseline (produced by a separate change; cited by `AGENTS.md` §2) | Visual tokens. No requirement here derives from it. |
| 5 | Existing code conventions | Local consistency. |

Every requirement carries a `Traces` line naming the `AGENTS.md` section it derives from. A requirement without a trace is a defect in this document, not a new rule.

### 1.3 Requirement keywords

`MUST` and `MUST NOT` state absolute obligations. `SHOULD` states a recommendation that may be deviated from only with a recorded reason. `MAY` states a permitted option. These follow RFC 2119 and appear only in requirement statements, never in acceptance criteria.

### 1.4 Requirement IDs

| Range | Area |
|---|---|
| `FR-1xx` | Detection: local engine, scoring, explanation, Deep Scan, model integrity |
| `FR-2xx` | Humanization: strength, tone, chunking, retry loop, protection, reports |
| `FR-3xx` | Providers and keys: transport, pools, failover, rate limits, local discovery |
| `FR-4xx` | Storage, settings, migrations |
| `FR-5xx` | UI shell, navigation, input validation, i18n/RTL, screens |
| `NFR-1xx` | Performance and resource use |
| `NFR-2xx` | Security and privacy |
| `NFR-3xx` | Reliability and offline behavior |
| `NFR-4xx` | Accessibility and internationalization |

Rules: an ID is assigned once and never renumbered; a removed requirement becomes `Withdrawn` in place; a new requirement takes the next free number in its own range. Acceptance criteria carry the requirement ID as a prefix so the test name and the requirement are one string.

`FR-2xx` is humanization and `FR-3xx` is providers and keys. `AGENTS.md` §12 formerly grouped `FR-201..207` with `FR-301..304` under "skeleton + Providers & Security"; the approved correction was applied on 2026-10-21 and no FR range changed (`G-29`).

### 1.5 Requirement status vocabulary

| Status | Meaning |
|---|---|
| `Draft` | Written and testable; not withdrawn and not dependent on an open decision. Retained after document approval. |
| `Pending decision (G-xx)` | Part of the requirement depends on an unresolved decision recorded in §9. The remaining invariant stays testable; the dependent target is not invented. |
| `Withdrawn` | Removed from v1 scope, retained for ID stability. Not to be implemented. |

### 1.6 Document conventions

`MUST` statements describe observable behavior, not implementation. Numbers in this document are either fixed by `AGENTS.md` or marked `Pending decision (G-xx)`; none is estimated. A decision table inside §3 is normative and is referenced by the requirements around it.

---

## 2. System Overview

### 2.1 Local-first shape

Aref is a Windows desktop application built on Tauri 2 with a React 19 front end and a Rust core. There is no backend, no account, and no subscription. Detection is fully local. The only egress the application may produce is traffic to a provider endpoint that the user configured.

```
┌─────────────────────────────── Aref process ───────────────────────────────┐
│  React 19 + TypeScript (strict) + Tailwind CSS v4                          │
│    features/ detector humanizer providers settings                          │
│    ipc/  ← the only module allowed to call invoke or listen to events       │
│    i18n/ ar + en                                                            │
│                                  │ typed IPC                                 │
│  Rust core                       ▼                                          │
│    commands/   validate → call service → map error                          │
│    detector/   statistical features + ONNX classifier + explanation         │
│    humanizer/  chunking, retry loop, protected spans, reports               │
│    providers/  OpenAI-compatible client, key pool, rate limits              │
│    security/   keyring access, redaction                                    │
│    storage/    settings, migrations                                         │
└───────────────┬───────────────────────────────┬─────────────────────────────┘
                │ OS keyring                    │ user-configured HTTPS endpoint
         key material only              provider traffic only (never detection)
```

### 2.2 Boundaries and layering

The layering rule from `AGENTS.md` §5 is an architecture constraint, not a numbered requirement: `commands → services → infrastructure`. Commands contain no business logic; services do not import from commands. On the front end, only `src/ipc/` may call `invoke` or subscribe to Tauri events; components consume typed hooks. Where this boundary carries a product guarantee, the guarantee is stated as a requirement of its own (NFR-202).

Three boundaries carry product guarantees and are therefore specified as requirements rather than conventions:

- The front end never originates a provider request (NFR-202).
- Key material exists only in the OS keyring, and only a masked preview ever re-enters the UI (FR-310, NFR-201).
- The detection path performs no network I/O at all (FR-101, NFR-301).

### 2.3 Offline and egress model

| Path | Network behavior | Requirement |
|---|---|---|
| Detect | Zero network calls, including model loading and explanation generation | FR-101, NFR-301 |
| Detect, degraded | Zero network calls; the model is restored from the bundled copy, never fetched | FR-108, FR-109 |
| Humanize via cloud provider | Egress only to the endpoint the user configured; network loss affects this path only | NFR-302 |
| Humanize via local provider | No egress beyond loopback; works with the network disconnected | NFR-302 |
| Local provider discovery | Loopback only, ports 11434 and 1234, only after a user action | FR-304, NFR-206 |
| Everything else | No telemetry, analytics, crash reporting, or remote configuration | NFR-203 |

### 2.4 What v1 excludes

v1 is Windows-only (NFR-307), carries no code signing (NFR-308), supports OpenAI-compatible providers only (FR-301), and ships no history feature (`FR-403` is `Withdrawn`). Quotas are excluded (G-21).

---

## 3. Functional Requirements

Each requirement states one behavior, one normative construction, and at least one acceptance criterion written in the form it will carry as a test name. Decision tables in this section are normative.

### 3.1 Detection (FR-1xx)

Table 3.1-1 — score bands (applies to the overall score and to every per-sentence score)

| Score range | Band | Presentation consequence |
|---|---|---|
| 0–39 | none | No AI-likeness highlight |
| 40–59 | yellow | Yellow highlight |
| 60–100 | red | Red highlight |

**FR-101** · `Draft`

The system MUST return a detection result for any input that passes validation, and MUST perform no network I/O on the detection path.

- AC `FR-101: returns a detection result with network access disabled`
- Traces `AGENTS.md` §7.1 steps 1–4; §3.1

**FR-102** · `Draft`

The system MUST return the overall AI-likeness score as an integer in the inclusive range 0 to 100.

- AC `FR-102: returns an overall score within 0..=100`
- Traces `AGENTS.md` §7.1 step 3

**FR-103** · `Draft`

The system MUST return one score for every sentence it detects in the input.

- AC `FR-103: returns one score per detected sentence`
- Traces `AGENTS.md` §7.1 step 3

**FR-104** · `Draft`

The system MUST return a non-empty explanation with every detection result.

- AC `FR-104: returns a non-empty explanation with every result`
- Traces `AGENTS.md` §7.1 step 3

**FR-105** · `Draft`

The system MUST classify an overall score and every per-sentence score into the band defined by Table 3.1-1, so that 40 is yellow, 60 is red, and 39 is below yellow.

- AC `FR-105: classifies 40 as yellow`
- AC `FR-105: classifies 60 as red`
- AC `FR-105: classifies 39 as below yellow`
- Traces `AGENTS.md` §7.1 (thresholds)

**FR-106** · `Draft`

When Deep Scan is enabled, the system MUST compute the overall score as the median of N pass scores, MUST default N to 3, MUST accept a user-configured N only within 3 to 5 inclusive, MUST make every pass differ from the others by one defined mechanism, and MUST flag a sentence only when at least two passes agree.

- AC `FR-106: reports the median of the pass scores`
- AC `FR-106: flags a sentence only when two of three passes agree`
- AC `FR-106: rejects a pass count outside 3..=5`
- Traces `AGENTS.md` §7.1 (Deep Scan)
- Note the differing mechanism is `Pending decision (G-02)`. Running N identical passes is prohibited by `AGENTS.md` §7.1 and is not an acceptable implementation of this requirement.

**FR-107** · `Draft`

When the ONNX runtime fails to load, the system MUST continue in statistical-only mode, MUST show a visible banner, and MUST NOT crash.

- AC `FR-107: degrades to statistical-only when the ONNX runtime fails to load`
- Traces `AGENTS.md` §8 row "ONNX runtime fails to load"

**FR-108** · `Draft`

At startup the system MUST verify the checksum of the bundled detection model and, when the model is missing or corrupt, MUST restore it only from the copy bundled with the application.

- AC `FR-108: restores a corrupt model from the bundled copy`
- AC `FR-108: never downloads a model during restore`
- Traces `AGENTS.md` §8 row "Models missing or corrupt"; §13 item 1

**FR-109** · `Draft`

When model restoration fails, the system MUST degrade to statistical-only detection, MUST show a visible banner, and MUST present a reinstall message.

- AC `FR-109: shows the reinstall message when model restoration fails`
- Traces `AGENTS.md` §8 row "Models missing or corrupt"

**FR-110** · `Draft`

The base detection model MUST be bundled inside the installer and MUST be usable on the first launch with no network access.

- AC `FR-110: detects text on first launch with no network`
- Traces `AGENTS.md` §13 item 1

### 3.2 Humanization (FR-2xx)

Table 3.2-1 — chunk attempt loop (normative)

| Condition | Required action |
|---|---|
| Score at or below the acceptance threshold and both checks pass | Accept the attempt as a candidate |
| Score above the acceptance threshold | Retry with tighter instructions |
| Span check fails (FR-206) | Reject the attempt; it counts toward the attempt cap |
| Meaning check fails (FR-207) | Reject the attempt; it counts toward the attempt cap |
| Provider outcome invalid (timeout, interrupted stream, empty, `finish_reason=length`, refusal, wrong language) | Treat as a failed attempt; it counts toward the attempt cap |
| Two consecutive attempts fail to improve the score | Stop early |
| Attempt count reaches the configured maximum | Stop |
| At least one attempt passes both checks | Keep the lowest-scoring passing attempt |

Two invariants hold regardless of the run-level outcome: no failing attempt is ever emitted as the rewritten text, and the last attempt is never selected by recency. The run-level outcome when nothing passes — return the source text, warn on the best rejected attempt, or fail the chunk — remains `Pending decision (G-06)`.

The acceptance threshold value is `Pending decision (G-03)`; the default maximum attempt count within the 3–5 bound is `Pending decision (G-11)`.

**FR-201** · `Draft`

The Humanizer MUST expose rewrite strength as an integer on the 1 to 10 scale, MUST map the light, medium, and radical presets to 3, 6, and 9 respectively, and MUST let advanced settings edit the integer directly.

- AC `FR-201: maps the light preset to strength 3`
- AC `FR-201: rejects strength outside 1..=10`
- Traces `AGENTS.md` §7.2 step 1
- Note 3, 6, and 9 are the values `AGENTS.md` §7.2 step 1 suggests; fixing them as the v1 mapping is to be recorded in the PRD Decision Log.

**FR-202** · `Draft`

The Humanizer MUST include the tone the user selected in every chunk prompt it sends.

- AC `FR-202: includes the selected tone in every provider prompt`
- Traces `AGENTS.md` §7.2 step 1
- Note the set of selectable tone values is `Pending decision (G-05)`; this requirement constrains pass-through, not the vocabulary.

**FR-203** · `Draft`

The Humanizer MUST split input into chunks only at paragraph or sentence boundaries, MUST NOT split at a fixed character count, and MUST NOT expose chunking as a user-visible limit.

- AC `FR-203: splits input only at paragraph or sentence boundaries`
- AC `FR-203: exposes no chunk count or chunk-size limit in the UI`
- Traces `AGENTS.md` §7.2 step 2
- Note chunk boundaries stay an internal detail: neither a chunk count nor a chunk-size limit is a user-facing setting.

**FR-204** · `Draft`

For every chunk the Humanizer MUST build a prompt, request a rewrite from the configured provider, and score the returned text with the local Detector before that rewrite can be accepted.

- AC `FR-204: scores every provider rewrite with the local Detector before acceptance`
- Traces `AGENTS.md` §7.2 step 3

**FR-205** · `Draft`

While a chunk's score remains above the acceptance threshold, the Humanizer MUST retry with tighter instructions, MUST NOT exceed the configured maximum attempts, MUST accept a configured maximum only within the inclusive range 3 to 5, and MUST stop early when two consecutive attempts do not improve the score; every attempt rejected by FR-206, FR-207, or FR-309 MUST count toward that maximum.

- AC `FR-205: stops after two consecutive non-improving attempts`
- AC `FR-205: never exceeds the configured maximum attempts`
- AC `FR-205: counts a span-rejected attempt toward the cap`
- AC `FR-205: counts a truncated-output attempt toward the cap`
- Traces `AGENTS.md` §7.2 step 4; §8 rows "Timeout / stream interrupted" and "Empty / truncated / refused / wrong language"

**FR-206** · `Draft`

After every attempt the Humanizer MUST verify that numbers, names, code blocks, quotations, URLs, and any span the user marked protected are unchanged, and MUST reject an attempt that alters any of them.

- AC `FR-206: rejects an attempt that alters a protected number`
- AC `FR-206: rejects an attempt that drops a protected URL`
- Traces `AGENTS.md` §7.2 step 5
- Note how names are recognized as protected is `Pending decision (G-16)`; the obligation applies whether the set is detected or user-declared.

**FR-207** · `Draft`

After every attempt the Humanizer MUST compute semantic similarity against the source chunk and MUST reject the attempt when similarity falls below the configured threshold constant.

- AC `FR-207: rejects an attempt below the semantic-similarity threshold`
- Traces `AGENTS.md` §7.2 step 5; §13 item 4
- Note the threshold is a configuration constant, never a literal at the call site; its value is tied to the tuned detector thresholds (`AGENTS.md` §13 item 4).

**FR-208** · `Draft`

The Humanizer MUST keep the lowest-scoring attempt among those that pass both the protected-span check and the semantic-similarity check, and MUST NOT use recency as the selection rule.

- AC `FR-208: keeps the best passing attempt rather than the last`
- Traces `AGENTS.md` §7.2 step 6

**FR-209** · `Draft`

After all chunks are merged the Humanizer MUST run the Detector exactly once on the full merged text and MUST report that result as the final score.

- AC `FR-209: runs the full-text detector exactly once after merge`
- Traces `AGENTS.md` §7.2 step 7

**FR-210** · `Draft`

The Humanizer MUST emit one progress event per attempt carrying the chunk index, the attempt number, the score before the attempt, and the score after it.

- AC `FR-210: emits a progress event with before and after scores for every attempt`
- Traces `AGENTS.md` §7.2 step 8
- Note the remaining payload semantics are `Pending decision (G-17)`.

**FR-211** · `Draft`

The final report MUST label every score as being according to Aref's local detector and MUST NOT claim any result against a third-party detector.

- AC `FR-211: labels the report as Aref's local detector`
- AC `FR-211: contains no third-party detector claim`
- Traces `AGENTS.md` §7.2 step 8; §1 positioning

**FR-212** · `Draft`

The system MUST derive Humanize enablement from persisted configuration only — a usable cloud key and/or a local provider the user enabled through a user-initiated action — without any network request at decision time, and MUST disable Humanize with guidance to Settings when neither is present while Detect stays available.

- AC `FR-212: disables humanize when no provider is configured`
- AC `FR-212: keeps detect enabled with no provider`
- Traces `AGENTS.md` §7.2 step 9; §8 bullet "Local providers need no key"
- Note a local provider that is enabled but not reachable at the moment of the run is a failed attempt, not the disabled state; "no key" never means "cannot humanize" (FR-303).

**FR-213** · `Draft`

When the user cancels a run, or edits or replaces the input mid-run, the system MUST abort all in-flight provider requests, MUST keep the chunks already completed, and MUST return a partial report.

- AC `FR-213: aborts in-flight requests on cancel`
- AC `FR-213: returns completed chunks in a partial report`
- AC `FR-213: aborts in-flight requests when input is replaced mid-run`
- Traces `AGENTS.md` §8 row "User cancels, or edits/replaces the input mid-run"

### 3.3 Providers & Keys (FR-3xx)

Table 3.3-1 — HTTP 401 handling (normative)

| Step | Required behavior |
|---|---|
| 1 | Mark the responding key `invalid` |
| 2 | Fail over to the next key in the pool |
| 3 | Show a notice on the Providers screen |
| 4 | Retain the key; never delete it automatically |
| 5 | Send no further request with that key until the user edits it |

Table 3.3-2 — HTTP 429 handling (normative)

| Condition | Required behavior |
|---|---|
| `Retry-After` present and parseable | Wait for the stated duration |
| `Retry-After` absent or unparseable | Wait for an exponential backoff interval; base and cap are `Pending decision (G-13)` |
| Only some keys rate-limited | Fail over within the pool per FR-302 |
| All keys rate-limited | Show "try again in X minutes" with the computed wait time |

The header MUST be treated as optional and MUST NOT be assumed present.

**FR-301** · `Draft`

The system MUST call providers exclusively through OpenAI-compatible endpoints and MUST NOT add a provider-specific SDK.

- AC `FR-301: calls an OpenAI-compatible chat-completions endpoint`
- AC `FR-301: adds no provider-specific SDK dependency`
- Traces `AGENTS.md` §3.7; §8 bullet 1; §4
- Note how the model identifier for a provider is determined is `Pending decision (G-09)`.

**FR-302** · `Draft`

The system MUST support multiple keys per provider as a pool and MUST fail over to the next key when a key fails.

- AC `FR-302: fails over to the next key when one key fails`
- Traces `AGENTS.md` §8 bullet 2

**FR-303** · `Draft`

The system MUST permit a local provider to be used with no API key, and MUST NOT report "no provider configured" merely because the key pool is empty when a local provider is enabled in configuration.

- AC `FR-303: humanizes with a local provider and no key`
- AC `FR-303: does not report no-provider when a local provider is configured`
- Traces `AGENTS.md` §8 bullet 3

**FR-304** · `Draft`

Local-provider discovery MUST occur only after an explicit user action, MUST target only loopback ports 11434 for Ollama and 1234 for LM Studio, and MUST NOT scan the local network or probe in the background.

- AC `FR-304: probes only loopback ports 11434 and 1234`
- AC `FR-304: does not probe without user action`
- AC `FR-304: never contacts a non-loopback address`
- Traces `AGENTS.md` §3.6; §8 bullet 3

**FR-305** · `Draft`

On an HTTP 401 response the system MUST apply Table 3.3-1: mark that key invalid, fail over, and surface a notice in Settings.

- AC `FR-305: marks a key invalid on 401 and fails over`
- Traces `AGENTS.md` §8 row "401 / key rejected"

**FR-306** · `Draft`

The system MUST NOT delete an invalid key and MUST NOT send a request with it again until the user edits that key.

- AC `FR-306: keeps an invalid key stored`
- AC `FR-306: skips an invalid key until it is edited`
- Traces `AGENTS.md` §8 row "401 / key rejected"

**FR-307** · `Draft`

On an HTTP 429 response the system MUST apply Table 3.3-2, honoring `Retry-After` when present and otherwise applying exponential backoff, and when every key is rate-limited the UI MUST show the computed wait time.

- AC `FR-307: waits for Retry-After seconds`
- AC `FR-307: backs off exponentially without Retry-After`
- AC `FR-307: shows the wait time when all keys are rate-limited`
- Traces `AGENTS.md` §8 row "429 on all keys"

**FR-308** · `Draft`

On a network drop mid-run the system MUST retry with exponential backoff and, when the final retry fails, MUST return the chunks already completed.

- AC `FR-308: retries with exponential backoff after a network drop`
- AC `FR-308: returns completed chunks after the final network failure`
- Traces `AGENTS.md` §8 row "Network drop mid-run"

**FR-309** · `Draft`

The system MUST treat a timeout, an interrupted stream, empty output, `finish_reason=length`, a refusal, and wrong-language output as a failed attempt and MUST retry it under the rules of FR-205.

- AC `FR-309: retries truncated output as a failed attempt`
- AC `FR-309: retries wrong-language output as a failed attempt`
- AC `FR-309: retries a timeout as a failed attempt`
- Traces `AGENTS.md` §8 rows "Timeout / stream interrupted" and "Empty / truncated / refused / wrong language"
- Note the wrong-language criterion is `Pending decision (G-15)`. Script mixing alone MUST NOT be treated as wrong language.

**FR-310** · `Draft`

The system MUST store API keys only in the OS keyring and MUST return to the UI only a masked preview of any stored key.

- AC `FR-310: stores the key only in the OS keyring`
- AC `FR-310: returns only a masked preview after adding a key`
- Traces `AGENTS.md` §3.3; §8; §9
- Note the masking rule and its non-reversibility test are `Pending decision (G-27)`. A raw key may cross the IPC boundary once, from the key-entry field to the Rust core, and the front end MUST NOT retain or re-read it.

**FR-311** · `Draft`

The Providers screen MUST list the configured providers and their keys with masked previews, MUST allow adding, editing, and removing a provider and a key, and MUST display a notice for each key whose status is invalid.

- AC `FR-311: lists keys with masked previews`
- AC `FR-311: shows an invalid-key notice`
- Traces `AGENTS.md` §8 (multiple keys per provider; a rejected key shows a notice in Settings); §9 (every user-reachable failure has a designed UI state)

**FR-312** · `Draft`

When more than one provider is enabled in configuration, the system MUST select providers in the order OpenCode, Ollama, LM Studio, then cloud, and MUST NOT require a network request to make that selection.

- AC `FR-312: prefers an enabled local provider over a cloud key`
- AC `FR-312: selects a provider without any network request`
- Traces `AGENTS.md` §8 bullet 1
- Note an earlier draft of this criterion said "reachable"; selection reads enabled configuration, so the test asserts configured preference and no egress rather than a freshly observed provider (FR-212). The OpenCode endpoint and port are unknown (`Pending decision (G-19)`); no port and no probe rule is defined for OpenCode in v1.

**FR-313** · `Draft`

When the OS keyring is unavailable the system MUST block key storage with a clear error and MUST NOT fall back to any plaintext store.

- AC `FR-313: blocks key storage when the keyring is unavailable`
- AC `FR-313: writes no key to disk when the keyring is unavailable`
- Traces `AGENTS.md` §8 row "Keyring unavailable"; §3.3
- Note the behavior when existing keys cannot be read is `Pending decision (G-20)`; the system MUST NOT keep a cached or decrypted copy, and MUST NOT delete the key.

### 3.4 Storage, Settings & Migrations (FR-4xx)

**FR-401** · `Draft`

The system MUST persist user-editable settings across restarts, and MUST persist the UI language at minimum.

- AC `FR-401: restores the UI language after restart`
- Traces `AGENTS.md` §5 (`storage/`); §10 (i18n)
- Note the full settings inventory beyond language, provider configuration, and detection options is `Pending decision (G-12)`.

**FR-402** · `Draft`

A local store MUST NOT contain API key material, whether it is the settings store, a key-value store, or a SQL database.

- AC `FR-402: finds no key material in any local store`
- Traces `AGENTS.md` §3.3; §8
- Note the obligation is stated over every local store that exists, and history itself is deferred (`FR-403`), so no history store is in scope.

**FR-403** · `Withdrawn`

Withdrawn — history deferred (G-08). No history feature is specified for v1: no entity, no command, no event, no screen, no retention rule. The ID is reserved and MUST NOT be reused.

**FR-404** · `Draft`

The application MUST run schema migrations at startup and MUST preserve existing settings records across a migration.

- AC `FR-404: migrates an older schema without data loss`
- Traces `AGENTS.md` §5 (`storage/` migrations)

**FR-405** · `Draft`

The system MUST persist key metadata separately from key material, keeping the material only in the OS keyring.

- AC `FR-405: persists key status without persisting the key`
- Traces `AGENTS.md` §3.3; §5

### 3.5 UI Shell, i18n & Validation (FR-5xx)

Table 3.5-1 — input validation (normative)

| Input state | UI (FR-503) | Rust (FR-504) | Error |
|---|---|---|---|
| Empty or whitespace-only | Block before invoke, inline message, action disabled | Reject | `E_INPUT_EMPTY` |
| 1 to 5,000 words | Accept | Accept | — |
| More than 5,000 words | Block before invoke, inline message showing the limit | Reject | `E_INPUT_TOO_LONG` |

The word-count method is `Pending decision (G-14)`; the UI and the Rust validator MUST use one shared method, so that a text counted at the boundary yields the same verdict on both sides.

**FR-501** · `Draft`

The main window MUST open centered at 1240 by 820 logical pixels and MUST enforce a minimum size of 1000 by 680 logical pixels.

- AC `FR-501: opens centered at 1240x820`
- AC `FR-501: enforces a 1000x680 minimum`
- Traces `AGENTS.md` §10

**FR-502** · `Draft`

The application MUST provide sidebar navigation to the Detector, Humanizer, Providers, and Settings areas.

- AC `FR-502: navigates to all four areas from the sidebar`
- Traces `AGENTS.md` §10

**FR-503** · `Draft`

The UI MUST reject empty input and input exceeding 5,000 words for both Detect and Humanize before invoking the core, MUST show a clear message, and MUST use the same word-count method as the Rust validator.

- AC `FR-503: blocks detect on empty input`
- AC `FR-503: blocks humanize on 5001 words`
- AC `FR-503: counts words identically to the Rust validator on the shared fixture set`
- Traces `AGENTS.md` §7.1 step 1; §8 rows "Empty input" and "Input > 5,000 words"

**FR-504** · `Draft`

The Rust core MUST re-validate every detect and humanize request as non-empty and at most 5,000 words, using the same word-count method as the UI.

- AC `FR-504: rejects an empty detect_text request`
- AC `FR-504: rejects a humanize_text request over 5000 words`
- AC `FR-504: counts words identically to the UI validator on the shared fixture set`
- Traces `AGENTS.md` §8 rows "Empty input" and "Input > 5,000 words"; §9

**FR-505** · `Draft`

The application MUST switch the entire UI language between Arabic and English at runtime without a restart.

- AC `FR-505: switches the entire UI language without restart`
- Traces `AGENTS.md` §4 (i18n ar + en); §10

**FR-506** · `Draft`

The UI MUST render right-to-left in Arabic and left-to-right in English, and MUST use logical CSS properties rather than physical left and right.

- AC `FR-506: mirrors the layout in Arabic RTL`
- AC `FR-506: renders English LTR`
- Traces `AGENTS.md` §10

**FR-507** · `Draft`

Content of unknown direction MUST be rendered with `dir="auto"`, and mixed Arabic and English text and numbers MUST render with correct direction.

- AC `FR-507: applies dir=auto to mixed-direction content`
- Traces `AGENTS.md` §10

**FR-508** · `Draft`

Every user-reachable error MUST render a translated message and a next action derived from the error code, and MUST NOT display a stack trace or a raw message from the core.

- AC `FR-508: renders a translated message and next action for every error code`
- Traces `AGENTS.md` §9

**FR-509** · `Draft`

Each feature area MUST render the idle, empty, running, partial, done, and error states.

- AC `FR-509: renders all six states for detector and humanizer`
- Traces `AGENTS.md` §9; §11 (state transitions)

**FR-510** · `Draft`

The Detector screen MUST show the overall score, per-sentence highlights, the explanation, the detection mode in effect, and the Deep Scan option.

- AC `FR-510: shows per-sentence highlights and explanation`
- AC `FR-510: shows the statistical-only banner when degraded`
- Traces `AGENTS.md` §7.1 step 3; §8 row "ONNX runtime fails to load"

**FR-511** · `Draft`

The Humanizer screen MUST show per-attempt progress, the partial report after a cancel, and the final report carrying the local-detector label.

- AC `FR-511: renders per-attempt progress`
- AC `FR-511: renders the partial report after cancel`
- Traces `AGENTS.md` §7.2 steps 7–8; §8 row "User cancels"
- Note the final report field set is `Pending decision (G-18)`.

**FR-512** · `Draft`

The user MUST be able to mark any span of the input as protected, and the UI MUST send the marked spans with the humanize request.

- AC `FR-512: sends user-marked protected spans with the request`
- Traces `AGENTS.md` §7.2 step 5

---

## 4. Non-Functional Requirements

NFRs state measurable targets. Where `AGENTS.md` fixes no number, the target is marked `Pending decision (G-xx)`; no target in this document is estimated.

### 4.1 Performance (NFR-1xx)

**NFR-101** · `Pending decision (G-23)`

The system MUST run detector inference off the UI thread so that the interface stays responsive during a run.

- AC `NFR-101: the shell accepts input while a detection run is in progress`
- Traces `AGENTS.md` §7.1 step 2
- Note the latency budget is `Pending decision (G-23)`.

**NFR-102** · `Pending decision (G-23)`

The application MUST meet the startup-time and memory budgets set for v1.

- No acceptance criterion can be written until G-23 fixes the budgets; writing a number here would invent a product decision.
- Traces `AGENTS.md` §7.1 step 2; §13.

**NFR-103** · `Draft`

The bundled detection model MUST be delivered inside the installer and MUST occupy approximately 60 to 80 MB.

- AC `NFR-103: the bundled model adds between 60 MB and 80 MB to the installer`
- Traces `AGENTS.md` §13 item 1

### 4.2 Security & Privacy (NFR-2xx)

**NFR-201** · `Draft`

API keys MUST exist only in the OS keyring and MUST NOT appear in the settings store, a SQL database, `localStorage`, logs, error messages, or frontend state beyond a masked preview.

- AC `NFR-201: no key material appears outside the OS keyring`
- Traces `AGENTS.md` §3.3

**NFR-202** · `Draft`

The front end MUST NOT receive a stored raw key and MUST NOT originate a provider request; all provider traffic MUST go through the Rust core.

- AC `NFR-202: the frontend receives only a masked preview of any stored key`
- AC `NFR-202: no provider request originates from frontend code`
- Traces `AGENTS.md` §3.4; §10

**NFR-203** · `Draft`

The application MUST NOT include telemetry, analytics, crash reporting, or remote-configuration fetching.

- AC `NFR-203: contains no telemetry client, analytics endpoint, or remote-config fetch`
- Traces `AGENTS.md` §3.2

**NFR-204** · `Draft`

User text MUST NOT be logged; logs MAY contain only lengths, counts, durations, error codes, and provider names.

- AC `NFR-204: log output contains no user text across a detect run and a humanize run`
- Traces `AGENTS.md` §3.5; §9

**NFR-205** · `Draft`

Every error crossing IPC MUST be redacted of secrets and user text before it is logged or serialized, and MUST carry the envelope `code`, `message_key`, `params`.

- AC `NFR-205: serialized errors carry code, message_key, and params and no user text`
- Traces `AGENTS.md` §9

**NFR-206** · `Draft`

The application MUST NOT scan, enumerate, or probe the local network, and MUST limit discovery probes to loopback ports 11434 and 1234 after a user action.

- AC `NFR-206: performs no network probe without a user action and never leaves loopback`
- Traces `AGENTS.md` §3.6; §8 bullet 3

**NFR-207** · `Draft`

The repository, its tests, and its fixtures MUST NOT contain real secrets, real API keys, or real user text.

- AC `NFR-207: the repository contains no key-shaped string or real user text`
- Traces `AGENTS.md` §14

### 4.3 Reliability & Offline (NFR-3xx)

**NFR-301** · `Draft`

The detection path MUST perform zero network calls.

- AC `NFR-301: the detect path performs zero network calls under a denied-network harness`
- Traces `AGENTS.md` §3.1; §7.1 step 4

**NFR-302** · `Draft`

Humanize MUST work fully offline with a local provider; network loss MUST affect cloud providers only.

- AC `NFR-302: humanizes with a local provider while the network is disconnected`
- Traces `AGENTS.md` §8 bullet 4

**NFR-303** · `Pending decision (G-04)`

The detector's false-positive rate MUST NOT exceed 5% on the English evaluation corpus and 10% on the Arabic evaluation corpus.

- AC `NFR-303: the tuned detector reports an FPR at or below 5% English and 10% Arabic on the evaluation corpus`
- Traces `AGENTS.md` §7.1 (false-positive targets)
- Note no measured value exists in this document and none may be claimed; the corpus is absent (`Pending decision (G-04)`). This is a target, not a result.

**NFR-304** · `Pending decision (G-04)`

A detector or model change MUST NOT worsen the false-positive rate on the evaluation corpus, and every such change MUST report the before and after rate.

- AC `NFR-304: a detector change that worsens the false-positive rate fails the corpus gate`
- Traces `AGENTS.md` §7.1; §11

**NFR-305** · `Draft`

Tests MUST be deterministic and MUST run offline; provider behavior MUST be tested against a mock HTTP server and never against a real endpoint.

- AC `NFR-305: the test suite passes with the network disabled and contacts no real endpoint`
- Traces `AGENTS.md` §11

**NFR-306** · `Draft`

User input MUST NOT cause a panic or an `unwrap`/`expect`, and every failure MUST surface as one `AppError` variant carrying a stable `code` and the §7 envelope.

- AC `NFR-306: malformed and boundary inputs produce an AppError and never a panic`
- Traces `AGENTS.md` §9

**NFR-307** · `Draft`

v1 MUST target Windows only, MUST use `std::path` for path handling, and MUST NOT add macOS or Linux code paths or CI jobs.

- AC `NFR-307: the workspace builds for Windows and declares no non-Windows target`
- Traces `AGENTS.md` §3.8

**NFR-308** · `Draft`

The build MUST NOT include code signing or Authenticode steps.

- AC `NFR-308: the release pipeline contains no signing step`
- Traces `AGENTS.md` §3.9

### 4.4 Accessibility & i18n (NFR-4xx)

**NFR-401** · `Draft`

Every user-facing string MUST have an Arabic and an English entry, and MUST NOT be hard-coded in a component.

- AC `NFR-401: no user-facing string is hard-coded and every key exists in both locales`
- Traces `AGENTS.md` §4; §10

**NFR-402** · `Draft`

The UI MUST use logical CSS properties only, so that switching between RTL and LTR needs no physical-property override.

- AC `NFR-402: no physical left or right property appears in the styles`
- Traces `AGENTS.md` §10

**NFR-403** · `Draft`

Direction-unknown content MUST use `dir="auto"`, and numbers and mixed Arabic-English text MUST render correctly in both directions.

- AC `NFR-403: numbers and mixed-direction text render with correct direction in both locales`
- Traces `AGENTS.md` §10

**NFR-404** · `Pending decision (G-24)`

The UI MUST use semantic elements and MUST expose a visible focus state on every interactive control; the minimum contrast ratio is `Pending decision (G-24)`.

- AC `NFR-404: every interactive control is reachable by keyboard with a visible focus state`
- Traces `AGENTS.md` §10
- Note a full accessibility pass is deferred beyond v1, but no regressions are permitted.

**NFR-405** · `Pending decision (G-22)`

Quality effort — corpus, tuning, and review — MUST prioritize Arabic over English, and the English baseline quality bar is `Pending decision (G-22)`.

- AC `NFR-405: every quality gate defined for English has an equal or stricter Arabic counterpart`
- Traces `AGENTS.md` §1 (Arabic-first)

**NFR-406** · `Draft`

Meaning preservation MUST be gated by human review of 50 texts per language.

- AC `NFR-406: meaning preservation is reviewed by humans on 50 texts per language`
- Traces `AGENTS.md` §11

**NFR-407** · `Draft`

Any local store MUST be safe by default: it MUST NOT hold key material and MUST NOT hold user text beyond what the user explicitly chose to save, and it MUST NOT retain data the user did not ask to keep.

- AC `NFR-407: a fresh install persists no user text and no key material beyond what the user explicitly saved`
- Traces `AGENTS.md` §3.3; §3.5; §9

---

## 5. Data Model

Four entities exist in v1. There is no history entity: `FR-403` is `Withdrawn` (G-08).

### 5.1 Setting

| Field | Type | Notes |
|---|---|---|
| `key` | text, primary key | Namespaced, for example `ui.language` |
| `value` | JSON text | Never key material (FR-402) |
| `schema_version` | integer | Matches the last applied migration |
| `updated_at` | timestamp | Set on every write |

### 5.2 ProviderConfig

| Field | Type | Notes |
|---|---|---|
| `id` | text, primary key | Generated locally |
| `kind` | enum `openai_compatible` / `local` | Determines whether a key is required (FR-303) |
| `local_flavor` | enum `opencode` / `ollama` / `lm_studio`, nullable | Present only when `kind` is `local`; the discriminator the FR-312 order needs. No endpoint or port is defined for OpenCode (`Pending decision (G-19)`) |
| `base_url` | text | Loopback only for `local` kinds |
| `model_id` | text | How it is chosen is `Pending decision (G-09)` |
| `label` | text | User-facing name |
| `enabled` | boolean | Participation in provider selection (FR-312) |

### 5.3 KeyMetadata

| Field | Type | Notes |
|---|---|---|
| `id` | text, primary key | Generated locally |
| `provider_id` | text, references ProviderConfig | One provider, many keys (FR-302) |
| `label` | text | User-facing name |
| `masked_preview` | text | Display only; never reversible to the key (G-27) |
| `status` | enum `valid` / `invalid` | Set to `invalid` on 401 (FR-305) |
| `last_error_code` | text, nullable | From the §7 catalog |
| `invalid_since` | timestamp, nullable | Set with `status = invalid` |
| `updated_at` | timestamp | Set on every write; editing clears `invalid` (FR-306) |

### 5.4 Migration

| Field | Type | Notes |
|---|---|---|
| `version` | integer, primary key | Applied in order at startup (FR-404) |
| `applied_at` | timestamp | Set when the migration runs |

### 5.5 What must never be persisted

- Raw API keys, or any fragment from which a key can be reconstructed.
- Key material in logs, error payloads, error `params`, or any frontend store.
- User text in logs or error payloads.
- Prompt bodies or provider response bodies.
- Telemetry events, analytics identifiers, or remotely supplied configuration.
- Any user text the user did not explicitly ask to keep (NFR-407).

---

## 6. IPC Commands & Events

This surface is proposed. It is to be ratified by the architecture change that implements the skeleton; field names and the split between command responses and events are not yet binding. Only `src/ipc/` may call these commands or subscribe to these events.

### 6.1 Commands

| Command | Direction | Request (proposed) | Response (proposed) | Rules served |
|---|---|---|---|---|
| `detect_text` | UI → Rust | `text`, `deep_scan`, `passes` | `overall`, `sentences` (`index`, `span`, `score`), `explanation`, `mode`, `passes_used` | FR-101 to FR-107, FR-504, NFR-101 |
| `humanize_text` | UI → Rust | `text`, `strength`, `tone`, `protected_spans` (`start`, `end`), `max_attempts` | `run_id` | FR-201 to FR-206, FR-512, FR-504 |
| `cancel_humanize` | UI → Rust | `run_id` | `acknowledged` | FR-213 |
| `get_app_status` | UI → Rust | — | `detector_mode`, `humanize_available`, `provider_summary`, `app_version` | FR-107, FR-212, FR-509 |
| `list_providers` | UI → Rust | — | `providers` (`kind`, `local_flavor`, `base_url`, `model_id`, `label`, `enabled`) with nested `keys` metadata | FR-311, FR-312 |
| `add_provider` | UI → Rust | `kind`, `local_flavor` (when `kind` is `local`), `base_url`, `model_id`, `label`, `enabled` | ProviderConfig | FR-301, FR-304, FR-312 |
| `update_provider` | UI → Rust | `id` plus mutable fields: `label`, `enabled`, `base_url`, `model_id`, `local_flavor` | ProviderConfig | FR-311, FR-312 |
| `remove_provider` | UI → Rust | `id` | `removed` | FR-311 |
| `add_key` | UI → Rust | `provider_id`, `label`, `secret` | KeyMetadata (masked) | FR-310, FR-313 |
| `edit_key` | UI → Rust | `key_id`, `secret` | KeyMetadata (masked, `status = valid`) | FR-306, FR-310 |
| `remove_key` | UI → Rust | `key_id` | `removed` | FR-311 |
| `probe_local_providers` | UI → Rust | — | `found` (`kind`, `local_flavor`, `base_url`, `port`) | FR-304, NFR-206 |
| `get_settings` | UI → Rust | — | `settings` | FR-401 |
| `update_settings` | UI → Rust | `entries` (`key`, `value`) | `settings` | FR-401, FR-505 |

Notes. `humanize_text` returning a `run_id` with results delivered by events is one of two shapes consistent with `AGENTS.md` §7.2; the alternative — results in the command response — is equally valid and is left to the architecture change. `secret` is the only request field carrying key material and it is never echoed back. `model_id` is carried on the provider record, and how it is chosen or discovered is `Pending decision (G-09)`. `local_flavor` is what lets FR-312 tell OpenCode, Ollama, and LM Studio apart; no endpoint or port is asserted for OpenCode (`Pending decision (G-19)`).

### 6.2 Events

| Event | Direction | Payload (proposed) | Rules served |
|---|---|---|---|
| `humanize://progress` | Rust → UI | `run_id`, `chunk_index`, `attempt`, `before_score`, `after_score` | FR-210 (`Pending decision (G-17)` for the remaining semantics) |
| `humanize://completed` | Rust → UI | `run_id`, `final_text`, `final_score`, `chunks`, `label` | FR-209, FR-211 (`Pending decision (G-18)`) |
| `humanize://partial` | Rust → UI | `run_id`, `reason`, `completed_chunks`, `label` | FR-213, FR-308 |
| `humanize://failed` | Rust → UI | `run_id`, `error` (envelope) | FR-205, FR-208, FR-309 |
| `detector://degraded` | Rust → UI | `mode`, `reason` (`onnx_load_failed` / `model_restore_failed`) | FR-107, FR-109 |
| `providers://key_invalid` | Rust → UI | `provider_id`, `key_id`, `masked_preview` | FR-305, FR-311 |
| `providers://rate_limited` | Rust → UI | `provider_id`, `retry_after_seconds`, `resets_at` | FR-307 |

Every event payload that carries an error carries the §7 envelope, never a formatted sentence.

---

## 7. Error Codes

Errors cross IPC as `{ code, message_key, params }`. `params` carries values such as `limit`, `minutes`, `provider`, `masked`, and `reason`; it never carries user text or key material. The UI resolves `message_key` through the i18n catalog and renders the next action from the table below.

| Code | `message_key` | Trigger | Required UI state and next action | FR |
|---|---|---|---|---|
| `E_INPUT_EMPTY` | `error.input.empty` | Empty or whitespace-only input | Inline message at the input; the action stays disabled until the field is non-empty | FR-503, FR-504 |
| `E_INPUT_TOO_LONG` | `error.input.too_long` (`limit` = 5000) | More than 5,000 words | Inline message stating the limit; shorten the text before retrying | FR-503, FR-504 |
| `E_NO_PROVIDER` | `error.provider.none` (`reason`) | No usable cloud key and no enabled local provider | Humanize disabled with a Settings call to action: add a key or enable a local provider; Detect stays enabled | FR-212, FR-303 |
| `E_KEY_INVALID` | `error.provider.key_invalid` (`provider`, `masked`) | HTTP 401 | Notice on the Providers screen, failover already attempted; edit or replace the key, which is retained until the user edits it | FR-305, FR-306 |
| `E_RATE_LIMITED` | `error.provider.rate_limited` (`minutes`) | HTTP 429 on every key | Banner stating the wait time; retry offered after it elapses | FR-307 |
| `E_NETWORK_OFFLINE` | `error.provider.network_offline` | Network drop that survives the final retry | Partial report listing completed chunks; retry action | FR-308 |
| `E_PROVIDER_TIMEOUT` | `error.provider.timeout` | Provider request timed out | Attempt marked failed; retried while attempts remain, otherwise the run-level state applies | FR-309 |
| `E_STREAM_INTERRUPTED` | `error.provider.stream_interrupted` | Stream cut before completion | Attempt marked failed; retry per FR-205 | FR-309 |
| `E_OUTPUT_REJECTED` | `error.provider.output_rejected` (`reason` = `empty`, `truncated`, `refused`, `wrong_language`) | Invalid provider output | Attempt marked failed and retried automatically; no user action until the run ends | FR-309 |
| `E_HUMANIZE_FAILED` | `error.humanize.failed` | Attempts exhausted or no attempt passed both checks | Run-level failure state with a translated message and a retry action; the specific run-level outcome is `Pending decision (G-06)` | FR-205, FR-208 |
| `E_DETECTOR_DEGRADED` | `notice.detector.statistical_only` | ONNX runtime failed to load | Persistent banner; no action available — Detect continues in statistical-only mode | FR-107 |
| `E_MODEL_RESTORE_FAILED` | `error.detector.model_restore_failed` | Bundled model restore failed | Banner plus reinstall message; reinstall the app to restore the bundled model | FR-109 |
| `E_KEYRING_UNAVAILABLE` | `error.security.keyring_unavailable` | OS keyring unavailable | Block the save and explain the cause; unlock or repair the OS keyring and retry; no plaintext alternative | FR-313 |
| `E_INTERNAL` | `error.internal` | Unexpected failure | Translated generic message with a retry action; no stack trace | NFR-306 |

Every code above has exactly one `ar` entry and one `en` entry, and no other user-facing wording may be produced by the core (`AGENTS.md` §9).

---

## 8. Acceptance Criteria Index

Table 8.1 counts the acceptance criteria defined in §3. An `FR` with zero active criteria is `Withdrawn` and is listed for ID stability only.

### 8.1 Per-requirement counts

| FR | Area | Status | AC count |
|---|---|---|---|
| FR-101 | Detection | Draft | 1 |
| FR-102 | Detection | Draft | 1 |
| FR-103 | Detection | Draft | 1 |
| FR-104 | Detection | Draft | 1 |
| FR-105 | Detection | Draft | 3 |
| FR-106 | Detection | Draft | 3 |
| FR-107 | Detection | Draft | 1 |
| FR-108 | Detection | Draft | 2 |
| FR-109 | Detection | Draft | 1 |
| FR-110 | Detection | Draft | 1 |
| FR-201 | Humanization | Draft | 2 |
| FR-202 | Humanization | Draft | 1 |
| FR-203 | Humanization | Draft | 2 |
| FR-204 | Humanization | Draft | 1 |
| FR-205 | Humanization | Draft | 4 |
| FR-206 | Humanization | Draft | 2 |
| FR-207 | Humanization | Draft | 1 |
| FR-208 | Humanization | Draft | 1 |
| FR-209 | Humanization | Draft | 1 |
| FR-210 | Humanization | Draft | 1 |
| FR-211 | Humanization | Draft | 2 |
| FR-212 | Humanization | Draft | 2 |
| FR-213 | Humanization | Draft | 3 |
| FR-301 | Providers & keys | Draft | 2 |
| FR-302 | Providers & keys | Draft | 1 |
| FR-303 | Providers & keys | Draft | 2 |
| FR-304 | Providers & keys | Draft | 3 |
| FR-305 | Providers & keys | Draft | 1 |
| FR-306 | Providers & keys | Draft | 2 |
| FR-307 | Providers & keys | Draft | 3 |
| FR-308 | Providers & keys | Draft | 2 |
| FR-309 | Providers & keys | Draft | 3 |
| FR-310 | Providers & keys | Draft | 2 |
| FR-311 | Providers & keys | Draft | 2 |
| FR-312 | Providers & keys | Draft | 2 |
| FR-313 | Providers & keys | Draft | 2 |
| FR-401 | Storage & settings | Draft | 1 |
| FR-402 | Storage & settings | Draft | 1 |
| FR-403 | Storage & settings | Withdrawn | 0 |
| FR-404 | Storage & settings | Draft | 1 |
| FR-405 | Storage & settings | Draft | 1 |
| FR-501 | UI & i18n | Draft | 2 |
| FR-502 | UI & i18n | Draft | 1 |
| FR-503 | UI & i18n | Draft | 3 |
| FR-504 | UI & i18n | Draft | 3 |
| FR-505 | UI & i18n | Draft | 1 |
| FR-506 | UI & i18n | Draft | 2 |
| FR-507 | UI & i18n | Draft | 1 |
| FR-508 | UI & i18n | Draft | 1 |
| FR-509 | UI & i18n | Draft | 1 |
| FR-510 | UI & i18n | Draft | 2 |
| FR-511 | UI & i18n | Draft | 2 |
| FR-512 | UI & i18n | Draft | 1 |

Totals: 53 rows, 52 active functional requirements, 1 withdrawn, 89 acceptance criteria.

### 8.2 Coverage of the `AGENTS.md` §8 condition table

The §8 table has eleven rows; its "no provider configured" bullet is the twelfth condition that changes user-visible behavior. All twelve are covered.

| # | Condition | Requirements | Covered behavior |
|---|---|---|---|
| 1 | `429` on all keys | FR-307 | `Retry-After` when present, exponential backoff otherwise, wait time shown |
| 2 | `401` / key rejected | FR-305, FR-306 | Mark invalid, fail over, notice, retain, no retry until edited |
| 3 | Network drop mid-run | FR-308, FR-213 | Backoff retries, completed chunks returned on final failure |
| 4 | Timeout / stream interrupted | FR-309 | Failed attempt, retried under the loop rules |
| 5 | Empty / truncated / refused / wrong language | FR-309, FR-205 | Failed attempt, counts toward the cap, retried with tighter instructions |
| 6 | Cancel or input edit/replace mid-run | FR-213 | In-flight requests aborted, completed chunks kept, partial report |
| 7 | ONNX runtime fails to load | FR-107 | Statistical-only, visible banner, no crash |
| 8 | Models missing or corrupt | FR-108, FR-109 | Checksum at startup, restore from the bundled copy, reinstall message on failure |
| 9 | Keyring unavailable | FR-313 | Block key storage, clear error, no plaintext fallback |
| 10 | Empty input | FR-503, FR-504 | Rejected in the UI and again in Rust |
| 11 | Input over 5,000 words | FR-503, FR-504 | Rejected in the UI and again in Rust, with the limit stated |
| 12 | No provider configured | FR-212, FR-303 | Humanize disabled with Settings guidance, Detect stays enabled |

### 8.3 Coverage of the `AGENTS.md` §7.1 detection flow

| Step | Flow element | Requirements |
|---|---|---|
| 1 | UI validates then calls `detect_text` | FR-503, FR-504 |
| 2 | Statistical analyzer plus ONNX classifier on a blocking thread | FR-101, NFR-101 |
| 3 | Overall score, per-sentence scores, explanation | FR-102, FR-103, FR-104, FR-105 |
| 4 | No network I/O on this path | FR-101, NFR-301 |
| — | Thresholds 40 / 60 | FR-105 |
| — | Deep Scan: 3 passes, up to 5, median, two-pass agreement, passes differ | FR-106 |
| — | False-positive targets and no-regression rule | NFR-303, NFR-304 |
| — | Model delivery and integrity | FR-108, FR-109, FR-110, NFR-103 |

### 8.4 Coverage of the `AGENTS.md` §7.2 humanization flow

| Step | Flow element | Requirements |
|---|---|---|
| 1 | Strength presets and tone | FR-201, FR-202 |
| 2 | Paragraph or sentence chunking, never a character count, never a user-visible limit | FR-203 |
| 3 | Prompt, provider request, local scoring | FR-204 |
| 4 | Retry above threshold, 3 to 5 attempts, early stop | FR-205 |
| 5 | Protected spans and semantic similarity | FR-206, FR-207, FR-512 |
| 6 | Keep the best passing attempt | FR-208 |
| 7 | One detector run on the merged text | FR-209 |
| 8 | Progress events and the local-detector label | FR-210, FR-211 |
| 9 | No provider: Humanize disabled, Detect available | FR-212 |

### 8.5 Coverage of the `AGENTS.md` §9 error envelope

| Rule | Requirements |
|---|---|
| One `AppError` enum with a stable `code` | NFR-306 |
| No `unwrap` or `expect` outside tests, no panic on user input | NFR-306 |
| Errors carry `code`, `message_key`, `params` | NFR-205, FR-508 |
| The UI translates `message_key`, never a hard-coded sentence | FR-508, NFR-401 |
| Secrets and user text redacted before logging or serialization | NFR-204, NFR-205 |
| Every user-reachable failure has a designed UI state and next action | FR-508, FR-509, §7 |

### 8.6 Coverage of the `AGENTS.md` §3 non-negotiables

| # | Non-negotiable | Requirements |
|---|---|---|
| 1 | Detection works with zero network access | FR-101, NFR-301 |
| 2 | No telemetry, analytics, crash reporting, or remote config | NFR-203 |
| 3 | API keys only in the OS keyring | FR-310, FR-402, NFR-201 |
| 4 | Front end never sees a raw key and never calls a provider | NFR-202, FR-311 |
| 5 | User text is never logged | NFR-204 |
| 6 | Local-provider detection is loopback-only and user-initiated | FR-304, NFR-206 |
| 7 | OpenAI-compatible providers only, no provider SDKs | FR-301 |
| 8 | Windows-only v1 | NFR-307 |
| 9 | No code-signing setup | NFR-308 |

---

## 9. Open Questions

This is the only place where each unresolved item is defined in full; requirements reference its ID. Every requirement in §3 and §4 is written so that it stays testable while the rows below remain undecided.

Status values: `Resolved`, `Open`, `Deferred`. These describe gaps only; the requirement-status vocabulary is defined in §1.5 and is separate.

Gate values: `G1` document approval; `G2` first implementation change (skeleton); `G3` detector change; `G4` humanizer change; `G5` providers and security change; `G6` evaluation-corpus and tuning track; `G7` design change; `None` informative only.

| ID | Status | Question | Owner | Affected IDs | Blocking gate |
|---|---|---|---|---|---|
| G-01 | Resolved | Requirement ID ranges: confirmed on 2026-10-21 that the `FR-xxx` and `NFR-xxx` ranges follow `openspec/changes/add-prd-srs/design.md` §2. The separate `AGENTS.md` §12 citation correction was applied on 2026-10-21 (`G-29`) | Product | All FR and NFR ranges | None |
| G-02 | Open | What mechanism makes Deep Scan passes differ, given that the ONNX classifier is deterministic? | Detector track | FR-106 | G3 |
| G-03 | Open | What value does the humanizer acceptance threshold constant take? | Product + Detector track | FR-205, FR-208 | G4, G6 |
| G-04 | Open | The evaluation corpus is absent; which corpus, and therefore which measured FPR, applies to the 5% English and 10% Arabic targets? | Corpus track | NFR-303, NFR-304 | G6 |
| G-05 | Open | Which tone values are selectable, and how are they presented in Arabic and English? | Product | FR-202 | G4 |
| G-06 | Open | What is the outcome when no attempt passes the span and meaning checks — return the source text, return the best rejected attempt with a warning, or fail the chunk? | Product | FR-205, FR-208 | G4 |
| G-07 | Open | How is an enabled-but-unverified local provider presented in the interface? | Product | FR-212, FR-303 | G4, G5 |
| G-08 | Deferred | History is a v1 Won't and `FR-403` stays `Withdrawn`; whether history ships at all is deferred to a later decision | Product | FR-403 | None |
| G-09 | Open | How is a provider's model identifier determined — declared per provider entry, or read from an OpenAI-compatible models route? | Architecture | ProviderConfig, FR-301 | G5 |
| G-10 | Resolved | Attempt counting: span and meaning rejections and invalid provider outputs count toward the 3 to 5 cap, encoded in FR-205 | Product | FR-205 | None |
| G-11 | Open | Which default maximum attempt count is used inside the 3 to 5 bound? | Product | FR-205 | G4 |
| G-12 | Open | What is the full settings inventory beyond UI language, provider configuration, and detection options? | Product | FR-401 | G2 |
| G-13 | Open | What are the exponential backoff base and cap for 429 and network-drop retries? | Architecture | FR-307, FR-308 | G5 |
| G-14 | Open | Which word-count method is shared by the UI and the Rust validator? | Architecture | FR-503, FR-504 | G2 |
| G-15 | Open | How is wrong-language output defined? Script mixing alone must never reject an attempt | Product | FR-309 | G4 |
| G-16 | Open | How are names recognized as protected spans, given that numbers, code blocks, quotations, and URLs are structurally detectable? | Humanizer track | FR-206 | G4 |
| G-17 | Open | What are the remaining `humanize://progress` payload semantics beyond chunk index, attempt, and before/after scores? | Architecture | FR-210 | G4 |
| G-18 | Open | What fields does the final report carry besides the merged text, the final score, and the local-detector label? | Product | FR-211, FR-511 | G4 |
| G-19 | Open | What are OpenCode's endpoint and port? No port and no probe rule is defined for OpenCode in v1 | Architecture | FR-312 | G5 |
| G-20 | Open | What happens when the keyring is readable at write time but returns existing keys unreadable at run time? No cached copy is permitted and no key may be deleted | Security track | FR-313 | G5 |
| G-21 | Resolved | Quotas and usage accounting are excluded from v1; they are unsupported by the current architecture | Product | — | None |
| G-22 | Open | What is the English baseline quality bar, given that Arabic is the primary quality target? | Product | NFR-405 | G6 |
| G-23 | Open | What are the startup-time, memory, and detection-latency budgets? | Product + Architecture | NFR-101, NFR-102 | G2 |
| G-24 | Open | What minimum contrast ratio applies, and against which surface tokens? | Design | NFR-404 | G7 |
| G-25 | Resolved | Optional larger model downloads are roadmap only; v1 ships the bundled model and does not fetch models | Product | FR-110 | None |
| G-26 | Deferred | The license choice is deferred; the v1 action is settled as no LICENSE file and no license headers | Product | — | None |
| G-27 | Open | What is the masking rule, and which test proves a masked preview cannot be reversed to the key? | Security track | FR-310 | G5 |
| G-28 | Resolved | Missing `npm` and `cargo` scripts are owned by the first implementation change that needs them | Architecture | — | None |
| G-29 | Resolved | The `AGENTS.md` §12 build-order citation named `FR-201..207` as providers and security. The approved replacement line was applied on 2026-10-21, and no FR range changed | Product | All FR ranges | None |

Note on `G-07`. The enablement rule itself is decided and is not open: Humanize enablement is computed from persisted configuration only — a usable cloud key and/or a local provider the user enabled — with no network request at decision time, and a configured-but-unreachable provider is a failed attempt, not the disabled state (`FR-212`, `FR-303`). Only the presentation of an enabled-but-unverified provider remains open.

Note on `G-08`. History is a v1 Won't: no entity, command, event, screen, or retention rule exists in v1, and `FR-403` stays `Withdrawn`. The status is `Deferred` because the product decision to ship history at all has not been made — it is postponed, not settled.

---

End of document. This SRS is approved by the product owner and binding from 2026-10-21, while individual requirement statuses remain as defined in §1.5 and the open decisions in §9 remain open.
