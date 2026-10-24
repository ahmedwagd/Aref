# Design

## 0. Scope, and what this record decides

This record turns the three delta specs of this change (26 requirements: `app-shell` 9, `providers` 13, `key-security` 4) into one buildable contract for the four PR slices in `proposal.md`. It is the architecture change that `docs/SRS.md` line 887 names as the ratifier of the §6.1 surface — but only for the commands this change serves. `detect_text`, `humanize_text`, `cancel_humanize`, `get_settings`, and `update_settings` stay *proposed*; their own changes ratify them.

| Kind | What this record does |
|---|---|
| **Ratifies** | The IPC command and event names, request fields, response shapes, and error envelope for the nine commands and two events this change ships. |
| **Fixes** | Module boundaries, the keyring trait, the retry seam, the selection/discovery split, migration `0001`, the mask contract (§5.3, §6.5), the `refused` definition (§4.7), the test seams. |
| **Cites, never re-decides** | `G-09` (SRS line 1106), `G-13` (1110), `G-19` (1116), `G-20` (1117), `G-27` (1124). Each is named where it binds; its text is not restated as a new rule. |
| **Leaves open** | §13, items 3, 5, and 6. None blocks slices 1–3. |
| **Fixes no visual value** | No colour, spacing, radius, typography, motion, or elevation is set here. §9 states the `docs/design/DESIGN.md` token rule for slice 4. The mask width and mask character *are* fixed here (§6.5), but as a security constant every stored preview encodes — not as a design token. |

Status: `Draft` — a design, not an approval. Requirement statuses are untouched; `FR-301`–`FR-313` stay `Draft`.

## 1. Context and constraints

### 1.1 Settled decisions, cited where they bind

| Decision | Binds | Where this design honours it |
|---|---|---|
| `G-09` | `FR-301` | §4.2 `ChatRequest` has no `model` field; the client injects `provider.model_id`. The caller *cannot* choose a model. No `/v1/models` call exists outside discovery's liveness probe, which parses no models. |
| `G-13` | `FR-307`, `FR-308`, `FR-309` | §6: one `BackoffPolicy`, one injected `Jitter`, one injected `Clock`, `Retry-After` replacing — not combining with — the computed delay. |
| `G-19` | `FR-312`, `FR-304` | §7.1: `opencode` ranks first and is never probed; §7.2: `PROBE_TARGETS` is exactly 11434 and 1234. |
| `G-20` | `FR-313`, `NFR-201` | §5.2/§5.4: unreadable ≠ `invalid`; skip, keep status, never cache, decrypt, or delete. |
| `G-27` | `FR-310`, `NFR-201` | §5.3: preview derived once from the submitted value, stored, never re-derived. |

### 1.2 Non-negotiables and where they bind

| Non-negotiable | Structural enforcement in this design |
|---|---|
| Keys only in the OS keyring | `security::keyring::KeyStore` is the only writer; §8 migration has no key column; `tauri-plugin-store` is not registered at all. |
| No key material in store/SQLite/localStorage/logs/errors | `Secret` is not `Serialize`, has a redacting `Debug`, and is not `Clone`; `AppError::params` admits only scalars from a closed set (§4.6); the frontend holds the secret in one form field for the duration of one submit (§9). |
| Frontend never holds a raw key, never calls a provider | No response or event type in §4 contains a secret; only `src/ipc/` calls `invoke`; no provider URL or credential reaches the frontend, so it has nothing to call. |
| No network on the detect path | `reqwest` is referenced only inside `providers/`; §3.2 turns that into a test. |
| Loopback-only, user-initiated discovery | §7.2: `PROBE_TARGETS` is a `const` array of two loopback literals; the only caller is the command; no timer, no startup hook, redirects disabled, proxy disabled. |
| No telemetry | §3.4: no telemetry module, no analytics dependency, no startup egress; tested by a recording transport (§10). |
| Windows-only | No `cfg` branch for another OS anywhere; paths via `std::path`; `tauri.conf.json` declares a Windows bundle only (slice 1). |
| OpenAI-compatible only | §4.2/§5: one `Transport`, one request shape, no per-provider branch beyond `kind`/`local_flavor` ordering. |

### 1.3 Quality attributes at stake

- **Offline determinism of tests** (`NFR-305`): every provider test is a scripted double or a loopback socket (§10).
- **No silent failure** (`NFR-205`, `FR-508`): every branch below returns an `AppError` from the existing §7 catalog; no new code is invented (§4.6).
- **Redaction before serialization** (`NFR-201`, `NFR-204`, `NFR-205`): `params` is a closed-scalar map, so an unredacted string cannot be added without a visible type change.
- **Reversibility**: the one-way doors are listed in §11 before any of them is walked through.

## 2. Goals and non-goals

**Goals.** One error type; one IPC boundary; one place that touches key material; one place that speaks HTTP; a selection function that cannot egress; a retry policy that is a pure function of attempt number, header, and injected draws.

**Non-goals.** No detector, no humanizer, no chunking, no prompt building, no history, no settings commands, no logging backend, no visual design, no new dependency.

## 3. Module boundaries

### 3.1 Tree

```
src-tauri/src/
├─ main.rs                  # calls lib::run()
├─ lib.rs                   # composition root: plugins, state, invoke_handler. No logic.
├─ error.rs                 # AppError: code(), message_key(), params(); Serialize -> envelope
├─ clock.rs                 # Clock trait, SystemClock, FakeClock (test)
├─ status.rs                # AppStatusService + DetectorModeSource seam  (get_app_status)
├─ commands/
│  ├─ mod.rs                # the nine handlers, registered in one list
│  ├─ app.rs                # get_app_status
│  ├─ providers.rs          # list/add/update/remove provider; add/edit/remove key; probe
│  ├─ dto.rs                # request/response DTOs, mirroring src/ipc/types.ts
│  └─ parse.rs              # parse::<T>(&InvokeBody) -> Result<T, AppError>
├─ providers/
│  ├─ mod.rs                # ProviderService: the only public entry to the service
│  ├─ config.rs             # providers/ constants (§6.5) — no literals at call sites
│  ├─ selection.rs          # pure ordering (FR-312)
│  ├─ pool.rs               # key pool, failover, 401/429 classification
│  ├─ retry.rs              # BackoffPolicy, Jitter, Retry-After
│  ├─ client.rs             # Transport trait, ReqwestTransport, OpenAI-compatible request
│  ├─ output.rs             # response validation (FR-309, refusal §4.7)
│  ├─ discovery.rs          # PROBE_TARGETS + probe (FR-304)
│  ├─ keys.rs               # KeyService: add/edit/remove + preview + metadata
│  └─ events.rs             # EventSink trait, TauriEventSink, RecordingSink (test)
├─ security/
│  ├─ mod.rs
│  ├─ keyring.rs            # KeyStore trait, OsKeyring, KeyStoreError
│  ├─ mask.rs               # preview(); MASK_WIDTH / MASK_CHAR / VISIBLE_TAIL
│  └─ redact.rs             # Secret newtype, redaction helpers
└─ storage/
   ├─ mod.rs
   ├─ db.rs                 # pool from DbInstances; migration registration; backup/restore
   ├─ providers_repo.rs     # providers + provider_keys
   ├─ settings_repo.rs      # settings (table only; no command in this change)
   └─ migrations/0001_init.sql
```

Deviations from the `AGENTS.md` §5 sketch, and why: `clock.rs` and `status.rs` are top-level because both are shared infrastructure/aggregation rather than a feature; `commands/{dto,parse}.rs` exist so no handler contains parsing or shape logic; `providers/` is split by seam (policy vs transport vs validation) so each is testable without the others. The separation of concerns in §5 is unchanged.

### 3.2 Layering rules, each enforceable by a check

1. `commands → services (providers, status) → infrastructure (security, storage, clock)`.
2. **Commands hold no business logic.** A handler is exactly: `parse → validate presence/type → call one service method → map AppError`. No SQL, no HTTP, no branching on provider kind. A handler that needs an `if` beyond validation is a missing service method.
3. **Services never import `commands`.** `rg "crate::commands" src-tauri/src/{providers,security,storage}` returns nothing (§10, architecture test).
4. **`reqwest` appears only under `providers/`.** `rg "reqwest" src-tauri/src --glob '!providers/**'` returns nothing. This is the guard that keeps the future detector from inheriting an HTTP client: a detector module that wants one has to break a test to get it.
5. **Only `src/ipc/` calls `invoke` or listens.** §9.
6. **Only `security::keyring` writes secrets.** `KeyStore::write` has one caller (`providers::keys`), asserted by a grep test on `KeyStore::write`.

### 3.3 Composition root and test seam

`lib.rs` builds one `ProviderService` and one `KeyService` and stores them in Tauri state:

```rust
pub struct ProviderService {
    transport: Arc<dyn Transport>,   // ReqwestTransport | ScriptedTransport
    keystore:  Arc<dyn KeyStore>,    // OsKeyring | InMemoryKeyStore
    repo:      Arc<dyn ProvidersRepo>,
    clock:     Arc<dyn Clock>,
    jitter:    Arc<dyn Jitter>,
    events:    Arc<dyn EventSink>,
}
```

Every field is a trait object so a test assembles the same struct with doubles and exercises the real `pool.rs`. Nothing in `providers/` reads a global, an environment variable, or `tauri::AppHandle` directly; the Tauri-bound implementations (`ReqwestTransport`, `TauriEventSink`) live behind these traits. This is the single seam the whole testing strategy in §10 hangs from.

### 3.4 Deliberate absences

No `telemetry`, `analytics`, `crash`, `update`, or `remote_config` module, and no plugin that would add one. No background task, timer, or `setup` hook that performs I/O. No logging backend: `log::` records are emitted with counts, ids, durations, and codes only, and slices 1–3 ship none by decision, confirmed 2026-10-22 (§13, item 4); with no backend installed they are inert, which is the safe default.

## 4. IPC contract (ratified)

### 4.1 Envelope

Every command returns `Result<T, AppError>`; `AppError` serializes to exactly three fields, so a Tauri rejection *is* the envelope:

```json
{ "code": "E_RATE_LIMITED", "message_key": "error.provider.rate_limited", "params": { "minutes": 2 } }
```

Rules, all of them testable:

- `code` is one of the §7 catalog strings — no new code is introduced (§4.6).
- `message_key` is a stable catalog key, never a sentence, and exists in both `ar` and `en` (§10 asserts the set).
- `params` is `Record<string, string | number | boolean | null>`. No nested objects, no arrays, no free-form text, so an unredacted payload cannot be added without changing the type.
- `params.provider` carries the provider **id**, never its `label` — a label is user-entered text and `NFR-205` forbids user text in `params`. The UI resolves id → label from its own list state.
- `params.masked` carries the **stored** `masked_preview` (`G-27`), never a fresh derivation.
- Field names are `snake_case` on the wire, matching SRS §6.1 verbatim; no `serde(rename_all)` anywhere. This is a contract choice, ratified once, so the Rust structs and `src/ipc/types.ts` are literal mirrors.

### 4.2 Commands

All nine are `async`. Reason: eight read or write SQLite through `sqlx`, `probe_local_providers` performs network I/O, and the key commands do a blocking OS call as well; a sync handler would block the runtime. Keyring calls are **blocking OS calls**, so `KeyService` wraps each `KeyStore` call in `tokio::task::spawn_blocking` — the same rule `AGENTS.md` §7.1 applies to ONNX inference. The trait itself stays sync, which keeps the doubles trivial.

| Command | `async` | Request (ratified) | Response (ratified) |
|---|---|---|---|
| `get_app_status` | ✓ | — | `AppStatus` |
| `list_providers` | ✓ | — | `{ providers: ProviderView[] }` |
| `add_provider` | ✓ | `{ kind, local_flavor?, base_url, model_id, label, enabled }` | `ProviderView` |
| `update_provider` | ✓ | `{ id, label?, enabled?, base_url?, model_id?, local_flavor? }` | `ProviderView` |
| `remove_provider` | ✓ | `{ id }` | `{ removed: boolean }` |
| `add_key` | ✓ | `{ provider_id, label, secret }` | `KeyMetadata` |
| `edit_key` | ✓ | `{ key_id, secret }` | `KeyMetadata` (`status = "valid"`) |
| `remove_key` | ✓ | `{ key_id }` | `{ removed: boolean }` |
| `probe_local_providers` | ✓ | — | `{ found: DiscoveredProvider[] }` |

```ts
// src/ipc/types.ts — the single mirror of the Rust DTOs (AGENTS.md §10)
type AppStatus = {
  detector_mode: "onnx" | "statistical_only";
  humanize_available: boolean;
  provider_summary: {
    configured: number;              // provider rows
    enabled: number;                 // enabled rows (FR-312 input)
    usable_keys: number;             // keys with status "valid" on enabled providers (FR-212)
    selected_provider_id: string | null; // output of §7.1; null when none
  };
  app_version: string;
};

type ProviderView = {
  id: string; kind: "openai_compatible" | "local";
  local_flavor: "opencode" | "ollama" | "lm_studio" | null;
  base_url: string; model_id: string; label: string; enabled: boolean;
  keys: KeyMetadata[];               // empty array, never null
};

type KeyMetadata = {
  id: string; provider_id: string; label: string; masked_preview: string;
  status: "valid" | "invalid";
  last_error_code: string | null; invalid_since: string | null; updated_at: string;
};

type DiscoveredProvider = {
  kind: "local"; local_flavor: "ollama" | "lm_studio";
  base_url: string; port: number;
};
```

`AppStatus.detector_mode` is produced by `status::DetectorModeSource`. Until the detector change lands, the binary bundles no classifier, so the only truthful value is `statistical_only`; the seam is a constant now and a mode query later, behind the same field. `app_version` comes from `AppPackageInfo`, not a literal. `humanize_available` and `selected_provider_id` both come from the *same* pure function as the request path (§7.1), so the button and the run cannot disagree.

Two request semantics worth fixing now:

- `update_provider` fields are optional and `null`/absent means **unchanged**, never "clear". `kind` is immutable after creation: changing it would invalidate the key policy (a `local` entry needs no key; an `openai_compatible` one does), so a user who picked wrong removes and re-adds. Ratified here because it is cheaper to fix now than after a UI exists.
- `remove_provider`/`remove_key` are **idempotent**: removing an absent id returns `{ removed: false }` rather than an error, because the §7 catalog has no not-found code and inventing one is out of scope.

### 4.3 Events

Ratified unchanged from SRS §6.2, payloads `snake_case`, error-carrying payloads use §4.1:

| Event | Payload | Emitted by |
|---|---|---|
| `providers://key_invalid` | `{ provider_id, key_id, masked_preview }` | `pool.rs` on a 401, after the metadata write, before failover (FR-305) |
| `providers://rate_limited` | `{ provider_id, retry_after_seconds, resets_at }` | `pool.rs` when the pool is exhausted by 429s (FR-307) |

`resets_at` is an RFC 3339 UTC string. `masked_preview` is the stored value. Both events exist in this change even though no humanize run can trigger them yet: the pool is the emitter, and the pool is testable with `RecordingSink` (§10). No other event is added — `get_app_status` is re-read by the UI after each mutation rather than pushed.

### 4.4 Deltas against SRS §6.1/§6.2

Kept exactly: every command name, every request field name and type, `secret` appearing only on `add_key`/`edit_key`, `probe_local_providers` returning `found` entries of `kind`/`local_flavor`/`base_url`/`port`, and both event names and payloads.

Changed, and flagged because §6.1 is proposed text that this change ratifies:

| # | Delta | Why |
|---|---|---|
| 1 | Provider entries gain `id`. | `update_provider` and `remove_provider` take `id`; a list without it cannot drive either. §6.1 lists the other six provider fields and omits this one. |
| 2 | `add_provider` and `update_provider` return `ProviderView` (ProviderConfig **plus** `keys`) instead of a bare ProviderConfig. | One DTO for all three provider responses; the UI can replace one row from one response without a second `list_providers` round trip. `keys` is `[]` on add. |
| 3 | `remove_provider`/`remove_key` return an object `{ removed }`, not a bare value. | It can gain fields later without a breaking change; §6.1 fixes only the name `removed`. |
| 4 | `provider_summary`'s inner shape is fixed to the four counts above. | §6.1 names the field but not its contents; `FR-212` needs `usable_keys` + `enabled`, and `FR-312` needs a selection result. Counts only — no labels, no URLs, no text. |
| 5 | `get_app_status` adds nothing else. | The disabled-Humanize guidance is rendered from the static key `error.provider.none`, so no extra field is needed to explain `humanize_available: false`. |

### 4.5 Malformed payloads

`NFR-306` and the `app-shell` requirement "malformed input yields an error envelope, not a panic" cannot be met with typed command arguments: Tauri deserializes those *before* the handler runs and rejects with a plain string, which is not an envelope. So the six commands that take fields accept the raw body and parse it themselves:

```rust
#[tauri::command]
pub async fn add_key(
    app: tauri::AppHandle,
    request: tauri::ipc::Request<'_>,     // CommandArg; body() -> &InvokeBody
) -> Result<KeyMetadata, AppError> {
    let req: AddKeyRequest = commands::parse::body(request.body())?;   // E_INTERNAL on non-JSON
    req.validate()?;                                                  // E_INPUT_EMPTY { field }
    app.state::<AppState>().keys.add(req).await                        // one service call
}
```

`tauri::ipc::Request` implements `CommandArg` and exposes `body()`, so the frontend still sends flat fields (`invoke("add_key", { provider_id, label, secret })`) — the SRS field names survive. Every DTO field is `Option<T>`: a **missing** field is `E_INPUT_EMPTY { field }`, a **wrongly typed** field is a serde error mapped to `E_INTERNAL`, and an absent `null` in an update is "unchanged". Presence and type are two separate, separately testable checks. The three no-request commands take no body argument, because a command with no fields cannot receive a malformed one.

### 4.6 Error mapping this change produces

Only existing §7 codes. Each row is one branch in one function, so the mapping is auditable.

| Condition | `code` | `params` | Retry? |
|---|---|---|---|
| Required field absent/blank | `E_INPUT_EMPTY` | `field` | no — UI fixes it |
| Body not a JSON object, or a field has the wrong type | `E_INTERNAL` | — | no |
| Keyring unavailable on write | `E_KEYRING_UNAVAILABLE` | `reason: "unavailable"` | after the user repairs the keyring |
| Keyring entry reads back unreadable after write | `E_KEYRING_UNAVAILABLE` | `reason: "unreadable"` | same |
| No key readable, no keyless local provider selected | `E_KEYRING_UNAVAILABLE` | `reason: "unreadable"` | same (`G-20`) |
| No enabled provider at all | `E_NO_PROVIDER` | `reason` | after Settings |
| 401 on every key of every candidate | `E_KEY_INVALID` | `provider` (id), `masked` | no — edit the key |
| 429 on every key | `E_RATE_LIMITED` | `minutes` | after the wait |
| Network drop surviving the retry budget | `E_NETWORK_OFFLINE` | — | yes |
| Request timeout | `E_PROVIDER_TIMEOUT` | — | yes, under the loop rules |
| Body read cut after a 2xx status | `E_STREAM_INTERRUPTED` | — | yes |
| Empty / `finish_reason=length` / refusal (§4.7) | `E_OUTPUT_REJECTED` | `reason: "empty" \| "truncated" \| "refused"` | yes, tighter instructions |
| Any other non-2xx | `E_INTERNAL` | `status` (number) | 5xx: yes; 4xx: failover, then no |

Two notes on this table.

- **`E_INPUT_EMPTY` for provider fields.** `G-09`'s record explicitly delegates this: "which existing code it maps to is settled when the skeleton change ratifies §6." The catalog has one input-validation code, and `params.field` carries the field name. Consequence for the catalog: `error.input.empty` must read correctly **without** interpolating `field` (a generic "this field is required"); `field` exists so the UI can attach the message to the right input. The alternative — a new `E_INPUT_INVALID` — is an SRS §7 edit, not this change's to make; the product owner confirmed the reuse on 2026-10-22 (§13, item 1).
- **`E_INTERNAL` for other non-2xx.** The catalog has no "provider returned HTTP n" code, and `E_OUTPUT_REJECTED`'s `reason` enum is fixed at four values, so an HTTP error cannot be expressed there without inventing a fifth. `E_INTERNAL` with `params.status` is the least-wrong existing entry; it carries no body text, which also keeps provider error bodies (which can echo the prompt) out of the UI.

### 4.7 `refused`: the term `FR-309` names

`FR-309` lists a refusal among the invalid-output classes that count as a failed attempt, and SRS §7 carries `refused` in `E_OUTPUT_REJECTED`'s `reason` enum, but neither the SRS nor `AGENTS.md` says what makes a response a refusal. This subsection fixes the narrow operational meaning. The `providers` delta spec's scenario on empty, truncated, and refused output carries the binding assertion; this record is where the term that scenario tests is defined.

**The marker.** A refusal is an HTTP **200** whose response body carries an explicit protocol marker that the model declined. Concretely, and only: `choices[0].finish_reason == "content_filter"` in the OpenAI-compatible response. The marker is read from the parsed envelope before the content is examined, because a filtered response normally arrives with null or empty content and the empty-output check would otherwise claim it — the protocol has already labelled the response, and that label wins. No other `finish_reason` value is a refusal: `stop` is ordinary output, and `length` is the truncation row of §4.6.

**The classification.** `E_OUTPUT_REJECTED` with `params.reason = "refused"` — the same code and the same handling as `empty` and `truncated`: a failed attempt that counts toward the attempt cap and is retried with tighter instructions (§4.6). The UI resolves it through `error.provider.output_rejected` like the other reasons; no reason-specific copy is required. It is not a transport failure and not a credential or rate-limit condition: the status was 2xx, so no key is marked `invalid`, no `providers://key_invalid` event is emitted, and §6.3's `Retry-After` handling does not apply.

**What is returned.** Nothing. `output::validate` returns `Err(AppError)`; no text from that response becomes the rewritten text, is cached, is emitted in an event, or is written to the metadata row. Only the enum value travels in `params`, so refusal prose — provider- and prompt-derived text — never crosses IPC (`NFR-201`, `NFR-205`) and is never logged (`AGENTS.md` §3.5).

**Explicitly out of scope.** A free-text refusal that carries no protocol marker — a 200 with `finish_reason = "stop"` whose content says in prose that the model will not help — is indistinguishable from ordinary output at the protocol level, and this change does not try to detect it: matching refusal prose is a heuristic over user-adjacent text, locale-dependent, and false-positive-prone in Arabic. It is treated as ordinary output, and whether it is acceptable is the humanizer's judgement, not this layer's. Wrong-language output stays outside this definition entirely: `G-15` is open, so `FR-309`'s fourth `reason` value is not exercised by this change.

## 5. Keyring abstraction

### 5.1 Trait

```rust
// security/keyring.rs — infrastructure. Sync: callers adapt with spawn_blocking.
pub trait KeyStore: Send + Sync {
    fn write(&self, id: &KeyId, secret: &Secret) -> Result<(), KeyStoreError>;
    fn read(&self, id: &KeyId) -> Result<Option<Secret>, KeyStoreError>;
    fn delete(&self, id: &KeyId) -> Result<(), KeyStoreError>;
}

pub enum KeyStoreError {
    Unavailable,      // no OS credential store, access denied, locked
    Unreadable,       // entry exists, value cannot be obtained
    Backend(BackendKind), // platform failure kind only — never a message, never a value
}
```

`KeyId` is the `KeyMetadata.id`; the entry name is derived, not stored: `KEYRING_SERVICE = "aref"` and user `key:{id}`. Deriving it keeps one source of truth and adds no column (§8.2). `Backend(BackendKind)` carries an enum, not the platform's error string, because a platform message is the one place an OS could echo entry data.

`Secret` (in `security/redact.rs`) is the only type that holds key material: not `Clone`, not `Serialize`, `Debug` prints `<redacted>`, and there is no `as_str()` — the transport gets `&Secret` and formats the `Authorization` header inside `client.rs`, the single place allowed to do so. Memory zeroization is **not** attempted in v1; no requirement asks for it and `zeroize` would be a dependency with no measurable promise on a managed-heap `String`. That is a stated limitation, not an oversight.

### 5.2 The three states

| State | Detected by | Behaviour |
|---|---|---|
| **Writable** | `write` returns `Ok` **and** read-back returns the same value | metadata row is inserted; command returns `KeyMetadata` |
| **Write fails** (`Unavailable`) | `write` returns `Unavailable` | `E_KEYRING_UNAVAILABLE { reason: "unavailable" }`; nothing is persisted anywhere; no fallback store is attempted (`FR-313`) |
| **Read-back unreadable** | `write` was `Ok`, `read` returns `Unreadable`/`None`/a different value | `E_KEYRING_UNAVAILABLE { reason: "unreadable" }`; **no metadata row**; the entry is **not** deleted (`G-20`: a write that reads back unreadable is the same case, and is not undone by deleting the key) |

Order is fixed: `write → read-back → metadata row`. Writing the row first would let a failed read-back leave a "saved" claim the core cannot verify. The accepted cost: a read-back failure can leave one inert keyring entry with no metadata row. Nothing reads it (no metadata points at it), it is never logged, and `G-20` forbids deleting it. The id is a fresh UUID, so it cannot collide with a later save.

Read-back comparison is a plain `==`. This is a local, single-user comparison with no attacker in the loop; a constant-time comparison would be cargo-culted here.

`edit_key` reuses the same sequence with `status = valid`, `invalid_since = NULL`, `last_error_code = NULL`, and a freshly derived preview — an edit *is* a save, which is what `FR-306`'s "editing it makes it eligible again" means.

### 5.3 `G-27` in the trait boundary

`mask::preview(&Secret) -> MaskedPreview` is called **only** from `keys.rs` at save time, on the value already in hand. It is never called from a read path: `list_providers` selects `masked_preview` from SQLite and never touches `KeyStore::read`. A test asserts that listing N keys performs **zero** `KeyStore::read` calls — that is the mechanical form of "computed once, never recomputed".

All three mask values are fixed in this record: `VISIBLE_TAIL = 4` (`G-27`), `MASK_WIDTH = 8`, and `MASK_CHAR = '•'` (U+2022). Composition has two branches, and they are separate rules rather than one rule plus an edge case:

- **Above the threshold** (`s.chars().count() > VISIBLE_TAIL`): `preview(s) = MASK_CHAR × MASK_WIDTH + last VISIBLE_TAIL characters of s`.
- **At or below it** (`s.chars().count() ≤ VISIBLE_TAIL`): `preview(s) = MASK_CHAR × (MASK_WIDTH + VISIBLE_TAIL)` — twelve mask characters and no tail at all, so no character of the secret is read. §6.5 states the binding rule, why the branch is derived rather than chosen, and the one-line assertion for it.

Width is a different claim from composition: in both branches the output is exactly `MASK_WIDTH + VISIBLE_TAIL` = 12 characters, at every key length — including 1–4, where all twelve are mask characters. Composition varies with length; width does not. Neither branch reveals a character of the prefix. `G-27`'s pair criterion therefore holds by construction: for keys above `VISIBLE_TAIL`, two keys that differ only outside the final four characters produce byte-identical previews, because no other character of the key is read; a key at or below the threshold reads none of its characters, so it cannot distinguish any pair. The threshold sits at `VISIBLE_TAIL` rather than below it because at exactly four characters the "last four" *are* the whole secret — the tail is revealed only when it is a proper suffix.

`MASK_WIDTH` and `MASK_CHAR` are a **security constant, not a design token** (§6.5). That is why they are decided here rather than in `docs/design/DESIGN.md`: `G-27`'s fixed width is a property of the non-reversible preview, and a visual token is free to change between releases while a persisted preview is not.

- **Tests may now assert the literal.** `preview` of a fixture above `VISIBLE_TAIL` is exactly eight `•` followed by its last four characters. Task 2.6's property set stays literal-free — equal width across lengths, the tail present only above `VISIBLE_TAIL`, mask-only at or below it, prefix absent, byte-identical pair — and task 2.13 is the single place the values themselves are asserted (§6.5).
- **The width is pinned by the first `add_key`, not by this document.** Every stored row carries a preview computed with these values, and `G-27` forbids recomputing one. A change after release would break `FR-310`'s equal-length criterion across the existing set, so the values are effectively permanent once a build exposes `add_key` (§11, `R-4`).

### 5.4 `G-20` in the trait boundary

`KeyStore::read` returning `Unreadable` is **not** an error the caller reports; it is a *skip*:

```
for key in provider.keys where status == valid:
    match keystore.read(key.id) {              // spawn_blocking
        Ok(Some(secret))   => try the request with it,
        Ok(None) | Err(Unreadable) => continue, // no candidate, status untouched
        Err(Unavailable)   => record "keyring unusable", continue,
    }
```

No write path is entered, so the status cannot drift to `invalid` (`G-20`, and `invalid` means "the provider rejected the credential", `FR-305`). No cache is populated: the `read` is repeated on the next attempt, which is exactly "a later attempt reads the keyring again rather than a cached copy". If every candidate yields no readable key and no keyless local provider is selected, the run returns `E_KEYRING_UNAVAILABLE { reason: "unreadable" }`; if a keyless local provider *is* selected, the run proceeds and raises nothing (`G-20`, `FR-303`, `FR-312`).

### 5.5 Test double

```rust
#[cfg(any(test, feature = "test-support"))]
pub struct InMemoryKeyStore { /* HashMap<KeyId, String> + Fault */ }
pub enum Fault { None, Unavailable, Unreadable, WriteOkReadFails, FailAfter(u32) }
```

It lives behind `cfg(test)`/`test-support` so it is not in the shipped binary. `FailAfter(n)` gives deterministic sequences (readable for two attempts, then unreadable) without sleeping or randomness. The double also counts `read` calls, which is what makes the `G-27` "never recomputed" test possible.

## 6. Retry seam

### 6.1 Where the curve lives

`providers/retry.rs` owns the whole policy as a pure function:

```rust
pub struct BackoffPolicy { pub base: Duration, pub cap: Duration, pub max_attempts: u32 }
impl BackoffPolicy {
    pub fn computed(&self, n: u32) -> Duration;   // min(cap, base * 2^(n-1)), saturating
    pub fn wait(&self, n: u32, jitter: &dyn Jitter) -> Duration; // draw in [0, computed]
    pub fn resolve(&self, n: u32, header: Option<Duration>, jitter: &dyn Jitter) -> Duration;
}
```

`base` and `cap` are `BACKOFF_BASE = 1 s` and `BACKOFF_CAP = 30 s` — requirement values from `G-13`, not tunables. `n` is 1 for the first failed attempt; `2u32.pow(n-1)` is computed with `checked_shl`/saturating arithmetic so a large `n` clamps to `cap` instead of panicking (`NFR-306`).

`resolve` is the **only** place a wait is produced, and it is called by all three cases — 429 without a usable header, network drop, timeout — which is how `FR-308`'s "one curve serves every backoff case" is satisfied by construction rather than by three parallel implementations. `Retry-After` is an argument, so "replace, don't combine" is a one-line property: `header.unwrap_or_else(|| self.wait(n, jitter))`.

The pool **never sleeps**. It returns the wait (in `E_RATE_LIMITED.minutes`, or to its caller's retry loop) and the caller decides. That keeps the request path a pure function of `(config, keyring, transport, policy)` and makes the schedule assertable without a real clock. `Clock::sleep` exists for the later humanize loop.

### 6.2 Clock and jitter

```rust
pub trait Clock: Send + Sync { fn now(&self) -> Timestamp; fn sleep(&self, d: Duration) -> BoxFuture<'_, ()>; }
pub trait Jitter: Send + Sync { fn draw(&self, upper: Duration) -> Duration; } // uniform [0, upper]
```

- `SystemClock` for production; `FakeClock` advances on `sleep` and records every requested duration, so a test asserts the exact schedule (`[0.4s, 1.6s, 6.4s, …]`) with no wall-clock dependency.
- `FixedJitter(d)` and `SequenceJitter([..])` for tests; `StdJitter` for production.
- `StdJitter` is a ~15-line SplitMix64 seeded from the system time, not the `rand` crate. Jitter needs uniformity, not cryptographic quality, and `AGENTS.md` §4 prefers fewer dependencies. It is not used for anything security-relevant, and that is stated in its doc comment so nobody later reuses it for a token.

### 6.3 `Retry-After`

`parse_retry_after(&HeaderMap) -> Option<Duration>` accepts **delta-seconds only** (`str::parse::<u64>`), std-only, no dependency. An HTTP-date value is treated as unusable and the curve applies — which is the spec's own "absent or unparseable" branch. The header is never assumed present (`AGENTS.md` §8: "Never assume the header exists"), and a malformed value never errors: it degrades to the curve. Date-form support is a deliberate narrowing, confirmed by the product owner on 2026-10-22 (§13, item 2): the `httpdate` dependency was declined, not deferred.

`params.minutes` is `ceil(seconds / 60)`, minimum 1. Rounding up is the only safe direction: a UI that offers "retry" after a rounded-down wait would retry into another 429.

### 6.4 Attempt sequence

```
pool::execute(candidates)
  for provider in candidates:                       // §7.1 order, pure
    for key in provider.keys(status = valid):
      secret = keystore.read(key.id)                // skip on unreadable (G-20)
      for n in 1..=policy.max_attempts:
        r = transport.send(provider, secret?, chat_request)
        2xx                -> output::validate -> Ok | E_OUTPUT_REJECTED
        401                -> mark invalid, emit key_invalid, break to next key
        429                -> header? -> exhaust this key, try next key immediately
                              none    -> wait(n) -> retry or exhaust
        5xx / transport    -> wait(n) -> retry or exhaust
        other 4xx          -> break to next key
      // after the pool: all 401 -> E_KEY_INVALID; all 429 -> E_RATE_LIMITED(minutes)
```

`max_attempts` is `MAX_TRANSPORT_ATTEMPTS` (default 3) in `providers/config.rs`, injectable so the humanize change can pass its own budget instead of a second constant. It is an engineering default, not a requirement value, and it is bounded — unbounded retry is an anti-pattern.

### 6.5 Named constants, in one place

| Constant | Value | Source |
|---|---|---|
| `BACKOFF_BASE`, `BACKOFF_CAP` | 1 s, 30 s | `G-13` (requirement) |
| `VISIBLE_TAIL` | 4 | `G-27` (requirement) |
| `MASK_WIDTH` | 8 | this record — security constant (§5.3), **not** a design token |
| `MASK_CHAR` | `'•'` (U+2022) | this record — security constant (§5.3), **not** a design token |
| `PROBE_TARGETS` | `127.0.0.1:11434` (ollama), `127.0.0.1:1234` (lm_studio) | `FR-304` (requirement) |
| `KEYRING_SERVICE` | `"aref"` | this record |
| `MAX_TRANSPORT_ATTEMPTS` | 3 | engineering default, injectable |
| `PROVIDER_CONNECT_TIMEOUT` | 10 s | engineering default |
| `PROVIDER_REQUEST_TIMEOUT` | 120 s | engineering default |
| `PROBE_TIMEOUT` | 1 s | engineering default |

The last four are defaults chosen so the product behaves sanely, not product decisions; they live in one file so a later change (or the open `G-23` latency budgets) can revise them without touching a call site.

`MASK_WIDTH` and `MASK_CHAR` are fixed here as a **security constant, not a design token**. `G-27` requires a width that does not vary with key length, and that requirement is about the non-reversible preview, not about the visual language: a visual token may be revised in a later release, these two may not once a key is saved (§5.3). They are not `docs/design/DESIGN.md`'s to define, and §9's token rule does not cover them.

All three mask constants — `VISIBLE_TAIL = 4`, `MASK_WIDTH = 8`, `MASK_CHAR = '•'` — live in `src-tauri/src/security/mask.rs`, beside `preview()` (§3.1). They are not in `providers/config.rs`: `security/` is the lower layer and must not import `providers/` (§3.2 rule 1), so the constants sit where they are used. `VISIBLE_TAIL` is a `G-27` requirement value; `MASK_WIDTH` and `MASK_CHAR` are this record's decision, taken 2026-10-22, before any key exists to store.

**Secrets of `VISIBLE_TAIL` characters or fewer — settled 2026-10-22.** `G-27` builds the preview from the key's last four characters, an expression that has no value when the secret holds fewer than four and that *is* the whole secret when it holds exactly four. The rule is: for any submitted secret whose length is at or below `VISIBLE_TAIL` — predicate `s.chars().count() ≤ VISIBLE_TAIL`, measured on the value in hand at save time and never on a stored copy — `preview(s)` is exactly `MASK_CHAR × (MASK_WIDTH + VISIBLE_TAIL)`: twelve characters, all `•`, no character of the secret. The tail is revealed only when it is a proper suffix, i.e. when the secret is strictly longer than `VISIBLE_TAIL`. Length is counted in `char`s and the tail is taken on a `char` boundary, so a multi-byte character is never split. Nothing else moves: no key is rejected for being short (the only rejection is the blank-field check, `E_INPUT_EMPTY`, §4.6 — presence, not a minimum length), the three constants above keep their values, and the output stays `MASK_WIDTH + VISIBLE_TAIL` = 12 characters at every length including 1–4, so `FR-310`'s fixed-width criterion and `G-27`'s pair criterion both still hold by construction. The branch is derived rather than chosen: with `MASK_WIDTH` fixed as the mask's own width and the total fixed at twelve, the slots a withheld tail leaves can only be filled with `MASK_CHAR` — anything else either shows characters that would then be the whole secret or changes the total width. §13 item 7 records the closure.

**The assertion for this branch, in one line:** a fixture of length 1 to 4 renders exactly twelve mask characters — `MASK_CHAR × (MASK_WIDTH + VISIBLE_TAIL)` — carrying none of its characters; a fixture longer than `VISIBLE_TAIL` renders twelve characters ending in its last four (2.6 as a literal-free property over lengths 1–4, 2.13 as the literal).

**For task 2.13:** it must now assert that `security/mask.rs` declares `MASK_WIDTH = 8` and `MASK_CHAR = '•'` as named constants — values in code, not read from any document — and that `preview()` of a fixture key is exactly eight `•` followed by that key's last four characters when the key is longer than `VISIBLE_TAIL`, and exactly twelve `•` at or below it — twelve characters at every fixture length. It no longer waits on `docs/design/DESIGN.md`.

## 7. Selection and discovery

### 7.1 Selection (`FR-312`, `G-19`)

```rust
pub fn candidates(providers: &[ProviderConfig], keys: &[KeyMetadata]) -> Vec<Candidate>;
fn rank(p: &ProviderConfig) -> u8;  // opencode 0, ollama 1, lm_studio 2, cloud 3
```

Eligibility: `p.enabled` **and** (`p.kind == Local` **or** at least one of its keys has `status == "valid"`). Order: `(rank, id)` — a deterministic tie-break for two entries of the same flavour, since §5.2 has no priority column and adding one is a schema change.

The signature is the guarantee: it takes two slices and returns a `Vec`. It is `fn`, not `async fn`; it has no `Transport`, no `Clock`, no `reqwest` in scope. "Selection makes no network request" is therefore not a behaviour to test but a property of the type — the test still exists, and it asserts the recording transport saw nothing while `get_app_status` answered.

A local provider with an empty pool is eligible (`FR-303`); a cloud provider with only `invalid` keys is not (`FR-306`: an invalid key receives no request, so the provider cannot serve). `humanize_available` is `!candidates.is_empty()`, which makes `FR-212`'s "enabled from persisted configuration only" true by construction — an enabled-but-unreachable local provider is still a candidate, and its unreachability becomes a failed attempt, exactly as `FR-212`'s note requires.

`opencode` entries are user-configured and never discovered (`G-19`): `rank` reads `local_flavor` from the row, and no code path creates one.

### 7.2 Discovery (`FR-304`, `NFR-206`)

```rust
pub const PROBE_TARGETS: [ProbeTarget; 2] = [
    ProbeTarget { flavor: Ollama,    base_url: "http://127.0.0.1:11434/v1", port: 11434 },
    ProbeTarget { flavor: LmStudio,  base_url: "http://127.0.0.1:1234/v1",  port: 1234  },
];
pub async fn probe(transport: &dyn Transport) -> Vec<DiscoveredProvider>;
```

Five properties, each one checkable:

1. **Only a user action.** `probe` has exactly one caller: the `probe_local_providers` command. No `setup` hook, no timer, no retry loop, no call from `get_app_status` or `selection`. A refused connection means "not running" and is not retried — retrying would be the background probing `FR-304` forbids.
2. **Loopback literal, not `localhost`.** `127.0.0.1` needs no name resolution, so there is no resolver step that could return something else. The destination list is a `const` array: there is no parameter that could carry another host.
3. **No proxy.** The discovery client is built with `.no_proxy()` and `redirect::Policy::none()`. Without the first, an `HTTP_PROXY` environment variable could route a "loopback" probe through a proxy on the network; without the second, a `302` could take the probe off loopback. Both are `NFR-206` violations that a normal `reqwest::Client` permits by default.
4. **Flavour comes from the port, not the body.** The probe is `GET {base_url}/models` with a 1 s timeout and no credential; success is any 2xx. No model list is parsed, returned, or used (`G-09`).
5. **Deterministic output.** The two probes run concurrently and the result is emitted in `PROBE_TARGETS` order, so the UI list is stable regardless of which port answers first.

`probe` returns what it found and nothing else: it does not create, enable, or select a provider. Adding a discovered entry is a separate user action through `add_provider`, which keeps `FR-304`'s "user-initiated" true for the *effect* as well as the probe.

### 7.3 Why neither path can egress

The provider `Client` is built lazily, once, inside `ReqwestTransport::send` — never at startup, never in `lib.rs`'s `setup`. `get_app_status` reads SQLite, calls the pure `candidates`, and returns; it holds no transport handle. So the `NFR-203` cold-start scenario is satisfiable by construction: constructing the app state performs no I/O at all, and the test that asserts it (§10) would fail the moment a `Client` moved into `setup`.

## 8. Migration `0001`

### 8.1 Tables

`storage/migrations/0001_init.sql`, applied by `tauri-plugin-sql`'s migrator (`Migration { version: 1, kind: Up }`) at startup, before any command can run (`FR-404`).

```sql
CREATE TABLE settings (                        -- SRS §5.1
  key TEXT PRIMARY KEY, value TEXT NOT NULL,   -- value is JSON; never key material
  schema_version INTEGER NOT NULL, updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE providers (                       -- SRS §5.2
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('openai_compatible','local')),
  local_flavor TEXT CHECK (local_flavor IN ('opencode','ollama','lm_studio')),
  base_url TEXT NOT NULL, model_id TEXT NOT NULL, label TEXT NOT NULL,
  enabled INTEGER NOT NULL CHECK (enabled IN (0,1)),
  updated_at TEXT NOT NULL,
  CHECK ((kind = 'local') = (local_flavor IS NOT NULL))
) STRICT;

CREATE TABLE provider_keys (                   -- SRS §5.3
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL REFERENCES providers(id),
  label TEXT NOT NULL, masked_preview TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('valid','invalid')),
  last_error_code TEXT, invalid_since TEXT, updated_at TEXT NOT NULL
) STRICT;
CREATE INDEX provider_keys_by_provider ON provider_keys(provider_id);
```

Notes that prevent bugs later:

- **No reliance on `ON DELETE CASCADE`.** `PRAGMA foreign_keys` is per-connection and the pool belongs to the plugin, so the pragma is not ours to guarantee. `remove_provider` deletes the key rows and the provider row explicitly in one transaction. The `REFERENCES` clause stays as documentation and as protection if a later change enables the pragma.
- Timestamps are RFC 3339 UTC `TEXT`; `enabled` is `INTEGER` with a `CHECK`, because SQLite has no boolean.
- `STRICT` tables reject a wrongly typed write at the boundary, which is cheap insurance for a schema read by hand-written SQL.
- The migrator's own bookkeeping table satisfies SRS §5.4's `Migration` entity (`version`, `applied_at`); a second `migrations` table would be a duplicate source of truth, so none is created. The name differs (`_sqlx_migrations`), which is recorded here rather than papered over.

### 8.2 What is deliberately absent

No `secret`, `ciphertext`, `encrypted_secret`, `key_hash`, `last_four`, `key_length`, or `keyring_ref` column, in any table, at any version. No `history` table (`FR-403` is `Withdrawn`). No `usage`, `telemetry`, or `remote_config` table. No `rate_limited_until` column: rate-limit state is a per-call fact, not a stored one, and `G-20`'s reasoning against persisting a read-time condition applies equally here. No provider-level `masked_preview` or `has_key` flag.

The only key-derived column in the database is `provider_keys.masked_preview`. That is permitted and required by `FR-310`/`G-27`: it is a fixed-width mask plus the last four characters only when the secret is longer than four (§6.5), and `G-27`'s pair criterion (two keys differing outside the tail produce byte-identical previews) is exactly the statement that the column cannot be inverted to a key.

`tauri-plugin-store` is **not registered**. Settings live in SQLite per §5.1, and not registering the store removes a second place a future bug could write a secret to. The plugin's SQL commands are also not granted to any window (§9), so the frontend cannot query this database directly.

### 8.3 Rollback

`db.rs` copies `aref.db` to `aref.db.pre-<version>` (in the app data dir, via `std::path`) before applying pending migrations, deletes the copy after success, and restores it if the migrator returns an error. SQLite DDL is transactional, so a failed migration also rolls back inside the file; the copy is the belt for the case where it does not. `FR-404`'s "preserve existing settings records" is asserted by a test that seeds `settings`, runs `0001`, and reads the row back.

## 9. Frontend boundary and the slice-4 token rule

**Slice 1 (unblocked).** `src/ipc/` is the only module that calls `invoke` or `listen`; it exposes typed functions and typed hooks. `src/i18n/` holds `ar.json`, `en.json`, and `error-keys.json` (the list of every `message_key` the core can emit, read by a Rust test *and* a frontend test, so the two cannot drift). `dir="auto"` on user- or provider-supplied content; `dir="ltr"` plus `<bdi>` on `masked_preview`, whose direction is known (mask characters plus at most four key characters, never Arabic) and must not mirror. Logical CSS properties only. No visual token is invented here: `src/styles/` imports Tailwind and the approved tokens of `docs/design/DESIGN.md`, approved by the product owner on 2026-10-22 — a value that document leaves `[Proposed]` or `[Open]` is a gap to raise, not a constant to add in this slice.

**Slice 4 (unblocked 2026-10-22).** Component boundaries only:

| Unit | Responsibility | Consumes |
|---|---|---|
| `features/providers/hooks.ts` | the only consumer of `src/ipc` for this feature; owns query/mutation state | `listProviders`, `addProvider`, …, `probeLocalProviders`, `onKeyInvalid`, `onRateLimited` |
| `ProviderList` | one row per `ProviderView`; selection badge from `provider_summary.selected_provider_id` | `ProviderView[]` |
| `KeyRow` | `masked_preview`, `status`, per-key invalid notice (`FR-311`) | `KeyMetadata` |
| `KeyForm` | the one place a secret exists in the frontend | `addKey`/`editKey` |
| `DiscoveryPanel` | the explicit button that calls `probeLocalProviders`, then offers `addProvider` per hit | `DiscoveredProvider[]` |
| `HumanizeDisabledNotice` | renders `error.provider.none` when `humanize_available` is false | `AppStatus` |

States the screen must model: `loading`, `empty`, `ready`, `error`, plus `keyring_unavailable` as a blocking state on `KeyForm` and `invalid` as a per-key notice. `KeyForm` keeps the secret in one field and a ref, clears it on submit completion whether the promise resolved or rejected, and never puts it in a context, a cache, or a URL — `NFR-201`'s "frontend state beyond a masked preview" is satisfied by the value having a lifetime of one submit.

**Gate.** No colour, spacing, radius, typography, motion, or elevation value may be introduced here. `AGENTS.md` §10 names `docs/design/DESIGN.md` as the source of derived tokens and forbids inventing new ones; the product owner approved that document as written on 2026-10-22, so slice 4 is implementable and cites its approved decisions — `D-01`, `D-02`, `D-03`, `D-06`, `D-08`. Only those are citable: `D-04`, `D-05`, `D-07`, `D-09` and the amber value inside `D-03` are not approved, and a value the document does not define is a gap to raise, not a local constant to add. The mask is **not** part of this rule: `MASK_WIDTH` and `MASK_CHAR` are fixed in §6.5 as a security constant, and `KeyRow` renders the stored `masked_preview` verbatim, deriving nothing from `DESIGN.md`. What the gate forbids is visual invention, not slice 4.

## 10. Testing strategy

All tests are deterministic and offline (`NFR-305`): no real endpoint, no wall clock, no random draw, no OS keyring.

| Layer | Double | What it proves |
|---|---|---|
| `security::mask` | none | property set of §5.3: equal width across lengths, last four present only above `VISIBLE_TAIL`, mask-only at or below it (§6.5), prefix absent, byte-identical pair, and zero `KeyStore::read` calls during a list |
| `security::keyring` | `InMemoryKeyStore` + `Fault` | every §5.2 state; read-back failure writes no metadata and deletes nothing; `Unavailable` never falls back |
| `providers::selection` | none (pure) | `FR-312` order and the disable-one-at-a-time walk; cloud without a valid key excluded; keyless local included; recording transport saw nothing |
| `providers::retry` | `FakeClock`, `FixedJitter`, `SequenceJitter` | `min(cap, base*2^(n-1))` at n=1..6 and at saturation; full-jitter bounds; `Retry-After` replaces the delay; unparseable header falls back; one policy serves 429/drop/timeout |
| `providers::pool` | `ScriptedTransport` + `InMemoryKeyStore` + `RecordingSink` | 401 → invalid + event + failover; invalid never sent; `edit_key` re-eligibility; one 429 fails over immediately; all 429 → `E_RATE_LIMITED` with `minutes`; unreadable skipped with status unchanged; keyless local proceeds; none readable → `E_KEYRING_UNAVAILABLE` |
| `providers::client` | loopback mock server (`TcpListener` on `127.0.0.1:0`) | request carries `model_id` from the entry; **no** request to any models route; no `Authorization` header when keyless; no proxy; 2xx-then-cut-body → `E_STREAM_INTERRUPTED`; connect refused → network drop |
| `providers::discovery` | same loopback harness | exactly two destinations, both loopback; zero probes without the command; a `302` is not followed; output order is `PROBE_TARGETS` order |
| `storage` | temp file DB | `0001` applies, re-runs as a no-op, preserves seeded settings; backup restored when a test-only failing migration aborts |
| `commands` | real services with doubles | malformed body → envelope, same input twice → same `code`, next valid call still works; no `secret` in any response |
| architecture | grep-style tests | §3.2 rules 3–6, and "no `Client` is constructed before the first request" |

The loopback mock server is a ~80-line `TcpListener` harness under `src-tauri/tests/support/`, not a dependency: it needs a status line, a couple of headers, and a JSON body, and a scripted request log. `wiremock` is the alternative if the harness ever needs chunked encoding or TLS — that would be a §4 dependency request, not a decision taken here.

Frontend: state-transition tests per screen (`loading → empty → ready → error`), the six states of `FR-509`, RTL **and** LTR rendering of every new screen, catalog key parity (`ar` vs `en` vs `error-keys.json`), an offline scan for hard-coded user-facing strings, and an offline scan for physical direction properties. Fixtures use non-key-shaped values (`fixture-secret-a`, never an `sk-…` pattern) so `NFR-207`'s "no key-shaped string in the repository" stays true.

One deliberate non-test: nothing asserts a specific timeout, because the three timeouts stay engineering defaults this record may revise. The mask is no longer in that class — §6.5 fixes both values, so 2.13 asserts them directly, while the §5.3 property set remains the contract that survives any future change to them.

## 11. Risks and reversibility

One-way doors, most expensive first:

| # | Door | Why it is one-way | Cost if we get it wrong |
|---|---|---|---|
| `R-1` | Keyring entry naming (`aref` + `key:{id}`) | Renaming orphans every stored secret; nothing can migrate them without reading them, and no path may read them for migration. | Every user re-enters every key. Pin the name before the first save; it is a constant, reviewed once. |
| `R-2` | Error `code` strings and envelope field names | Consumed by the UI catalog, tests, and the specs' own scenarios. | A rename touches the catalog, every test name, and any spec that names a code. Ratified here; changing it is a new §7 decision. |
| `R-3` | `masked_preview` persisted | Re-deriving old previews requires re-reading keys, which `G-27` forbids. | A width change breaks `FR-310`'s equal-length criterion across the stored set. Hence the property-based tests (§5.3). |
| `R-4` | `MASK_WIDTH = 8` / `MASK_CHAR = '•'` fixed in this record (§6.5) | The values are now pinned **before** any key is saved, so a fresh install carries no placeholder that could outlive the decision — the one-way-door risk is closed for slice 2. What remains one-way is narrower and later: the preview is **persisted**, so once a build ships `add_key` every row encodes these two values, `G-27` forbids recomputing one, and a later change would break `FR-310`'s equal-length criterion across the existing set. | Before release: a one-line edit with no data to migrate. After release: either mixed widths in the stored set or re-reading keys, which is forbidden — the constants are effectively permanent. **Mitigation:** closed for slice 2 by this pin; the residual "permanent once shipped" property is exactly why they are a security constant (§6.5) and not a design token. |
| `R-5` | Migration `0001` schema | After release, only additive migrations; a destructive change loses user configuration. | Acceptable: the schema is §5's own, and every column is used. |
| `R-6` | `base_url` includes the `/v1` suffix | It leaks into stored rows and into what users type. | A later convention change needs a data migration; cheap while no rows exist, so fix it in slice 3. |
| `R-7` | `E_INPUT_EMPTY` reused for provider-field validation | A later `E_INPUT_INVALID` would change an IPC `code`. | Bounded: the UI maps codes to catalog keys, so a new code is a catalog addition plus a mapping row, not a redesign. |

Reversible, and deliberately so: `MAX_TRANSPORT_ATTEMPTS` and the three timeouts (one file, no call sites); the `Transport`/`KeyStore`/`EventSink` traits (add an implementation, change no caller); `provider_summary`'s inner fields (additive); the discovery probe path (`GET /models` → any liveness request, inside one function).

Two risks that are not doors:

- **`E_INTERNAL` for other non-2xx** gives the user a generic message for a 404 caused by a mistyped model name. `G-09` accepted that cost explicitly ("an unknown or wrong model name is an ordinary provider failure"), and the alternative is an SRS edit.
- **No logging backend** means a field failure leaves no trace in slices 1–3. Accepted for a first slice by decision on 2026-10-22 (§13, item 4): every failure has a designed UI state, and adding a backend later changes no call site because every argument is already a count, an id, or a code.

## 12. Requirement coverage

Every requirement of the three delta specs, and where it is served. No requirement is left unserved.

| # | Requirement (capability) | Served by |
|---|---|---|
| 1 | Every IPC error crosses in one redacted envelope (`app-shell`) | §4.1 closed-scalar `params`, §4.6 mapping, §10 redaction test |
| 2 | One `AppError` and no panic on user input (`app-shell`) | §4.5 raw-body parsing, §6.1 saturating arithmetic, no `unwrap` outside tests |
| 3 | No telemetry, analytics, crash reporting, remote configuration (`app-shell`) | §3.4 absences, §7.3 lazy client, §10 cold-start assertion |
| 4 | v1 targets Windows only (`app-shell`) | §1.2, §3.1 (no `cfg` for another OS), slice-1 bundle config |
| 5 | No code signing in the build (`app-shell`) | §1.2; slice-1 `tauri.conf.json` declares a Windows bundle and no signing step |
| 6 | Application status response (`app-shell`) | §4.2 `AppStatus`, `status.rs`, §7.1 shared selection |
| 7 | Every user-facing string resolves from the locale catalog (`app-shell`) | §4.1 `message_key` rule, §9 `error-keys.json`, §10 parity test |
| 8 | Logical CSS properties only (`app-shell`) | §9 rule, §10 offline scan |
| 9 | Correct direction for numbers and mixed-direction text (`app-shell`) | §9 `dir="auto"` / `dir="ltr"` + `<bdi>` rules |
| 10 | OpenAI-compatible request path only (`providers`) | §4.2 `ChatRequest` (no `model`), §5.1 `client.rs` sole HTTP site, §7.3 |
| 11 | Keys form a pool with failover (`providers`) | §6.4 loop, §10 pool tests |
| 12 | A local provider needs no key (`providers`) | §7.1 eligibility, §6.4 keyless branch |
| 13 | Discovery is user-initiated and loopback-only (`providers`) | §7.2 properties 1–2 |
| 14 | A rejected key is invalidated, failed over, noticed (`providers`) | §6.4 401 branch, §4.3 `providers://key_invalid` |
| 15 | An invalid key is retained and skipped until edited (`providers`) | §7.1 eligibility filter, §5.2 `edit_key` reset |
| 16 | Rate limits honor `Retry-After`, otherwise back off (`providers`) | §6.1 `resolve`, §6.3, `E_RATE_LIMITED.minutes` |
| 17 | Network drop retries, then returns completed chunks (`providers`) | §6.1/§6.4 retry; the pool owns no caller output, so nothing is discarded; the partial-report *shape* is `humanize://partial`, owned by the humanize change (§13 item 3) |
| 18 | Invalid provider output is a failed attempt (`providers`) | §4.6 rows for timeout/interrupted/empty/truncated/refused, §4.7 refusal definition; `output.rs` returns no partial text |
| 19 | The Providers screen lists and manages providers and keys (`providers`) | §4.2 command set, §9 component boundaries (tokens from the approved `docs/design/DESIGN.md`) |
| 20 | Selection order is configured and network-free (`providers`) | §7.1 pure signature, `G-19` rank |
| 21 | The application never scans the network (`providers`) | §7.2 properties 1–4, §7.3 |
| 22 | Provider and key command surface (`providers`) | §4.2, §4.4 deltas |
| 23 | Keys live only in the OS keyring, return only a masked preview (`key-security`) | §5.1 trait, §5.3 `G-27`, §6.5 at-or-below-threshold rule, §8.2 |
| 24 | An unavailable keyring blocks storage and never falls back (`key-security`) | §5.2 three states, §5.4 `G-20` |
| 25 | No key material outside the keyring (`key-security`) | §5.1 `Secret`, §4.1 `params`, §8.2 absent columns, §9 `KeyForm` lifetime |
| 26 | The frontend never holds a raw key and never calls a provider (`key-security`) | §4.2 (no response carries a secret), §9 (`src/ipc` only), §7.3 |

## 13. Open questions

Nothing here blocks slices 1–3. Items 3, 5, and 6 remain decisions this record deliberately did not take. Items 1, 2, 4, and 7 were settled by the product owner on 2026-10-22 and keep their numbers so existing cross-references do not move.

1. **`E_INPUT_EMPTY` as the provider-field validation code** (§4.6) — **resolved 2026-10-22.** Approved by the product owner: provider-field validation reuses `E_INPUT_EMPTY`, with `params.field` naming the offending field. Taken because `G-09` delegates the mapping here and the catalog has no other input code. The rejected alternative, a new `E_INPUT_INVALID` row in SRS §7, is an SRS edit and is not taken in this change; should it ever be taken, `code` is `R-2`, so it would be a catalog addition plus a mapping row. Consequence already recorded in §4.6: `error.input.empty` must read correctly without interpolating `field`.
2. **`Retry-After` in HTTP-date form** (§6.3) — **resolved 2026-10-22.** Approved by the product owner: the header is parsed as delta-seconds only, and an HTTP-date value is unusable, so the computed curve applies. Supporting both would need one small dependency (`httpdate`) and a §4 justification; no spec scenario requires it, so `httpdate` is not added.
3. **`FR-308`'s partial report.** The provider layer guarantees a failure is a value and never returns partial stream text; the report's shape (`humanize://partial`, completed-chunk list) is the humanize change's contract. This change does not define it, and the providers spec's third scenario is served at the seam, not end to end. Confirming that reading is the one thing in §12 that deserves a second opinion.
4. **Logging backend** — **resolved 2026-10-22 for slices 1–3.** Approved by the product owner: slices 1–3 ship no logging backend. `log::` records are emitted but inert (§3.4), every argument is already a count, an id, a duration, or a code, and every failure has a designed UI state (§11). Which backend a later slice installs, and where it writes, is a new decision in that change, not this item reopened; it still owns the redaction audit of every future call site.
5. **A provider connectivity check** ("does this entry answer?") has no command in this change, per the `G-19` record's note. It would be a new command row and a new change.
6. **Two entries of the same flavour** are ordered by `id` (§7.1). A user-meaningful order needs a priority column, which is a schema change; not needed until someone configures two Ollama endpoints.
7. **A secret of `VISIBLE_TAIL` characters or fewer** (§5.3, §6.5) — **resolved 2026-10-22.** `G-27` composes the preview from the key's last four characters, which for a four-character secret is the whole secret and for a shorter one is more than it holds — a reversible preview, contrary to `FR-310`'s non-reversibility. Decision (approved by the product owner on 2026-10-22): the tail is revealed only when the secret is **longer** than `VISIBLE_TAIL`; at `VISIBLE_TAIL` characters or fewer the preview is `MASK_CHAR × (MASK_WIDTH + VISIBLE_TAIL)` — twelve characters, no character of the secret — and no key is rejected on length grounds. It changes no pinned constant: the predicate is on the input's length, not on a constant, so `VISIBLE_TAIL = 4`, `MASK_WIDTH = 8`, and `MASK_CHAR = '•'` keep their §6.5 values. Nor does it change any requirement: the preview is twelve characters at every length, so the `key-security` scenario "The mask width does not vary with key length" is served unchanged, and the scenario that names the last four characters presupposes the secret is longer than four. The `key-security` delta spec carries the binding scenario for a secret at or below `VISIBLE_TAIL`; `tasks.md` task 2.13 carries the literal assertion. §6.5 states the rule once and §5.3 the property set; this item keeps its number so existing cross-references do not move.
