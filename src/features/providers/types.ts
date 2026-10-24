/**
 * The feature's callback vocabulary.
 *
 * These live in a `.ts` module on purpose: `tests/hardcoded-strings.test.ts` scans `.tsx`
 * sources for letter-bearing JSX text nodes with a deliberately blunt pattern, and a
 * `props: () => Promise<Thing>` annotation reads like one. Keeping the annotations out of
 * the component files leaves that guard strict about actual copy.
 */
import type { DiscoveredProvider, KeyMetadata } from './hooks';

/**
 * What a key save hands to the caller: the submitted secret and, in add mode only, the
 * label. It is a function argument and never a state value, so it dies with the call.
 */
export type KeySubmission = { secret: string; label: string | null };

export type SubmitKeyHandler = (submission: KeySubmission) => Promise<void>;

export type AddKeyHandler = (providerId: string, submission: KeySubmission) => Promise<void>;

export type ReplaceKeyHandler = (metadata: KeyMetadata, secret: string) => Promise<void>;

export type ReplaceSecretHandler = (secret: string) => Promise<void>;

export type DiscoveryDraft = { label: string; model_id: string };

export type AddDiscoveredHandler = (hit: DiscoveredProvider, input: DiscoveryDraft) => Promise<void>;

export type ProbeHandler = () => Promise<readonly DiscoveredProvider[]>;
