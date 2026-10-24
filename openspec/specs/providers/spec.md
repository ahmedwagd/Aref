# providers Specification

## Purpose
Discovery, configuration, selection, and use of OpenAI-compatible providers: the request path, the multi-key pool, and the failure taxonomy for rejected keys, rate limits, network loss, and invalid output.

## Requirements

### Requirement: OpenAI-compatible request path only

Provider traffic SHALL use OpenAI-compatible chat-completions endpoints, no provider-specific SDK SHALL be added, and every request SHALL carry the `model_id` stored on the provider entry, read from configuration and never from a models route, as settled by `G-09` (resolved 2026-10-22) (`FR-301`).

#### Scenario: A request targets the configured chat-completions endpoint
- **WHEN** a request is issued for a provider whose `base_url` and `model_id` are configured
- **THEN** the mock provider receives a chat-completions request carrying that `model_id`
- **THEN** the mock provider receives no request for a models list or any model-discovery route

#### Scenario: No provider-specific SDK is present
- **WHEN** the dependency manifests and lockfiles are inspected
- **THEN** no provider-specific SDK dependency is declared

### Requirement: A provider's keys form a pool with failover

A provider SHALL support multiple keys as a pool, and when a key fails the next request SHALL fail over to the next key in that pool (`FR-302`).

#### Scenario: A failing key fails over to the next key
- **WHEN** the first key of a provider fails and a second key exists in that provider's pool
- **THEN** the request is attempted with the second key
- **THEN** the outcome is taken from that second attempt rather than reported as the first key's failure

### Requirement: A local provider needs no key

A local provider SHALL be usable with no API key, and an enabled local provider in configuration SHALL prevent a "no provider configured" result when every key pool is empty (`FR-303`).

#### Scenario: A keyless local provider serves a request
- **WHEN** the only enabled provider is a local provider and no key exists in any pool
- **THEN** the request is issued to that provider's configured `base_url` with no credential
- **THEN** no `E_NO_PROVIDER` is produced

### Requirement: Discovery is user-initiated and loopback-only

Local-provider discovery SHALL occur only after an explicit user action, SHALL target only loopback ports 11434 for Ollama and 1234 for LM Studio, and SHALL NOT scan the local network or probe in the background (`FR-304`).

#### Scenario: Only the two loopback ports are probed
- **WHEN** the user runs a discovery action
- **THEN** the only destinations attempted are loopback addresses on ports 11434 and 1234

#### Scenario: No discovery runs without a user action
- **WHEN** the application starts, is configured, and idles without a discovery action
- **THEN** no discovery probe is attempted

#### Scenario: No non-loopback address is contacted
- **WHEN** the recording harness observes a discovery action
- **THEN** every connection attempt is addressed to loopback and no LAN scan or enumeration occurs

### Requirement: A rejected key is invalidated, failed over, and noticed

On an HTTP 401 the system SHALL mark that key `invalid`, fail over to the next key in the pool, and surface a notice naming that key, applying Table 3.3-1 (`FR-305`).

#### Scenario: 401 invalidates the key and fails over
- **WHEN** the mock provider returns 401 for the first key and a second key is in the pool
- **THEN** the first key's status becomes `invalid`
- **THEN** the request is retried with the second key
- **THEN** a notice naming the invalid key by its masked preview is surfaced to the UI (SRS §6.2 `providers://key_invalid`)

### Requirement: An invalid key is retained and skipped until edited

An invalid key SHALL be retained, SHALL receive no further request until the user edits it, and editing it SHALL make it eligible again (`FR-306`; `edit_key` in SRS §6.1).

#### Scenario: An invalid key stays stored and receives no request
- **WHEN** a key has been marked `invalid` and a later request runs
- **THEN** the key is still listed among the provider's keys
- **THEN** the mock provider receives no request carrying that key

#### Scenario: Editing the key makes it eligible again
- **WHEN** the user edits that key
- **THEN** `edit_key` returns its metadata with `status` `valid`
- **THEN** the next request may use the key

### Requirement: Rate limits honor Retry-After and otherwise back off

On an HTTP 429 the system SHALL honor a present parseable `Retry-After`, SHALL otherwise apply the backoff curve settled by `G-13` (resolved 2026-10-22), and SHALL show the resulting wait time when every key is rate-limited (`FR-307`, Table 3.3-2).

#### Scenario: A present Retry-After sets the wait
- **WHEN** every key of the provider returns 429 with a parseable `Retry-After`
- **THEN** no further request is issued before the stated duration elapses on the fake clock
- **THEN** the reported wait derives from the header and not from the computed delay

#### Scenario: A missing Retry-After falls back to the curve
- **WHEN** every key returns 429 without a usable `Retry-After`
- **THEN** no further request is issued before the wait computed by the curve settled by `G-13` elapses on the fake clock

#### Scenario: All keys rate-limited shows the wait time
- **WHEN** every key of the provider is rate-limited
- **THEN** `E_RATE_LIMITED` is returned with the wait in `params.minutes` (SRS §7)
- **THEN** the UI shows the wait time from that value

#### Scenario: One rate-limited key fails over without waiting
- **WHEN** one key returns 429 and another key in the pool is usable
- **THEN** the request fails over immediately to that key

### Requirement: A network drop retries and then returns completed chunks

On a network drop mid-run the system SHALL retry with the backoff curve settled by `G-13` (resolved 2026-10-22) and, when the final retry fails, SHALL return the chunks already completed (`FR-308`); one curve SHALL serve a network drop, a timeout, and a 429 without a usable `Retry-After`, and a present parseable `Retry-After` SHALL replace the computed delay (`FR-308`, `G-13`).

#### Scenario: A network drop is retried on the injected schedule
- **WHEN** a request fails with a network drop and the injected jitter source returns a fixed draw
- **THEN** the retry schedule is reproduced identically offline
- **THEN** each wait is the draw from that source within the delay computed by the curve settled by `G-13`

#### Scenario: The final failure returns completed chunks
- **WHEN** the last permitted retry after a network drop fails
- **THEN** the failure is reported as `E_NETWORK_OFFLINE`
- **THEN** the chunks already completed are returned in the partial report instead of being discarded

#### Scenario: One curve serves every backoff case
- **WHEN** a network drop, a timeout, and a 429 without a usable `Retry-After` are each observed
- **THEN** all three draw their waits from the same curve settled by `G-13`
- **THEN** a present parseable `Retry-After` replaces the computed delay rather than combining with it

### Requirement: Invalid provider output is a failed attempt

A timeout, an interrupted stream, empty output, `finish_reason=length`, and a refusal SHALL each be a failed attempt and SHALL NOT be returned as the rewritten text (`FR-309`). A refusal SHALL mean an HTTP 200 response whose `finish_reason` is `content_filter`; a free-text refusal with no such marker is not classified here. Wrong-language output is not specified here because `G-15` is open.

#### Scenario: A timeout is a failed attempt
- **WHEN** the provider request times out
- **THEN** the attempt is reported failed with `E_PROVIDER_TIMEOUT`
- **THEN** no text from that attempt is returned as the rewritten text

#### Scenario: An interrupted stream is a failed attempt
- **WHEN** the response stream is cut before completion
- **THEN** the attempt is reported failed with `E_STREAM_INTERRUPTED`
- **THEN** no partial stream text is returned as the rewritten text

#### Scenario: Empty and truncated output are failed attempts
- **WHEN** the response body is empty or ends with `finish_reason=length`, and `finish_reason` is not `content_filter`
- **THEN** the attempt is reported failed with `E_OUTPUT_REJECTED` and `reason` set to `empty` or `truncated` respectively
- **THEN** no text from that attempt is returned as the rewritten text

#### Scenario: A protocol-marked refusal is a failed attempt
- **WHEN** the loopback mock provider returns a fixture response with HTTP 200, assistant content, and `finish_reason` `content_filter`
- **THEN** the attempt is reported failed with `E_OUTPUT_REJECTED` and `reason` `refused`
- **THEN** no text from that response is returned as the rewritten text, even though content is present

### Requirement: The Providers screen lists and manages providers and keys

The Providers screen SHALL list the configured providers and their keys with masked previews, SHALL allow adding, editing, and removing a provider and a key, and SHALL display a notice for each key whose status is `invalid` (`FR-311`).

#### Scenario: The list shows masked previews only
- **WHEN** the Providers screen loads with configured providers and keys
- **THEN** every provider and every key is listed with its masked preview
- **THEN** no raw key appears in the list state

#### Scenario: Providers and keys can be added, edited, and removed
- **WHEN** the user adds, edits, and removes a provider and a key
- **THEN** the list reflects each change after the command returns

#### Scenario: Each invalid key shows a notice
- **WHEN** a key's status is `invalid`
- **THEN** a notice is displayed for that key

### Requirement: Selection order is configured and network-free

When more than one provider is enabled, selection SHALL follow OpenCode, Ollama, LM Studio, then cloud, and SHALL require no network request (`FR-312`). An OpenCode entry is user-configured only and never discovered, as settled by `G-19` (resolved 2026-10-22).

#### Scenario: Enabled providers are preferred in the documented order
- **WHEN** an OpenCode entry, an Ollama entry, an LM Studio entry, and a cloud provider with a usable key are all enabled
- **THEN** the selected provider is the OpenCode entry
- **THEN** disabling the OpenCode entry selects the Ollama entry, then the LM Studio entry, then the cloud provider

#### Scenario: Selection makes no network request
- **WHEN** a provider is selected from enabled configuration
- **THEN** the recording harness observes no network request during selection

#### Scenario: A local provider is preferred over a cloud key
- **WHEN** an enabled local provider and a cloud provider with a usable key both exist
- **THEN** the local provider is selected

### Requirement: The application never scans the network

The application SHALL NOT scan, enumerate, or probe the local network, and SHALL limit discovery probes to loopback ports 11434 and 1234 after a user action (`NFR-206`).

#### Scenario: Startup, configuration, and idle cause no probe
- **WHEN** the application starts, is configured, and idles without a user-initiated discovery
- **THEN** the recording harness observes no probe to any port

#### Scenario: A discovery action never leaves loopback
- **WHEN** a user-initiated discovery runs
- **THEN** every attempt is addressed to a loopback address on port 11434 or 1234

### Requirement: Provider and key command surface

The provider and key commands of SRS §6.1 SHALL accept the documented request fields and return the documented response shape: `list_providers`, `add_provider`, `update_provider`, `remove_provider`, `add_key`, `edit_key`, `remove_key`, and `probe_local_providers` (`FR-301`, `FR-304`, `FR-306`, `FR-310`, `FR-311`, `FR-312`, `FR-313`, `NFR-206`; SRS §6.1).

#### Scenario: list_providers returns configuration and key metadata
- **WHEN** `list_providers` is invoked
- **THEN** each provider entry carries `kind`, `local_flavor`, `base_url`, `model_id`, `label`, and `enabled`
- **THEN** each nested key entry carries key metadata including `masked_preview` and `status` and no raw key

#### Scenario: Provider commands accept and return their documented fields
- **WHEN** `add_provider`, `update_provider`, and `remove_provider` are invoked with their documented request fields
- **THEN** each returns its documented response, and a removed provider no longer appears in `list_providers`

#### Scenario: Key commands accept the secret once and never echo it
- **WHEN** `add_key`, `edit_key`, and `remove_key` are invoked
- **THEN** `secret` is accepted on `add_key` and `edit_key` only and is never echoed in any response or event
- **THEN** `add_key` and `edit_key` return masked key metadata and `remove_key` returns `removed`

#### Scenario: probe_local_providers returns the documented shape
- **WHEN** the user invokes `probe_local_providers`
- **THEN** each found entry carries `kind`, `local_flavor`, `base_url`, and `port`
