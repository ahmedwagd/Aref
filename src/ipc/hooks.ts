import { useEffect, useRef, useState } from 'react';

import { getAppStatus, onKeyInvalid, onRateLimited } from './client';
import type { AppStatus, ErrorEnvelope, KeyInvalidEvent, RateLimitedEvent } from './types';

/** The five UI states every screen models (design.md §9), minus `partial`, which is humanize-only. */
export type AsyncState<T> =
  | { status: 'idle'; data: null; error: null }
  | { status: 'loading'; data: T | null; error: null }
  | { status: 'ready'; data: T; error: null }
  | { status: 'error'; data: null; error: ErrorEnvelope };

/**
 * A Tauri rejection is already the envelope (design.md §4.1), but a transport-level
 * failure arrives as a string. Normalising here keeps every component free of shape
 * checks and keeps raw rejection text out of the UI (AGENTS.md §9).
 */
export function toErrorEnvelope(value: unknown): ErrorEnvelope {
  if (typeof value === 'object' && value !== null) {
    const candidate = value as Partial<ErrorEnvelope>;
    if (typeof candidate.code === 'string' && typeof candidate.message_key === 'string') {
      return {
        code: candidate.code,
        message_key: candidate.message_key,
        params: candidate.params ?? {},
      };
    }
  }
  return { code: 'E_INTERNAL', message_key: 'error.internal', params: {} };
}

export function useAppStatus(): AsyncState<AppStatus> {
  const [state, setState] = useState<AsyncState<AppStatus>>({
    status: 'loading',
    data: null,
    error: null,
  });

  useEffect(() => {
    let active = true;
    getAppStatus()
      .then((data) => {
        if (active) setState({ status: 'ready', data, error: null });
      })
      .catch((error: unknown) => {
        if (active) setState({ status: 'error', data: null, error: toErrorEnvelope(error) });
      });
    return () => {
      active = false;
    };
  }, []);

  return state;
}

function useSubscription<T>(subscribe: (handler: (payload: T) => void) => Promise<() => void>, handler: (payload: T) => void): void {
  const handlerRef = useRef(handler);
  const subscribeRef = useRef(subscribe);

  useEffect(() => {
    handlerRef.current = handler;
  }, [handler]);

  useEffect(() => {
    const attach = subscribeRef.current;
    let active = true;
    let unlisten: (() => void) | undefined;

    attach((payload) => handlerRef.current(payload))
      .then((dispose) => {
        if (active) unlisten = dispose;
        else dispose();
      })
      .catch(() => {
        // A listener that cannot attach must not take the shell down; the next
        // status read still shows the same state.
      });

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);
}

export function useKeyInvalidEvents(handler: (payload: KeyInvalidEvent) => void): void {
  useSubscription(onKeyInvalid, handler);
}

export function useRateLimitedEvents(handler: (payload: RateLimitedEvent) => void): void {
  useSubscription(onRateLimited, handler);
}
