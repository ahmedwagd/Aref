/**
 * The IPC boundary. Import from here — never from `client` or `hooks` directly, and
 * never from `@tauri-apps/api` (design.md §3.2 rule 5).
 */
export * from './client';
export * from './hooks';
export type * from './types';
