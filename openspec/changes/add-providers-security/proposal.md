# Proposal

## Why

Aref's second product promise is that the user humanizes text through their own provider, and today there is nothing to humanize with and nowhere to keep a key: the repository has requirements and a contract but no shell, no provider client, and no key store. v1 therefore cannot send a single request, cannot persist a key without breaking `NFR-201`, and has no error envelope for the UI to render — so no other build step can be tested end to end either.

`AGENTS.md` §12 orders this step immediately after the requirements documents and delivers it as four sequential PRs. The scope has to be fixed before the spec, design, and task owners each write against it. This change also creates the first capability specs: `openspec/specs/` is empty, so the three capabilities named here are the boundaries every later change inherits.

## What Changes

Three new capabilities and four PR slices, all inside one change:

- **Capability `app-shell`** — the Tauri 2 + React 19 + Tailwind v4 shell, `ar`/`en` i18n with RTL, one `AppError` carrying the `NFR-205` envelope, and the `get_app_status` response shape.
- **Capability `providers`** — OpenAI-compatible client with key-pool failover, `401` invalidation without deletion, `429` and network-failure backoff, loopback-only discovery, and configured-only provider selection.
- **Capability `key-security`** — keyring-only key storage, the fixed-width masked preview, and the unavailable-keyring behavior.
- **Slice 1** — scaffold the Tauri/React/Tailwind app with `ar` + `en` locales, RTL layout, and the `AppError` type; no provider or key code.
- **Slice 2** — storage migrations plus the keyring trait, key metadata records, and the masked preview computed once at save.
- **Slice 3** — the provider client: failover, `401` invalidate-not-delete, full-jitter backoff, loopback-only discovery, and selection reading enabled configuration with no egress.
- **Slice 4** — the providers and keys UI against the capability in `app-shell`, including the disabled-Humanize state.

The four slices are PR boundaries inside one approved change, not four changes: a PR may land part of a change, and the capability specs are only whole once all four have landed.

## Capabilities

### New Capabilities

- `app-shell`: application shell and the process-wide contracts every feature depends on — the single `AppError` and its `code`/`message_key`/`params` envelope, the no-telemetry and Windows-only platform invariants, and the `get_app_status` response that reports detector mode and Humanize availability.
- `providers`: discovery, configuration, selection, and use of OpenAI-compatible providers — the request path, the multi-key pool, and the failure taxonomy for auth, rate limits, network loss, and timeouts.
- `key-security`: the custody of key material — keyring-only storage, the non-reversible masked preview, the unreadable-keyring behavior, and the rule that no raw key crosses into the frontend.

Every capability here needs its delta spec; this change does not set `skip_specs`, because archive must produce the three capabilities under `openspec/specs/`. None exist today.

### Modified Capabilities

None — `openspec/specs/` is empty, so there is no existing capability whose requirements change.

## Impact

| Capability | Requirements | Status after this change |
|---|---|---|
| `app-shell` | `NFR-203`, `NFR-205`, `NFR-306`, `NFR-307`, `NFR-308`; `get_app_status` shape serving `FR-107`, `FR-212`, `FR-509` | unchanged, `Draft` |
| `providers` | `FR-301`–`FR-309`, `FR-311`, `FR-312`, and `FR-304` with `NFR-206`; commands `list_providers`, `add_provider`, `update_provider`, `remove_provider`, `add_key`, `edit_key`, `remove_key`, `probe_local_providers` (SRS §6.1) | unchanged, `Draft` |
| `key-security` | `FR-310`, `FR-313`, `NFR-201`, `NFR-202`; governs the key-material behavior of `add_key`, `edit_key`, `remove_key`, and the key metadata in `list_providers`, while `providers` declares those commands' surface | unchanged, `Draft` |

- **Status:** no requirement is promoted, reworded, or withdrawn. `FR-301`–`FR-313` stay `Draft`; promotion is a separate decision.
- **Resolved decisions are cited, not restated:** `G-09`, `G-13`, `G-19`, `G-20`, `G-27` — `docs/SRS.md` §9 at lines 1106, 1110, 1116, 1117, 1124. Specs cite them by ID; the text is not copied into a spec, so a future edit to a row is not silently forked.
- **Files and layers:** new `src/` (`features/providers`, `features/settings`, `components`, `ipc`, `i18n`, `styles`) and `src-tauri/src/` (`commands/`, `providers/`, `security/`, `storage/`, `error.rs`), following the `commands → services → infrastructure` layering of `AGENTS.md` §5. No detector or humanizer code.
- **Non-negotiable most at risk, keyring-only keys:** slice 2 introduces the only code in the product that touches key material, and the tempting shortcut — a plaintext settings-store fallback when the keyring is unavailable — is exactly what `FR-313` and `NFR-201` forbid. The correct behavior is to block storage with a designed error, never to write the key elsewhere.
- **Non-negotiable most at risk, no network on the detect path:** slice 3 introduces the first HTTP client in the binary. Detect does not exist yet, so the risk is architectural: the provider module must not become a shared prerequisite that the detector later imports, no probe may run without a user action, and no probe may leave loopback ports 11434 and 1234.
- **Slice 4 token gate, closed:** `docs/design/DESIGN.md` now exists and was approved by the product owner on 2026-10-22 under `add-design-baseline`, so the derived-token source `AGENTS.md` §10 requires is in place and slice 4 is unblocked. Only the approved decisions are citable — `D-01`, `D-02`, `D-03`, `D-06`, `D-08`; `D-04`, `D-05`, `D-09`, `D-07` and the amber value inside `D-03` remain unapproved, and a value the document does not define is still a gap to raise, not a local constant to add.
- **Excluded, `FR-309` wrong-language clause:** `G-15` is still `Open` (`docs/SRS.md` §9 line 1112), so the criterion "retries wrong-language output as a failed attempt" is out of scope. The remaining `FR-309` criteria — empty, truncated, refused, timeout — stay in scope. Nothing here defines wrong language, and script mixing is not treated as one.
- **Excluded, detector and humanizer:** any `FR-1xx`, `FR-2xx`, `FR-4xx`, or `FR-5xx` behavior beyond the `get_app_status` response shape. `AGENTS.md` §12 states that `FR-2xx` is humanization, not providers; only the disabled-Humanize state and its guidance to Settings are in scope.
- **Stale upstream wording, not corrected here:** `AGENTS.md` §12 still describes OpenCode's endpoint as "still open under `G-19`". `G-19` is `Resolved`; the spec text follows the resolved row and this change does not edit `AGENTS.md`.
- **Citation correction:** no-telemetry is `NFR-203`, not `NFR-202`. `NFR-202` is the frontend/provider boundary (no raw key in the frontend, no provider request from frontend code) and is specced under `key-security`, where it governs `add_key` and `list_providers`.
