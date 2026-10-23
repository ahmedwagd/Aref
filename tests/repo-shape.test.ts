import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { repoRoot } from './support/sources';

const manifest = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8')) as {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
};

const declared = [
  ...Object.keys(manifest.dependencies ?? {}),
  ...Object.keys(manifest.devDependencies ?? {}),
];

/**
 * The locked stack (AGENTS.md §4) plus its build, lint and test tooling. A package
 * outside this set is a dependency request, not a scaffold detail.
 */
const allowed = new Set([
  '@eslint/js',
  '@tailwindcss/vite',
  '@tauri-apps/api',
  '@tauri-apps/cli',
  '@testing-library/dom',
  '@testing-library/jest-dom',
  '@testing-library/react',
  '@types/node',
  '@types/react',
  '@types/react-dom',
  '@vitejs/plugin-react',
  'eslint',
  'eslint-plugin-react-hooks',
  'globals',
  'jsdom',
  'react',
  'react-dom',
  'tailwindcss',
  'typescript',
  'typescript-eslint',
  'vite',
  'vitest',
]);

const forbidden = /(?:telemetry|analytics|sentry|segment|posthog|mixpanel|datadog|newrelic|bugsnag|crash|openai|anthropic|groq|mistral|cohere|ollama|lmstudio|lm-studio)/i;

describe('repository shape', () => {
  it('declares only the locked stack', () => {
    expect(declared.filter((name) => !allowed.has(name))).toEqual([]);
  });

  it('declares no provider SDK, telemetry, analytics or crash reporting package', () => {
    expect(declared.filter((name) => forbidden.test(name))).toEqual([]);
  });

  it('has no tailwind.config.js (Tailwind v4 is configured in CSS)', () => {
    expect(existsSync(join(repoRoot, 'tailwind.config.js'))).toBe(false);
    expect(existsSync(join(repoRoot, 'tailwind.config.ts'))).toBe(false);
  });

  it('has package-lock.json as the only lockfile', () => {
    const lockfiles = ['package-lock.json', 'pnpm-lock.yaml', 'yarn.lock', 'bun.lockb'].filter(
      (name) => existsSync(join(repoRoot, name)),
    );
    expect(lockfiles).toEqual(['package-lock.json']);
  });

  it('names the npm scripts AGENTS.md §6 requires', () => {
    const scripts = JSON.parse(readFileSync(join(repoRoot, 'package.json'), 'utf8')) as {
      scripts?: Record<string, string>;
    };
    for (const script of ['dev', 'typecheck', 'lint', 'test', 'build']) {
      expect(scripts.scripts?.[script], script).toBeTruthy();
    }
  });
});
