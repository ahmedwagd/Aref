/**
 * The Providers feature's single door to the IPC boundary (design.md §9).
 *
 * Every read, mutation, and event subscription below goes through `src/ipc`, which is the
 * only module allowed to reach the core (AGENTS.md §10). Components receive plain data and
 * callbacks, so none of them needs a transport type and none of them holds key material:
 * the secret exists in one form field for the length of one submit (`KeyForm`).
 *
 * No state here holds a secret either. `MutationState.error` is the redacted envelope the
 * core returned (`code`, `message_key`, scalar `params`), never the submitted value.
 */
import { useEffect, useState } from 'react';

import {
  addKey,
  addProvider,
  editKey,
  listProviders,
  onKeyInvalid,
  onRateLimited,
  probeLocalProviders,
  removeKey,
  removeProvider,
  toErrorEnvelope,
  updateProvider,
  type AddKeyRequest,
  type AddProviderRequest,
  type AsyncState,
  type DiscoveredProvider,
  type EditKeyRequest,
  type ErrorEnvelope,
  type KeyInvalidEvent,
  type KeyMetadata,
  type ListProvidersResponse,
  type ProviderView,
  type RateLimitedEvent,
  type RemoveKeyRequest,
  type RemoveProviderRequest,
  type UpdateProviderRequest,
} from '../../ipc';

/**
 * Re-exported so the rest of the feature imports no other module of `src/ipc`
 * (design.md §9: one consumer per feature).
 */
export type {
  AddKeyRequest,
  AddProviderRequest,
  AppStatus,
  AsyncState,
  DiscoveredFlavor,
  DiscoveredProvider,
  EditKeyRequest,
  ErrorEnvelope,
  KeyInvalidEvent,
  KeyMetadata,
  KeyStatus,
  ListProvidersResponse,
  LocalFlavor,
  ProviderKind,
  ProviderSummary,
  ProviderView,
  RateLimitedEvent,
  RemoveKeyRequest,
  RemoveProviderRequest,
  UpdateProviderRequest,
} from '../../ipc';

/** The commands this feature drives. `operation` names the one that failed. */
export type MutationName =
  | 'addProvider'
  | 'updateProvider'
  | 'removeProvider'
  | 'addKey'
  | 'editKey'
  | 'removeKey'
  | 'probeLocalProviders';

export type MutationState = {
  status: 'idle' | 'pending' | 'done' | 'error';
  error: ErrorEnvelope | null;
  /**
   * Which command produced `error`. A key save renders its failure inside the form that
   * submitted it, and a provider change renders it on the screen, so the two notices never
   * repeat the same sentence.
   */
  operation: MutationName | null;
};

export type ProviderEvents = {
  keyInvalid: KeyInvalidEvent | null;
  rateLimited: RateLimitedEvent | null;
};

export type ProvidersPage = {
  providers: AsyncState<ListProvidersResponse>;
  mutation: MutationState;
  events: ProviderEvents;
  reload: () => void;
  addProvider: (request: AddProviderRequest) => Promise<ProviderView>;
  updateProvider: (request: UpdateProviderRequest) => Promise<ProviderView>;
  removeProvider: (request: RemoveProviderRequest) => Promise<boolean>;
  addKey: (request: AddKeyRequest) => Promise<KeyMetadata>;
  editKey: (request: EditKeyRequest) => Promise<KeyMetadata>;
  removeKey: (request: RemoveKeyRequest) => Promise<boolean>;
  probe: () => Promise<DiscoveredProvider[]>;
};

type Subscribe<T> = (handler: (payload: T) => void) => Promise<() => void>;

/**
 * One event channel. A subscription that cannot attach leaves the notice empty rather than
 * taking the screen down: the state it would report is also reachable by re-reading the
 * provider list.
 */
function usePayload<T>(subscribe: Subscribe<T>): T | null {
  const [payload, setPayload] = useState<T | null>(null);

  useEffect(() => {
    let active = true;
    let detach: (() => void) | undefined;
    subscribe((next) => {
      if (active) setPayload(next);
    })
      .then((dispose) => {
        if (active) detach = dispose;
        else dispose();
      })
      .catch(() => {
        // An unattached channel is not a user-reachable failure; nothing is claimed here.
      });
    return () => {
      active = false;
      detach?.();
    };
  }, [subscribe]);

  return payload;
}

const initialQuery: AsyncState<ListProvidersResponse> = {
  status: 'loading',
  data: null,
  error: null,
};

const initialMutation: MutationState = { status: 'idle', error: null, operation: null };

/**
 * An IPC failure travelling as a JS rejection. `@typescript-eslint/only-throw-error` is
 * right that a thrown value should be an `Error`; the redacted envelope rides along instead
 * of being reconstituted from the message.
 */
export class IpcFailure extends Error {
  readonly envelope: ErrorEnvelope;

  constructor(envelope: ErrorEnvelope) {
    // The catalog key: never a sentence, never a submitted value.
    super(envelope.message_key);
    this.name = 'IpcFailure';
    this.envelope = envelope;
  }
}

/**
 * Normalises either shape a caller can hand back: the feature's own `IpcFailure`, an
 * envelope object from a double, or a transport-level string from the Tauri bridge.
 */
export function asErrorEnvelope(value: unknown): ErrorEnvelope {
  return value instanceof IpcFailure ? value.envelope : toErrorEnvelope(value);
}

export function useProvidersPage(): ProvidersPage {
  const [providers, setProviders] = useState<AsyncState<ListProvidersResponse>>(initialQuery);
  const [mutation, setMutation] = useState<MutationState>(initialMutation);
  const [readCount, setReadCount] = useState(0);

  const keyInvalid = usePayload<KeyInvalidEvent>(onKeyInvalid);
  const rateLimited = usePayload<RateLimitedEvent>(onRateLimited);

  // Back to `loading` first, so a re-read never shows stale rows under a new error.
  const reload = (): void => {
    setProviders(initialQuery);
    setReadCount((value) => value + 1);
  };

  useEffect(() => {
    let active = true;
    listProviders()
      .then((data) => {
        if (active) setProviders({ status: 'ready', data, error: null });
      })
      .catch((rejection: unknown) => {
        if (active) {
          setProviders({ status: 'error', data: null, error: asErrorEnvelope(rejection) });
        }
      });
    return () => {
      active = false;
    };
  }, [readCount]);

  /**
   * One mutation wrapper: pending → done, or pending → error with the envelope the core
   * returned. Only a command that changed stored configuration re-reads the list;
   * discovery changes nothing, so it reports its own state and leaves the rows alone.
   */
  const write = async <T>(
    name: MutationName,
    operation: () => Promise<T>,
    reloadOnSuccess: boolean,
  ): Promise<T> => {
    setMutation({ status: 'pending', error: null, operation: name });
    try {
      const value = await operation();
      setMutation({ status: 'done', error: null, operation: name });
      if (reloadOnSuccess) reload();
      return value;
    } catch (rejection: unknown) {
      const error = asErrorEnvelope(rejection);
      setMutation({ status: 'error', error, operation: name });
      throw new IpcFailure(error);
    }
  };

  const addProviderCommand = (request: AddProviderRequest): Promise<ProviderView> =>
    write('addProvider', () => addProvider(request), true);

  const updateProviderCommand = (request: UpdateProviderRequest): Promise<ProviderView> =>
    write('updateProvider', () => updateProvider(request), true);

  const removeProviderCommand = async (request: RemoveProviderRequest): Promise<boolean> => {
    const response = await write('removeProvider', () => removeProvider(request), true);
    return response.removed;
  };

  const addKeyCommand = (request: AddKeyRequest): Promise<KeyMetadata> =>
    write('addKey', () => addKey(request), true);

  const editKeyCommand = (request: EditKeyRequest): Promise<KeyMetadata> =>
    write('editKey', () => editKey(request), true);

  const removeKeyCommand = async (request: RemoveKeyRequest): Promise<boolean> => {
    const response = await write('removeKey', () => removeKey(request), true);
    return response.removed;
  };

  // Discovery persists nothing, so it reports its own state and leaves the list untouched.
  const probe = async (): Promise<DiscoveredProvider[]> => {
    const response = await write('probeLocalProviders', () => probeLocalProviders(), false);
    return response.found;
  };

  return {
    providers,
    mutation,
    events: { keyInvalid, rateLimited },
    reload,
    addProvider: addProviderCommand,
    updateProvider: updateProviderCommand,
    removeProvider: removeProviderCommand,
    addKey: addKeyCommand,
    editKey: editKeyCommand,
    removeKey: removeKeyCommand,
    probe,
  };
}
