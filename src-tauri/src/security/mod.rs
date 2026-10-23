//! Custody of key material.
//!
//! Three responsibilities, one module:
//!
//! * `redact` — the one type that holds a raw key (`NFR-201`).
//! * `mask` — the only key-derived value that may leave this module (`FR-310`).
//! * `keyring` — the `KeyStore` seam and the OS credential-store implementation
//!   (`FR-310`, `FR-313`).
//!
//! Layering (design.md §3.2 rule 1): `security` is infrastructure. It imports nothing
//! from `providers`, `storage`, or `commands`, and it touches no network.

pub mod keyring;
pub mod mask;
pub mod redact;
