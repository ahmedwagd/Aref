import { useRef, useState, type FormEvent } from 'react';

import { Button } from '../../components/Button';
import { ErrorMessage } from '../../components/ErrorMessage';
import type { MessageKey, Translate } from '../../i18n';
import { asErrorEnvelope, type ErrorEnvelope } from './hooks';
import type { SubmitKeyHandler } from './types';

type KeyFormProps = {
  t: Translate;
  mode: 'add' | 'edit';
  onSubmit: SubmitKeyHandler;
  onCancel: () => void;
};

/**
 * The one place in the frontend where a secret exists.
 *
 * The secret lives in one uncontrolled field behind `secretRef` — not in state, not in a
 * context, not in a query string — and the field is emptied when the submit settles,
 * whether it resolved or rejected (design.md §9, `NFR-201`). Nothing derived from it is
 * stored: the core answers with a masked preview, and that is what the list renders.
 *
 * A `keyring_unavailable` answer is a blocking state rather than a retryable field error:
 * `FR-313` forbids any fallback store, so the form stops offering the action instead of
 * inviting a second attempt that cannot succeed until the keyring is repaired.
 */
export function KeyForm({ t, mode, onSubmit, onCancel }: KeyFormProps) {
  const secretRef = useRef<HTMLInputElement>(null);
  const [label, setLabel] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<ErrorEnvelope | null>(null);

  const blocked = error?.code === 'E_KEYRING_UNAVAILABLE';

  const validation = (field: 'label' | 'secret'): ErrorEnvelope => ({
    code: 'E_INPUT_EMPTY',
    message_key: 'error.input.empty',
    params: { field },
  });

  const submit = async (): Promise<void> => {
    const field = secretRef.current;
    const secret = field === null ? '' : field.value;

    if (secret.trim() === '') {
      setError(validation('secret'));
      return;
    }
    if (mode === 'add' && label.trim() === '') {
      setError(validation('label'));
      return;
    }

    setPending(true);
    setError(null);
    try {
      await onSubmit({ secret, label: mode === 'add' ? label.trim() : null });
    } catch (rejection: unknown) {
      setError(asErrorEnvelope(rejection));
    } finally {
      // Both paths, on purpose: a rejected save must not leave the value in the DOM for a
      // second attempt to resend.
      if (secretRef.current !== null) secretRef.current.value = '';
      setPending(false);
    }
  };

  const handleSubmit = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    void submit();
  };

  const saveKey: MessageKey = mode === 'add' ? 'providers.key.save' : 'providers.key.replace';

  return (
    <form data-testid="key-form" onSubmit={handleSubmit} className="flex flex-col gap-3">
      {mode === 'add' ? (
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          <span>{t('providers.key.label')}</span>
          <input
            type="text"
            value={label}
            disabled={blocked}
            onChange={(event) => setLabel(event.target.value)}
            className="h-9 rounded-lg border border-input bg-background px-3 text-sm text-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none"
          />
        </label>
      ) : null}

      <label className="flex flex-col gap-1 text-xs text-muted-foreground">
        <span>{t('providers.key.secret')}</span>
        <input
          ref={secretRef}
          type="password"
          autoComplete="off"
          disabled={blocked}
          data-testid="key-form-secret"
          className="h-9 rounded-lg border border-input bg-background px-3 font-mono text-sm text-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none"
        />
      </label>
      <p className="text-xs text-muted-foreground">{t('providers.key.secret_hint')}</p>

      {error !== null && !blocked ? <ErrorMessage t={t} envelope={error} /> : null}

      {blocked ? (
        <div
          data-testid="keyring-blocked"
          role="alert"
          className="rounded-lg border border-destructive/40 bg-destructive/10 p-3"
        >
          <p className="text-sm font-medium text-destructive">{t('providers.key.blocked_title')}</p>
          <p className="mt-1 text-xs text-destructive">{t('error.security.keyring_unavailable')}</p>
        </div>
      ) : null}

      <div className="flex flex-wrap items-center gap-2">
        <Button type="submit" variant="primary" disabled={pending || blocked}>
          {t(saveKey)}
        </Button>
        <Button variant="ghost" onClick={onCancel}>
          {t('providers.key.cancel')}
        </Button>
      </div>
    </form>
  );
}
