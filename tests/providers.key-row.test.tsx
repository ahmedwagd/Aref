import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { KeyRow } from '../src/features/providers/KeyRow';
import { applyDocumentLocale, createTranslator, directionFor, locales, type Locale } from '../src/i18n';
import { keyMetadata, maskedPreview } from './support/providers';

const resolved = (): Promise<void> => Promise.resolve();
const noop = (): void => {};

describe.each(locales)('KeyRow in %s', (locale: Locale) => {
  const t = createTranslator(locale);

  it('shows a notice for a key whose status is invalid', () => {
    render(
      <KeyRow
        t={t}
        metadata={keyMetadata({ status: 'invalid', invalid_since: '2026-10-22T00:00:00Z' })}
        onReplace={resolved}
        onRemove={noop}
      />,
    );

    const notice = screen.getByTestId('key-invalid-notice');
    expect(notice).toBeInTheDocument();
    expect(notice).toHaveTextContent(t('error.provider.key_invalid'));
    expect(screen.getByTestId('key-row')).toHaveAttribute('data-key-status', 'invalid');
  });

  it('shows no notice for a valid key', () => {
    render(<KeyRow t={t} metadata={keyMetadata()} onReplace={resolved} onRemove={noop} />);

    expect(screen.queryByTestId('key-invalid-notice')).toBeNull();
    expect(screen.getByTestId('key-row')).toHaveAttribute('data-key-status', 'valid');
  });

  it('keeps the masked preview unmirrored inside an Arabic page', () => {
    applyDocumentLocale('ar');
    // The page itself is right-to-left; the preview must not inherit that direction.
    expect(document.documentElement.dir).toBe(directionFor('ar'));
    const arabicLabel = keyMetadata({ label: 'مفتاح العمل' });

    render(<KeyRow t={t} metadata={arabicLabel} onReplace={resolved} onRemove={noop} />);

    const preview = screen.getByTestId('masked-preview');
    expect(preview).toHaveAttribute('dir', 'ltr');
    // `<bdi>` isolates the value from the surrounding right-to-left text.
    expect(preview.firstElementChild?.tagName).toBe('BDI');
    // jsdom has no glyph layout, so the assertion is the invariant that produces the visual
    // order: one text node, in logical order, inside an element the browser treats as LTR.
    expect(preview.childNodes).toHaveLength(1);
    expect(preview.textContent).toBe(maskedPreview);
    const characters = [...(preview.textContent ?? '')];
    expect(characters.slice(0, 8)).toEqual(['•', '•', '•', '•', '•', '•', '•', '•']);
    expect(characters.slice(8).join('')).toBe('a1b2');
  });

  it('keeps the same order in the other direction', () => {
    applyDocumentLocale(locale);
    render(<KeyRow t={t} metadata={keyMetadata()} onReplace={resolved} onRemove={noop} />);

    expect(document.documentElement.dir).toBe(directionFor(locale));
    expect(screen.getByTestId('masked-preview')).toHaveTextContent(maskedPreview);
  });
});
