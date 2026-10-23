//! Task 2.13 — the mask constants are pinned, and no call site repeats either literal.
//!
//! Task 2.6's unit tests are deliberately literal-free: they assert
//! `MASK_WIDTH + VISIBLE_TAIL` and never the numbers. This file is the one place the
//! values themselves are written down, so a change to either constant fails here, and a
//! call site that hides a copy of one fails the scan below.

use std::fs;
use std::path::{Path, PathBuf};

use aref_lib::security::mask::{preview, MASK_CHAR, MASK_WIDTH};
use aref_lib::security::redact::Secret;

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => panic!("cannot read {}: {error}", path.display()),
    }
}

fn rust_sources(directory: &Path, found: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => panic!("cannot read {}: {error}", directory.display()),
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => panic!("cannot read an entry: {error}"),
        };
        let path = entry.path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn the_width_is_pinned_to_eight() {
    assert_eq!(MASK_WIDTH, 8);
}

#[test]
fn the_mask_character_is_pinned_to_the_bullet() {
    assert_eq!(MASK_CHAR, '•');
    // U+2022, not a hyphen, an asterisk, or a middle dot: the pressed value is the
    // bullet, and a lookalike would change every stored preview.
    assert_eq!(u32::from(MASK_CHAR), 0x2022);
}

#[test]
fn a_four_character_key_renders_twelve_mask_characters_and_reveals_nothing() {
    let key = "abcd";
    let rendered = preview(&Secret::new(key)).to_string();
    assert_eq!(rendered, "••••••••••••");
    assert_eq!(rendered.chars().count(), 12);

    // Character by character, so a single revealed character cannot hide behind a
    // length or a string comparison.
    for (position, character) in rendered.chars().enumerate() {
        assert_eq!(character, '•', "position {position} is not MASK_CHAR");
        assert!(
            !key.contains(character),
            "position {position} reproduced a character of the key"
        );
    }
}

#[test]
fn a_five_character_key_renders_eight_mask_characters_and_its_last_four() {
    let rendered = preview(&Secret::new("abcde")).to_string();
    assert_eq!(rendered, "••••••••bcde");
    assert_eq!(rendered.chars().count(), 12);
    // Exactly one character stays hidden, and it is the first.
    assert!(!rendered.contains('a'));
}

#[test]
fn a_longer_key_keeps_the_same_width_and_the_same_four_revealed_characters() {
    let rendered = preview(&Secret::new("fixture-secret-9876")).to_string();
    assert_eq!(rendered, "••••••••9876");
    assert_eq!(rendered.chars().count(), 12);
    assert!(!rendered.contains('f'));
}

#[test]
fn both_constants_are_declared_as_named_constants_in_the_mask_module() {
    let source = read(&manifest_dir().join("src").join("security").join("mask.rs"));
    assert!(source.contains("pub const MASK_WIDTH: usize"), "MASK_WIDTH");
    assert!(source.contains("pub const MASK_CHAR: char"), "MASK_CHAR");
}

#[test]
fn no_call_site_repeats_either_literal() {
    let mut paths = Vec::new();
    rust_sources(&manifest_dir().join("src"), &mut paths);
    assert!(
        paths.len() >= 8,
        "the scan found too few files to be meaningful"
    );

    let declaration_site = PathBuf::from("security").join("mask.rs");
    let mut offenders: Vec<String> = Vec::new();
    for path in paths {
        if path.ends_with(&declaration_site) {
            continue;
        }
        let text = read(&path);
        let relative = path
            .strip_prefix(manifest_dir())
            .unwrap_or(&path)
            .display()
            .to_string();
        for needle in ["MASK_WIDTH", "MASK_CHAR", "'•'"] {
            if text.contains(needle) {
                offenders.push(format!("{relative}: {needle}"));
            }
        }
    }
    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "a mask literal is repeated outside security/mask.rs"
    );
}
