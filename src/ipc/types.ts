/**
 * The single frontend mirror of the Rust IPC DTOs (AGENTS.md §10).
 *
 * Field names are `snake_case` and match `design.md` §4.2 field for field: the Rust
 * structs carry no `serde(rename_all)`, so this file is a literal mirror and the two
 * sides change together. Nothing here is derived or prettified.
 */

export type DetectorMode = 'onnx' | 'statistical_only';
export type ProviderKind = 'openai_compatible' | 'local';
export type LocalFlavor = 'opencode' | 'ollama' | 'lm_studio';
export type DiscoveredFlavor = 'ollama' | 'lm_studio';
export type KeyStatus = 'valid' | 'invalid';

/** `params` is a closed map: no nested objects, no arrays (design.md §4.1). */
export type ErrorParams = Record<string, string | number | boolean | null>;

export type ErrorEnvelope = {
  code: string;
  message_key: string;
  params: ErrorParams;
};

export type ProviderSummary = {
  configured: number;
  enabled: number;
  usable_keys: number;
  selected_provider_id: string | null;
};

export type AppStatus = {
  detector_mode: DetectorMode;
  humanize_available: boolean;
  provider_summary: ProviderSummary;
  app_version: string;
};

export type KeyMetadata = {
  id: string;
  provider_id: string;
  label: string;
  masked_preview: string;
  status: KeyStatus;
  last_error_code: string | null;
  invalid_since: string | null;
  updated_at: string;
};

export type ProviderView = {
  id: string;
  kind: ProviderKind;
  local_flavor: LocalFlavor | null;
  base_url: string;
  model_id: string;
  label: string;
  enabled: boolean;
  /** Empty array, never null (design.md §4.2). */
  keys: KeyMetadata[];
};

export type DiscoveredProvider = {
  kind: 'local';
  local_flavor: DiscoveredFlavor;
  base_url: string;
  port: number;
};

export type ListProvidersResponse = {
  providers: ProviderView[];
};

export type RemoveProviderResponse = {
  removed: boolean;
};

export type RemoveKeyResponse = {
  removed: boolean;
};

export type ProbeLocalProvidersResponse = {
  found: DiscoveredProvider[];
};

export type AddProviderRequest = {
  kind: ProviderKind;
  local_flavor?: LocalFlavor | null;
  base_url: string;
  model_id: string;
  label: string;
  enabled: boolean;
};

/** Absent or `null` means "unchanged", never "clear" (design.md §4.2). */
export type UpdateProviderRequest = {
  id: string;
  label?: string | null;
  enabled?: boolean | null;
  base_url?: string | null;
  model_id?: string | null;
  local_flavor?: LocalFlavor | null;
};

export type RemoveProviderRequest = {
  id: string;
};

export type AddKeyRequest = {
  provider_id: string;
  label: string;
  secret: string;
};

export type EditKeyRequest = {
  key_id: string;
  secret: string;
};

export type RemoveKeyRequest = {
  key_id: string;
};

/** `providers://key_invalid` (design.md §4.3). */
export type KeyInvalidEvent = {
  provider_id: string;
  key_id: string;
  masked_preview: string;
};

/** `providers://rate_limited`; `resets_at` is an RFC 3339 UTC string. */
export type RateLimitedEvent = {
  provider_id: string;
  retry_after_seconds: number;
  resets_at: string;
};
