import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { MaskedPreview } from '../src/components/MaskedPreview';
import { NumberText } from '../src/components/NumberText';
import { SuppliedText } from '../src/components/SuppliedText';
import { applyDocumentLocale, locales, type Locale } from '../src/i18n';

/** Fixtures are non-key-shaped on purpose (design.md §10, `NFR-207`). */
const masked = '••••••••a1b2';
const arabicWithDigits = 'النص 2026 م';

describe.each(locales)('direction rules in %s', (locale: Locale) => {
  it('renders masked_preview with dir="ltr" and keeps its characters in order', () => {
    applyDocumentLocale(locale);
    render(<MaskedPreview value={masked} />);

    const preview = screen.getByTestId('masked-preview');
    expect(preview).toHaveAttribute('dir', 'ltr');
    // `<bdi>` isolates the value from the surrounding RTL or LTR text.
    expect(preview.firstElementChild?.tagName).toBe('BDI');
    expect(preview.textContent).toBe(masked);
    expect(preview.textContent?.endsWith('a1b2')).toBe(true);
  });

  it('renders supplied content with dir="auto"', () => {
    applyDocumentLocale(locale);
    render(<SuppliedText value={arabicWithDigits} />);

    const supplied = screen.getByText(arabicWithDigits);
    expect(supplied).toHaveAttribute('dir', 'auto');
  });

  it('keeps digits in reading order inside mixed text', () => {
    applyDocumentLocale(locale);
    render(
      <p>
        <span>{arabicWithDigits.slice(0, 5)}</span>
        <NumberText value="2026" />
      </p>,
    );

    const number = screen.getByTestId('number-text');
    expect(number).toHaveAttribute('dir', 'ltr');
    expect(number.textContent).toBe('2026');
    // The digits are one text node, so no bidi reordering can split them.
    expect(number.childNodes).toHaveLength(1);
  });
});
