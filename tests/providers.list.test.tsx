import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { ProviderList } from '../src/features/providers/ProviderList';
import type { KeyMetadata, ProviderView } from '../src/features/providers/hooks';
import { createTranslator, locales, type Locale } from '../src/i18n';
import {
  fixtureSecret,
  keyMetadata,
  localProvider,
  maskedPreview,
  providerView,
} from './support/providers';

const noopProvider = (): void => {};
const noopKey = (): void => {};
const resolved = (): Promise<void> => Promise.resolve();

/**
 * A key object carrying a property the contract does not have. If any row spread its
 * `KeyMetadata` into the DOM, the canary below would surface — the check is that the tree is
 * built field by field, not that the fixture happens to be free of secrets.
 */
function keyWithCanary(): KeyMetadata {
  return {
    ...keyMetadata({ id: 'key-cloud', provider_id: 'provider-cloud' }),
    secret: fixtureSecret,
  } as unknown as KeyMetadata;
}

function threeProviders(): ProviderView[] {
  return [
    localProvider('opencode'),
    localProvider('ollama'),
    providerView({ keys: [keyWithCanary()] }),
  ];
}

function renderList(locale: Locale, providers: ProviderView[], selectedProviderId: string | null) {
  const t = createTranslator(locale);
  return render(
    <ProviderList
      t={t}
      providers={providers}
      selectedProviderId={selectedProviderId}
      onToggleEnabled={noopProvider}
      onRemoveProvider={noopProvider}
      onAddKey={resolved}
      onReplaceKey={resolved}
      onRemoveKey={noopKey}
    />,
  );
}

describe.each(locales)('ProviderList in %s', (locale: Locale) => {
  const t = createTranslator(locale);

  it('renders one row per provider', () => {
    renderList(locale, threeProviders(), 'provider-cloud');
    expect(screen.getAllByTestId('provider-row')).toHaveLength(3);
  });

  it('puts the selection badge on the id the status names, not on the first row', () => {
    // Ollama is the second row and is not the highest-ranked entry, so a badge derived from
    // position or from `FR-312`'s order locally would land on the wrong row.
    renderList(locale, threeProviders(), 'provider-ollama');

    const selected = screen
      .getAllByTestId('provider-row')
      .filter((row) => row.getAttribute('data-provider-selected') === 'true');

    expect(selected).toHaveLength(1);
    expect(selected[0]?.getAttribute('data-provider-id')).toBe('provider-ollama');
    expect(screen.getAllByText(t('providers.selection.badge'))).toHaveLength(1);
    expect(
      within(selected[0] as HTMLElement).getByText(t('providers.selection.badge')),
    ).toBeInTheDocument();
  });

  it('shows no badge when the core selected nothing', () => {
    renderList(locale, threeProviders(), null);
    expect(screen.queryByText(t('providers.selection.badge'))).toBeNull();
  });

  it('renders the stored masked preview and no secret anywhere in the tree', () => {
    const { container } = renderList(locale, threeProviders(), 'provider-cloud');

    expect(container.textContent).toContain(maskedPreview);
    expect(container.innerHTML).not.toContain(fixtureSecret);
    expect(container.textContent ?? '').not.toContain(fixtureSecret);
  });

  it('gives each provider a stable id on its row and its supplied text dir auto', () => {
    renderList(locale, threeProviders(), 'provider-cloud');

    const rows = screen.getAllByTestId('provider-row');
    expect(rows.map((row) => row.getAttribute('data-provider-id'))).toEqual([
      'provider-opencode',
      'provider-ollama',
      'provider-cloud',
    ]);

    const cloud = rows[2];
    expect(cloud).toBeDefined();
    if (cloud === undefined) return;
    expect(within(cloud).getByText('https://api.example.invalid/v1')).toHaveAttribute('dir', 'auto');
    expect(within(cloud).getByText('fixture-model-a')).toHaveAttribute('dir', 'auto');
    expect(within(cloud).getByText('Cloud entry')).toHaveAttribute('dir', 'auto');
  });
});
