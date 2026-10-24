import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { HumanizeDisabledNotice } from '../src/features/providers/HumanizeDisabledNotice';
import { createTranslator, locales, type Locale } from '../src/i18n';

describe.each(locales)('HumanizeDisabledNotice in %s', (locale: Locale) => {
  const t = createTranslator(locale);

  it('renders the catalog sentence and the Settings action when humanize is unavailable', () => {
    const onOpenSettings = vi.fn((): void => {});

    render(<HumanizeDisabledNotice t={t} available={false} onOpenSettings={onOpenSettings} />);

    const notice = screen.getByTestId('humanize-disabled-notice');
    // The copy is the core's own `error.provider.none` string, not a second explanation that
    // could drift from the command's (design.md §4.4 delta 5).
    expect(notice).toHaveTextContent(t('error.provider.none'));
    expect(notice).toHaveTextContent(t('providers.humanize_disabled.title'));

    const action = screen.getByRole('button', { name: t('providers.humanize_disabled.action') });
    fireEvent.click(action);
    expect(onOpenSettings).toHaveBeenCalledTimes(1);
  });

  it('renders nothing when a provider is usable', () => {
    const { container } = render(
      <HumanizeDisabledNotice t={t} available onOpenSettings={() => {}} />,
    );

    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByTestId('humanize-disabled-notice')).toBeNull();
  });
});
