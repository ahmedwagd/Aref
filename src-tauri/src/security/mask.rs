//! The masked preview: the only key-derived value that leaves the core (`FR-310`).
//!
//! Composition has two branches, and they are separate rules rather than one rule plus
//! an edge case (design.md §5.3, §6.5; settled decision `D-4`):
//!
//! * **longer than [`VISIBLE_TAIL`]** — [`MASK_WIDTH`] mask characters followed by the
//!   secret's last [`VISIBLE_TAIL`] characters;
//! * **at or below [`VISIBLE_TAIL`]** — [`MASK_WIDTH`] + [`VISIBLE_TAIL`] mask
//!   characters and no tail at all, so no character of the secret is read. At exactly
//!   four characters the "last four" *are* the whole secret, so the tail is revealed
//!   only when it is a proper suffix.
//!
//! Width is a different claim from composition: both branches emit exactly
//! [`MASK_WIDTH`] + [`VISIBLE_TAIL`] characters at every key length, which is `FR-310`'s
//! fixed-width criterion.
//!
//! [`preview`] is called only at save time, on the value already in hand. No read path
//! calls it, because `G-27` forbids recomputing a stored preview by re-reading a key.

use std::fmt;

use serde::{Serialize, Serializer};

use crate::security::redact::Secret;

/// How many trailing characters a preview may reveal.
///
/// A `G-27` requirement value, not a tuning knob.
pub const VISIBLE_TAIL: usize = 4;

/// The mask's own width.
///
/// A security constant, not a design token: `FR-310` requires a width that does not vary
/// with key length, and a persisted preview cannot be revised once a key is saved
/// (design.md §6.5, `R-4`). It therefore lives here, beside its only use, and not in a
/// visual-token file.
pub const MASK_WIDTH: usize = 8;

/// The character a withheld character is replaced with — U+2022 BULLET.
///
/// Same standing as [`MASK_WIDTH`]: a security constant pinned in code, not a token.
pub const MASK_CHAR: char = '•';

/// A fixed-width, non-reversible rendering of a secret.
///
/// The wire type of `KeyMetadata.masked_preview` (design.md §4.2), which is why it is
/// serializable while [`Secret`] is not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskedPreview(String);

impl MaskedPreview {
    /// Rebuilds a preview that was persisted at save time.
    ///
    /// `G-27` forbids recomputing a stored preview by re-reading a key, so this is the
    /// only constructor a read path may use, and the name says which path it is. The
    /// caller is the repository, with the value of `provider_keys.masked_preview`.
    pub fn from_stored(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl fmt::Display for MaskedPreview {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for MaskedPreview {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

/// Derives the preview of `secret`.
///
/// Length is counted in `char`s and the tail is taken on a `char` boundary, so a
/// multi-byte character is never split. Nothing here can reveal a prefix character: the
/// tail is a suffix of the value, and the short branch reads no character at all.
pub fn preview(secret: &Secret) -> MaskedPreview {
    let value = secret.expose_secret();
    let length = value.chars().count();
    let mut rendered = String::with_capacity((MASK_WIDTH + VISIBLE_TAIL) * MASK_CHAR.len_utf8());

    if length > VISIBLE_TAIL {
        push_mask(&mut rendered, MASK_WIDTH);
        rendered.extend(value.chars().skip(length - VISIBLE_TAIL));
    } else {
        // Both slots are filled with the mask: a short secret leaves a gap that only
        // `MASK_CHAR` can fill without either revealing the whole secret or changing
        // the fixed width.
        push_mask(&mut rendered, MASK_WIDTH + VISIBLE_TAIL);
    }

    MaskedPreview(rendered)
}

fn push_mask(rendered: &mut String, count: usize) {
    for _ in 0..count {
        rendered.push(MASK_CHAR);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Distinct characters, so "no prefix character appears" is a real assertion rather
    /// than an accident of repetition. Latin, digits, and Arabic: the product is
    /// Arabic-first, and a `char`-boundary bug would show up on the Arabic range.
    const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyz0123456789ابتثجحخدذرزسشصضطظعغفقكلمنهوي";

    fn alphabet() -> Vec<char> {
        ALPHABET.chars().collect()
    }

    fn secret_of(length: usize) -> (Vec<char>, Secret) {
        let characters: Vec<char> = alphabet().into_iter().take(length).collect();
        assert_eq!(
            characters.len(),
            length,
            "the fixture alphabet is too short"
        );
        let secret = Secret::new(characters.iter().collect::<String>());
        (characters, secret)
    }

    fn rendered(secret: &Secret) -> Vec<char> {
        preview(secret).to_string().chars().collect()
    }

    #[test]
    fn the_width_is_one_value_at_every_key_length() {
        let expected = MASK_WIDTH + VISIBLE_TAIL;
        for length in 1..=alphabet().len() {
            let (_, secret) = secret_of(length);
            assert_eq!(
                rendered(&secret).len(),
                expected,
                "length {length} produced a different width"
            );
        }
    }

    #[test]
    fn above_the_threshold_the_tail_is_the_last_visible_tail_characters() {
        for length in (VISIBLE_TAIL + 1)..=alphabet().len() {
            let (characters, secret) = secret_of(length);
            let output = rendered(&secret);
            assert_eq!(output.len(), MASK_WIDTH + VISIBLE_TAIL, "length {length}");
            assert!(
                output[..MASK_WIDTH].iter().all(|c| *c == MASK_CHAR),
                "length {length} did not start with the mask"
            );
            assert_eq!(
                &output[MASK_WIDTH..],
                &characters[length - VISIBLE_TAIL..],
                "length {length} did not end with the tail"
            );
        }
    }

    #[test]
    fn one_and_four_character_fixtures_render_the_mask_only() {
        // The two named fixtures are the endpoints of this range, and the intermediate
        // lengths are covered because the rule is a predicate, not a special case.
        for length in 1..=VISIBLE_TAIL {
            let (characters, secret) = secret_of(length);
            let output = rendered(&secret);
            assert_eq!(
                output.len(),
                MASK_WIDTH + VISIBLE_TAIL,
                "length {length} was truncated to the secret's length"
            );
            for (position, rendered_char) in output.iter().enumerate() {
                assert_eq!(
                    *rendered_char, MASK_CHAR,
                    "length {length} revealed {rendered_char:?} at {position}"
                );
                assert!(
                    !characters.contains(rendered_char),
                    "length {length} reproduced one of its own characters at {position}"
                );
            }
        }
    }

    #[test]
    fn no_prefix_character_ever_appears() {
        for length in 1..=alphabet().len() {
            let (characters, secret) = secret_of(length);
            let output = rendered(&secret);
            let revealed_from = length.saturating_sub(VISIBLE_TAIL);
            for character in &characters[..revealed_from] {
                assert!(
                    !output.contains(character),
                    "length {length} revealed the prefix character {character:?}"
                );
            }
        }
    }

    #[test]
    fn keys_differing_only_outside_the_tail_share_a_preview_by_the_byte() {
        let first = Secret::new("fixture-secret-9876");
        let second = Secret::new("a-different-9876");
        assert_ne!(first, second);
        assert_eq!(preview(&first), preview(&second));
        assert_eq!(preview(&first).to_string(), preview(&second).to_string());
    }

    #[test]
    fn a_multi_byte_tail_is_never_split() {
        // Four Arabic characters are four `char`s and eight UTF-8 bytes: a byte-based
        // suffix would either split one or reveal the wrong count.
        let secret = Secret::new("fixture-secret-انتث");
        let output = preview(&secret).to_string();
        assert_eq!(output.chars().count(), MASK_WIDTH + VISIBLE_TAIL);
        assert!(output.ends_with("انتث"));
    }
}
