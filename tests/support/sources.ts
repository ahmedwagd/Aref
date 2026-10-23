import { readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

/**
 * Vitest runs with the project root as its working directory, and `import.meta.url` is
 * an `http://` URL under the jsdom environment, so the root is taken from `cwd`.
 */
export const repoRoot = resolve(process.cwd());

export function listFiles(directory: string, extensions: readonly string[]): string[] {
  const found: string[] = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const fullPath = join(directory, entry.name);
    if (entry.isDirectory()) {
      found.push(...listFiles(fullPath, extensions));
    } else if (extensions.some((extension) => entry.name.endsWith(extension))) {
      found.push(fullPath);
    }
  }
  return found;
}

export function readText(path: string): string {
  return readFileSync(path, 'utf8');
}

export function relativeToRepo(path: string): string {
  return path.slice(repoRoot.length + 1).replaceAll('\\', '/');
}

export type Match = { file: string; line: number; text: string };

/** Returns every line whose content matches `pattern`, so a failure names the offender. */
export function findMatches(paths: readonly string[], pattern: RegExp): Match[] {
  const matches: Match[] = [];
  for (const path of paths) {
    const lines = readText(path).split(/\r?\n/);
    lines.forEach((text, index) => {
      if (pattern.test(text)) {
        matches.push({ file: relativeToRepo(path), line: index + 1, text: text.trim() });
      }
    });
  }
  return matches;
}
