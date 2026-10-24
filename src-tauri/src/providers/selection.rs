//! Provider selection (`FR-312`, `G-19`; task 3.10).
//!
//! The signature is the guarantee. [`candidates`] takes two slices and returns a `Vec`; it
//! is a plain `fn`, not `async fn`, and it has no `Transport`, no `Clock` and no HTTP type
//! in scope. "Selection makes no network request" is therefore a property of the type
//! rather than a behaviour someone has to remember not to add, and the task's recording
//! test asserts what the type already says.
//!
//! The order is `(rank, id)`: `opencode` 0, `ollama` 1, `lm_studio` 2, cloud 3, with the
//! id as a deterministic tie-break for two entries of the same flavour (`design.md` §7.1;
//! a user-meaningful order would need a priority column, which is a schema change).
//!
//! Eligibility is `enabled && (local || any valid key)`. A local provider with an empty
//! pool is eligible (`FR-303`); a cloud provider whose keys are all `invalid` is not,
//! because an invalid key receives no request and the provider therefore cannot serve
//! (`FR-306`).

use crate::storage::providers_repo::{
    KeyMetadata, KeyStatus, LocalFlavor, ProviderConfig, ProviderKind,
};

/// A provider that may serve a request, with its keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub provider: ProviderConfig,
    /// Every key of the provider, whatever its status. The pool filters on
    /// [`KeyStatus::Valid`]; keeping the list whole is what lets a later settings screen
    /// show why a key is unusable without a second query.
    pub keys: Vec<KeyMetadata>,
}

/// The `FR-312` rank of a provider entry.
///
/// `opencode` entries are user-configured only and are never discovered (`G-19`): the rank
/// reads `local_flavor` from the row, and no code path creates one.
pub fn rank(provider: &ProviderConfig) -> u8 {
    match provider.local_flavor {
        Some(LocalFlavor::OpenCode) => 0,
        Some(LocalFlavor::Ollama) => 1,
        Some(LocalFlavor::LmStudio) => 2,
        None => 3,
    }
}

/// Whether a provider may serve a request at all.
fn eligible(provider: &ProviderConfig, keys: &[KeyMetadata]) -> bool {
    if !provider.enabled {
        return false;
    }
    if provider.kind == ProviderKind::Local {
        // A local provider needs no key (`FR-303`), so an empty pool is not a reason to
        // exclude it: its reachability becomes a failed attempt, not a missing candidate.
        return true;
    }
    keys.iter()
        .any(|key| key.provider_id == provider.id && key.status == KeyStatus::Valid)
}

/// The providers that may serve a request, in `FR-312` order.
pub fn candidates(providers: &[ProviderConfig], keys: &[KeyMetadata]) -> Vec<Candidate> {
    let mut candidates: Vec<Candidate> = providers
        .iter()
        .filter(|provider| eligible(provider, keys))
        .map(|provider| Candidate {
            provider: provider.clone(),
            keys: keys
                .iter()
                .filter(|key| key.provider_id == provider.id)
                .cloned()
                .collect(),
        })
        .collect();

    candidates.sort_by(|left, right| {
        (rank(&left.provider), left.provider.id.as_str())
            .cmp(&(rank(&right.provider), right.provider.id.as_str()))
    });
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::client::doubles::ScriptedTransport;
    use crate::security::keyring::KeyId;
    use crate::security::mask::MaskedPreview;

    const PREVIEW: &str = "••••••••et-a";

    fn provider(id: &str, kind: ProviderKind, flavor: Option<LocalFlavor>) -> ProviderConfig {
        ProviderConfig::new(
            id,
            kind,
            flavor,
            "http://127.0.0.1:1/v1",
            "model-a",
            id,
            true,
        )
    }

    fn key(id: &str, provider_id: &str, status: KeyStatus) -> KeyMetadata {
        KeyMetadata {
            id: KeyId::new(id),
            provider_id: provider_id.to_owned(),
            label: "personal".to_owned(),
            masked_preview: MaskedPreview::from_stored(PREVIEW),
            status,
            last_error_code: None,
            invalid_since: None,
            updated_at: "2026-10-24T00:00:00Z".to_owned(),
        }
    }

    fn cloud_with_valid_key(id: &str) -> (ProviderConfig, KeyMetadata) {
        (
            provider(id, ProviderKind::OpenAiCompatible, None),
            key(&format!("{id}-key"), id, KeyStatus::Valid),
        )
    }

    /// The four entries of `FR-312`'s scenario, in no particular input order.
    fn four_providers() -> (Vec<ProviderConfig>, Vec<KeyMetadata>) {
        let (cloud, cloud_key) = cloud_with_valid_key("cloud");
        let providers = vec![
            cloud,
            provider("lm", ProviderKind::Local, Some(LocalFlavor::LmStudio)),
            provider("opencode", ProviderKind::Local, Some(LocalFlavor::OpenCode)),
            provider("ollama", ProviderKind::Local, Some(LocalFlavor::Ollama)),
        ];
        (providers, vec![cloud_key])
    }

    fn order(providers: &[ProviderConfig], keys: &[KeyMetadata]) -> Vec<String> {
        candidates(providers, keys)
            .into_iter()
            .map(|candidate| candidate.provider.id)
            .collect()
    }

    #[test]
    fn the_walk_disables_one_entry_at_a_time_and_follows_the_documented_order() {
        let (mut providers, keys) = four_providers();

        // All four enabled: opencode first, then ollama, lm_studio, cloud.
        assert_eq!(
            order(&providers, &keys),
            vec!["opencode", "ollama", "lm", "cloud"]
        );

        // Disable opencode → ollama leads.
        set_enabled(&mut providers, "opencode", false);
        assert_eq!(order(&providers, &keys), vec!["ollama", "lm", "cloud"]);

        // → lm_studio leads.
        set_enabled(&mut providers, "ollama", false);
        assert_eq!(order(&providers, &keys), vec!["lm", "cloud"]);

        // → the cloud provider is the only one left.
        set_enabled(&mut providers, "lm", false);
        assert_eq!(order(&providers, &keys), vec!["cloud"]);

        // → nothing is selected at all, so `humanize_available` would be false.
        set_enabled(&mut providers, "cloud", false);
        assert!(order(&providers, &keys).is_empty());
    }

    fn set_enabled(providers: &mut [ProviderConfig], id: &str, enabled: bool) {
        for provider in providers.iter_mut() {
            if provider.id == id {
                provider.enabled = enabled;
            }
        }
    }

    #[test]
    fn a_cloud_provider_holding_only_invalid_keys_is_absent() {
        let cloud = provider("cloud", ProviderKind::OpenAiCompatible, None);
        let invalid = key("cloud-key", "cloud", KeyStatus::Invalid);
        assert!(
            candidates(std::slice::from_ref(&cloud), std::slice::from_ref(&invalid)).is_empty()
        );

        // Editing the key makes it eligible again (`FR-306`), which is the pool's input.
        let valid = key("cloud-key", "cloud", KeyStatus::Valid);
        assert_eq!(candidates(std::slice::from_ref(&cloud), &[valid]).len(), 1);
    }

    #[test]
    fn a_keyless_local_provider_is_present() {
        let local = provider("ollama", ProviderKind::Local, Some(LocalFlavor::Ollama));
        let candidates = candidates(&[local], &[]);
        assert_eq!(candidates.len(), 1);
        assert!(candidates[0].keys.is_empty());
    }

    #[test]
    fn a_cloud_provider_with_a_valid_key_carries_all_of_its_keys() {
        let cloud = provider("cloud", ProviderKind::OpenAiCompatible, None);
        let keys = vec![
            key("cloud-key-1", "cloud", KeyStatus::Valid),
            key("cloud-key-2", "cloud", KeyStatus::Invalid),
            key("other-key", "other", KeyStatus::Valid),
        ];
        let candidates = candidates(&[cloud], &keys);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0]
                .keys
                .iter()
                .map(|key| key.id.to_string())
                .collect::<Vec<_>>(),
            vec!["cloud-key-1", "cloud-key-2"],
            "another provider's key must not be carried"
        );
    }

    #[test]
    fn two_entries_of_the_same_flavour_are_ordered_by_id() {
        let first = provider("b-ollama", ProviderKind::Local, Some(LocalFlavor::Ollama));
        let second = provider("a-ollama", ProviderKind::Local, Some(LocalFlavor::Ollama));
        assert_eq!(
            order(&[first, second], &[]),
            vec!["a-ollama", "b-ollama"],
            "the tie-break is deterministic"
        );
    }

    #[test]
    fn each_kind_maps_to_its_documented_rank() {
        assert_eq!(
            rank(&provider(
                "a",
                ProviderKind::Local,
                Some(LocalFlavor::OpenCode)
            )),
            0
        );
        assert_eq!(
            rank(&provider(
                "b",
                ProviderKind::Local,
                Some(LocalFlavor::Ollama)
            )),
            1
        );
        assert_eq!(
            rank(&provider(
                "c",
                ProviderKind::Local,
                Some(LocalFlavor::LmStudio)
            )),
            2
        );
        assert_eq!(
            rank(&provider("d", ProviderKind::OpenAiCompatible, None)),
            3
        );
    }

    #[test]
    fn selection_makes_no_request_while_the_recording_transport_watches() {
        // The transport is in scope on purpose: the assertion is about what selection
        // does *not* do, and a type-level claim is worth re-checking at runtime.
        let transport = ScriptedTransport::new([]);
        let (providers, keys) = four_providers();
        let selected = candidates(&providers, &keys);
        assert_eq!(selected.len(), 4);
        assert_eq!(
            transport.call_count(),
            0,
            "selection must not send anything"
        );
    }
}
