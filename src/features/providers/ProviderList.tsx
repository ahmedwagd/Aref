import { useState } from 'react';

import { Button } from '../../components/Button';
import { StatusPill } from '../../components/StatusPill';
import { SuppliedText } from '../../components/SuppliedText';
import type { Translate } from '../../i18n';
import type { KeyMetadata, ProviderView } from './hooks';
import { KeyForm } from './KeyForm';
import { KeyList } from './KeyRow';
import { providerKindKey } from './labels';
import type {
  AddKeyHandler,
  KeySubmission,
  ReplaceKeyHandler,
  SubmitKeyHandler,
} from './types';

type ProviderListProps = {
  t: Translate;
  providers: readonly ProviderView[];
  /** `provider_summary.selected_provider_id`: the entry the core would actually use. */
  selectedProviderId: string | null;
  onToggleEnabled: (provider: ProviderView) => void;
  onRemoveProvider: (provider: ProviderView) => void;
  onAddKey: AddKeyHandler;
  onReplaceKey: ReplaceKeyHandler;
  onRemoveKey: (metadata: KeyMetadata) => void;
};

/**
 * One card per `ProviderView` (DESIGN.md §7.3: cards are the unit provider management is
 * built from; a card never nests a card). The selection badge is the core's own answer —
 * `provider_summary.selected_provider_id` — not a local guess at `FR-312`'s order.
 *
 * Row state is a boolean for the add-key disclosure. No provider or key object is copied,
 * cached, or extended, so nothing can carry key material into a second place.
 */
export function ProviderList({
  t,
  providers,
  selectedProviderId,
  onToggleEnabled,
  onRemoveProvider,
  onAddKey,
  onReplaceKey,
  onRemoveKey,
}: ProviderListProps) {
  return (
    <ul data-testid="provider-list" className="flex flex-col gap-6">
      {providers.map((provider) => (
        <li key={provider.id}>
          <ProviderRow
            t={t}
            provider={provider}
            selected={provider.id === selectedProviderId}
            onToggleEnabled={() => onToggleEnabled(provider)}
            onRemoveProvider={() => onRemoveProvider(provider)}
            onAddKey={(submission) => onAddKey(provider.id, submission)}
            onReplaceKey={onReplaceKey}
            onRemoveKey={onRemoveKey}
          />
        </li>
      ))}
    </ul>
  );
}

type ProviderRowProps = {
  t: Translate;
  provider: ProviderView;
  selected: boolean;
  onToggleEnabled: () => void;
  onRemoveProvider: () => void;
  onAddKey: SubmitKeyHandler;
  onReplaceKey: ReplaceKeyHandler;
  onRemoveKey: (metadata: KeyMetadata) => void;
};

function ProviderRow({
  t,
  provider,
  selected,
  onToggleEnabled,
  onRemoveProvider,
  onAddKey,
  onReplaceKey,
  onRemoveKey,
}: ProviderRowProps) {
  const [adding, setAdding] = useState(false);

  const submitNewKey = async (submission: KeySubmission): Promise<void> => {
    await onAddKey(submission);
    setAdding(false);
  };

  return (
    <article
      data-testid="provider-row"
      data-provider-id={provider.id}
      data-provider-selected={selected ? 'true' : 'false'}
      className="rounded-3xl border bg-card p-5 text-card-foreground"
    >
      <header className="flex flex-wrap items-center gap-2">
        <h3 className="text-sm font-medium">
          <SuppliedText value={provider.label} />
        </h3>
        <StatusPill text={t(providerKindKey(provider))} />
        <StatusPill
          text={provider.enabled ? t('providers.provider.enabled') : t('providers.provider.disabled')}
          variant={provider.enabled ? 'outline' : 'secondary'}
        />
        {selected ? (
          <StatusPill text={t('providers.selection.badge')} variant="default" />
        ) : null}
        <div className="ms-auto flex items-center gap-2">
          <Button variant="ghost" onClick={onToggleEnabled}>
            {provider.enabled ? t('providers.provider.disable') : t('providers.provider.enable')}
          </Button>
          <Button variant="danger" onClick={onRemoveProvider}>
            {t('providers.provider.remove')}
          </Button>
        </div>
      </header>

      <dl className="mt-4 grid gap-2 text-xs sm:grid-cols-2">
        <div className="flex flex-col gap-0.5">
          <dt className="text-muted-foreground">{t('providers.provider.base_url')}</dt>
          <dd>
            <SuppliedText value={provider.base_url} className="font-mono break-all" />
          </dd>
        </div>
        <div className="flex flex-col gap-0.5">
          <dt className="text-muted-foreground">{t('providers.provider.model')}</dt>
          <dd>
            <SuppliedText value={provider.model_id} className="font-mono break-all" />
          </dd>
        </div>
      </dl>

      <div className="mt-4 border-t border-border pt-4">
        <h4 className="text-sm font-medium">{t('providers.provider.keys')}</h4>
        <div className="mt-2">
          <KeyList t={t} keys={provider.keys} onReplace={onReplaceKey} onRemove={onRemoveKey} />
        </div>
        <div className="mt-3">
          {adding ? (
            <KeyForm
              t={t}
              mode="add"
              onSubmit={submitNewKey}
              onCancel={() => setAdding(false)}
            />
          ) : (
            <Button variant="secondary" onClick={() => setAdding(true)}>
              {t('providers.provider.add_key')}
            </Button>
          )}
        </div>
      </div>
    </article>
  );
}
