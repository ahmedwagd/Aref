import type { MessageKey, Translate } from '../i18n';
import en from '../i18n/en.json';
import type { ErrorEnvelope } from '../ipc';

const knownMessageKeys: ReadonlySet<string> = new Set(Object.keys(en));

/**
 * The envelope's `message_key` is a `string` on the wire (design.md §4.1). The catalog is
 * the closed set, so membership is checked before the translator sees it — an unknown key
 * falls back to the generic message instead of rendering nothing, which would be the silent
 * failure AGENTS.md §9 forbids.
 */
export function messageKeyFor(envelope: ErrorEnvelope): MessageKey {
  return knownMessageKeys.has(envelope.message_key)
    ? (envelope.message_key as MessageKey)
    : 'error.internal';
}

type ErrorMessageProps = {
  t: Translate;
  envelope: ErrorEnvelope;
  className?: string;
};

/**
 * The one renderer of a core error. It shows the catalog sentence for `message_key` and
 * keeps `code` in a data attribute for diagnostics; `params` is never interpolated, so no
 * value from the core can become UI text (design.md §4.1).
 */
export function ErrorMessage({ t, envelope, className }: ErrorMessageProps) {
  const base = 'text-sm text-destructive';
  return (
    <p
      role="alert"
      data-testid="error-envelope"
      data-error-code={envelope.code}
      className={className === undefined ? base : `${base} ${className}`}
    >
      {t(messageKeyFor(envelope))}
    </p>
  );
}
