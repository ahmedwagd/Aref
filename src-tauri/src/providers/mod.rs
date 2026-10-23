//! Provider services.
//!
//! Slice 2 needs one: `keys`, the save/edit/remove path into the credential store. The
//! request path — `client`, `retry`, `pool`, `selection`, `discovery`, `output`,
//! `events`, `config` — belongs to slice 3 and is deliberately absent here
//! (design.md §3.1).

pub mod keys;
