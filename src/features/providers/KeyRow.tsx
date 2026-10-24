import { useState } from 'react';

import { Button } from '../../components/Button';
import { MaskedPreview } from '../../components/MaskedPreview';
import { StatusPill } from '../../components/StatusPill';
import { SuppliedText } from '../../components/SuppliedText';
import type { Translate } from '../../i18n';
import type { KeyMetadata } from './hooks';
import { KeyForm } from './KeyForm';
import type { KeySubmission, ReplaceKeyHandler, ReplaceSecretHandler } from './types';

type KeyRowProps = {
  t: Translate;
  metadata: KeyMetadata;
  onReplace: ReplaceSecretHandler;
  onRemove: () => void;
};

/**
 * One stored key. The only key-derived value that reaches this component is the stored
 * `masked_preview` (`FR-310`, `G-27`), rendered verbatim — it is never re-derived here.
 *
 * `masked_preview` has a known direction (mask characters plus at most four key
 * characters, never Arabic), so it renders `dir="ltr"` inside `<bdi>` and keeps its order
 * in an Arabic page instead of mirroring (DESIGN.md §7.6, `NFR-403`).
 *
 * An `invalid` key stays in the list and carries a notice: `FR-305` invalidates without
 * deleting, and `FR-306` makes an edit the way back.
 */
export function KeyRow({ t, metadata, onReplace, onRemove }: KeyRowProps) {
  const [replacing, setReplacing] = useState(false);
  const invalid = metadata.status === 'invalid';

  const submitReplacement = async (submission: KeySubmission): Promise<void> => {
    await onReplace(submission.secret);
    setReplacing(false);
  };

  return (
    <div data-testid="key-row" data-key-status={metadata.status} className="flex flex-col gap-2">
      <div className="flex flex-wrap items-center gap-2">
        <SuppliedText value={metadata.label} className="text-sm" />
        <MaskedPreview value={metadata.masked_preview} className="text-xs text-muted-foreground" />
        <StatusPill
          text={invalid ? t('providers.key.status.invalid') : t('providers.key.status.valid')}
          variant={invalid ? 'destructive' : 'outline'}
        />
        <div className="ms-auto flex items-center gap-2">
          <Button variant="ghost" onClick={() => setReplacing((open) => !open)}>
            {t('providers.key.replace')}
          </Button>
          <Button variant="danger" onClick={onRemove}>
            {t('providers.key.remove')}
          </Button>
        </div>
      </div>

      {invalid ? (
        <p data-testid="key-invalid-notice" role="alert" className="text-xs text-destructive">
          {t('error.provider.key_invalid')}
        </p>
      ) : null}

      {replacing ? (
        <KeyForm
          t={t}
          mode="edit"
          onSubmit={submitReplacement}
          onCancel={() => setReplacing(false)}
        />
      ) : null}
    </div>
  );
}

type KeyListProps = {
  t: Translate;
  keys: readonly KeyMetadata[];
  onReplace: ReplaceKeyHandler;
  onRemove: (metadata: KeyMetadata) => void;
};

/** Empty is a state of its own: a provider with no key is normal (`FR-303`). */
export function KeyList({ t, keys, onReplace, onRemove }: KeyListProps) {
  if (keys.length === 0) {
    return <p className="text-xs text-muted-foreground">{t('providers.provider.no_keys')}</p>;
  }
  return (
    <ul className="flex flex-col divide-y divide-border">
      {keys.map((metadata) => (
        <li key={metadata.id} className="py-2">
          <KeyRow
            t={t}
            metadata={metadata}
            onReplace={(secret) => onReplace(metadata, secret)}
            onRemove={() => onRemove(metadata)}
          />
        </li>
      ))}
    </ul>
  );
}
