# Spec Delta

## Purpose

Custody of key material: keyring-only storage, the non-reversible masked preview, the unreadable-keyring behavior, and the rule that no stored raw key ever reaches the frontend.

## ADDED Requirements

### Requirement: Keys live only in the OS keyring and return only as a masked preview

The system SHALL store API keys only in the OS keyring and SHALL return to the UI only a masked preview of any stored key (`FR-310`); the preview SHALL be twelve characters wide at every key length, SHALL reveal characters of the key only when it is longer than `VISIBLE_TAIL` and then only its last `VISIBLE_TAIL`, SHALL include no part of the key's prefix, SHALL be computed once at save, and SHALL never be recomputed by re-reading the key, as settled by `G-27` (resolved 2026-10-22).

#### Scenario: A saved key goes to the keyring and only a preview comes back
- **WHEN** a key is saved through `add_key`
- **THEN** the secret is present in the OS keyring entry for that key
- **THEN** the command returns key metadata whose preview is masked and contains no raw key

#### Scenario: The mask width does not vary with key length
- **WHEN** two saved keys of different lengths are listed
- **THEN** their masked previews have equal length

#### Scenario: The preview shows only the last four characters
- **WHEN** a masked preview of a key longer than `VISIBLE_TAIL` characters is inspected
- **THEN** it ends with the key's last four characters and contains no character from the key's prefix
- **THEN** every other character of the preview is `MASK_CHAR`

#### Scenario: A secret of `VISIBLE_TAIL` characters or fewer shows the mask only
- **WHEN** a key of one to four non-blank characters is saved and its masked preview is inspected
- **THEN** the preview is exactly twelve characters, every one of them `MASK_CHAR`
- **THEN** it contains no character of the key
- **THEN** the key is stored, because length alone never rejects a key

#### Scenario: The tail appears only from the fifth character
- **WHEN** a four-character key and a five-character key are saved and both previews are inspected
- **THEN** the four-character key's preview is twelve `MASK_CHAR` characters and reveals none of it
- **THEN** the five-character key's preview ends with its last four characters

#### Scenario: Two keys that differ only outside the last four share a preview
- **WHEN** two keys longer than `VISIBLE_TAIL` characters differ only in characters outside their last four
- **THEN** their masked previews are byte-identical

#### Scenario: The preview is computed once at save and never recomputed
- **WHEN** the provider list is read after the key was saved
- **THEN** the preview returned equals the one computed at save
- **THEN** the keyring is not read to produce it

### Requirement: An unavailable keyring blocks storage and never falls back

When the OS keyring is unavailable the system SHALL block key storage with a clear error and SHALL NOT fall back to any plaintext store (`FR-313`). A stored key the keyring cannot read SHALL be unusable but not `invalid`, SHALL be failed over, and SHALL never be cached, decrypted, or deleted, as settled by `G-20` (resolved 2026-10-22).

#### Scenario: Saving is blocked when the keyring is unavailable
- **WHEN** the OS keyring is unavailable and a key is saved
- **THEN** the save is blocked with `E_KEYRING_UNAVAILABLE`
- **THEN** no key is written to the settings store, a SQL database, or any other local store

#### Scenario: An unreadable key is failed over without being invalidated
- **WHEN** a stored key cannot be read and another usable key or an enabled keyless local provider is available
- **THEN** the core fails over past that key
- **THEN** the key's status stays unchanged and no request carries it

#### Scenario: No readable key reports the unreadable reason
- **WHEN** no stored key can be read and no keyless local provider is selected
- **THEN** the run reports `E_KEYRING_UNAVAILABLE` with `reason` `unreadable`
- **THEN** every key keeps its status unchanged

#### Scenario: An unreadable key is never cached, decrypted, or deleted
- **WHEN** a read fails, or a write reads back unreadable
- **THEN** no local store holds a copy of the key and the keyring entry is not deleted
- **THEN** a later attempt reads the keyring again rather than a cached copy

### Requirement: No key material outside the keyring

API keys SHALL exist only in the OS keyring and SHALL NOT appear in the settings store, a SQL database, `localStorage`, logs, error messages, or frontend state beyond a masked preview (`NFR-201`).

#### Scenario: No local store holds key material
- **WHEN** a key has been saved and the application has listed providers and made an attempt
- **THEN** a scan of the settings store, the SQL database, and `localStorage` finds no key material

#### Scenario: Logs and error payloads expose only the preview
- **WHEN** a failure occurs while a key is configured
- **THEN** the log output and the error envelope contain no key material
- **THEN** the only key-derived value is the masked preview

### Requirement: The frontend never holds a raw key and never calls a provider

The frontend SHALL NOT receive a stored raw key and SHALL NOT originate a provider request; all provider traffic SHALL go through the Rust core (`NFR-202`).

#### Scenario: No command returns a stored key
- **WHEN** every command in the provider and key surface is invoked after a key is saved
- **THEN** no response or event carries the raw key
- **THEN** the only key-derived value returned is the masked preview

#### Scenario: Provider requests originate in the core
- **WHEN** a provider request is made during a run
- **THEN** the mock provider's request log records it
- **THEN** the frontend's network log records no request to any provider endpoint
