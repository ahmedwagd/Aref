-- Migration 0001 — settings, providers, provider_keys (design.md §8.1).
--
-- Recorded as `Migration { version: 1, kind: Up }` and applied by sqlx's migrator under
-- `tauri-plugin-sql`, which keeps the version in `_sqlx_migrations`. That table is the
-- schema's only bookkeeping: SRS §5.4's Migration entity, with no duplicate table here.
--
-- No table holds API key material. The only key-derived value in this database is
-- `provider_keys.masked_preview`, which is a fixed-width mask plus the last four
-- characters of a key longer than four (design.md §6.5) and cannot be inverted to a key
-- (`G-27`, design.md §8.2). `REFERENCES` is documentation and protection if a later
-- change enables `PRAGMA foreign_keys`; no cascade is relied on, because the pragma is
-- per-connection and the pool belongs to the plugin.

CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  schema_version INTEGER NOT NULL,
  updated_at TEXT NOT NULL
) STRICT;

CREATE TABLE providers (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN ('openai_compatible','local')),
  local_flavor TEXT CHECK (local_flavor IN ('opencode','ollama','lm_studio')),
  base_url TEXT NOT NULL,
  model_id TEXT NOT NULL,
  label TEXT NOT NULL,
  enabled INTEGER NOT NULL CHECK (enabled IN (0,1)),
  updated_at TEXT NOT NULL,
  CHECK ((kind = 'local') = (local_flavor IS NOT NULL))
) STRICT;

CREATE TABLE provider_keys (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL REFERENCES providers(id),
  label TEXT NOT NULL,
  masked_preview TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('valid','invalid')),
  last_error_code TEXT,
  invalid_since TEXT,
  updated_at TEXT NOT NULL
) STRICT;

CREATE INDEX provider_keys_by_provider ON provider_keys(provider_id);
