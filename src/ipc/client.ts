/**
 * The only module in `src/` that calls `invoke` or `listen` (design.md §3.2 rule 5,
 * AGENTS.md §10). Everything else consumes these functions through typed hooks.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type {
  AddKeyRequest,
  AddProviderRequest,
  AppStatus,
  EditKeyRequest,
  KeyInvalidEvent,
  KeyMetadata,
  ListProvidersResponse,
  ProbeLocalProvidersResponse,
  ProviderView,
  RateLimitedEvent,
  RemoveKeyRequest,
  RemoveKeyResponse,
  RemoveProviderRequest,
  RemoveProviderResponse,
  UpdateProviderRequest,
} from './types';

export function getAppStatus(): Promise<AppStatus> {
  return invoke<AppStatus>('get_app_status');
}

export function listProviders(): Promise<ListProvidersResponse> {
  return invoke<ListProvidersResponse>('list_providers');
}

export function addProvider(request: AddProviderRequest): Promise<ProviderView> {
  return invoke<ProviderView>('add_provider', { ...request });
}

export function updateProvider(request: UpdateProviderRequest): Promise<ProviderView> {
  return invoke<ProviderView>('update_provider', { ...request });
}

export function removeProvider(request: RemoveProviderRequest): Promise<RemoveProviderResponse> {
  return invoke<RemoveProviderResponse>('remove_provider', { ...request });
}

export function addKey(request: AddKeyRequest): Promise<KeyMetadata> {
  return invoke<KeyMetadata>('add_key', { ...request });
}

export function editKey(request: EditKeyRequest): Promise<KeyMetadata> {
  return invoke<KeyMetadata>('edit_key', { ...request });
}

export function removeKey(request: RemoveKeyRequest): Promise<RemoveKeyResponse> {
  return invoke<RemoveKeyResponse>('remove_key', { ...request });
}

export function probeLocalProviders(): Promise<ProbeLocalProvidersResponse> {
  return invoke<ProbeLocalProvidersResponse>('probe_local_providers');
}

export function onKeyInvalid(
  handler: (payload: KeyInvalidEvent) => void,
): Promise<UnlistenFn> {
  return listen<KeyInvalidEvent>('providers://key_invalid', (event) => {
    handler(event.payload);
  });
}

export function onRateLimited(
  handler: (payload: RateLimitedEvent) => void,
): Promise<UnlistenFn> {
  return listen<RateLimitedEvent>('providers://rate_limited', (event) => {
    handler(event.payload);
  });
}

/** Re-exported so consumers never need to import a type from the Tauri package. */
export type { UnlistenFn };
