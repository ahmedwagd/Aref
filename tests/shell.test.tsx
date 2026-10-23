import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { App } from '../src/App';
import { directionFor, locales, type Locale } from '../src/i18n';

/**
 * jsdom has no layout engine, so the sidebar's side cannot be measured in pixels.
 * What is asserted instead is the invariant that produces `inline-start` in both
 * directions: the navigation is the shell's first in-flow child of a row container and
 * its hairline is `border-inline-end` (DESIGN.md §7.2, `D-02`). A physical `left`
 * would appear as a `left-*`/`border-l` utility or a `left:` declaration, which
 * `tests/logical-properties.test.ts` rejects.
 */
describe.each(locales)('shell in %s', (locale: Locale) => {
  it('sets document direction from the locale', () => {
    render(<App locale={locale} />);
    expect(document.documentElement.dir).toBe(directionFor(locale));
    expect(document.documentElement.lang).toBe(locale);
  });

  it('puts the navigation on the inline-start side', () => {
    render(<App locale={locale} />);
    const shell = screen.getByTestId('app-shell');
    const navigation = screen.getByTestId('app-navigation');

    expect(shell.contains(navigation)).toBe(true);
    // The skip link is `sr-only` until focused, so it does not occupy flow space.
    expect(navigation.previousElementSibling).toHaveAttribute('href', '#workspace');
    expect(navigation.previousElementSibling).toHaveClass('sr-only');
    expect(navigation).toHaveClass('border-e');
    expect(navigation).toHaveClass('bg-sidebar');
    expect(navigation.className).not.toMatch(/\b(?:left|right)-/);
  });

  it('renders the same structure with no locale-specific override', () => {
    render(<App locale={locale} />);
    expect(screen.getByRole('navigation')).toHaveAccessibleName();
    expect(screen.getAllByRole('button')).toHaveLength(4);
  });
});
