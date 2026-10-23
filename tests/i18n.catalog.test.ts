import { describe, expect, it } from 'vitest';

import ar from '../src/i18n/ar.json';
import en from '../src/i18n/en.json';
import errorKeys from '../src/i18n/error-keys.json';

const arCatalog: Record<string, string> = ar;
const enCatalog: Record<string, string> = en;

function keyDiff(left: Record<string, string>, right: Record<string, string>): string[] {
  const rightKeys = new Set(Object.keys(right));
  return Object.keys(left).filter((key) => !rightKeys.has(key));
}

describe('i18n catalogs', () => {
  it('has the same key set in ar and en', () => {
    expect(keyDiff(arCatalog, enCatalog)).toEqual([]);
    expect(keyDiff(enCatalog, arCatalog)).toEqual([]);
  });

  it('fails parity when a key is deleted from one catalog', () => {
    const withoutSettings: Record<string, string> = { ...arCatalog };
    delete withoutSettings['nav.settings'];
    expect(keyDiff(withoutSettings, enCatalog)).toEqual([]);
    expect(keyDiff(enCatalog, withoutSettings)).toEqual(['nav.settings']);
  });

  it('has a non-empty string for every key in both catalogs', () => {
    for (const [key, value] of Object.entries(enCatalog)) {
      expect(value.trim(), `en:${key}`).not.toBe('');
      expect(arCatalog[key]?.trim(), `ar:${key}`).not.toBe('');
    }
  });

  it('carries every core message_key in both catalogs', () => {
    expect(errorKeys.length).toBeGreaterThan(0);
    for (const messageKey of errorKeys) {
      expect(arCatalog[messageKey], `ar:${messageKey}`).toBeTruthy();
      expect(enCatalog[messageKey], `en:${messageKey}`).toBeTruthy();
    }
  });

  it('lists only catalog keys in error-keys.json', () => {
    const unknown = errorKeys.filter((messageKey) => !(messageKey in enCatalog));
    expect(unknown).toEqual([]);
  });
});

describe('error.input.empty (D-1)', () => {
  it('reads as a complete sentence with no interpolation', () => {
    // `params.field` only decides which input carries the message (design.md §4.6),
    // so the copy must not carry a placeholder of any kind.
    const placeholder = /\{\{?\s*[A-Za-z_]+\s*\}?\}|\$\{/;
    expect(arCatalog['error.input.empty']).not.toMatch(placeholder);
    expect(enCatalog['error.input.empty']).not.toMatch(placeholder);
    expect(arCatalog['error.input.empty']?.endsWith('.')).toBe(true);
    expect(enCatalog['error.input.empty']?.endsWith('.')).toBe(true);
  });

  it('does not introduce any code outside the SRS §7 catalog', () => {
    // The retired input code (`D-1`) is absent because `error-keys.json` is proved
    // equal to the core's catalog by `src-tauri/tests/i18n_catalog.rs`; this asserts the
    // frontend side has no second, drifting list of codes.
    const dump = JSON.stringify([arCatalog, enCatalog, errorKeys]);
    expect(dump).not.toMatch(/E_[A-Z_]+/);
  });
});
