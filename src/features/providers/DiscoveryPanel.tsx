import { useState } from 'react';

import { Button } from '../../components/Button';
import { ErrorMessage } from '../../components/ErrorMessage';
import { NumberText } from '../../components/NumberText';
import { SuppliedText } from '../../components/SuppliedText';
import type { Translate } from '../../i18n';
import { asErrorEnvelope, type DiscoveredProvider, type ErrorEnvelope } from './hooks';
import { flavorKeys } from './labels';
import type { AddDiscoveredHandler, ProbeHandler } from './types';

type DiscoveryPanelProps = {
  t: Translate;
  onProbe: ProbeHandler;
  onAdd: AddDiscoveredHandler;
};

/**
 * Discovery is user-initiated and loopback-only (`FR-304`, `NFR-206`): the probe runs when
 * this button is pressed and at no other time, so there is no effect and no timer in this
 * component — nothing here can fire on mount.
 *
 * A hit is an offer, not a change: `probe_local_providers` creates nothing, and adding is a
 * separate `add_provider` call, which keeps the user action true for the effect as well as
 * for the probe (design.md §7.2).
 */
export function DiscoveryPanel({ t, onProbe, onAdd }: DiscoveryPanelProps) {
  const [status, setStatus] = useState<'idle' | 'running' | 'done' | 'error'>('idle');
  const [found, setFound] = useState<readonly DiscoveredProvider[]>([]);
  const [error, setError] = useState<ErrorEnvelope | null>(null);

  const probe = async (): Promise<void> => {
    setStatus('running');
    setError(null);
    try {
      const hits = await onProbe();
      setFound(hits);
      setStatus('done');
    } catch (rejection: unknown) {
      setError(asErrorEnvelope(rejection));
      setStatus('error');
    }
  };

  return (
    <section
      data-testid="discovery-panel"
      className="rounded-3xl border bg-card p-5 text-card-foreground"
    >
      <div className="flex flex-col gap-1">
        <h3 className="text-sm font-medium">{t('providers.discovery.title')}</h3>
        <p className="text-xs text-muted-foreground">{t('providers.discovery.body')}</p>
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Button variant="primary" onClick={() => void probe()} disabled={status === 'running'}>
          {t('providers.discovery.action')}
        </Button>
        {status === 'running' ? (
          <p role="status" className="text-xs text-muted-foreground">
            {t('providers.discovery.running')}
          </p>
        ) : null}
      </div>

      {error !== null ? (
        <div className="mt-3">
          <ErrorMessage t={t} envelope={error} />
        </div>
      ) : null}

      {status === 'done' && found.length === 0 ? (
        <p data-testid="discovery-none" className="mt-3 text-xs text-muted-foreground">
          {t('providers.discovery.none')}
        </p>
      ) : null}

      {found.length > 0 ? (
        <ul className="mt-4 flex flex-col gap-4">
          {found.map((hit) => (
            <li key={hit.local_flavor}>
              <DiscoveryHit t={t} hit={hit} onAdd={onAdd} />
            </li>
          ))}
        </ul>
      ) : null}
    </section>
  );
}

type DiscoveryHitProps = {
  t: Translate;
  hit: DiscoveredProvider;
  onAdd: AddDiscoveredHandler;
};

function DiscoveryHit({ t, hit, onAdd }: DiscoveryHitProps) {
  const [label, setLabel] = useState<string>(() => t(flavorKeys[hit.local_flavor]));
  const [modelId, setModelId] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<ErrorEnvelope | null>(null);

  const submit = async (): Promise<void> => {
    if (modelId.trim() === '') {
      setError({
        code: 'E_INPUT_EMPTY',
        message_key: 'error.input.empty',
        params: { field: 'model_id' },
      });
      return;
    }
    setPending(true);
    setError(null);
    try {
      await onAdd(hit, { label: label.trim(), model_id: modelId.trim() });
    } catch (rejection: unknown) {
      setError(asErrorEnvelope(rejection));
    } finally {
      setPending(false);
    }
  };

  return (
    <div data-testid="discovery-hit" className="flex flex-col gap-2 rounded-2xl border p-4">
      <div className="flex flex-wrap items-center gap-2 text-xs">
        <span className="font-medium">{t(flavorKeys[hit.local_flavor])}</span>
        <SuppliedText value={hit.base_url} className="font-mono text-muted-foreground" />
        <span className="text-muted-foreground">{t('providers.discovery.port')}</span>
        <NumberText value={String(hit.port)} />
      </div>

      <div className="flex flex-wrap items-end gap-2">
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          <span>{t('providers.discovery.label')}</span>
          <input
            type="text"
            value={label}
            disabled={pending}
            onChange={(event) => setLabel(event.target.value)}
            className="h-9 rounded-lg border border-input bg-background px-3 text-sm text-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none"
          />
        </label>
        <label className="flex flex-col gap-1 text-xs text-muted-foreground">
          <span>{t('providers.discovery.model')}</span>
          <input
            type="text"
            value={modelId}
            disabled={pending}
            dir="ltr"
            onChange={(event) => setModelId(event.target.value)}
            className="h-9 rounded-lg border border-input bg-background px-3 font-mono text-sm text-foreground focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none"
          />
        </label>
        <Button variant="secondary" onClick={() => void submit()} disabled={pending}>
          {t('providers.discovery.add')}
        </Button>
      </div>

      {error !== null ? <ErrorMessage t={t} envelope={error} /> : null}
    </div>
  );
}
