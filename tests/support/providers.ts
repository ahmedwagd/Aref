import type {
  AppStatus,
  AsyncState,
  DiscoveredProvider,
  KeyMetadata,
  ProviderView,
} from '../../src/ipc';

/**
 * Deterministic, non-key-shaped fixtures (`NFR-207`, design.md §10). Every endpoint uses the
 * reserved `.invalid` TLD, so nothing here can be mistaken for a reachable provider.
 */
export const fixtureSecret = 'fixture-secret-a';

/** Eight mask characters and a four-character tail: `MASK_WIDTH` + `VISIBLE_TAIL`. */
export const maskedPreview = '••••••••a1b2';

export function keyMetadata(overrides: Partial<KeyMetadata> = {}): KeyMetadata {
  return {
    id: 'key-a',
    provider_id: 'provider-cloud',
    label: 'Work key',
    masked_preview: maskedPreview,
    status: 'valid',
    last_error_code: null,
    invalid_since: null,
    updated_at: '2026-10-22T00:00:00Z',
    ...overrides,
  };
}

export function providerView(overrides: Partial<ProviderView> = {}): ProviderView {
  return {
    id: 'provider-cloud',
    kind: 'openai_compatible',
    local_flavor: null,
    base_url: 'https://api.example.invalid/v1',
    model_id: 'fixture-model-a',
    label: 'Cloud entry',
    enabled: true,
    keys: [],
    ...overrides,
  };
}

export function localProvider(
  flavor: 'opencode' | 'ollama' | 'lm_studio',
  overrides: Partial<ProviderView> = {},
): ProviderView {
  return providerView({
    id: `provider-${flavor}`,
    kind: 'local',
    local_flavor: flavor,
    base_url: 'http://127.0.0.1:11434/v1',
    model_id: `fixture-${flavor}-model`,
    label: flavor,
    ...overrides,
  });
}

export function discovered(
  flavor: 'ollama' | 'lm_studio',
  port: number,
): DiscoveredProvider {
  return { kind: 'local', local_flavor: flavor, base_url: `http://127.0.0.1:${port}/v1`, port };
}

export function appStatus(overrides: Partial<AppStatus> = {}): AppStatus {
  return {
    detector_mode: 'statistical_only',
    humanize_available: true,
    provider_summary: {
      configured: 1,
      enabled: 1,
      usable_keys: 1,
      selected_provider_id: 'provider-cloud',
    },
    app_version: '0.1.0',
    ...overrides,
  };
}

export function readyStatus(overrides: Partial<AppStatus> = {}): AsyncState<AppStatus> {
  return { status: 'ready', data: appStatus(overrides), error: null };
}

/** A `Storage` read that keeps `no-unsafe-*` quiet and says what it sampled. */
export function storageDump(): string {
  const entries: string[] = [];
  for (const store of [window.localStorage, window.sessionStorage]) {
    for (let index = 0; index < store.length; index += 1) {
      const name = store.key(index);
      if (name !== null) entries.push(`${name}=${store.getItem(name) ?? ''}`);
    }
  }
  return entries.join('\n');
}
