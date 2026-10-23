import { join } from 'node:path';

import { describe, expect, it } from 'vitest';

import { findMatches, listFiles, readText, repoRoot } from './support/sources';

const srcDir = join(repoRoot, 'src');
const ipcDir = join(srcDir, 'ipc');

/** The ratified DTO field lists from `design.md` §4.2, field for field. */
const contract: ReadonlyArray<{ typeName: string; fields: readonly string[] }> = [
  { typeName: 'AppStatus', fields: ['detector_mode', 'humanize_available', 'provider_summary', 'app_version'] },
  {
    typeName: 'ProviderSummary',
    fields: ['configured', 'enabled', 'usable_keys', 'selected_provider_id'],
  },
  {
    typeName: 'ProviderView',
    fields: ['id', 'kind', 'local_flavor', 'base_url', 'model_id', 'label', 'enabled', 'keys'],
  },
  {
    typeName: 'KeyMetadata',
    fields: [
      'id',
      'provider_id',
      'label',
      'masked_preview',
      'status',
      'last_error_code',
      'invalid_since',
      'updated_at',
    ],
  },
  {
    typeName: 'DiscoveredProvider',
    fields: ['kind', 'local_flavor', 'base_url', 'port'],
  },
  {
    typeName: 'AddProviderRequest',
    fields: ['kind', 'local_flavor', 'base_url', 'model_id', 'label', 'enabled'],
  },
  {
    typeName: 'UpdateProviderRequest',
    fields: ['id', 'label', 'enabled', 'base_url', 'model_id', 'local_flavor'],
  },
  { typeName: 'AddKeyRequest', fields: ['provider_id', 'label', 'secret'] },
  { typeName: 'EditKeyRequest', fields: ['key_id', 'secret'] },
  { typeName: 'RemoveProviderRequest', fields: ['id'] },
  { typeName: 'RemoveKeyRequest', fields: ['key_id'] },
];

function fieldsOf(source: string, typeName: string): string[] {
  const start = source.indexOf(`export type ${typeName} = {`);
  if (start === -1) return [];
  const end = source.indexOf('};', start);
  if (end === -1) return [];
  const block = source.slice(start, end);
  return [...block.matchAll(/^\s{2}([a-z][a-z0-9_]*)\??:/gm)].map((match) => match[1] ?? '');
}

describe('IPC boundary', () => {
  const nonIpcFiles = listFiles(srcDir, ['.ts', '.tsx']).filter(
    (file) => !file.startsWith(ipcDir),
  );

  it('has files to scan', () => {
    expect(nonIpcFiles.length).toBeGreaterThan(3);
  });

  it('calls invoke only from src/ipc', () => {
    expect(findMatches(nonIpcFiles, /\binvoke\s*\(/)).toEqual([]);
  });

  it('calls listen only from src/ipc', () => {
    expect(findMatches(nonIpcFiles, /\blisten\s*[<(]/)).toEqual([]);
  });

  it('imports @tauri-apps/api only from src/ipc', () => {
    expect(findMatches(nonIpcFiles, /@tauri-apps\/api/)).toEqual([]);
  });

  it('mirrors design.md §4.2 field for field', () => {
    const types = readText(join(ipcDir, 'types.ts'));
    for (const entry of contract) {
      expect(fieldsOf(types, entry.typeName).sort(), entry.typeName).toEqual(
        [...entry.fields].sort(),
      );
    }
  });
});
