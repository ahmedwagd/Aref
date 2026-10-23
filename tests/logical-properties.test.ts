import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { findMatches, listFiles, repoRoot } from './support/sources';

const srcDir = join(repoRoot, 'src');

/**
 * Physical direction properties are banned: direction comes from `dir` on `<html>`
 * (DESIGN.md §7.6, DESIGN.md §10 `D-02`, AGENTS.md §10). Both the CSS spellings of
 * task 1.8 and the Tailwind utilities DESIGN.md names are scanned.
 */
const rules: ReadonlyArray<{ name: string; pattern: RegExp }> = [
  { name: 'css margin-inline', pattern: /(?:margin|padding)-(?:left|right)\b/ },
  { name: 'css border-inline', pattern: /border-(?:left|right)\b/ },
  { name: 'css text-align', pattern: /text-align:\s*(?:left|right)\b/ },
  { name: 'css left/right declaration', pattern: /(?:^|[\s;{])(?:left|right)\s*:/ },
  { name: 'tailwind ml/mr/pl/pr', pattern: /(?:^|[\s"`':(])(?:ml|mr|pl|pr)-(?:\d|\[|auto\b|px\b|full\b|0\b)/ },
  { name: 'tailwind text-left/right', pattern: /\btext-(?:left|right)\b/ },
  { name: 'tailwind border-l/border-r', pattern: /\bborder-[lr]\b/ },
  { name: 'tailwind left-/right-', pattern: /(?:^|[\s"`':(])(?:left|right)-(?:\d|\[|auto|0\b|full\b|px\b)/ },
];

describe('logical CSS properties only', () => {
  const files = [...listFiles(join(srcDir, 'styles'), ['.css']), ...listFiles(srcDir, ['.tsx'])];

  it('has files to scan', () => {
    expect(files.length).toBeGreaterThan(3);
  });

  for (const rule of rules) {
    it(`has no physical ${rule.name} in src`, () => {
      expect(findMatches(files, rule.pattern)).toEqual([]);
    });
  }

  it('detects a planted violation for every rule', () => {
    // Without this control a broken pattern would pass the scan silently.
    const samples: ReadonlyArray<[string, string]> = [
      ['css margin-inline', '.x { margin-left: 4px; }'],
      ['css border-inline', '.x { border-right: 1px; }'],
      ['css text-align', '.x { text-align: left; }'],
      ['css left/right declaration', '.x { left: 0; }'],
      ['tailwind ml/mr/pl/pr', 'className="pl-4 text-sm"'],
      ['tailwind text-left/right', 'className="text-right"'],
      ['tailwind border-l/border-r', 'className="border-l"'],
      ['tailwind left-/right-', 'className="left-0 right-4"'],
    ];
    for (const [name, sample] of samples) {
      const rule = rules.find((candidate) => candidate.name === name);
      expect(rule, name).toBeDefined();
      expect(rule?.pattern.test(sample), name).toBe(true);
    }
  });
});
