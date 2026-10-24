//! Local-provider discovery (`FR-304`, `NFR-206`; tasks 3.11 and 3.12).
//!
//! Five properties, and each one is structural rather than a convention:
//!
//! 1. **Only a user action.** [`probe`] has exactly one caller: the `probe_local_providers`
//!    command. No `setup` hook, no timer, no retry loop, and no call from `get_app_status`
//!    or `selection` (task 3.12 asserts the single call site by name).
//! 2. **Loopback literal, not `localhost`.** The destination list is the `const` array
//!    [`PROBE_TARGETS`] re-exported from `providers::config`: there is no parameter through
//!    which another host could enter, and `127.0.0.1` needs no name resolution.
//! 3. **No proxy and no redirects.** Those are properties of the transport's probe client
//!    (`client::ReqwestTransport`), which is built with `no_proxy` and
//!    `redirect::Policy::none()`; this module only ever asks the transport to probe.
//! 4. **Flavour comes from the port, not the body.** The request is `GET {base_url}/models`
//!    and success is any 2xx; no model list is parsed, returned, or used (`G-09`).
//! 5. **Deterministic output.** The two probes run concurrently and the result is emitted
//!    in `PROBE_TARGETS` order, so the UI list is stable whichever port answers first.
//!
//! [`probe`] returns what it found and nothing else: it does not create, enable, or select
//! a provider. Adding a discovered entry stays a separate user action through
//! `add_provider`, which keeps `FR-304`'s "user-initiated" true for the effect as well as
//! for the probe. **`probe` retries nothing**: one attempt per target, because retrying a
//! refused connection is exactly the background probing `FR-304` forbids.

use crate::providers::client::Transport;
use crate::storage::providers_repo::LocalFlavor;

/// The only two destinations discovery may attempt: Ollama, then LM Studio.
///
/// Re-exported from `providers::config` rather than declared again, so there is one set of
/// loopback literals in the crate (task 3.1 pins them there, and
/// `tests/providers_config.rs` fails if a second copy appears).
pub use crate::providers::config::PROBE_TARGETS;

/// A local provider a probe found answering.
///
/// A service-layer type, not an IPC DTO: `commands::dto::DiscoveredProvider` maps it onto
/// the ratified wire shape, and this one keeps the flavour typed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiscoveredProvider {
    pub flavor: LocalFlavor,
    pub base_url: &'static str,
    pub port: u16,
}

/// Probes both loopback targets concurrently and returns the hits in [`PROBE_TARGETS`]
/// order.
///
/// Concurrency without a futures crate: `tokio::join!` polls both probes on this task, so
/// the slow port cannot delay the fast one by more than the timeout. The order of the
/// *results* is the order of the array, never the order the answers arrived in.
pub async fn probe(transport: &dyn Transport) -> Vec<DiscoveredProvider> {
    let first = transport.probe(&PROBE_TARGETS[0]);
    let second = transport.probe(&PROBE_TARGETS[1]);
    let (first, second) = tokio::join!(first, second);

    let mut found = Vec::new();
    for (target, outcome) in [(&PROBE_TARGETS[0], first), (&PROBE_TARGETS[1], second)] {
        // Only a 2xx counts, and only as "found": a refusal, a timeout, or a 302 is an
        // absence, never an error the user has to act on.
        if matches!(outcome, Ok(true)) {
            found.push(DiscoveredProvider {
                flavor: target.flavor,
                base_url: target.base_url,
                port: target.port,
            });
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::client::doubles::ScriptedTransport;

    fn ports(found: &[DiscoveredProvider]) -> Vec<u16> {
        found.iter().map(|entry| entry.port).collect()
    }

    #[tokio::test]
    async fn both_targets_are_probed_and_a_hit_carries_its_port_and_flavour() {
        let transport = ScriptedTransport::new([]).with_probe_script([Ok(true), Ok(true)]);

        let found = probe(&transport).await;

        assert_eq!(ports(&found), vec![11434, 1234]);
        assert_eq!(found[0].flavor, LocalFlavor::Ollama);
        assert_eq!(found[1].flavor, LocalFlavor::LmStudio);
        assert_eq!(found[0].base_url, "http://127.0.0.1:11434/v1");
        assert_eq!(found[1].base_url, "http://127.0.0.1:1234/v1");
    }

    #[tokio::test]
    async fn exactly_two_destinations_are_attempted_both_loopback() {
        let transport = ScriptedTransport::new([]).with_probe_script([Ok(false), Ok(false)]);

        let found = probe(&transport).await;

        assert!(found.is_empty());
        let probes = transport.probes();
        assert_eq!(probes.len(), 2, "one attempt per target, no retry");
        let destinations: Vec<(String, u16)> = probes
            .iter()
            .map(|target| {
                let host = target
                    .base_url
                    .split("://")
                    .nth(1)
                    .and_then(|rest| rest.split('/').next())
                    .and_then(|authority| authority.split(':').next())
                    .unwrap_or_default()
                    .to_owned();
                (host, target.port)
            })
            .collect();
        assert_eq!(
            destinations,
            vec![
                ("127.0.0.1".to_owned(), 11434),
                ("127.0.0.1".to_owned(), 1234)
            ]
        );
    }

    #[tokio::test]
    async fn out_of_order_answers_still_yield_target_order() {
        // The first target yields before answering, so the second finishes first.
        let transport = ScriptedTransport::new([])
            .with_probe_script([Ok(true), Ok(true)])
            .with_slow_probe(11434);

        let found = probe(&transport).await;

        assert_eq!(
            transport.probe_completions(),
            vec![1234, 11434],
            "the second target answered first"
        );
        assert_eq!(
            ports(&found),
            vec![11434, 1234],
            "the output follows PROBE_TARGETS, not arrival order"
        );
    }

    #[tokio::test]
    async fn a_refused_connection_is_one_attempt_and_an_absence() {
        let transport = ScriptedTransport::new([]).with_probe_script([
            Err(crate::providers::client::TransportError::Dropped),
            Ok(true),
        ]);

        let found = probe(&transport).await;

        assert_eq!(
            ports(&found),
            vec![1234],
            "only the answered port is reported"
        );
        assert_eq!(transport.probe_count(), 2, "no retry of the refused target");
    }

    #[tokio::test]
    async fn a_non_success_status_is_not_a_hit() {
        // A 302 is not followed by the probe client, so it arrives as `Ok(false)`.
        let transport = ScriptedTransport::new([]).with_probe_script([Ok(false), Ok(true)]);
        let found = probe(&transport).await;
        assert_eq!(ports(&found), vec![1234]);
    }
}
