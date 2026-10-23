import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { findMatches, listFiles, readText, relativeToRepo, repoRoot } from './support/sources';

const srcDir = join(repoRoot, 'src');
const tsxFiles = listFiles(srcDir, ['.tsx']);

const letters = '\\p{L}';

/** A JSX text node carrying letters would be copy that bypasses the catalogs. */
const jsxText = new RegExp(`>([^<>{}]*${letters}[^<>{}]*)<`, 'gu');

/** Literal values for the attributes that reach the accessibility tree or the UI. */
const literalAttribute = /(?:aria-label|placeholder|alt)\s*=\s*"[^"]*\p{L}[^"]*"/u;

/** `title` is scanned separately so `title={`...`}` stays fine while `title="..."` does not. */
const literalTitle = /\btitle\s*=\s*"[^"]*\p{L}[^"]*"/u;

describe('no hard-coded user-facing strings', () => {
  it('has tsx files to scan', () => {
    expect(tsxFiles.length).toBeGreaterThan(3);
  });

  it('has no letter-bearing JSX text node in src', () => {
    expect(findMatches(tsxFiles, jsxText)).toEqual([]);
  });

  it('has no literal accessibility or placeholder attribute in src', () => {
    expect(findMatches(tsxFiles, literalAttribute)).toEqual([]);
    expect(findMatches(tsxFiles, literalTitle)).toEqual([]);
  });

  it('detects a planted violation', () => {
    // Without this control a broken pattern would pass the scan silently.
    expect(new RegExp(jsxText.source, 'u').test('<p>Not translated</p>')).toBe(true);
    expect(literalAttribute.test('<div aria-label="Providers" />')).toBe(true);
    expect(literalTitle.test('<div title="Providers" />')).toBe(true);
  });

  it('routes every visible string through the catalogs', () => {
    const shell = readText(join(srcDir, 'components', 'AppShell.tsx'));
    expect(shell).toContain("t('app.name')");
    expect(shell).toContain("t('shell.workspace_placeholder')");
    const navigation = readText(join(srcDir, 'components', 'Navigation.tsx'));
    expect(navigation).toContain('t(labelKeys[item])');
    expect(relativeToRepo(join(srcDir, 'components', 'Navigation.tsx'))).toBe(
      'src/components/Navigation.tsx',
    );
  });
});
