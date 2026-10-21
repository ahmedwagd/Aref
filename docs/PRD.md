# Aref — Product Requirements Document (PRD)

> **Status: Approved — 2026-10-21.** Approved by the product owner on 2026-10-21. Document approval does not settle any open `G-xx` decision; those remain as recorded in `docs/SRS.md` §9. Requirement-level behavior, acceptance criteria, and error contracts live in `docs/SRS.md`; this document owns why, for whom, what, and how success is measured.

---

## 1. Summary

Aref is a local-first Windows desktop application for Arabic and English text. It does two jobs. The **Detector** scores how machine-written a piece of text reads: an overall score from 0 to 100, a score for every sentence, and a short explanation of why, produced entirely on the user's machine with no network access. The **Humanizer** rewrites the user's own text through a language model the user supplies — a cloud key they already own, or a model running locally — and iterates against the local detector until the result is acceptable or the attempt budget is spent.

There is no backend, no account, and no subscription. Nothing about the user's text leaves the machine on the detection path, and nothing is reported anywhere: no telemetry, no analytics, no crash reporting, no remote configuration. When the user humanizes, the only outbound traffic is to the provider they configured. API keys live only in the operating system keyring, and the interface only ever sees a masked preview.

v1 is Arabic-first. Quality effort — corpus, tuning, and human review — is spent on Arabic first, with English supported at a baseline level (`NFR-405`). Aref is a writing assistant: it helps a person revise their own draft. It does not promise, and cannot promise, any outcome with any other detector or any institution's process.

## 2. Problem

**Job one: knowing how a draft reads.** A writer who has used a language model to draft, outline, or polish their own text loses the ability to hear how it reads. The signals are local and sentence-level — a paragraph that is uniformly smooth, a rhythm that never varies, phrasing that is technically correct and slightly impersonal. A single number is not actionable; a writer needs to know *which* sentences carry the problem and *why*, or they end up rewriting the whole document. The judgment also has to be private: a draft under review, a client document, or an unpublished chapter cannot be pasted into an online service merely to be scored.

**Job two: revising without losing what matters.** Making text read more naturally is not a synonym swap. Numbers, names, dates, code, quotations, and URLs must survive untouched. The meaning must stay the author's meaning — this matters most in Arabic, where aggressive rewriting can produce fluent, natural-sounding prose that says something the author did not intend. A rewrite that lowers a score while drifting from the source is a worse outcome, not a better one. Doing this by hand is slow and repetitive; doing it with a single model call is unreliable, because the model has no way to check its own output.

**Why existing tooling falls short.** Most writing tools are English-first, and their notion of "natural" is trained and tuned on English prose. Arabic differs in morphology, in the density of clitics and affixes, in sentence length, and in how repetition functions rhetorically; signals that indicate machine authorship in English can fire on perfectly ordinary Arabic. A tool that is merely translated into Arabic will be confidently wrong in Arabic. Separately, the tools that do score AI-likeness typically require uploading the text to a service, which is the exact thing a privacy-sensitive writer cannot do.

## 3. Target Users & Personas

These are three working hypotheses drawn from the stated v1 scope. They are not research findings: no interviews, usage data, or market study back them. Each is labeled accordingly and is to be validated before any roadmap commitment depends on it.

### P1 — Arabic-first professional writer

> **Hypothesis — to be validated.**

- **Goal:** publish Arabic copy that reads like a person wrote it, and understand which sentences give it away when it does not.
- **Context:** writes Arabic daily for clients or an audience; occasionally works in English; revises the same text several times; often under time pressure.
- **What success looks like:** the score points at specific sentences, the explanation is understandable without statistics, and a rewrite keeps names, figures, and quotations exactly as written.
- **What would make them abandon it:** scores that feel arbitrary or unexplained; Arabic results that are visibly worse than English ones; any requirement to upload text to a service.

### P2 — Arabic-first student or academic writer

> **Hypothesis — to be validated.**

- **Goal:** revise their own draft — thesis chapter, essay, report — so it reads naturally and in their own voice, without altering citations, figures, or quoted material.
- **Context:** long documents, many protected fragments (citations, URLs, numbers, quoted passages), intermittent connectivity, and a strong aversion to losing meaning.
- **What success looks like:** protected fragments survive every rewrite, meaning is preserved, the work is possible offline, and nothing needs an account.
- **What would make them abandon it:** meaning drift, a broken citation or URL, an unclear failure with no way forward, or being asked to create an account.

This persona uses Aref on their own writing to improve clarity. Aref does not assess, predict, or influence how any institution evaluates a submission, and nothing in the product should suggest otherwise.

### P3 — Privacy-sensitive user with their own model

> **Hypothesis — to be validated.**

- **Goal:** get both jobs done without handing text to a third party, or while sending it only to a provider they already trust and pay for.
- **Context:** already has a cloud API key or runs a local model; comfortable configuring an endpoint; checks what a tool sends before trusting it.
- **What success looks like:** detection works with the network cable unplugged, humanization works against a local model, keys never appear in a file, and the app makes no call the user did not configure.
- **What would make them abandon it:** any telemetry, a key written to disk in plaintext, an unexplained outbound request, or a provider the tool insists on rather than one they chose.

## 4. Positioning

**Position:** Aref is a local-first, Arabic-first writing assistant that shows how machine-written your own text reads and helps you revise it — offline for detection, through your own model for rewriting.

| Aref is | Aref is not |
|---|---|
| A private detector of AI-likeness in text you wrote or are responsible for | A guarantee that text will pass, beat, evade, or defeat any detector |
| A revision loop that preserves protected content and checks meaning | A plagiarism checker, a grammar checker, or a fact checker |
| Bring-your-own-key and local-model by design | A hosted service that stores or processes your documents |
| Arabic-first in quality effort, with baseline English | A translated English product with Arabic strings |
| A writing assistant for your own drafts | A tool for misrepresenting authorship to an institution |

**The local-detector disclaimer.** Every score Aref reports is produced by Aref's local detector, and every humanizer report says so (`FR-211`). Aref has no knowledge of, access to, or relationship with any third-party detector, and makes no claim about how any other system would score the same text. A low score means "Aref's detector finds this text unlike machine-written text"; it does not mean "this will be accepted", "this will not be flagged", or "this is undetectable". No result is a guarantee against any third-party detector.

**The Arabic-first stance.** Arabic is the primary quality bar, not a translation target. Corpus work, threshold tuning, and human meaning review are sequenced so that Arabic is never the weaker language at release (`NFR-405`). Where a trade-off must be made, it is made in Arabic's favor, and the English baseline is stated as a baseline rather than implied parity.

## 5. v1 Scope

### 5.1 In scope

| Capability area | What ships | Requirement range |
|---|---|---|
| Detection | Offline scoring: overall score, per-sentence scores, explanation, 40/60 bands, optional Deep Scan, model integrity and graceful degradation | `FR-101` to `FR-110` |
| Humanization | Strength and tone controls, boundary-safe chunking, score-gated retry loop, protected spans, meaning guard, best-attempt selection, one full-text final score, progress, cancel and partial results | `FR-201` to `FR-213` |
| Providers and keys | OpenAI-compatible transport, key pools with failover, local providers without keys, user-initiated loopback discovery, rate-limit and network recovery, keyring-only storage, provider priority | `FR-301` to `FR-313` |
| Storage, settings, migrations | Persisted settings, no key material in any store, key metadata separate from key material, schema migrations | `FR-401`, `FR-402`, `FR-404`, `FR-405` |
| UI shell, i18n, validation | Window and navigation, input validation in UI and core, runtime ar/en switching, RTL/LTR, translated error states, detector and humanizer screens | `FR-501` to `FR-512` |

Non-functional constraints ship with these areas: performance and resource targets, security and privacy invariants, reliability and offline behavior, accessibility and internationalization (`NFR-101`–`NFR-103`, `NFR-201`–`NFR-207`, `NFR-301`–`NFR-308`, `NFR-401`–`NFR-407`).

### 5.2 Out of scope for v1

| Excluded | Why |
|---|---|
| History of past runs | A v1 **Won't**: no entity, command, event, screen, or retention rule exists, and the SRS records the corresponding requirement as withdrawn. Whether history ever ships is deferred (`G-08`). |
| Telemetry, accounts, subscriptions | Contradicts the product promise: no backend, no accounts, no subscription, no reporting (`NFR-203`). |
| Cloud sync of any kind | There is no backend and no account, so there is nothing to sync through. |
| Non-OpenAI provider SDKs | v1 speaks OpenAI-compatible HTTP only (`FR-301`). |
| macOS and Linux | Windows-only v1 (`NFR-307`); the code must not make other platforms impossible, but no paths or CI jobs are added. |
| Code signing | No Authenticode certificate exists; no signing step is added (`NFR-308`). |
| Guarantees against third-party detectors | Not knowable and not promised; every score is labeled as Aref's local detector (`FR-211`). |
| Quotas and usage accounting | Excluded by decision (`G-21`). |
| License choice | Deferred; no LICENSE file or headers in v1 (`G-26`). |

## 6. User Journeys

### 6.1 First launch: a detection result with the network off

1. The user installs Aref and opens it on Windows. The window opens centered at 1240×820 with a minimum of 1000×680, and a sidebar reaches Detector, Humanizer, Providers, and Settings (`FR-501`, `FR-502`).
2. On Detector, the user pastes a draft. Empty input is blocked in place; more than 5,000 words is refused with the limit shown (`FR-503`).
3. The core re-validates the same limits before doing work (`FR-504`).
4. Analysis runs off the UI thread: statistical features plus the bundled ONNX classifier, returning an overall score, one score per sentence, and an explanation (`FR-101`, `FR-102`, `FR-103`, `FR-104`, `NFR-101`).
5. The screen shows the overall score and per-sentence highlights banded at 39 below yellow, 40 yellow, 60 red, together with the detection mode in effect (`FR-105`, `FR-510`).
6. Optionally the user turns on Deep Scan: 3 passes by default, up to 5, median score, a sentence flagged only when at least two passes agree (`FR-106`).

**Success state:** a complete result with the network unavailable, because the detection path makes no network call at all (`FR-101`, `NFR-301`).

**Failure and degraded state:** if the ONNX runtime fails to load, the result is statistical-only with a visible banner and no crash (`FR-107`, `FR-510`). If the model is missing or corrupt, it is restored from the copy bundled with the app and never downloaded (`FR-108`); if restoration fails, the app shows a reinstall message and keeps working in statistical-only mode (`FR-109`).

### 6.2 Configuring a provider, then humanizing

1. The user opens Providers, which lists configured providers and keys with masked previews (`FR-311`).
2. Cloud route: the user adds an OpenAI-compatible endpoint and pastes a key. The key goes to the OS keyring and only a masked preview returns to the interface (`FR-310`). If the keyring is unavailable, the save is blocked with a clear error and no plaintext fallback exists (`FR-313`).
3. Local route: the user starts Ollama or LM Studio and asks Aref to look for it. Only loopback ports 11434 and 1234 are contacted, and only after that action (`FR-304`). No key is required (`FR-303`).
4. On Humanizer the user picks a strength — light 3, medium 6, radical 9, or any integer from 1 to 10 in advanced settings — and a tone (`FR-201`, `FR-202`).
5. The text is split at paragraph or sentence boundaries, never at a character count (`FR-203`). Each chunk gets a prompt, a provider request, and a local score before it can be accepted (`FR-204`).
6. Above the acceptance threshold, the loop retries with tighter instructions, never exceeding the configured maximum within 3 to 5, and stops early after two consecutive attempts that do not improve (`FR-205`).
7. After the chunks are merged, one detector run on the full text produces the final score, labeled as Aref's local detector (`FR-209`, `FR-211`).

**Success state:** a rewritten text with its final score and this run's attempt-by-attempt scores visible; when several providers are enabled, a local one is used before a cloud one (`FR-312`).

**Failure and degraded state:** a key rejected with 401 is marked invalid, failover continues on the next key, a notice appears in Settings, and the key is neither deleted nor tried again until the user edits it (`FR-305`, `FR-306`). If every key is rate-limited, the wait time is shown (`FR-307`). A network drop triggers exponential backoff, and a final failure returns the chunks already completed (`FR-308`).

### 6.3 A protected span and a rejected attempt

1. Before starting, the user marks spans that must not change — a date, a name, a citation (`FR-512`).
2. Attempt 1 returns a rewrite that alters the date and drops a URL. The attempt is rejected on the protected-span check and counts toward the cap (`FR-206`, `FR-205`).
3. Attempt 2 keeps both but drifts away from the source meaning; the similarity check rejects it, and it also counts (`FR-207`, `FR-205`).
4. Attempt 3 passes both checks and scores lower than any earlier passing attempt, so it is the one kept — not simply the last attempt produced (`FR-208`).
5. Throughout, each attempt reports its chunk, its number, and the score before and after, so the user can see the loop working (`FR-210`).

**Success state:** the kept rewrite preserves every protected fragment, and the final score comes from a single detector run on the merged text (`FR-209`).

**Failure and degraded state:** if the user cancels mid-run or replaces the input, in-flight requests are aborted at once, completed chunks are kept, and a partial report is returned (`FR-213`, `FR-511`). If no attempt passes both checks, no failing attempt is ever emitted and the last attempt is never chosen by recency; which run-level outcome applies — return the source text, warn on the best rejected attempt, or fail the chunk — is recorded as `G-06` in the SRS gap register (`FR-205`, `FR-208`).

### 6.4 Offline or degraded: what keeps working

1. With no network and no provider configured, Detect works exactly as before (`FR-101`, `NFR-301`), and Humanize is disabled with guidance to Settings (`FR-212`). "No provider" is never inferred from an empty key pool alone (`FR-303`).
2. With a local provider enabled, Humanize runs fully offline; only the cloud path is affected by the network (`NFR-302`).
3. If the ONNX runtime fails, detection continues in statistical-only mode with a banner, and the Detector screen states the mode in effect (`FR-107`, `FR-510`); if model restoration failed, a reinstall message accompanies the banner (`FR-109`).
4. Any failure surfaces as a translated message with a next action, never a stack trace and never a raw English string from the core (`FR-508`), in the language and direction the user selected (`FR-505`, `FR-506`).

**Success state:** detection is never blocked, and every degraded state still lets the user act.

**Failure and degraded state:** a local provider that is enabled but not running is a failed attempt with a retry path, not the disabled state (`FR-212`, `FR-308`); enablement is decided from persisted configuration with no network request at that moment.

## 7. Feature Set

Every feature below is defined in `docs/SRS.md`. The FR column is the contract; this table is the product view of the same behavior.

| Feature | What the user gets | FR IDs | Notes |
|---|---|---|---|
| Local detection | An overall AI-likeness score, a score for every sentence, and a short explanation, produced with no network | `FR-101`, `FR-102`, `FR-103`, `FR-104` | Zero network I/O on this path (`NFR-301`) |
| Risk bands | A readable verdict: 40 and above is yellow, 60 and above is red, below 40 is unmarked | `FR-105` | Bands apply to overall and per-sentence scores |
| Deep Scan | An optional, more careful reading: 3 passes by default, up to 5, median score, a sentence flagged when at least two passes agree | `FR-106` | The differing mechanism is `G-02`; identical repeats are prohibited |
| Detector integrity and degradation | A bundled model that works on first launch offline, a checksum check at startup, restore from the bundled copy, and a banner instead of a crash | `FR-107`, `FR-108`, `FR-109`, `FR-110` | Never downloads a model (`FR-108`) |
| Rewrite strength and tone | Presets light, medium, radical at 3, 6, 9, plus a direct 1–10 value in advanced settings, and a tone that is passed into every prompt | `FR-201`, `FR-202` | Tone vocabulary is `G-05` |
| Boundary-safe chunking | Long documents handled without cutting mid-sentence or mid-paragraph | `FR-203` | Chunking is internal and never shown as a limit |
| Score-gated rewrite loop | Retries with tighter instructions until the score is acceptable, bounded to 3–5 attempts, stopping early when two attempts in a row do not improve | `FR-204`, `FR-205` | Threshold value `G-03`; default maximum `G-11`; rejections count toward the cap (`G-10`) |
| Protected content and meaning guard | Numbers, names, code blocks, quotations, URLs, and any span the user marked survive; a rewrite that drifts in meaning is rejected | `FR-206`, `FR-207`, `FR-512` | Name recognition `G-16`; similarity threshold is a configuration constant |
| Best-attempt selection | The best passing rewrite is kept, not merely the last one produced | `FR-208` | Recency is explicitly not the rule |
| Final score and honest labeling | One detector run on the merged text, reported as Aref's local detector with no third-party claim | `FR-209`, `FR-211` | The label is a product requirement, not copy |
| Live attempt progress | Per-attempt feedback: which chunk, which attempt, score before and after | `FR-210` | Remaining payload semantics `G-17` |
| Availability without a provider | Humanize is disabled with a route to Settings, and Detect is untouched | `FR-212` | Enablement from persisted configuration only (`G-07` ruling) |
| Cancel and partial results | Stopping takes effect immediately, and finished chunks are not thrown away | `FR-213` | Partial reports carry the local-detector label |
| OpenAI-compatible transport | Works with any OpenAI-compatible endpoint the user already uses | `FR-301` | Model identifier handling `G-09` |
| Key pool and failover | Several keys per provider, automatic failover, and invalid keys marked rather than discarded | `FR-302`, `FR-305`, `FR-306` | A 401 never deletes a key |
| Local providers without keys | Ollama or LM Studio can be used with no API key at all | `FR-303` | "No key" never means "cannot humanize" |
| User-initiated loopback discovery | A deliberate search that contacts only the machine's own ports 11434 and 1234 | `FR-304` | No LAN scan, no background probing (`NFR-206`) |
| Rate limits, timeouts, and network recovery | A stated wait time instead of a mystery, backoff instead of a hard stop, and completed chunks returned on final failure | `FR-307`, `FR-308`, `FR-309` | Backoff base and cap `G-13`; wrong-language definition `G-15` |
| Key storage in the OS keyring | Keys never touch a file, a log, or an error message; only a masked preview is shown | `FR-310`, `FR-313` | Masking rule `G-27`; unreadable-keyring behavior `G-20` |
| Providers screen | One place to list, add, edit, and remove providers and keys, with notices when a key is invalid | `FR-311` | Masked previews only |
| Provider priority | The user's local setup is used before a cloud key | `FR-312` | Order: OpenCode, Ollama, LM Studio, then cloud; OpenCode endpoint `G-19` |
| Settings that survive restarts | Preferences, starting with interface language, persist across launches | `FR-401` | Full inventory `G-12` |
| Store safety | No key material in any local store, and key status stored separately from key material | `FR-402`, `FR-405` | Backed by `NFR-201` and `NFR-407` |
| Schema migrations | Upgrades keep existing settings instead of resetting them | `FR-404` | Runs at startup |
| App shell and navigation | A predictable window and a sidebar that reaches the four areas | `FR-501`, `FR-502` | 1240×820 default, 1000×680 minimum |
| Input validation | Empty or oversized input is refused early with a clear message, in both the interface and the core | `FR-503`, `FR-504` | Shared word-count method `G-14`; limit is 5,000 words |
| Language and direction | Switch between Arabic and English at runtime, with correct RTL and LTR layout and correct rendering of mixed text | `FR-505`, `FR-506`, `FR-507` | Logical CSS properties only |
| Error states | Every failure has a translated message and a next action; idle, empty, running, partial, done, and error are all designed | `FR-508`, `FR-509` | The twelve failure conditions in `AGENTS.md` §8 are mapped in SRS §8.2 |
| Detector screen | Overall score, per-sentence highlights, explanation, detection mode, Deep Scan control | `FR-510` | Statistical-only banner shown when degraded |
| Humanizer screen | Attempt progress, a partial report after cancelling, and a final report that names the local detector | `FR-511` | Final report fields `G-18` |

**Completeness check.** Thirty features. Fifty-two active functional requirements are mapped, zero unmapped: `FR-101`–`FR-110`, `FR-201`–`FR-213`, `FR-301`–`FR-313`, `FR-401`, `FR-402`, `FR-404`, `FR-405`, and `FR-501`–`FR-512`. The one withdrawn entry in the `FR-4xx` range is deliberately excluded and appears nowhere in this document. `FR-512` is cited under feature 8 because protecting a span is a user action whose obligation runs through both the interface and the pipeline.

## 8. Success Metrics

There is no telemetry, so every metric here is either a property that can be checked in the build, the tests, or the corpus — or it is not measured at all. No adoption, retention, or usage metric is defined for v1, because measuring them would require the one thing the product refuses to do.

| Metric | Target | How measured | Status |
|---|---|---|---|
| Detection works offline | Zero network calls on the detect path | Denied-network test harness (`NFR-301`) | Measurable now |
| False positives, English | At or below 5% on the evaluation corpus | Corpus evaluation (`NFR-303`) | Target only — corpus does not exist (`G-04`) |
| False positives, Arabic | At or below 10% on the evaluation corpus | Corpus evaluation (`NFR-303`) | Target only — corpus does not exist (`G-04`) |
| No false-positive regression | No detector or model change may worsen the rate; before and after reported | Corpus gate on every detector change (`NFR-304`) | Defined; cannot run until the corpus exists (`G-04`) |
| No telemetry | Zero telemetry, analytics, crash-reporting, or remote-config clients | Static inspection plus outbound capture (`NFR-203`) | Measurable now |
| No key material outside the keyring | Zero occurrences in any store, log, error, or frontend state | Store, log, and IPC inspection (`NFR-201`, `FR-402`) | Measurable now |
| Bilingual string coverage | 100% of user-facing strings have an `ar` and an `en` entry | i18n catalog check (`NFR-401`) | Measurable now |
| Meaning preservation | 50 human-reviewed texts per language | Human review panel (`NFR-406`) | Defined; not measurable until the corpus exists |
| Failure-state coverage | All twelve `AGENTS.md` §8 conditions have a designed UI state | Mapping table, SRS §8.2 (`FR-508`, `FR-509`) | Measurable now |
| Band correctness | 39 unmarked, 40 yellow, 60 red, on overall and per-sentence scores | Unit tests (`FR-105`) | Measurable now |
| Enablement without probing | No network request when deciding whether Humanize is available | Test with network denied (`FR-212`) | Measurable now |
| Arabic-first quality effort | Every quality gate defined for English has an equal or stricter Arabic counterpart | Gate inspection (`NFR-405`) | Measurable now for gate parity; the English baseline value is `Pending decision (G-22)` |
| Adoption and retention | None defined | Not measured — no telemetry by design | Not applicable in v1 |

## 9. Non-Goals

1. **History.** A v1 Won't: no list of past runs, no saved text, no retention rule. Whether it ever ships is deferred (`G-08`).
2. **Evasion and guarantees.** Aref is not built to pass, beat, evade, or defeat any detector, and never claims a result against one (`FR-211`).
3. **Cloud accounts and a backend.** No sign-in, no server, no user record.
4. **Telemetry and analytics.** No reporting of any kind (`NFR-203`).
5. **Subscriptions and pricing.** No paid tier, no license enforcement in the product.
6. **Provider-specific SDKs.** OpenAI-compatible HTTP only (`FR-301`).
7. **macOS and Linux.** Not in v1 (`NFR-307`).
8. **Code signing.** Not in v1 (`NFR-308`).
9. **A full accessibility pass.** Deferred to v2; regressions are still not permitted (`NFR-404`, contrast ratio `G-24`).
10. **Optional larger model downloads.** Roadmap only, not v1 (`G-25`); v1 ships one bundled model and fetches no model.

## 10. Roadmap

Nothing below carries a date or a commitment. Each item requires its own OpenSpec change and, where the item changes behavior, its own spec delta.

**v1 — the scope in §5.** Detection and humanization on Windows, Arabic-first, BYOK and local models, keys in the keyring, no network on the detection path.

**Candidates after v1, in no order:**

- **Bilingual parity.** Bring English up to the Arabic quality bar, which requires settling the English baseline bar (`G-22`).
- **Optional larger model download.** An opt-in model from Settings with hash verification, after int8 quantization has been tried (`G-25`).
- **History.** Only if the product owner reverses `G-08`; it would need its own retention and privacy design, since history is precisely the kind of local store that must stay safe by default.
- **Full accessibility pass.** Keyboard paths, focus order, contrast (`G-24`), and screen-reader behavior across both directions.
- **Other platforms.** Only if explicitly decided; v1 deliberately avoids making them impossible, which is not the same as supporting them.

## 11. Decision Log

Status vocabulary: `Decided` (the decision is made and applies to v1), `Deferred` (explicitly postponed), `Target only` (a target stands but cannot yet be verified).

| Date | Decision | Rationale | Source | Status |
|---|---|---|---|---|
| 2026-10-21 | Requirement ID ranges follow `openspec/changes/add-prd-srs/design.md` §2. The separate `AGENTS.md` §12 citation correction was applied on 2026-10-21 using the approved replacement line; no FR range changed | Stable IDs are what pull requests, tests, and future changes cite; the previous §12 grouping mislabeled the `FR-2xx` range | `G-01`, `G-29`; design.md §2 | Decided |
| 2026-10-21 | History is deferred and recorded as an explicit v1 non-goal | Keeps v1 to the two jobs, and avoids designing retention for user text before there is a reason to | `G-08` | Deferred |
| 2026-10-21 | Humanize enablement is computed from persisted configuration only — a usable cloud key and/or a local provider the user enabled — with no network request at decision time; a configured-but-unreachable local provider is a failed attempt, not the disabled state | The enable/disable decision must be offline, instant, and predictable; probing on startup or on click would break the no-hidden-network promise | `G-07` ruling; `FR-212`, `FR-303` | Decided |
| 2026-10-21 | Rejected outputs count toward the attempt cap: span rejections, meaning rejections, timeouts, truncation, refusals, and wrong-language output | An attempt that costs a provider call must cost budget, or the loop has no bound in practice | `G-10`; `FR-205` | Decided |
| 2026-10-21 | Strength presets are fixed at light 3, medium 6, radical 9 for v1 | The contract suggests these values; fixing them makes the control testable instead of interpretive | `FR-201`; `AGENTS.md` §7.2 step 1 | Decided |
| 2026-10-21 | Quotas and usage accounting are excluded from v1 | Not supported by the architecture and not part of the promise | `G-21` | Decided |
| 2026-10-21 | No LICENSE file and no license headers in v1 | The license decision is deferred; adding one now would pre-empt it | `G-26` | Deferred |
| 2026-10-21 | The false-positive targets of 5% English and 10% Arabic remain targets only; no measured value is claimed | The evaluation corpus does not exist yet, so any number would be invented | `G-04`; `NFR-303` | Target only |
| 2026-10-21 | Aref is a writing assistant; every score is Aref's local detector; no third-party detector claim anywhere in the product | Positioning and honesty are product requirements, not marketing copy | `AGENTS.md` §1; `FR-211` | Decided |
| 2026-10-21 | The base detection model is bundled in the installer at roughly 60–80 MB, with no download in v1 | Detection must work on first launch with no network and no hosting dependency | `AGENTS.md` §13 item 1; `FR-110` | Decided |
| 2026-10-21 | Keys exist only in the OS keyring, and all provider traffic goes through the Rust core | The front end never holds a stored key and never calls a provider, so no key can leak through UI state or logs | `AGENTS.md` §3; `NFR-201`, `NFR-202` | Decided |
| 2026-10-21 | `docs/PRD.md` and `docs/SRS.md` are approved by the product owner as documents | Records the approved baseline that later changes cite. Document approval does not resolve any open `G-xx` decision; those stay open in SRS §9, and per-requirement statuses are unchanged | User approval, 2026-10-21 | Decided |
