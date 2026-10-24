import { join } from 'node:path';

import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  addKey,
  editKey,
  listProviders,
  onKeyInvalid,
  onRateLimited,
  probeLocalProviders,
  type KeyInvalidEvent,
} from '../src/ipc';
import type * as IpcModule from '../src/ipc';
import { IpcFailure, asErrorEnvelope, useProvidersPage } from '../src/features/providers/hooks';
import {
  discovered,
  fixtureSecret,
  keyMetadata,
  maskedPreview,
  providerView,
} from './support/providers';
import { findMatches, listFiles, repoRoot } from './support/sources';

/**
 * `src/ipc` is mocked at the module boundary — the same seam the feature uses in production
 * (design.md §9) — so these tests drive the real hooks against recorded calls and never
 * reach Tauri.
 */
vi.mock('../src/ipc', async (importOriginal) => {
  const actual = await importOriginal<typeof IpcModule>();
  return {
    ...actual,
    listProviders: vi.fn(),
    addProvider: vi.fn(),
    updateProvider: vi.fn(),
    removeProvider: vi.fn(),
    addKey: vi.fn(),
    editKey: vi.fn(),
    removeKey: vi.fn(),
    probeLocalProviders: vi.fn(),
    onKeyInvalid: vi.fn(),
    onRateLimited: vi.fn(),
  };
});

const detach = (): void => {};

beforeEach(() => {
  // `restoreMocks` clears every implementation between tests, so the two subscriptions are
  // re-armed here: an event channel that resolves to nothing is what the hook expects.
  vi.mocked(onKeyInvalid).mockImplementation(() => Promise.resolve(detach));
  vi.mocked(onRateLimited).mockImplementation(() => Promise.resolve(detach));
  vi.mocked(listProviders).mockResolvedValue({ providers: [] });
});

describe('providers hooks', () => {
  it('moves the query from loading to ready through the mocked boundary', async () => {
    vi.mocked(listProviders).mockResolvedValue({ providers: [providerView()] });
    const { result } = renderHook(() => useProvidersPage());

    expect(result.current.providers.status).toBe('loading');

    await waitFor(() => expect(result.current.providers.status).toBe('ready'));
    expect(result.current.providers.data?.providers).toHaveLength(1);
    expect(vi.mocked(listProviders)).toHaveBeenCalledTimes(1);
  });

  it('records the envelope on a query failure instead of throwing', async () => {
    vi.mocked(listProviders).mockRejectedValue({
      code: 'E_INTERNAL',
      message_key: 'error.internal',
      params: {},
    });
    const { result } = renderHook(() => useProvidersPage());

    await waitFor(() => expect(result.current.providers.status).toBe('error'));
    expect(result.current.providers.error?.code).toBe('E_INTERNAL');
  });

  it('moves a mutation from idle to done and re-reads the list', async () => {
    vi.mocked(addKey).mockResolvedValue(keyMetadata());
    const { result } = renderHook(() => useProvidersPage());
    await waitFor(() => expect(result.current.providers.status).toBe('ready'));
    expect(result.current.mutation.status).toBe('idle');

    await act(async () => {
      await result.current.addKey({ provider_id: 'provider-cloud', label: 'Work', secret: fixtureSecret });
    });

    expect(result.current.mutation.status).toBe('done');
    expect(result.current.mutation.operation).toBe('addKey');
    expect(result.current.mutation.error).toBeNull();
    await waitFor(() => expect(vi.mocked(listProviders)).toHaveBeenCalledTimes(2));
  });

  it('records a failed mutation and rejects with the same envelope', async () => {
    const envelope = { code: 'E_KEYRING_UNAVAILABLE', message_key: 'error.security.keyring_unavailable', params: { reason: 'unavailable' } };
    vi.mocked(addKey).mockRejectedValue(envelope);
    const { result } = renderHook(() => useProvidersPage());
    await waitFor(() => expect(result.current.providers.status).toBe('ready'));

    let failure: unknown = null;
    await act(async () => {
      try {
        await result.current.addKey({ provider_id: 'p', label: 'l', secret: fixtureSecret });
      } catch (rejection: unknown) {
        failure = rejection;
      }
    });

    expect(failure).toBeInstanceOf(IpcFailure);
    expect(result.current.mutation.status).toBe('error');
    expect(result.current.mutation.error?.code).toBe('E_KEYRING_UNAVAILABLE');
    expect(result.current.mutation.operation).toBe('addKey');
    // A failed write leaves the list alone: nothing was persisted to re-read.
    expect(vi.mocked(listProviders)).toHaveBeenCalledTimes(1);
  });

  it('probes once per call and does not reload the list', async () => {
    vi.mocked(probeLocalProviders).mockResolvedValue({ found: [discovered('ollama', 11434)] });
    const { result } = renderHook(() => useProvidersPage());
    await waitFor(() => expect(result.current.providers.status).toBe('ready'));

    let found = 0;
    await act(async () => {
      found = (await result.current.probe()).length;
    });

    expect(found).toBe(1);
    expect(vi.mocked(probeLocalProviders)).toHaveBeenCalledTimes(1);
    expect(vi.mocked(listProviders)).toHaveBeenCalledTimes(1);
  });

  it('reports the remaining mutations through the same wrapper', async () => {
    vi.mocked(editKey).mockResolvedValue(keyMetadata());
    const { result } = renderHook(() => useProvidersPage());
    await waitFor(() => expect(result.current.providers.status).toBe('ready'));

    await act(async () => {
      await result.current.editKey({ key_id: 'key-a', secret: fixtureSecret });
    });
    expect(result.current.mutation.operation).toBe('editKey');
    expect(result.current.mutation.status).toBe('done');
  });

  it('records both pushed events', async () => {
    let invalidHandler: ((payload: KeyInvalidEvent) => void) | null = null;
    vi.mocked(onKeyInvalid).mockImplementation((handler) => {
      invalidHandler = handler;
      return Promise.resolve(detach);
    });

    const { result } = renderHook(() => useProvidersPage());
    await waitFor(() => expect(invalidHandler).not.toBeNull());

    act(() => {
      invalidHandler?.({ provider_id: 'provider-cloud', key_id: 'key-a', masked_preview: maskedPreview });
    });

    expect(result.current.events.keyInvalid?.key_id).toBe('key-a');
    expect(result.current.events.keyInvalid?.masked_preview).toBe(maskedPreview);
  });
});

describe('the feature reaches the core only through src/ipc', () => {
  const featureFiles = [
    ...listFiles(join(repoRoot, 'src', 'features', 'providers'), ['.ts', '.tsx']),
    ...listFiles(join(repoRoot, 'src', 'components'), ['.tsx']),
  ];

  it('has files to scan', () => {
    expect(featureFiles.length).toBeGreaterThan(8);
  });

  it('calls neither invoke nor listen, as a call or as a substring', () => {
    // The same mechanical rule as `src-tauri/tests/provider_architecture.rs` rule 5, which
    // scans `src/` for the two names: a comment or a variable that repeats one fails there,
    // so it fails here first.
    expect(findMatches(featureFiles, /\binvoke\s*\(/)).toEqual([]);
    expect(findMatches(featureFiles, /\blisten\s*[<(]/)).toEqual([]);
    expect(findMatches(featureFiles, /invoke|listen/i)).toEqual([]);
  });

  it('detects a planted violation', () => {
    expect(/\binvoke\s*\(/.test('await invoke("list_providers")')).toBe(true);
    expect(/invoke|listen/i.test('// the listener stays')).toBe(true);
  });
});

describe('asErrorEnvelope', () => {
  it('unwraps an IpcFailure, accepts a plain envelope, and falls back to E_INTERNAL', () => {
    const envelope = { code: 'E_INTERNAL', message_key: 'error.internal', params: {} };

    expect(asErrorEnvelope(new IpcFailure(envelope))).toEqual(envelope);
    // A caller or a double may hand back the envelope itself.
    expect(asErrorEnvelope(envelope)).toEqual(envelope);
    // Anything else is the generic message, never raw rejection text (AGENTS.md §9).
    expect(asErrorEnvelope('boom').message_key).toBe('error.internal');
  });
});
