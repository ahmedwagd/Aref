import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { DiscoveryPanel } from '../src/features/providers/DiscoveryPanel';
import type { DiscoveredProvider } from '../src/features/providers/hooks';
import type { AddDiscoveredHandler } from '../src/features/providers/types';
import { createTranslator, locales, type Locale } from '../src/i18n';
import { addProvider, probeLocalProviders } from '../src/ipc';
import type * as IpcModule from '../src/ipc';
import { discovered } from './support/providers';

vi.mock('../src/ipc', async (importOriginal) => {
  const actual = await importOriginal<typeof IpcModule>();
  return {
    ...actual,
    addProvider: vi.fn(),
    probeLocalProviders: vi.fn(),
  };
});

beforeEach(() => {
  vi.mocked(probeLocalProviders).mockResolvedValue({
    found: [discovered('ollama', 11434), discovered('lm_studio', 1234)],
  });
});

function renderPanel(locale: Locale, onAdd: AddDiscoveredHandler) {
  const t = createTranslator(locale);
  const onProbe = (): Promise<readonly DiscoveredProvider[]> =>
    probeLocalProviders().then((response) => response.found);
  return render(<DiscoveryPanel t={t} onProbe={onProbe} onAdd={onAdd} />);
}

function inputByLabel(scope: HTMLElement, label: string): HTMLInputElement {
  const element = within(scope).getByLabelText(label);
  if (!(element instanceof HTMLInputElement)) {
    throw new TypeError(`${label} is not an input`);
  }
  return element;
}

function hitAt(index: number): HTMLElement {
  const hits = screen.getAllByTestId('discovery-hit');
  const hit = hits[index];
  if (hit === undefined) {
    throw new TypeError(`no discovery hit at ${index}`);
  }
  return hit;
}

describe.each(locales)('DiscoveryPanel in %s', (locale: Locale) => {
  const t = createTranslator(locale);

  it('probes nothing until the button is pressed, then exactly once', async () => {
    const onAdd = vi.fn(() => Promise.resolve());
    renderPanel(locale, onAdd);

    // `FR-304`/`NFR-206`: no probe on mount, on render, or on any timer.
    expect(vi.mocked(probeLocalProviders)).not.toHaveBeenCalled();
    expect(screen.queryAllByTestId('discovery-hit')).toHaveLength(0);
    expect(vi.mocked(addProvider)).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: t('providers.discovery.action') }));

    await waitFor(() => expect(screen.getAllByTestId('discovery-hit')).toHaveLength(2));
    expect(vi.mocked(probeLocalProviders)).toHaveBeenCalledTimes(1);
    // Discovery is an offer: it persists nothing.
    expect(vi.mocked(addProvider)).not.toHaveBeenCalled();
    expect(onAdd).not.toHaveBeenCalled();
  });

  it('offers one add action per hit and passes the typed model id', async () => {
    const onAdd = vi.fn(() => Promise.resolve());
    renderPanel(locale, onAdd);

    fireEvent.click(screen.getByRole('button', { name: t('providers.discovery.action') }));
    await waitFor(() => expect(screen.getAllByTestId('discovery-hit')).toHaveLength(2));

    expect(screen.getAllByRole('button', { name: t('providers.discovery.add') })).toHaveLength(2);

    const second = hitAt(1);
    fireEvent.change(inputByLabel(second, t('providers.discovery.model')), {
      target: { value: 'fixture-lm-studio-model' },
    });
    fireEvent.click(within(second).getByRole('button', { name: t('providers.discovery.add') }));

    await waitFor(() => expect(onAdd).toHaveBeenCalledTimes(1));
    expect(onAdd).toHaveBeenCalledWith(discovered('lm_studio', 1234), {
      label: t('providers.flavor.lm_studio'),
      model_id: 'fixture-lm-studio-model',
    });
  });

  it('refuses to add a hit with no model id and says which field is missing', async () => {
    const onAdd = vi.fn(() => Promise.resolve());
    renderPanel(locale, onAdd);

    fireEvent.click(screen.getByRole('button', { name: t('providers.discovery.action') }));
    await waitFor(() => expect(screen.getAllByTestId('discovery-hit')).toHaveLength(2));

    fireEvent.click(within(hitAt(0)).getByRole('button', { name: t('providers.discovery.add') }));

    expect(onAdd).not.toHaveBeenCalled();
    expect(screen.getByTestId('error-envelope')).toHaveAttribute('data-error-code', 'E_INPUT_EMPTY');
  });

  it('says so when no local server answers', async () => {
    vi.mocked(probeLocalProviders).mockResolvedValue({ found: [] });
    renderPanel(locale, () => Promise.resolve());

    fireEvent.click(screen.getByRole('button', { name: t('providers.discovery.action') }));

    await waitFor(() => expect(screen.getByTestId('discovery-none')).toBeInTheDocument());
    expect(screen.getByTestId('discovery-none')).toHaveTextContent(t('providers.discovery.none'));
    expect(screen.queryAllByRole('button', { name: t('providers.discovery.add') })).toHaveLength(0);
  });

  it('keeps each port in reading order inside the line', async () => {
    renderPanel(locale, () => Promise.resolve());
    fireEvent.click(screen.getByRole('button', { name: t('providers.discovery.action') }));
    await waitFor(() => expect(screen.getAllByTestId('discovery-hit')).toHaveLength(2));

    const ports = screen.getAllByTestId('number-text');
    expect(ports.map((port) => port.textContent)).toEqual(['11434', '1234']);
    for (const port of ports) {
      expect(port).toHaveAttribute('dir', 'ltr');
    }
  });
});
