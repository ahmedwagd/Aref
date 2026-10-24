import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { App } from '../src/App';
import { ProvidersScreen } from '../src/features/providers/ProvidersScreen';
import type { AsyncState, KeyInvalidEvent, RateLimitedEvent } from '../src/features/providers/hooks';
import type * as IpcModule from '../src/ipc';
import type { AppStatus, ListProvidersResponse } from '../src/ipc';
import { applyDocumentLocale, createTranslator, directionFor, locales, type Locale } from '../src/i18n';
import { addKey, listProviders, onKeyInvalid, onRateLimited } from '../src/ipc';
import {
  appStatus,
  fixtureSecret,
  keyMetadata,
  localProvider,
  maskedPreview,
  providerView,
  readyStatus,
} from './support/providers';

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
  vi.mocked(onKeyInvalid).mockImplementation(() => Promise.resolve(detach));
  vi.mocked(onRateLimited).mockImplementation(() => Promise.resolve(detach));
  vi.mocked(listProviders).mockResolvedValue({ providers: [] });
});

function inputByLabel(scope: HTMLElement, label: string): HTMLInputElement {
  const element = within(scope).getByLabelText(label);
  if (!(element instanceof HTMLInputElement)) {
    throw new TypeError(`${label} is not an input`);
  }
  return element;
}

function renderScreen(
  locale: Locale,
  status: AsyncState<AppStatus> = readyStatus(),
  onOpenSettings: () => void = () => {},
) {
  const t = createTranslator(locale);
  return render(<ProvidersScreen t={t} status={status} onOpenSettings={onOpenSettings} />);
}

/** A promise the test settles, so the loading state is observable rather than skipped. */
function deferredQuery(): { promise: Promise<ListProvidersResponse>; settle: (value: ListProvidersResponse) => void } {
  let settle: (value: ListProvidersResponse) => void = () => {};
  const promise = new Promise<ListProvidersResponse>((resolve) => {
    settle = resolve;
  });
  return { promise, settle };
}

describe.each(locales)('ProvidersScreen in %s', (locale: Locale) => {
  const t = createTranslator(locale);

  it('walks loading to ready to error in one screen', async () => {
    const pending = deferredQuery();
    vi.mocked(listProviders).mockImplementation(() => pending.promise);

    renderScreen(locale);
    expect(screen.getByTestId('providers-loading')).toHaveTextContent(t('providers.loading'));

    act(() => {
      pending.settle({ providers: [] });
    });

    expect(await screen.findByTestId('providers-empty')).toHaveTextContent(
      t('providers.empty.title'),
    );
    expect(screen.queryByTestId('providers-loading')).toBeNull();

    vi.mocked(listProviders).mockRejectedValue({
      code: 'E_INTERNAL',
      message_key: 'error.internal',
      params: {},
    });
    fireEvent.click(screen.getByRole('button', { name: t('providers.refresh') }));

    const failure = await screen.findByTestId('providers-error');
    expect(failure).toHaveTextContent(t('providers.error.title'));
    expect(within(failure).getByTestId('error-envelope')).toHaveTextContent(t('error.internal'));
    expect(
      within(failure).getByRole('button', { name: t('providers.error.retry') }),
    ).toBeInTheDocument();
  });

  it('retries a failed read from the error state', async () => {
    vi.mocked(listProviders).mockRejectedValueOnce({
      code: 'E_INTERNAL',
      message_key: 'error.internal',
      params: {},
    });
    renderScreen(locale);

    const failure = await screen.findByTestId('providers-error');
    vi.mocked(listProviders).mockResolvedValue({ providers: [providerView()] });
    fireEvent.click(within(failure).getByRole('button', { name: t('providers.error.retry') }));

    await waitFor(() => expect(screen.getAllByTestId('provider-row')).toHaveLength(1));
  });

  it('renders the ready state, with the badge on the selected provider', async () => {
    vi.mocked(listProviders).mockResolvedValue({
      providers: [localProvider('opencode'), providerView()],
    });
    renderScreen(locale, readyStatus());

    await waitFor(() => expect(screen.getAllByTestId('provider-row')).toHaveLength(2));

    const selected = screen
      .getAllByTestId('provider-row')
      .filter((row) => row.getAttribute('data-provider-selected') === 'true');
    expect(selected).toHaveLength(1);
    expect(selected[0]?.getAttribute('data-provider-id')).toBe('provider-cloud');
    expect(screen.queryByTestId('humanize-disabled-notice')).toBeNull();
  });

  it('renders the empty state and the disabled-Humanize guidance', async () => {
    const onOpenSettings = vi.fn((): void => {});
    renderScreen(locale, readyStatus({ humanize_available: false }), onOpenSettings);

    expect(await screen.findByTestId('providers-empty')).toHaveTextContent(
      t('providers.empty.body'),
    );
    const notice = screen.getByTestId('humanize-disabled-notice');
    expect(notice).toHaveTextContent(t('error.provider.none'));

    fireEvent.click(
      within(notice).getByRole('button', { name: t('providers.humanize_disabled.action') }),
    );
    expect(onOpenSettings).toHaveBeenCalledTimes(1);
  });

  it('renders the blocking keyring state when a key save is refused', async () => {
    vi.mocked(listProviders).mockResolvedValue({ providers: [providerView()] });
    vi.mocked(addKey).mockRejectedValue({
      code: 'E_KEYRING_UNAVAILABLE',
      message_key: 'error.security.keyring_unavailable',
      params: { reason: 'unavailable' },
    });

    renderScreen(locale);
    await waitFor(() => expect(screen.getAllByTestId('provider-row')).toHaveLength(1));

    fireEvent.click(screen.getByRole('button', { name: t('providers.provider.add_key') }));
    const form = screen.getByTestId('key-form');
    fireEvent.change(inputByLabel(form, t('providers.key.label')), {
      target: { value: 'Work' },
    });
    fireEvent.change(inputByLabel(form, t('providers.key.secret')), {
      target: { value: fixtureSecret },
    });
    fireEvent.click(within(form).getByRole('button', { name: t('providers.key.save') }));

    const blocked = await screen.findByTestId('keyring-blocked');
    expect(blocked).toHaveTextContent(t('error.security.keyring_unavailable'));
    expect(inputByLabel(form, t('providers.key.secret')).value).toBe('');
    // `FR-313`: the screen keeps working — the list is still there, and no other store was
    // offered in place of the keyring.
    expect(screen.getAllByTestId('provider-row')).toHaveLength(1);
  });

  it('shows a notice on the invalid key and keeps the masked preview', async () => {
    vi.mocked(listProviders).mockResolvedValue({
      providers: [
        providerView({
          keys: [keyMetadata({ status: 'invalid', invalid_since: '2026-10-22T00:00:00Z' })],
        }),
      ],
    });

    renderScreen(locale);

    expect(await screen.findByTestId('key-invalid-notice')).toHaveTextContent(
      t('error.provider.key_invalid'),
    );
    expect(screen.getByTestId('key-row')).toHaveAttribute('data-key-status', 'invalid');
    expect(screen.getByTestId('masked-preview')).toHaveTextContent(maskedPreview);
  });

  it('surfaces both pushed events with no secret in either', async () => {
    let invalid: ((payload: KeyInvalidEvent) => void) | null = null;
    let limited: ((payload: RateLimitedEvent) => void) | null = null;
    vi.mocked(onKeyInvalid).mockImplementation((next) => {
      invalid = next;
      return Promise.resolve(detach);
    });
    vi.mocked(onRateLimited).mockImplementation((next) => {
      limited = next;
      return Promise.resolve(detach);
    });

    renderScreen(locale);
    await waitFor(() => expect(invalid).not.toBeNull());
    await waitFor(() => expect(limited).not.toBeNull());

    act(() => {
      invalid?.({
        provider_id: 'provider-cloud',
        key_id: 'key-a',
        masked_preview: maskedPreview,
      });
      limited?.({
        provider_id: 'provider-cloud',
        retry_after_seconds: 120,
        resets_at: '2026-10-22T00:02:00Z',
      });
    });

    expect(screen.getByTestId('key-invalid-event')).toHaveTextContent(maskedPreview);
    expect(screen.getByTestId('rate-limited-event')).toHaveTextContent('120');
  });

  it('sets the document direction from the locale and keeps the preview unmirrored', async () => {
    applyDocumentLocale(locale);
    vi.mocked(listProviders).mockResolvedValue({
      providers: [providerView({ keys: [keyMetadata()] })],
    });

    renderScreen(locale);

    expect(document.documentElement.dir).toBe(directionFor(locale));
    expect(document.documentElement.lang).toBe(locale);
    const preview = await screen.findByTestId('masked-preview');
    expect(preview).toHaveAttribute('dir', 'ltr');
    expect(preview.textContent).toBe(maskedPreview);
  });
});

describe('ProvidersScreen through the shell', () => {
  it('mounts on the Providers navigation item', async () => {
    const locale: Locale = 'ar';
    const t = createTranslator(locale);
    vi.mocked(listProviders).mockResolvedValue({ providers: [providerView()] });

    render(<App locale={locale} />);
    fireEvent.click(screen.getByRole('button', { name: t('nav.providers') }));

    expect(await screen.findByTestId('providers-screen')).toBeInTheDocument();
    expect(await screen.findByTestId('provider-row')).toHaveAttribute(
      'data-provider-id',
      'provider-cloud',
    );
    // The placeholder card is replaced by the screen, not stacked above it.
    expect(screen.queryByText(t('shell.workspace_placeholder'))).toBeNull();
  });

  it('carries the status the shell read into the Humanize notice', async () => {
    const locale: Locale = 'en';
    const t = createTranslator(locale);
    renderScreen(locale, { status: 'ready', data: appStatus({ humanize_available: false }), error: null });

    expect(await screen.findByTestId('humanize-disabled-notice')).toHaveTextContent(
      t('error.provider.none'),
    );
  });
});
