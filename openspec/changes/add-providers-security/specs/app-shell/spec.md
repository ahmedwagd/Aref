# Spec Delta

## Purpose

The application shell and the process-wide contracts every Aref feature depends on: one redacted error envelope across IPC, the no-telemetry and Windows-only platform invariants, and the application-status response.

## ADDED Requirements

### Requirement: Every IPC error crosses in one redacted envelope

Every error crossing IPC SHALL be redacted of secrets and user text before it is logged or serialized and SHALL carry the envelope `code`, `message_key`, `params` (`NFR-205`); `message_key` SHALL be a stable catalog key the UI resolves, not a formatted sentence (SRS §7).

#### Scenario: A failed command returns the three-field envelope
- **WHEN** a command fails with a user-reachable error
- **THEN** the serialized error carries `code`, `message_key`, and `params`
- **THEN** `params` carries no user text

#### Scenario: Secrets and user text never reach the serialized error or the log
- **WHEN** a command fails while a key from the pool was used for the attempt
- **THEN** neither the serialized error nor the log line contains the key, any prefix of it, or the text being processed
- **THEN** the only key-derived value in `params` is the key's masked preview

#### Scenario: The message key resolves in both locales
- **WHEN** the UI catalog is asked for the `message_key` of a returned error
- **THEN** the key exists in both the `ar` and the `en` catalog

### Requirement: One AppError and no panic on user input

Every failure SHALL surface as one `AppError` variant carrying a stable `code` and the §7 envelope, and user input SHALL NOT cause a panic or an `unwrap`/`expect` (`NFR-306`).

#### Scenario: Malformed input yields an error envelope, not a panic
- **WHEN** a command receives a malformed payload, such as a missing required field or a wrong value type
- **THEN** the response is an error envelope
- **THEN** the process stays alive and the next valid command returns a normal response

#### Scenario: A code is stable for the same failure
- **WHEN** the same failing input is submitted twice
- **THEN** both responses carry the same `code`

### Requirement: No telemetry, analytics, crash reporting, or remote configuration

The application SHALL NOT include telemetry, analytics, crash reporting, or remote-configuration fetching (`NFR-203`).

#### Scenario: Cold start makes no outbound request
- **WHEN** the application starts and reaches idle with the network denied
- **THEN** it starts and `get_app_status` responds
- **THEN** the recording harness observes no outbound connection attempt

#### Scenario: A failure sends no report
- **WHEN** a command fails
- **THEN** no crash report, analytics event, or remote-configuration request leaves the machine

### Requirement: v1 targets Windows only

v1 SHALL target Windows only and SHALL NOT add macOS or Linux code paths or CI jobs (`NFR-307`).

#### Scenario: Only a Windows target is declared
- **WHEN** the declared build targets and CI jobs are inspected
- **THEN** a Windows target builds
- **THEN** no macOS or Linux target or CI job is declared

### Requirement: No code signing in the build

The build SHALL NOT include code-signing or Authenticode steps (`NFR-308`).

#### Scenario: The release pipeline has no signing step
- **WHEN** the release pipeline definition is inspected
- **THEN** it contains no signing or Authenticode step
- **THEN** the produced artifact is unsigned

### Requirement: Application status response

The `get_app_status` command SHALL return `detector_mode`, `humanize_available`, `provider_summary`, and `app_version`, answered from persisted state with no network request (SRS §6.1, serving `FR-107`, `FR-212`, `FR-509`).

#### Scenario: The status response carries the four fields
- **WHEN** the UI invokes `get_app_status` with the network denied
- **THEN** the response carries `detector_mode`, `humanize_available`, `provider_summary`, and `app_version`
- **THEN** no outbound request is made while the call is served

### Requirement: Every user-facing string resolves from the locale catalog

Every user-facing string SHALL have an Arabic and an English entry and SHALL NOT be hard-coded in a component (`NFR-401`; SRS §4.4).

#### Scenario: No user-facing string is hard-coded
- **WHEN** the offline string check scans the UI sources for literal user-facing text outside the locale catalogs
- **THEN** it reports no hard-coded user-facing string

#### Scenario: Every key exists in both locales
- **WHEN** the `ar` and the `en` catalog are compared key by key
- **THEN** every key present in one catalog is present in the other
- **THEN** no rendered screen shows a raw key or a built-in fallback sentence in either locale

### Requirement: Logical CSS properties only

The UI SHALL use logical CSS properties only, so that switching between RTL and LTR needs no physical-property override (`NFR-402`; SRS §4.4).

#### Scenario: No physical left or right property appears in the styles
- **WHEN** the stylesheets are scanned offline for physical direction properties, such as `margin-left`, `padding-right`, `left`, `right`, `text-align: left`, `text-align: right`, `border-left`, and `border-right`
- **THEN** none is present

#### Scenario: One stylesheet serves both directions
- **WHEN** the same screen is rendered in `ar` and in `en`
- **THEN** no locale-specific physical-property override is applied in either direction

### Requirement: Correct direction for numbers and mixed-direction text

Direction-unknown content SHALL use `dir="auto"`, and numbers and mixed Arabic-English text SHALL render with the correct direction in both locales (`NFR-403`; SRS §4.4).

#### Scenario: Direction-unknown content carries dir auto
- **WHEN** a component renders text whose direction is unknown, such as user-supplied or provider-supplied content
- **THEN** the element carries `dir="auto"`

#### Scenario: Numbers and mixed-direction text render correctly in both locales
- **WHEN** text mixing Arabic and English with digits, such as a count or a percentage, is rendered in `ar` and in `en`
- **THEN** the digits keep their order
- **THEN** the Arabic and the English segments keep their expected visual order in both locales
