# Design

## Scope of this record

One section per decision, added in the order approved in `proposal.md` (`G-19` → `G-09` → `G-27` → `G-20` → `G-13`). This change is documentation-only: a section records a rule the product owner has already approved and states the exact wording that replaces the deferral in `docs/SRS.md`. It adds no requirement, defines no new behavior, and changes no requirement status. The `G-20` section also carries one completion — the missing `docs/SRS.md` target for a clause that decision already states. It is marked as a completion, is not a sixth decision, and fixes no new rule.

At this revision all five decisions in this record are settled, as of 2026-10-22: `G-19`, `G-09`, `G-27`, `G-20`, and `G-13`. No decision in scope remains open.

---

## G-19 — OpenCode endpoint and port

| | |
|---|---|
| **Status** | `Resolved` |
| **Date** | 2026-10-22 |
| **Approver** | Product owner (this session) |
| **Owner** | Architecture |
| **Affected requirements** | `FR-312` (primary); `FR-304` (its port set is unchanged and explicitly reaffirmed, not widened) |
| **Blocking gate** | `G5` |
| **SRS row** | line 1106; citations at lines 472, 838, 898 |

### Decision

OpenCode stays first in the `FR-312` selection order, but only as a user-configured provider entry: `kind` is `local`, `local_flavor` is `opencode`, the user supplies a loopback `base_url` and a `model_id`, no key is required, the entry is off by default, and Aref never probes for it. v1 gains no OpenCode endpoint, no OpenCode port, and no OpenCode probe rule; `FR-304`'s probe targets remain exactly 11434 (Ollama) and 1234 (LM Studio).

### Rationale

- **The order survives; the discovery question was never load-bearing.** `FR-312` names OpenCode first in the selection order, and the decision keeps that text true while removing the only genuinely open part — how Aref would locate the server. Configuration, not discovery, is what the requirement actually reads, so nothing about the order had to change.
- **`FR-312` forbids a network request at selection time.** Selection reads enabled configuration. A configured entry is readable without egress; a probe would make selection depend on a freshly observed provider, which is exactly what `FR-312`'s note at line 472 and its AC `selects a provider without any network request` exclude.
- **`FR-303` already defines keyless local operation.** A `local` entry with no key is not an exception; `FR-312`'s criterion `prefers an enabled local provider over a cloud key` and `E_NO_PROVIDER`'s trigger ("no usable cloud key and no enabled local provider") both read configured enablement, not keys.
- **`FR-304`'s probe surface is closed and stays closed.** The requirement enumerates two loopback ports and forbids LAN scanning and background probing. Adding 4096 would amend a requirement; this decision may not, and does not.
- **No new field, entity, or migration.** `ProviderConfig` already carries `local_flavor` ("the discriminator the FR-312 order needs"), `base_url` (loopback only for `local` kinds), and `model_id`. The approved rule is expressible with the fields that exist, and its data-model note at line 838 is a deferral, not a gap.
- **Evidence, verified against public sources.** `opencode serve` defaults to `127.0.0.1:4096`; it authenticates only when `OPENCODE_SERVER_PASSWORD` is set; it is user-started. Its API is a custom OpenAPI 3.1 surface (`POST /session/:id/message`) with no `/v1/chat/completions` — a request to add one was closed as not planned. The OpenAI-compatible `opencode.ai/inference` endpoint is hosted cloud. A default port is therefore an upstream convenience, not a contract: it can change without notice, and a decision that recorded it would be asserting a fact about a third party rather than a rule about Aref. The user-supplied `base_url` is the only form of the endpoint Aref can honestly hold.

### Alternatives considered

| Option | Content | Outcome and reason |
|---|---|---|
| **A** | OpenCode is a user-configured `kind=local` / `local_flavor=opencode` entry: loopback `base_url`, `model_id`, no key, off by default, never probed | **Selected.** Uses existing fields, satisfies `FR-312` (no egress), `FR-303` (no key), `FR-304` (no new port, no probing), `FR-301` (no provider SDK). |
| **B** | Drop OpenCode from the `FR-312` order; Ollama first | Rejected. It would rewrite an approved requirement rather than settle a gap: `FR-312`'s order and its citation of `AGENTS.md` §8 bullet 1 are approved text, and the user's need is to *use* OpenCode deliberately, not to remove it. |
| **C** | Probe loopback port 4096 during discovery | Rejected. It widens `FR-304`'s enumerated ports, introduces an automatic path where the SRS permits only a user-initiated one, risks false discovery (any listener on 4096 looks like OpenCode), and binds the product to an upstream default that the provider does not guarantee. |
| **D** | Treat OpenCode as a cloud provider | Rejected. It contradicts `FR-303` and `FR-312`'s local-first intent, and it would mislabel the hosted `opencode.ai/inference` path — a keyed, networked service — as if it were the local `opencode serve` the user runs. |

### Consequences

- **`FR-312`'s first entry is a blank, user-filled form until the user configures it.** With no such entry, the order degrades to Ollama → LM Studio → cloud with no special-casing, no auto-creation, and no "missing OpenCode" state: `get_app_status` simply reports what configuration holds, and Humanize decides availability from that (`FR-212`).
- v1 adds no entity, no field, no command, no event, no dependency, no port, and no default endpoint. `ProviderConfig` is unchanged.
- A configured entry is only usable once `model_id` has a value, which is why `G-09` gates completing the entry rather than merely validating it.
- An entry whose endpoint does not answer in an OpenAI-compatible shape is not a special case: it surfaces as a provider error and counts as a failed attempt under `FR-309`, exactly like any other configured provider that does not answer. Aref adds no adapter for OpenCode's native `POST /session/:id/message` surface, because that would be a provider-specific integration (`FR-301`).
- Security posture is unchanged: no key is stored for this kind, so nothing new enters the keyring, and no user text reaches a new destination unless the user configured it.

### Note — not a decision, and not in this change

A user-facing way to confirm that a configured OpenCode entry actually answers is worth having: a user who mistypes the port or points `base_url` at a dead process will otherwise discover it only after a humanize run fails. This change does **not** decide that, does not add a `test_provider` command, and defines no requirement ID for it. It is raised as a separate requirement under the future skeleton change, where the IPC surface is ratified; `docs/SRS.md` §6 already states that surface is proposed. Until then, the failure is covered by the existing error path (`FR-309`).

### Exact replacement wording for `docs/SRS.md`

Line numbers are as of the current file. Each entry states the current text and the text that replaces it, both verbatim.

#### Line 472 — `FR-312` note

Current (last two sentences of the note):

```text
The OpenCode endpoint and port are unknown (`Pending decision (G-19)`); no port and no probe rule is defined for OpenCode in v1.
```

Replace with:

```text
OpenCode is user-configured only (`G-19`, resolved 2026-10-22): an entry with `kind` `local` and `local_flavor` `opencode` carries a loopback `base_url` and a `model_id` supplied by the user, requires no key, and is disabled by default. Aref never discovers or probes for it; `FR-304`'s ports 11434 and 1234 remain the only probe targets, and no default endpoint, default port, or probe rule is defined for OpenCode in v1.
```

#### Line 838 — `ProviderConfig.local_flavor`

Current (last sentence of the cell):

```text
No endpoint or port is defined for OpenCode (`Pending decision (G-19)`).
```

Replace with:

```text
An `opencode` entry is user-configured only: the user supplies a loopback `base_url` and a `model_id`, no key is required, the entry is disabled by default, and it is never discovered or probed (`G-19`, resolved 2026-10-22).
```

#### Line 898 — §6.1 notes

Current (final clause of the paragraph, after the semicolon):

```text
no endpoint or port is asserted for OpenCode (`Pending decision (G-19)`).
```

Replace with:

```text
for OpenCode no endpoint, port, or probe rule is asserted, because the user supplies `base_url` and `model_id` on the provider entry and the core never fills them in (`G-19`, resolved 2026-10-22).
```

Keep the preceding semicolon, so the sentence reads: "…is what lets FR-312 tell OpenCode, Ollama, and LM Studio apart; for OpenCode no endpoint, port, or probe rule is asserted, because…".

#### Line 1106 — §9 row `G-19`

Current row:

```text
| G-19 | Open | What are OpenCode's endpoint and port? No port and no probe rule is defined for OpenCode in v1 | Architecture | FR-312 | G5 |
```

Replace with:

```text
| G-19 | Resolved | OpenCode has no default endpoint, no default port, and no probe rule in v1: it is a user-configured entry with `kind` `local` and `local_flavor` `opencode`, carrying a loopback `base_url` and a `model_id`, requiring no key, disabled by default, and never discovered by Aref. `FR-304` keeps ports 11434 and 1234 as the only probe targets. Decided 2026-10-22 | Architecture | FR-312, FR-304 | G5 |
```

`FR-304` is added to the affected-ID cell because the decision settles what the row is about — whether a third probe target exists — as no. No requirement text and no requirement status changes: `FR-304` and `FR-312` both remain `Draft`.

### Verification after applying

- Grepping `Pending decision \(G-19\)` in `docs/SRS.md` returns zero matches.
- The `G-19` row reads `Resolved`; lines 472, 838, and 898 no longer describe the OpenCode endpoint or port as unknown or undefined.
- `FR-304` at line 397 and `FR-312` at line 465 still read `Draft`, and line 397 still enumerates exactly ports 11434 and 1234.
- No other `G-xx` row changed, so the closing line 1124 ("the open decisions in §9 remain open") stays accurate.

---

## G-09 — Provider model identifier

| | |
|---|---|
| **Status** | `Resolved` |
| **Date** | 2026-10-22 |
| **Approver** | Product owner (this session) |
| **Owner** | Architecture |
| **Affected requirements** | `FR-301` (primary); `FR-312` (not amended — its "no network request at selection" criterion is the constraint the rejected alternatives would have broken) |
| **Blocking gate** | `G5` |
| **SRS row** | line 1096; citations at lines 380, 840, 898 |

### Decision

`model_id` is a **required** field on the provider entry, supplied by the user, and **authoritative**. Provider selection and every provider request read that one field from configuration. Aref never reads an OpenAI-compatible models route to decide what to send, and never chooses a model on the user's behalf. A user-initiated listing MAY fill the field as an assist, and the user still confirms the value that is saved. An unknown or wrong model name is not pre-validated: it surfaces as an ordinary provider failure and counts as a failed attempt under `FR-309`.

### Rationale

- **`FR-301` names the model, and it names it as a request field.** The requirement constrains Aref to OpenAI-compatible endpoints and forbids a provider-specific SDK. In that contract the model identifier appears in exactly one place — the `model` field of the chat-completions request body — so carrying it on the provider entry is the only shape that needs no provider knowledge and no second source of truth.
- **`FR-312` already forbids the network at selection time.** `selects a provider without any network request` is an approved criterion (line 470). Reading a models route to learn the model would make selection depend on a live call; reading configuration keeps that criterion true by construction rather than by exception.
- **A model list is a catalogue, not a contract.** Where a models route exists at all, its contents and ordering belong to the provider. Building selection on it would make *which* model rewrites the user's Arabic text depend on an upstream list, and would change silently when that list changed. That is the same class of error `G-19` refused for a default port: asserting a fact about someone else's server as if it were a rule about Aref.
- **Auto-selection removes the user's only lever.** Strength and tone are user choices; the model is the third. When the product picks, a bad rewrite has no attribution — the user cannot tell whether the prompt or the model is at fault, and cannot fix it from the interface.
- **A wrong name lands in a path that already exists.** `FR-309` and the §7 catalog already define what happens when a provider does not answer with usable output. A model name the server does not know returns an ordinary HTTP error and falls into the same path; honouring it costs no new mechanism.
- **No field, no migration, no command.** `ProviderConfig.model_id` already exists (line 840), and `list_providers`, `add_provider`, and `update_provider` already carry it (lines 887–889). The decision is expressible with the surface that is already documented.
- **Evidence and confidence.** `[Fact]` `FR-312`'s criterion at line 470 reads `selects a provider without any network request`, and `model_id` is already present in §5.2 and §6.1. `[Assumption]` An OpenAI-compatible server is not guaranteed to implement a models route, and where it does, the contents and order are the provider's to change; this is plausible rather than verified server by server, and the decision does not depend on it — the rule holds whether or not such a route exists.

### Alternatives considered

| Option | Content | Outcome and reason |
|---|---|---|
| **A** | `model_id` required on the provider entry, user-supplied, authoritative; selection and every request read configuration only; no models route is read on any path | **Selected.** Uses the field that already exists, keeps `FR-312`'s no-network criterion true, keeps `FR-301` free of provider-specific knowledge, and leaves the user in control of the one input that decides rewrite quality. |
| **B** | Selection reads the provider's `GET /v1/models`, picks a model, and sends it | Rejected. Breaks `FR-312`'s `selects a provider without any network request` and makes provider usability depend on a live provider reaching a route it may not implement. It also demands a choice rule — first entry, newest, largest — which is product policy this decision may not set, and which would change which model runs without the user acting. |
| **C** | `model_id` optional at save time; the core resolves it lazily on the first request and remembers it | Rejected. It moves a configuration error into a runtime failure and leaves no place for the user to record intent. It also needs a resolved/unresolved state and a second persisted value on `ProviderConfig` — a data-model change with a migration — and `G-09` settles how the identifier is determined; it may not add a field. |
| **D** | Aref ships curated default model names per flavour (one for Ollama, one for LM Studio, one for cloud) and falls back to them when the field is blank | Rejected. A default model name is the same kind of claim as a default port (`G-19`): a fact about a third party that goes stale without notice. With Ollama it fails in the user's face — a name that has not been pulled returns a not-found that reads as a provider outage. Neither `FR-301` nor `FR-312` asks for a default. |

### Consequences

- **One source of truth.** The value the user typed decides the `model` field of every request, the value `list_providers` displays, and the value named when a run fails. There is no reconciliation, and no "which model did it actually use?" question left for a later change to answer.
- **Cloud and local are symmetric.** No per-flavour default, no per-provider catalogue, no provider-specific branch — which is what `FR-301` requires in the first place, and it keeps the `G-19` decision intact: an `opencode` entry and an Ollama entry are filled the same way.
- **Required means the entry cannot be completed without it.** `add_provider` and `update_provider` already list `model_id` among their request fields, so a blank value is rejected by the command's validation step before any service is called — the `validate → call service → map error` shape of `AGENTS.md` §5. No error code is asserted here: the §7 catalog has none for this case and this decision may not add one, so which existing code it maps to is settled when the skeleton change ratifies §6.
- **A typo costs one failed attempt, not a rejected form.** Because the field is not validated against a live catalogue, a wrong model name is discovered at the first request: an ordinary provider failure under `FR-309`, retried within the attempt cap and reported through the normal error path. This is the accepted cost of keeping every path offline-capable and provider-agnostic; validating against the list would reintroduce the network dependency option B was rejected for.
- **The listing assist is deferred, and its bounds are fixed now.** `docs/SRS.md` §6.1 lists no command that returns a model list, and this decision adds none — no command row, no command name, no requirement ID. If the assist is built, it is **one user-initiated command in a later change**, and this decision already constrains it: it runs only on a user action, it reads only the provider the user selected, it *fills* the field rather than saving it, the user confirms the value, and the request goes through the Rust core like any other provider traffic, so no raw key reaches the front end (`NFR-202`).
- **The field-only path is complete on its own.** The skeleton change may ship providers with the required `model_id` and no listing surface at all; that is this decision implemented, not a partial version of it. Nothing in the skeleton depends on the assist existing, and shipping the field first violates nothing here.
- **Nothing new is contacted, stored, or transmitted.** No endpoint is called from the providers screen, no keyring item, table, or column is added, and no migration is required.
- **`FR-301` and `FR-312` remain `Draft`, unamended.** This record settles a deferral; it changes no requirement text, status, or acceptance criterion.

### Note — what this decision does not add

No IPC command, no event, no error code, no persisted field, no dependency, and no library. The listing assist has no identifier and no home in the current surface; naming one here would be an assertion a later change is entitled to make and this one is not. `docs/SRS.md` §6 already states that the command surface is proposed and is ratified by the change that implements the skeleton.

### Exact replacement wording for `docs/SRS.md`

Line numbers are as of the current file. Each entry states the current text and the text that replaces it, both verbatim.

#### Line 380 — `FR-301` note

Current:

```text
- Note how the model identifier for a provider is determined is `Pending decision (G-09)`.
```

Replace with:

```text
- Note the model identifier for a provider is a required, user-supplied `model_id` on the provider entry and is authoritative: selection and every request read that field from configuration, never an OpenAI-compatible models route, and no model is ever chosen on the user's behalf (`G-09`, resolved 2026-10-22). A user-initiated listing MAY fill the field as an assist, and an unknown or wrong model name is an ordinary provider failure under FR-309.
```

#### Line 840 — `ProviderConfig.model_id`

Current (the Notes cell only):

```text
How it is chosen is `Pending decision (G-09)`
```

Replace with:

```text
Required, supplied by the user on the provider entry, and authoritative: selection and every request read this field, never a models route, and no model is auto-selected (`G-09`, resolved 2026-10-22)
```

The cell carries no trailing period in the current file, and the replacement keeps it that way — the same style applied to line 838 under `G-19`.

#### Line 898 — §6.1 notes

Current (one sentence of the paragraph, which continues after it):

```text
`model_id` is carried on the provider record, and how it is chosen or discovered is `Pending decision (G-09)`.
```

Replace with:

```text
`model_id` is carried on the provider record, is supplied by the user, and is authoritative for every request; no command in this surface lists, selects, or infers a model, and a listing assist, if one is added in a later change, is user-initiated and only fills the field (`G-09`, resolved 2026-10-22).
```

The sentence that follows it, beginning "`local_flavor` is what lets FR-312 tell OpenCode, Ollama, and LM Studio apart", is unchanged.

#### Line 1096 — §9 row `G-09`

Current row:

```text
| G-09 | Open | How is a provider's model identifier determined — declared per provider entry, or read from an OpenAI-compatible models route? | Architecture | ProviderConfig, FR-301 | G5 |
```

Replace with:

```text
| G-09 | Resolved | A provider's model identifier is a required, user-supplied `model_id` on the provider entry and is authoritative: provider selection and every request read that field from configuration, never an OpenAI-compatible models route, and no model is ever chosen on the user's behalf. A user-initiated listing MAY fill the field as an assist; an unknown or wrong model name is an ordinary provider failure under FR-309. Decided 2026-10-22 | Architecture | ProviderConfig, FR-301 | G5 |
```

Owner, affected-ID cell, and gate are unchanged. Unlike `G-19` — which added `FR-304` because it settled whether a third probe target exists — this row adds no ID: the decision settles how an existing field is populated and amends no requirement.

### Verification after applying

- Grepping `Pending decision \(G-09\)` in `docs/SRS.md` returns zero matches.
- The `G-09` row reads `Resolved`, with owner `Architecture`, affected IDs `ProviderConfig, FR-301`, and gate `G5`, all unchanged.
- Lines 380, 840, and 898 state how the model identifier is determined instead of deferring it, and line 898's following sentence is still intact.
- `FR-301` at line 373 and `FR-312` at line 465 still read `Draft`; no `AC FR-301:` line is added, removed, or reworded.
- The closing sentence of §9 ("the open decisions in §9 remain open") is still accurate, because every out-of-scope row remains `Open` (this bullet predates the settlement of `G-27`, `G-20`, and `G-13`; it no longer lists any settled decision as open).

---

## G-27 — Masking rule and its non-reversibility test

| | |
|---|---|
| **Status** | `Resolved` |
| **Date** | 2026-10-22 |
| **Approver** | Product owner (this session) |
| **Owner** | `software-architect`, which owns all five rows of this change, the security-track ones included (`proposal.md` line 11, approved 2026-10-22) |
| **Affected requirements** | `FR-310` (primary); `NFR-201` (added: a preview that can be reversed to the key is key material outside the keyring, which is what `NFR-201` forbids) |
| **Blocking gate** | `G5` |
| **SRS row** | line 1114; citations at lines 455 and 851 |

**Owner cell, and why it does not contradict this header.** The §9 row's owner cell stays `Security track`: this change rewrites no owner cell, and that cell names the area the question belongs to. The 2026-10-22 approval names who decides it. The replacement row at line 1114 below leaves the cell exactly as it is, so the two records agree rather than conflict.

### Decision

The masked preview is the key's last four characters behind a mask of fixed width: a constant number of mask characters, one repeated mask character, never scaled to the key's length, and covering every character before the tail — the key's prefix included. It is derived once, from the value submitted to `add_key` or `edit_key`, and stored in `KeyMetadata.masked_preview`. No path re-reads a stored key to produce it or refresh it.

Non-reversibility is a property of the mapping, not a promise about the output string: two keys that differ only outside the last four characters produce byte-identical previews, and a mapping that returns the same string for two different inputs cannot be inverted to either of them.

### Rationale

- **The preview is the only key-derived value that leaves the keyring.** `FR-310` and `NFR-201` both turn on that boundary, so every character the preview shows is a channel that has to be justified. The tail earns its four characters because the tail is what a user recognises a key by; everything else is masked, so nothing else escapes.
- **A mask proportional to the key leaks its length.** Length is not the secret, but it narrows a search space and it makes two keys comparable by the shape of their previews alone — a comparison the user never asked for and the keyring never intended. A fixed width removes the channel instead of shrinking it.
- **The prefix is the least informative part of a key.** In every format that has one, the leading characters are a constant, identical across keys of that format. Spending visible characters on them would identify nothing, and leaving them visible would make the mask's width depend on the format — the opposite of fixed width.
- **Deriving once at save keeps the key out of every later read.** A preview recomputed by re-reading the stored key would take key material out of the keyring on each listing and each startup, purely for display, and would make the preview's availability depend on the keyring's readability. Deriving it from the value already in hand at save is the only shape where the raw key crosses the boundary exactly once, as the surviving sentence at line 455 requires, and where later rendering touches no secret.
- **The pair test is the proof that can actually be run.** Byte-identical previews for two keys differing outside the shown characters is deterministic, offline, needs no keyring and no network, and asserts non-injectivity — precisely what non-reversibility requires. Weaker tests do not survive contact with the definition: "the preview does not contain the key" fails, because the tail is part of the key, and "the shown characters cannot reconstruct the key" needs an attacker model and a key-space bound this record cannot fix.
- **Four characters are a recognisability budget, not a security boundary.** The rule does not claim four characters are safe to reveal in every threat model; it fixes how many are revealed and guarantees that no other character of the key is.

### Alternatives considered

| Option | Content | Outcome and reason |
|---|---|---|
| **A** | Fixed-width mask plus the last four characters, prefix masked, derived once at save | **Selected.** Uses the field that already exists, keeps every later render secret-free, and gives the non-reversibility test a property that is decidable offline. |
| **B** | Mask sized to the hidden characters, so the preview's width tracks the key's length | Rejected. It leaks length, makes previews of different keys comparable by shape, and turns a display string into a partial description of the secret — the channel `NFR-201` exists to close. |
| **C** | Show the first four and the last four, the familiar key-manager pattern | Rejected. The leading characters of a formatted key are a format constant, so the extra visible characters identify nothing; and because prefix length varies by format, the mask's width would vary with the key's format, which the fixed-width rule forbids. |
| **D** | Show no key-derived characters at all: one constant mask for every key | Rejected. `FR-311` lists keys with masked previews, and a preview with no key-derived character cannot tell two keys of the same provider apart. The user's only recovery would be deleting and re-adding keys to find out which was which. |
| **E** | Keep a reversible form of the key — an encrypted local copy, or a "reveal" action that reads the keyring on demand | Rejected. Any reversible store contradicts `FR-310`, `NFR-201`, and §5.5's first entry; a reveal action would still re-read the stored key, which this decision excludes, and would put key material into the front end a second time. |

### Consequences

- **What the user loses.** The preview shows four characters and nothing else, so two keys of one provider whose tails coincide look identical in the Providers screen; the label is then the only way to tell them apart. The preview also stops carrying any format hint: a user cannot confirm from it that a value looked like the format it was meant to be, because the prefix is masked like everything before the tail. Both are the accepted cost of a preview that reveals a fixed amount rather than an amount proportional to what it hides.
- **What must never be displayed.** Any character of the key other than the last four; the key's prefix; the key's length, directly or through the mask's width; and any preview produced by re-reading a stored key. The `providers://key_invalid` payload carries the stored `masked_preview` (line 909) — that value, never a fresh derivation from the key.
- **Nothing new is stored.** `masked_preview` already exists on `KeyMetadata` (line 851), and `add_key` and `edit_key` already return `KeyMetadata` (lines 891–892). The preview is written on save alongside the metadata, and no column, table, migration, command field, or event field is added.
- **One writer, one rule.** `add_key` and `edit_key` are the only paths that produce a preview, and both produce it from the request's `secret` before that value is stored. Renaming a key, changing a provider, listing providers, or a 401 never recomputes it.
- **Mask width and mask character are named constants**, not literals at a call site — `AGENTS.md` §4 and §14 applied to the one value this decision leaves open. Their values are set with the screen that renders them (`docs/design/DESIGN.md` tokens); this record fixes only that both are constant.
- **No requirement text, status, command, event, error code, dependency, library, or schema field changes.** `FR-310` and `NFR-201` both remain `Draft`; the four criteria below are added to a block that already exists, and the two criteria at lines 452–453 are untouched.

### Note — outside this decision

- A secret shorter than the shown tail is outside the rule's domain: the pair test quantifies over keys that differ outside the shown characters, and no such pair exists below that length. A minimum secret length is already a validation concern of `add_key` (`AGENTS.md` §5), and its value is not set here.
- No reveal, copy-to-clipboard, or "show key" affordance is added or removed by this decision. Any future one would re-read the keyring, which this decision excludes, and would need its own record.

### Exact replacement wording for `docs/SRS.md`

Line numbers are as of the current file. Each entry states the current text and the text that replaces it, both verbatim.

#### Lines 448–455 — `FR-310` block, four new acceptance criteria

Current: the block carries two criteria at lines 452–453, the `Traces` line at 454, and the note at 455. Insert the four lines below **after line 453 and before the `Traces` line at 454**, so the block keeps its criteria → traces → note order. No existing line is reworded or removed:

```text
- AC `FR-310: shows a fixed-width mask that does not vary with key length`
- AC `FR-310: shows only the last four characters of the key and no part of its prefix`
- AC `FR-310: returns byte-identical previews for two keys that differ only outside the last four characters`
- AC `FR-310: computes the preview once when the key is saved and never re-reads a stored key to recompute it`
```

#### Line 455 — `FR-310` note

Current (both sentences):

```text
- Note the masking rule and its non-reversibility test are `Pending decision (G-27)`. A raw key may cross the IPC boundary once, from the key-entry field to the Rust core, and the front end MUST NOT retain or re-read it.
```

Replace with (first sentence replaced; the raw-key sentence is carried over verbatim and stays last):

```text
- Note the masked preview is the key's last four characters behind a mask of fixed width that does not vary with key length and includes no part of the key's prefix (`G-27`, resolved 2026-10-22); it is computed once when the key is saved, is never recomputed by re-reading the stored key, and is not reversible to the key because two keys that differ only outside those four characters produce byte-identical previews. A raw key may cross the IPC boundary once, from the key-entry field to the Rust core, and the front end MUST NOT retain or re-read it.
```

#### Line 851 — `KeyMetadata.masked_preview`

Current (the Notes cell only):

```text
Display only; never reversible to the key (G-27)
```

Replace with:

```text
Display only, computed once at key save: a fixed-width mask that does not vary with key length and includes no part of the key's prefix, followed by the key's last four characters; never reversible to the key, because two keys that differ only outside those four characters produce byte-identical previews (`G-27`, resolved 2026-10-22)
```

The cell carries no trailing period in the current file, and the replacement keeps it that way — the same style applied to lines 838 and 840.

#### Line 1114 — §9 row `G-27`

Current row:

```text
| G-27 | Open | What is the masking rule, and which test proves a masked preview cannot be reversed to the key? | Security track | FR-310 | G5 |
```

Replace with:

```text
| G-27 | Resolved | The masked preview is the key's last four characters behind a mask of fixed width that does not vary with key length and includes no part of the key's prefix: it is computed once when the key is saved and is never recomputed by re-reading the stored key. Non-reversibility is proved by the criterion that two keys that differ only outside those four characters produce byte-identical previews. Decided 2026-10-22 | Security track | FR-310, NFR-201 | G5 |
```

`NFR-201` is added to the affected-ID cell because the masking rule is what keeps the preview from being key material outside the keyring. The owner cell stays `Security track` (see the owner note above) and the gate is unchanged. No requirement text and no requirement status changes: `FR-310` at line 448 and `NFR-201` at line 656 both remain `Draft`.

### Verification after applying

- Grepping `Pending decision \(G-27\)` in `docs/SRS.md` returns zero matches.
- The `G-27` row reads `Resolved`, with owner `Security track`, affected IDs `FR-310, NFR-201`, and gate `G5`.
- The `FR-310` block carries six `AC FR-310:` lines: the two originals at lines 452–453 unchanged, the four above inserted before the `Traces` line, and the `Traces` line still immediately before the note.
- Line 455's raw-key sentence is byte-identical to the current file's text and is still the closing sentence of the note.
- Line 851 still asserts non-reversibility, now through the pair criterion, and `masked_preview` remains the only key-derived field on `KeyMetadata`.
- `FR-310` at line 448 and `NFR-201` at line 656 still read `Draft`; no other `G-xx` row changed status; line 7 above carries the current settled/open split.
- The closing bullet of the `G-09` section was corrected on 2026-10-22 to drop `G-27` and `G-20` from the rows it lists as `Open`, because both are settled now; line 7 is the description of this revision.

---

## G-20 — Stored keys the keyring cannot read

| | |
|---|---|
| **Status** | `Resolved` |
| **Date** | 2026-10-22 |
| **Approver** | Product owner (this session) |
| **Owner** | `software-architect`, which owns all five rows of this change, the security-track ones included (`proposal.md` line 11, approved 2026-10-22) |
| **Affected requirements** | `FR-313` (primary); `NFR-201` (added: a cached or decrypted copy of a key the core cannot read is key material outside the keyring, which is what `NFR-201` forbids) |
| **Blocking gate** | `G5` |
| **SRS row** | line 1111; citations at lines 478–485, 856, 922, 938, 1023 |

**Owner cell, and why it does not contradict this header.** The §9 row's owner cell stays `Security track`: this change rewrites no owner cell, and that cell names the area the question belongs to. The 2026-10-22 approval names who decides it. The replacement row at line 1111 below leaves the cell exactly as it is, so the two records agree rather than conflict.

### Decision

A stored key the OS keyring cannot read is **unusable but not `invalid`**. The core fails over past it: it contributes no candidate to selection, and no request is attempted with it, because no value can be obtained to send. When no stored key can be read, the run surfaces the existing `E_KEYRING_UNAVAILABLE` with `params.reason` `unreadable` and asks the user to unlock or repair the OS keyring. The key is retained with its `status` unchanged, is never cached or decrypted, and is never deleted. A write that reads back unreadable is the same case: it is not reported as a success, and it is not undone by deleting the key.

The error is what replaces `E_NO_PROVIDER` in that situation, not a state that overrides selection: where `FR-303` and `FR-312` would select a keyless local provider, the run proceeds there and no keyring error is raised.

### Rationale

- **`invalid` is a verdict about the credential, not about the store.** `FR-305` sets it on a 401 — the provider rejected the key — and `FR-306` keeps it until the user edits the key. Reporting a locked keyring as `invalid` would tell the user their key is wrong when the fault is in the OS store, and would leave that wrong verdict persisted after the keyring is repaired, because nothing in the read path writes.
- **Unreadability is a per-read condition, not a stored fact.** The same key becomes readable the moment the user unlocks the keyring, and may be readable on one attempt and not the next. A state that is true only at read time has no honest representation as a column, and this decision may not add one: `status` is `valid` / `invalid` (line 856).
- **Both workarounds are already forbidden.** A cached copy and a decryptable local copy are what `NFR-201`, `FR-310`, and §5.5 (line 870) exclude, so "never cached or decrypted" is not a new rule but the existing one applied to a failure the SRS had not named.
- **The catalog and the envelope already carry the state.** `E_KEYRING_UNAVAILABLE` exists at line 938 with the next action this case needs ("unlock or repair the OS keyring and retry"); `params` already admits `reason` (line 922), and two rows use it that way (`E_NO_PROVIDER`, `E_OUTPUT_REJECTED`). No code, key, or field has to be invented.
- **Failover needs no new branch.** `FR-302` already skips a key that cannot serve; a key whose value cannot be read is the degenerate case of that. Treating it as a candidate that fails would be worse: it would spend a request and an attempt on a key that was never usable.
- **The asymmetry favours refusing to persist a claim.** Calling an unreadable key `invalid` makes the visible remedy "edit or replace the key" — the wrong action, and one the user may be unable to perform while the keyring is still unreadable. Calling it usable produces a provider error that misdescribes the cause. Storing neither claim keeps "unlock or repair the keyring" in front of the user, which is the action that actually resolves it.

### Alternatives considered

| Option | Content | Outcome and reason |
|---|---|---|
| **A** | Unreadable key is unusable but not `invalid`; fail over past it; `E_KEYRING_UNAVAILABLE` with `reason` `unreadable` when none can be read; retained, status unchanged, never cached/decrypted/deleted | **Selected.** States the condition where it is true — at read time — using the code, the param, and the selection rule that already exist. |
| **B** | Mark the key `invalid` and reuse the 401 path | Rejected. `invalid` means the provider rejected the credential (`FR-305`), carries `invalid_since` and `last_error_code`, and is sticky until the user edits the key (`FR-306`). It would persist a claim the core cannot verify, show the invalid-key notice (`FR-311`, line 929) whose remedy is "edit or replace the key" — not the remedy here — and stay wrong after the keyring is unlocked. |
| **C** | Add a third `status` value `unreadable`, or a new `E_KEY_UNREADABLE` code | Rejected. It needs a schema value, a migration, and a §7 row, none of which this change may add; and it would still be a persisted snapshot of a condition that changes on the next read without anyone writing the row. The information the user needs is already carried by `reason` on the existing code. |
| **D** | Keep a decrypted copy in memory or a local store so a read cannot fail | Rejected. It contradicts `NFR-201`, `FR-310`, and §5.5 line 870, and converts a recoverable keyring state into standing key material outside the keyring — what the security track exists to prevent. Deleting the key to force a re-add is worse still: it destroys a possibly valid secret. |

### Consequences

- **What the user sees.** The Providers screen still lists the key with the `masked_preview` and `status` already stored, because listing reads metadata and never the keyring (`G-27`: the preview is computed once at save). An unreadable key does not blank the screen and gains no badge this decision defines. On a run, unreadable keys are skipped; if a local provider is selected the run proceeds; if the pool holds keys and none can be read, Humanize fails with `E_KEYRING_UNAVAILABLE` (`reason` `unreadable`) and the next action is to unlock or repair the OS keyring and retry (line 938).
- **What `invalid` continues to mean.** Exactly one thing: the provider rejected the key with a 401 (`FR-305`), the key is retained (`FR-306`), and the Providers screen shows the notice fed by `providers://key_invalid` with the stored `masked_preview`. No unreadable key enters that state, and no `providers://key_invalid` event is emitted for one.
- **Nothing new is added.** No error code, no schema field, no status value, no migration, no command, no event, no dependency. `KeyMetadata` is unchanged, `status` stays `valid` / `invalid` (line 856), `E_KEYRING_UNAVAILABLE` is reused verbatim, and `reason` is an existing `params` value (line 922) — the §7 row at line 938 is unchanged, because "OS keyring unavailable" already describes this trigger. That row's UI-state cell is written for the write path ("block the save"); at read time the part that applies is its next action, "unlock or repair the OS keyring and retry". A read-time-specific wording would be an SRS edit outside this decision's approved text and is not made here.
- **The write path is the same case, not a new one.** `add_key` and `edit_key` that read back unreadable surface the same error instead of reporting success, and the key is not removed as a rollback — the user's secret is never destroyed by a failed read. The preview is still derived from the submitted value at save, so producing it needs no read-back.
- **Detect is unaffected.** The detect path opens no keyring item and makes no network call (`AGENTS.md` §7.1, §3.1); a keyring the app cannot read does not disable Detect, does not degrade detection, and emits no `detector://degraded` event.
- **`FR-313` remains `Draft`.** This record settles a deferral: the requirement text is unchanged and three criteria are added to a block that already exists.

### Note — outside this decision

- No badge, label, or message wording is defined here; the state renders through the existing code's `message_key` and next action, and a distinct visual treatment would be a design decision with its own record.
- Nothing is decided about how long a keyring read may block, or about retrying the read within one run; the SRS defines no timeout for keyring access and this decision adds none.
- Whether `last_error_code` records this condition is not decided here. The field exists and is nullable, and no requirement constrains it for a key that is not `invalid`.

### Exact replacement wording for `docs/SRS.md`

Line numbers are as of the current file. Each entry states the current text and the text that replaces it, both verbatim.

#### Lines 478–485 — `FR-313` block, three new acceptance criteria

Current: the block carries two criteria at lines 482–483, the `Traces` line at 484, and the note at 485. Insert the three lines below **after line 483 and before the `Traces` line at 484**, so the block keeps its criteria → traces → note order. No existing line is reworded or removed:

```text
- AC `FR-313: fails over past a stored key the keyring cannot read without marking it invalid`
- AC `FR-313: reports E_KEYRING_UNAVAILABLE with reason unreadable when no stored key can be read and retains each key with its status unchanged`
- AC `FR-313: never caches, decrypts, or deletes a key it cannot read, and treats a write that reads back unreadable the same way`
```

#### Line 485 — `FR-313` note, first clause

Current (both clauses):

```text
- Note the behavior when existing keys cannot be read is `Pending decision (G-20)`; the system MUST NOT keep a cached or decrypted copy, and MUST NOT delete the key.
```

Replace with (first clause replaced; the clause after the semicolon is carried over byte-identical and stays last):

```text
- Note the behavior when existing keys cannot be read is that such a key is unusable but not `invalid`: the core fails over past it, its status is left unchanged, and when no stored key can be read the run reports `E_KEYRING_UNAVAILABLE` with `reason` `unreadable` and asks the user to unlock or repair the OS keyring (`G-20`, resolved 2026-10-22); the system MUST NOT keep a cached or decrypted copy, and MUST NOT delete the key.
```

#### Line 1111 — §9 row `G-20`

Current row:

```text
| G-20 | Open | What happens when the keyring is readable at write time but returns existing keys unreadable at run time? No cached copy is permitted and no key may be deleted | Security track | FR-313 | G5 |
```

Replace with:

```text
| G-20 | Resolved | A stored key the keyring cannot read is unusable but not `invalid`: the core fails over past it, keeps its status unchanged, and never caches, decrypts, or deletes it. When no stored key can be read, the run reports `E_KEYRING_UNAVAILABLE` with `reason` `unreadable` and asks the user to unlock or repair the OS keyring; a write that reads back unreadable is the same case. Decided 2026-10-22 | Security track | FR-313, NFR-201 | G5 |
```

`NFR-201` is added to the affected-ID cell because the "never cached, never decrypted" clause is what keeps an unreadable key from becoming key material outside the keyring — the same reason `G-27` added it. The owner cell stays `Security track` and the gate is unchanged. No requirement text, status, or other row changes: `FR-313` at line 478 and `NFR-201` at line 660 both remain `Draft`.

### Verification after applying

- Grepping `Pending decision \(G-20\)` in `docs/SRS.md` returns zero matches.
- The `G-20` row reads `Resolved`, with owner `Security track`, affected IDs `FR-313, NFR-201`, and gate `G5`.
- The `FR-313` block carries five `AC FR-313:` lines: the two originals at lines 482–483 unchanged, the three above inserted before the `Traces` line, and the `Traces` line still immediately before the note.
- Line 485's second clause is byte-identical to the current file's text: `; the system MUST NOT keep a cached or decrypted copy, and MUST NOT delete the key.`
- `FR-313` at line 478 and `NFR-201` at line 660 still read `Draft`; line 938 is unchanged, so `E_KEYRING_UNAVAILABLE` keeps one catalog row and one `message_key`.
- §8.2 row 9 (line 1023) still describes the keyring condition as "Block key storage, clear error, no plaintext fallback"; this decision adds the read-time branch without amending that row's requirement link.
- No other `G-xx` row changed status, so the closing sentence of §9 ("the open decisions in §9 remain open") stays accurate for the out-of-scope rows (corrected 2026-10-22: it previously listed `G-13`, which the section below settles).

### Completion — the keyless-local clause in the line 488 note (added 2026-10-22)

**Not a new decision.** This supplies the `docs/SRS.md` target that the `G-20` list above omitted for a rule the approved decision already states. The omission is recorded in `tasks.md` (task 4.1 evidence): the design defined no target for that sentence, so none was applied.

**The gap.** The Decision's second paragraph reads: "The error is what replaces `E_NO_PROVIDER` in that situation, not a state that overrides selection: where `FR-303` and `FR-312` would select a keyless local provider, the run proceeds there and no keyring error is raised." The first Consequences bullet says the same ("if a local provider is selected the run proceeds"). The note applied at line 488 carries the `E_KEYRING_UNAVAILABLE` rule without that clause, so its condition — "when no stored key can be read" — can be read as ending a run that would otherwise proceed to a keyless local provider.

**The fix, and its bounds.** One contiguous insertion inside the existing clause. The retention rule, the no-cache rule, the do-not-delete rule, and the `G-20` marker are untouched. It adds no requirement — `FR-303` and `FR-312` are cited as they already read, and neither is amended — and no error code, `reason` value, state, acceptance criterion, or constant. It belongs to task 4.1 (`G-20`); this record holds the wording, and applying it is the product-manager's step, not a new user gate, because it restates a rule the product owner approved on 2026-10-22.

#### Line 488 — `FR-313` note, the `E_KEYRING_UNAVAILABLE` clause

Current (the clause between the colon and the semicolon):

```text
when no stored key can be read the run reports `E_KEYRING_UNAVAILABLE` with `reason` `unreadable` and asks the user to unlock or repair the OS keyring (`G-20`, resolved 2026-10-22)
```

Replace with:

```text
when no stored key can be read and no keyless local provider is selected under FR-303 and FR-312, the run reports `E_KEYRING_UNAVAILABLE` with `reason` `unreadable` and asks the user to unlock or repair the OS keyring (`G-20`, resolved 2026-10-22)
```

With the untouched tail in place, the note then reads: "…asks the user to unlock or repair the OS keyring (`G-20`, resolved 2026-10-22); the system MUST NOT keep a cached or decrypted copy, and MUST NOT delete the key."

**Why this wording.** The added condition is the decision's own sentence in the decision's own terms: the error is what the run reports when selection yields no keyless local provider, and `FR-303` and `FR-312` remain the requirements that decide selection. It introduces no new state, no second error, and no second condition on the wrap-up clauses. Left as applied, the note would contradict the record it cites; read this way, the two agree.

The `G-20` row at line 1114 summarises the same rule and is not changed here. This completion is the single target the decision's text calls for; the row is a summary and the operative rule is the note. If the row's summary is later read as carrying the same gap, that is a separate recorded edit, not a second change folded into this one.

---

## G-13 — Exponential backoff base and cap

| | |
|---|---|
| **Status** | `Resolved` |
| **Date** | 2026-10-22 |
| **Approver** | Product owner (this session) |
| **Owner** | `software-architect`, which owns all five rows of this change (`proposal.md` line 11, approved 2026-10-22); the §9 row's owner cell stays `Architecture` and is not rewritten |
| **Affected requirements** | `FR-307` (primary: the wait when a 429 carries no usable `Retry-After`); `FR-308` (primary: the wait after a network drop); `FR-309` (added: a timeout is the third failure class the curve governs; its text and criteria are unchanged) |
| **Blocking gate** | `G5` |
| **SRS row** | line 1107; citations at lines 367, 369, 426, 434–435, 444, 933, 1018 |

**The `FR-309` entry, and where the criteria land.** The §9 row's question names 429 and network-drop retries; a timeout reaches the same wait through `FR-309`'s "retry it under the rules of FR-205" (line 440). Adding `FR-309` to the affected-ID cell records that scope, the same way `G-19` added `FR-304` and `G-27` added `NFR-201`: no requirement text, status, or criterion is altered. The three new criteria sit in the `FR-308` block rather than in `FR-307`'s, because a retry wait is stated once — in the block whose requirement is the retry — and the 429-facing half of the rule is carried by the Table 3.3-2 row at line 367. Two criteria lists asserting the same schedule would be two rules to keep in step.

### Decision

One backoff curve serves every failure that reaches a retry: a 429 with no usable `Retry-After`, a network drop, and a timeout. After the `n`-th failed attempt in a chunk's attempt loop, `n` counting from 1, the computed delay is `min(cap, base * 2^(n-1))`, with base 1 s and cap 30 s per wait. The actual wait is drawn with full jitter, uniformly in `[0, computed delay]`, so it is never longer than the computed delay and may be shorter. A `Retry-After` that is present and parseable replaces the computed delay for that wait with the stated duration: it is not added to it, not maxed with it, and not jittered above or below it. The jitter draw is supplied to the calculation by its caller, so a test fixes the draw and asserts the schedule with no clock and no network. The 3–5 attempt cap per chunk (`FR-205`) is the only bound on total waiting: this rule adds no second constant and no second stopping condition.

### Rationale

- **The three failure classes are one condition seen from three sides.** A network drop and a timeout are transport failures; a 429 without a usable header is a provider that wants time but does not say how much. From Aref's side all three mean the same thing — no usable answer now, and a retry worth making shortly — and `FR-205` and `FR-309` already route all three into one attempt loop. Two or three schedules inside that loop would make the wait after failure `n` depend on which failure ended the attempt: harder to state, harder to test, and asked for by none of the three requirements. [Fact] `FR-307` and `FR-308` both mandate "exponential backoff" (lines 423, 432) and a timeout is retried "under the rules of `FR-205`" (line 440), so settling only the rate-limit half would reproduce the deferral this change exists to remove: the other two classes would still point at a number the SRS does not carry.
- **1 s is the smallest wait that is a wait; 30 s is a ceiling rather than a target.** Below about a second, a wait is indistinguishable from an immediate retry: against a rate-limited provider it spends an attempt to receive the same 429, and against a local process that is busy for a moment it changes nothing. One second absorbs a dropped socket without costing the user anything noticeable in the run's pace. The cap does not bind inside the v1 loop — with five attempts the doubling yields waits of 1, 2, 4, and 8 s and no wait follows the fifth failure; with three, 1 and 2 s — so 30 s is not tuned to today's ceiling. It exists so the rule is total: the expression yields a number for any `n`, and no single wait outlasts the point at which the honest answer is "try again later" (`E_RATE_LIMITED`, line 933) rather than a spinner.
- **`Retry-After` replaces the computed delay because it is the provider's own figure.** The header is information, not a hint: the provider has said when it will accept work again, and Table 3.3-2's first row already says "Wait for the stated duration" (line 366), as does `FR-307`'s first criterion (line 425). Adding backoff to that number, or taking the larger of the two, can only lengthen the wait beyond what the provider asked; in the all-keys case it would also make the figure the banner quotes (line 369) disagree with the header the user could have seen. And because a server that publishes the header is not the case the exponential formula exists for, combining the two guarantees that one of them is never observable. There is no fleet to de-synchronise here: one desktop process, the user's own keys. [Assumption] A provider's stated duration is also long enough for the retry to be accepted; Aref takes the header at face value and does not verify it. Jitter is deliberately not applied to it in either direction — a shorter wait re-hits the 429, and a longer one contradicts the number the user is shown.
- **An injected draw is what makes the schedule a test.** [Fact] `AGENTS.md` §11 requires tests to be deterministic and to run offline. A backoff that reaches for a process-global random source cannot be asserted: a test could only check a bound, and a bound whose margin depends on luck is how a suite becomes flaky. Passing the draw in — the jitter source is a parameter of the calculation, not a call inside it — lets a test pin it and assert the exact sequence of waits, and lets the production path supply a real draw without a new seam being invented later. Full jitter, rather than equal jitter or a fixed spread, keeps the rule to one inequality — every wait lies in `[0, min(cap, base * 2^(n-1))]` — while halving the expected wait, which matters here because the party waiting is a user watching a progress bar, not a server absorbing a herd.

### Alternatives considered

| Option | Content | Outcome and reason |
|---|---|---|
| **A** | No backoff: retry at once, or after a fixed short delay, with the 3–5 attempt cap as the only bound | Rejected. Against a rate-limited provider it spends attempts faster than the window closes — three attempts can be gone inside one `Retry-After` window, and the run then fails on a limit it could have outlasted. Against a process that is still starting, it does the same. It also leaves the row unanswered rather than settled: "no wait" is a value, but the row asks for a base and a cap, and the next reader cannot tell whether zero was chosen or forgotten. |
| **B** | One curve `min(cap, base * 2^(n-1))`, base 1 s, cap 30 s per wait, full jitter in `[0, computed]` from an injected source; a present parseable `Retry-After` replaces the computed delay | **Selected.** Fixes both numbers, covers all three failure classes with one testable rule, keeps Table 3.3-2's header row exact, and leaves nothing for a settings screen to expose. |
| **C** | A separate curve per failure class — shorter for a network drop and a timeout, longer for a 429 — or a per-provider curve | Rejected. It multiplies the numbers a test must assert and puts two schedules inside one attempt loop (`FR-205`), so the wait after failure `n` depends on which class failed. None of `FR-307`, `FR-308`, or `FR-309` asks for that, and the failures differ in cause, not in what a desktop client should do next. Per-provider curves would also make the wait depend on which key happened to serve the attempt. |
| **D** | The same curve plus a per-run wait budget: a total allowance across the whole run that ends retrying once it is spent | Rejected. It adds a second constant and a second stopping rule beside the attempt cap, so two rules would decide when a run ends and a user could not tell which one fired. The cap already bounds the wait — at most 15 s of computed delay per chunk at five attempts — so the budget would either never bind (a setting that does nothing) or bind first and fail a run that still had attempts left. It would also need a value, and that value is the settings inventory `G-12` owns, still `Open`. |

### Consequences

- **Worst-case wall clock per chunk.** Full jitter can shorten a wait but never lengthen one, so the time a chunk can spend waiting is at most the sum of the computed delays: 3 s at three attempts (1 + 2), 15 s at five (1 + 2 + 4 + 8). The 30 s cap contributes to neither figure — it bounds one wait, and the attempt cap bounds the sum. Request duration is outside both figures: no SRS line fixes a provider request timeout, and this decision fixes none.
- **No new setting, and none is implied.** Base and cap are properties of the retry rule, not user preferences. The settings inventory beyond UI language, provider configuration, and detection options is `G-12`, still `Open`, and this decision adds nothing to it. A user-settable base would also make the figure above unbounded relative to the attempt cap, because the cap counts attempts and not time.
- **Both values are compile-time named constants.** 1 s and 30 s appear in one expression in the retry code, as named constants rather than literals at a call site or values read from configuration; the test asserts the sequence of waits, not the spelling of the names. This is `AGENTS.md` §4 and §14 applied to the numbers the decision fixes.
- **The rule exists in three places, and they agree.** The Table 3.3-2 row at line 367 carries the numbers and the override; the three criteria in the `FR-308` block assert the schedule, the jitter bound, and the single curve; the §9 row records the decision. `FR-307`'s criteria at lines 425–427 and the rest of Table 3.3-2 are untouched.
- **The catalog and the §8.2 row keep their behavior.** `E_RATE_LIMITED` (line 933) still shows a banner with the wait time and offers retry once it elapses, and §8.2 row 1 (line 1018) still reads "`Retry-After` when present, exponential backoff otherwise, wait time shown". What changes is that the wait time is now a number in the SRS rather than a deferral.
- **Nothing new is added.** No error code, no command, no event, no setting, no field, no migration, and no dependency. The jitter source is a parameter of the calculation; which generator supplies its production value is decided where the code is written, not here, because naming a crate in this record would be a dependency decision this change may not take (`AGENTS.md` §4).
- **`FR-307`, `FR-308`, and `FR-309` remain `Draft`.** This record settles a deferral: no requirement text or status moves, and the three criteria join a block that already exists.

### Note — outside this decision

- The provider request timeout value is not set here. `FR-309` treats a timeout as a failed attempt and the catalog carries `E_PROVIDER_TIMEOUT` (line 935), but no SRS line fixes how long a request may run. This decision fixes the wait after a failure, not when a failure is declared.
- Whether a wait is surfaced on `humanize://progress` is not decided here. The payload beyond chunk index, attempt, and before/after scores is `G-17`, still `Open`.
- Table 3.3-2 row 4 (line 369, "try again in X minutes") is unchanged. The decision fixes the wait the banner quotes, not the unit it is rendered in.

### Exact replacement wording for `docs/SRS.md`

Line numbers are as of the current file. Each entry states the current text and the text that replaces it, both verbatim. The three criteria inserted at line 435 shift every later line by three, so the §9 row is line 1107 now and 1110 once they are applied.

#### Line 367 — Table 3.3-2, `Retry-After` absent or unparseable

Current row:

```text
| `Retry-After` absent or unparseable | Wait for an exponential backoff interval; base and cap are `Pending decision (G-13)` |
```

Replace with:

```text
| `Retry-After` absent or unparseable | Wait `min(cap, base * 2^(n-1))` after the `n`-th failed attempt, with base 1 s and cap 30 s per wait, and draw the actual wait with full jitter uniformly in `[0, computed]`. A `Retry-After` that is present and parseable replaces this computed delay rather than being combined with it (`G-13`, resolved 2026-10-22) |
```

#### Lines 430–436 — `FR-308` block, three new acceptance criteria

Current: the block carries two criteria at lines 434–435, the `Traces` line at 436, and no note. Insert the three lines below **after line 435 and before the `Traces` line at 436**, so the block keeps its criteria → traces order. No existing line is reworded or removed:

```text
- AC `FR-308: waits min(cap, base * 2^(n-1)) with base 1 s and cap 30 s after the n-th failed attempt`
- AC `FR-308: draws each wait with full jitter uniformly in [0, computed delay] from an injected source, so a fixed draw reproduces the schedule offline`
- AC `FR-308: uses one curve for a network drop, a timeout, and a 429 without a usable Retry-After, and lets a present parseable Retry-After replace the computed delay instead of combining with it`
```

#### Line 1107 — §9 row `G-13`

Current row:

```text
| G-13 | Open | What are the exponential backoff base and cap for 429 and network-drop retries? | Architecture | FR-307, FR-308 | G5 |
```

Replace with:

```text
| G-13 | Resolved | One backoff curve serves a 429 without a usable `Retry-After`, a network drop, and a timeout: the delay after the `n`-th failed attempt is `min(cap, base * 2^(n-1))` with base 1 s and cap 30 s per wait, and the actual wait is drawn with full jitter uniformly in `[0, computed]`, from an injected source so tests are deterministic and offline. A `Retry-After` that is present and parseable replaces the computed delay with the stated duration rather than combining with it. Decided 2026-10-22 | Architecture | FR-307, FR-308, FR-309 | G5 |
```

`FR-309` is added to the affected-ID cell because the decision settles the wait after a timeout, which `FR-309` routes into the same loop; `FR-307` and `FR-308` stay in the cell, the owner cell stays `Architecture`, and the gate is unchanged. No requirement text or status changes: `FR-307` at line 421, `FR-308` at line 430, and `FR-309` at line 438 all remain `Draft`.

### Verification after applying

- Grepping `Pending decision \(G-13\)` in `docs/SRS.md` returns zero matches.
- The `G-13` row reads `Resolved`, with owner `Architecture`, affected IDs `FR-307, FR-308, FR-309`, gate `G5`, and the date 2026-10-22.
- Line 367 states base 1 s, cap 30 s per wait, the doubling rule, full jitter in `[0, computed]`, and the `Retry-After` override, with no deferral marker and no second constant.
- The `FR-308` block carries five `AC FR-308:` lines: the two originals at lines 434–435 unchanged, the three above inserted before the `Traces` line, which is still immediately before the next requirement.
- Line 366 ("Wait for the stated duration") is byte-identical, and no `AC FR-307:` or `AC FR-309:` line is added, removed, or reworded.
- `FR-307` (line 421), `FR-308` (line 430), and `FR-309` (line 438) all still read `Draft`; `E_RATE_LIMITED` (line 933) and §8.2 row 1 (line 1018) are unchanged.
- The closing sentence of §9 ("the open decisions in §9 remain open") is still accurate, because out-of-scope rows remain `Open`.
- Line 7 of this record states all five decisions settled, and no section still calls a settled decision open.
