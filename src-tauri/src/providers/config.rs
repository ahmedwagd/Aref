//! Provider constants, in one place (design.md §6.5; task 3.1).
//!
//! Two kinds of value live here, and they are not the same kind of decision:
//!
//! * **Requirement values** — [`BACKOFF_BASE`] and [`BACKOFF_CAP`] come from `G-13`;
//!   [`PROBE_TARGETS`] comes from `FR-304`.
//! * **Engineering defaults** — [`MAX_TRANSPORT_ATTEMPTS`], the two provider timeouts,
//!   and [`PROBE_TIMEOUT`]. They are bounded and injectable so a later change (or the
//!   open `G-23` latency budgets) can revise them without touching a call site.
//!
//! Why one file: a call site that repeats `1` or `11434` is a second source of truth
//! that a constant change would miss. `tests/providers_config.rs` scans the production
//! sources for both kinds of literal and fails if one reappears outside this file.
//!
//! [`KEYRING_SERVICE`] is **re-exported**, not re-declared: `security/` is the lower
//! layer and must not import `providers/` (design.md §3.2 rule 1, and §6.5 says the same
//! about the mask constants that sit beside their only use). The single definition stays
//! in `security::keyring`, where `entry_name` builds every entry from it, and the name is
//! reachable here so no provider call site has to repeat the string.

use std::time::Duration;

use crate::storage::providers_repo::LocalFlavor;

pub use crate::security::keyring::KEYRING_SERVICE;

/// The first wait of the backoff curve (`G-13`): one second.
pub const BACKOFF_BASE: Duration = Duration::from_secs(1);

/// The ceiling of the backoff curve (`G-13`): thirty seconds.
pub const BACKOFF_CAP: Duration = Duration::from_secs(30);

/// How many attempts one credential gets before the pool moves on.
///
/// An engineering default, not a requirement value: bounded, because unbounded retry is
/// an anti-pattern, and injected through [`BackoffPolicy`](crate::providers::retry)
/// rather than read from here, so the humanize change can pass its own budget.
pub const MAX_TRANSPORT_ATTEMPTS: u32 = 3;

/// How long establishing a connection may take.
pub const PROVIDER_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// How long a whole request may take, body included.
pub const PROVIDER_REQUEST_TIMEOUT: Duration = Duration::from_secs(120);

/// How long a discovery probe may take. Short on purpose: a local model server that is
/// running answers a liveness request immediately, and a user waits for the panel.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(1);

/// One loopback destination discovery may contact (`FR-304`, `NFR-206`).
///
/// A `const` array of literals, not a value built from input: there is no parameter
/// through which another host could enter, which is what makes "loopback-only" a property
/// of the type rather than a check the runtime could skip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProbeTarget {
    pub flavor: LocalFlavor,
    pub base_url: &'static str,
    pub port: u16,
}

/// The only two destinations discovery may attempt: Ollama, then LM Studio.
///
/// `127.0.0.1` rather than `localhost`: the numeric loopback needs no name resolution, so
/// no resolver step could return something else (design.md §7.2 property 2).
pub const PROBE_TARGETS: [ProbeTarget; 2] = [
    ProbeTarget {
        flavor: LocalFlavor::Ollama,
        base_url: "http://127.0.0.1:11434/v1",
        port: 11434,
    },
    ProbeTarget {
        flavor: LocalFlavor::LmStudio,
        base_url: "http://127.0.0.1:1234/v1",
        port: 1234,
    },
];
