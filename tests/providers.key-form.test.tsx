import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { KeyForm } from '../src/features/providers/KeyForm';
import { IpcFailure, type ErrorEnvelope } from '../src/features/providers/hooks';
import { createTranslator, locales, type Locale } from '../src/i18n';
import { fixtureSecret, storageDump } from './support/providers';

const noop = (): void => {};

function inputByLabel(label: string): HTMLInputElement {
  const element = screen.getByLabelText(label);
  if (!(element instanceof HTMLInputElement)) {
    throw new TypeError(`${label} is not an input`);
  }
  return element;
}

/** Everything a value could be copied into besides the field itself (`NFR-201`). */
function placesOutsideTheField(): string {
  return [window.location.href, storageDump(), document.documentElement.outerHTML].join('\n');
}

/** The shape the feature's own hook rejects with: an Error carrying the redacted envelope. */
function rejectWith(envelope: ErrorEnvelope): Promise<never> {
  return Promise.reject(new IpcFailure(envelope));
}

describe.each(locales)('KeyForm in %s', (locale: Locale) => {
  const t = createTranslator(locale);

  it('clears the field when the submit rejects and leaves no copy behind', async () => {
    const onSubmit = vi.fn(() =>
      rejectWith({ code: 'E_INTERNAL', message_key: 'error.internal', params: {} }),
    );

    render(<KeyForm t={t} mode="add" onSubmit={onSubmit} onCancel={noop} />);

    const field = inputByLabel(t('providers.key.secret'));
    fireEvent.change(field, { target: { value: fixtureSecret } });
    expect(field.value).toBe(fixtureSecret);
    // The value is a DOM property for the life of the submit: it is not an attribute, a
    // storage entry, or a query parameter at any point.
    expect(placesOutsideTheField()).not.toContain(fixtureSecret);

    fireEvent.change(inputByLabel(t('providers.key.label')), { target: { value: 'Work' } });
    fireEvent.click(screen.getByRole('button', { name: t('providers.key.save') }));

    await waitFor(() => expect(field.value).toBe(''));
    expect(onSubmit).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId('error-envelope')).toHaveAttribute('data-error-code', 'E_INTERNAL');
    expect(placesOutsideTheField()).not.toContain(fixtureSecret);
    expect(screen.getByRole('button', { name: t('providers.key.save') })).toBeEnabled();
  });

  it('renders the blocking state for a keyring_unavailable answer', async () => {
    const onSubmit = vi.fn(() =>
      rejectWith({
        code: 'E_KEYRING_UNAVAILABLE',
        message_key: 'error.security.keyring_unavailable',
        params: { reason: 'unavailable' },
      }),
    );

    render(<KeyForm t={t} mode="edit" onSubmit={onSubmit} onCancel={noop} />);

    fireEvent.change(inputByLabel(t('providers.key.secret')), {
      target: { value: fixtureSecret },
    });
    fireEvent.click(screen.getByRole('button', { name: t('providers.key.replace') }));

    const blocked = await screen.findByTestId('keyring-blocked');
    expect(blocked).toHaveTextContent(t('providers.key.blocked_title'));
    expect(blocked).toHaveTextContent(t('error.security.keyring_unavailable'));
    // Blocking, not retryable: `FR-313` forbids any fallback store, so every control is
    // disabled rather than inviting a second attempt that cannot succeed, and the value the
    // user typed is gone from the field.
    const field = inputByLabel(t('providers.key.secret'));
    expect(field).toBeDisabled();
    expect(field.value).toBe('');
    expect(screen.getByRole('button', { name: t('providers.key.replace') })).toBeDisabled();
    expect(placesOutsideTheField()).not.toContain(fixtureSecret);
  });

  it('never hands a blank secret to the caller', () => {
    const onSubmit = vi.fn(() => Promise.resolve());

    render(<KeyForm t={t} mode="add" onSubmit={onSubmit} onCancel={noop} />);
    fireEvent.change(inputByLabel(t('providers.key.label')), { target: { value: 'Work' } });
    fireEvent.click(screen.getByRole('button', { name: t('providers.key.save') }));

    expect(onSubmit).not.toHaveBeenCalled();
    expect(screen.getByTestId('error-envelope')).toHaveAttribute('data-error-code', 'E_INPUT_EMPTY');
  });

  it('sends the label in add mode and none in edit mode', async () => {
    const added = vi.fn(() => Promise.resolve());
    const edited = vi.fn(() => Promise.resolve());

    const first = render(<KeyForm t={t} mode="add" onSubmit={added} onCancel={noop} />);
    fireEvent.change(inputByLabel(t('providers.key.label')), { target: { value: 'Work' } });
    fireEvent.change(inputByLabel(t('providers.key.secret')), {
      target: { value: fixtureSecret },
    });
    fireEvent.click(screen.getByRole('button', { name: t('providers.key.save') }));
    await waitFor(() => expect(added).toHaveBeenCalledTimes(1));
    expect(added).toHaveBeenCalledWith({ secret: fixtureSecret, label: 'Work' });
    first.unmount();

    render(<KeyForm t={t} mode="edit" onSubmit={edited} onCancel={noop} />);
    fireEvent.change(inputByLabel(t('providers.key.secret')), {
      target: { value: fixtureSecret },
    });
    fireEvent.click(screen.getByRole('button', { name: t('providers.key.replace') }));
    await waitFor(() => expect(edited).toHaveBeenCalledTimes(1));
    expect(edited).toHaveBeenCalledWith({ secret: fixtureSecret, label: null });
  });
});
