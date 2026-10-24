import type { ReactNode } from 'react';

import { Button } from '../../components/Button';
import { ErrorMessage } from '../../components/ErrorMessage';
import { MaskedPreview } from '../../components/MaskedPreview';
import { NumberText } from '../../components/NumberText';
import type { Translate } from '../../i18n';
import { DiscoveryPanel } from './DiscoveryPanel';
import { HumanizeDisabledNotice } from './HumanizeDisabledNotice';
import { ProviderList } from './ProviderList';
import {
  useProvidersPage,
  type AppStatus,
  type AsyncState,
  type DiscoveredProvider,
  type ProviderEvents,
} from './hooks';
import type { KeySubmission } from './types';

type ProvidersScreenProps = {
  t: Translate;
  /** The shell's status read: `humanize_available` and the selected provider come from it. */
  status: AsyncState<AppStatus>;
  onOpenSettings: () => void;
};

/**
 * The Providers screen (requirement 19 of the delta specs; `FR-311`, `FR-312`).
 *
 * It models the states design.md §9 names — `loading`, `empty`, `ready`, `error` — plus the
 * two that are not the list's: `keyring_unavailable` as the blocking state inside `KeyForm`
 * (`FR-313`), and `invalid` as the per-key notice (`FR-305`). Every string resolves from the
 * catalog in both locales, and every direction-dependent element uses a logical property, so
 * the same tree renders in `ar` and in `en`.
 */
export function ProvidersScreen({ t, status, onOpenSettings }: ProvidersScreenProps) {
  const page = useProvidersPage();
  const query = page.providers;

  const available = status.status !== 'ready' || status.data.humanize_available;
  const selectedProviderId =
    status.status === 'ready' ? status.data.provider_summary.selected_provider_id : null;

  // A key save renders its own failure inside the form that submitted it.
  const mutationError = page.mutation.status === 'error' ? page.mutation.error : null;
  const formOwnsError =
    page.mutation.operation === 'addKey' || page.mutation.operation === 'editKey';

  let body: ReactNode;
  if (query.status === 'loading' || query.status === 'idle') {
    body = (
      <p role="status" data-testid="providers-loading" className="text-sm text-muted-foreground">
        {t('providers.loading')}
      </p>
    );
  } else if (query.status === 'error') {
    body = (
      <section
        data-testid="providers-error"
        className="flex flex-col gap-3 rounded-3xl border bg-card p-5 text-card-foreground"
      >
        <h3 className="text-sm font-medium">{t('providers.error.title')}</h3>
        <ErrorMessage t={t} envelope={query.error} />
        <div>
          <Button variant="secondary" onClick={page.reload}>
            {t('providers.error.retry')}
          </Button>
        </div>
      </section>
    );
  } else if (query.data.providers.length === 0) {
    body = (
      <section
        data-testid="providers-empty"
        className="rounded-3xl border bg-card p-5 text-card-foreground"
      >
        <h3 className="text-sm font-medium">{t('providers.empty.title')}</h3>
        <p className="mt-1 text-xs text-muted-foreground">{t('providers.empty.body')}</p>
      </section>
    );
  } else {
    body = (
      <ProviderList
        t={t}
        providers={query.data.providers}
        selectedProviderId={selectedProviderId}
        onToggleEnabled={(provider) => {
          void page
            .updateProvider({ id: provider.id, enabled: !provider.enabled })
            .catch(() => {
              // The failure is in `page.mutation` and rendered by the notice above.
            });
        }}
        onRemoveProvider={(provider) => {
          void page.removeProvider({ id: provider.id }).catch(() => {
            // Same notice; a removal cannot fail silently.
          });
        }}
        onAddKey={async (providerId: string, submission: KeySubmission) => {
          await page.addKey({
            provider_id: providerId,
            label: submission.label ?? '',
            secret: submission.secret,
          });
        }}
        onReplaceKey={async (metadata, secret) => {
          await page.editKey({ key_id: metadata.id, secret });
        }}
        onRemoveKey={(metadata) => {
          void page.removeKey({ key_id: metadata.id }).catch(() => {
            // Same notice.
          });
        }}
      />
    );
  }

  return (
    <section data-testid="providers-screen" className="flex flex-col gap-6">
      <header className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex flex-col gap-1">
          <h2 className="text-sm font-medium">{t('providers.title')}</h2>
          <p className="max-w-prose text-xs text-muted-foreground">{t('providers.intro')}</p>
        </div>
        <Button variant="secondary" onClick={page.reload} disabled={query.status === 'loading'}>
          {t('providers.refresh')}
        </Button>
      </header>

      <HumanizeDisabledNotice t={t} available={available} onOpenSettings={onOpenSettings} />

      <ProviderEventNotices t={t} events={page.events} />

      {mutationError !== null && !formOwnsError ? (
        <ErrorMessage t={t} envelope={mutationError} />
      ) : null}

      {body}

      <DiscoveryPanel
        t={t}
        onProbe={page.probe}
        onAdd={async (hit: DiscoveredProvider, input: { label: string; model_id: string }) => {
          await page.addProvider({
            kind: 'local',
            local_flavor: hit.local_flavor,
            base_url: hit.base_url,
            model_id: input.model_id,
            label: input.label,
            enabled: true,
          });
        }}
      />
    </section>
  );
}

type ProviderEventNoticesProps = {
  t: Translate;
  events: ProviderEvents;
};

/**
 * The two events the core pushes (`design.md` §4.3). `providers://key_invalid` names the key
 * by its stored masked preview — the only key-derived value that may cross IPC — and never
 * by a secret or a label.
 */
function ProviderEventNotices({ t, events }: ProviderEventNoticesProps) {
  if (events.keyInvalid === null && events.rateLimited === null) return null;

  return (
    <div className="flex flex-col gap-2">
      {events.keyInvalid !== null ? (
        <p
          data-testid="key-invalid-event"
          role="status"
          className="flex flex-wrap items-center gap-2 text-xs text-destructive"
        >
          <span>{t('error.provider.key_invalid')}</span>
          <MaskedPreview value={events.keyInvalid.masked_preview} className="text-xs" />
        </p>
      ) : null}
      {events.rateLimited !== null ? (
        <p
          data-testid="rate-limited-event"
          role="status"
          className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground"
        >
          <span>{t('error.provider.rate_limited')}</span>
          <span>{t('providers.rate_limited.wait')}</span>
          <NumberText value={String(events.rateLimited.retry_after_seconds)} />
          <span>{t('providers.rate_limited.seconds_short')}</span>
        </p>
      ) : null}
    </div>
  );
}
