//! The one type that holds a raw API key.
//!
//! `Secret` exists so that key material cannot travel by accident (design.md §5.1).
//! The properties are the contract, and each one closes a specific leak:
//!
//! * not `Clone` — a copy is a second place key material lives;
//! * not `Serialize` — no DTO, envelope, or event payload can carry it;
//! * `Debug` prints `<redacted>` — `{:?}` is how a key reaches a log line;
//! * no `Display` — `{}` is the other way;
//! * no `as_str` — the value is read through [`Secret::expose_secret`], whose name is
//!   the review checkpoint.
//!
//! Memory zeroization is not attempted in v1: no requirement asks for it, and `zeroize`
//! would be a dependency with no measurable promise on a managed-heap `String`
//! (design.md §5.1). That is a stated limitation, not an oversight.

use std::fmt;

/// A raw API key or token.
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The raw value.
    ///
    /// Visible crate-wide because Rust has no friend modules, but there are exactly two
    /// legitimate callers (design.md §3.2 rule 6, §5.1): `security::keyring` writes it,
    /// and `providers::client` formats the `Authorization` header from it. A `pub`
    /// accessor, a `Deref` to `str`, or an `as_str` name would make the accidental
    /// third caller trivial, which is why the name spells out what it does.
    pub(crate) fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("<redacted>")
    }
}

impl PartialEq for Secret {
    /// Plain `==` (design.md §5.2). This is the read-back comparison after a save: a
    /// local, single-user operation with no attacker in the loop, so a constant-time
    /// comparison would be cargo-culted here.
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for Secret {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_prints_the_redaction_marker_and_nothing_else() {
        let secret = Secret::new("fixture-secret-a");
        assert_eq!(format!("{secret:?}"), "<redacted>");
    }

    #[test]
    fn the_raw_value_is_unchanged_by_construction() {
        let secret = Secret::new("fixture-secret-a");
        assert_eq!(secret.expose_secret(), "fixture-secret-a");
        assert_eq!(secret, Secret::new("fixture-secret-a"));
        assert_ne!(secret, Secret::new("fixture-secret-b"));
    }

    #[test]
    fn a_value_with_leading_and_trailing_spaces_is_kept_verbatim() {
        // No trimming: the key is what the user pasted (`commands::parse` decides
        // presence, and a blank-only value is the only rejection).
        let secret = Secret::new(" fixture-secret-a ");
        assert_eq!(format!("{secret:?}"), "<redacted>");
        assert_eq!(secret.expose_secret(), " fixture-secret-a ");
    }
}
