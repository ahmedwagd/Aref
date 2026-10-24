/**
 * The two provider discriminants, mapped to catalog keys. One table each rather than a
 * ternary at every call site, so a new discriminant is a type error instead of a blank pill.
 */
import type { MessageKey } from '../../i18n';
import type { LocalFlavor, ProviderKind, ProviderView } from './hooks';

export const kindKeys: Record<ProviderKind, MessageKey> = {
  openai_compatible: 'providers.kind.openai_compatible',
  local: 'providers.kind.local',
};

export const flavorKeys: Record<LocalFlavor, MessageKey> = {
  opencode: 'providers.flavor.opencode',
  ollama: 'providers.flavor.ollama',
  lm_studio: 'providers.flavor.lm_studio',
};

/** A local entry is named by its flavour; a cloud entry by its kind (`G-19` rank order). */
export function providerKindKey(provider: ProviderView): MessageKey {
  if (provider.local_flavor !== null) {
    return flavorKeys[provider.local_flavor];
  }
  return kindKeys[provider.kind];
}
