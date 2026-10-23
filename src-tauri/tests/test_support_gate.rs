//! Task 2.7 — the credential-store double is absent from a default-feature build.
//!
//! Two independent facts make that true, and both are asserted here:
//!
//! 1. `test-support` is declared and the default feature set is empty, so no ordinary
//!    build enables the double by accident.
//! 2. Every item of the double is gated on `cfg(any(test, feature = "test-support"))`.
//!    A release build never sets `cfg(test)`, so the type cannot exist in it. The
//!    reason this is a proof and not a claim: integration tests build the library
//!    **without** `cfg(test)`, so an ungated reference to the double would already be a
//!    compile error in the run that executes this file.
//!
//! The `FailAfter(2)` sequence is asserted in `src/security/keyring.rs`, next to the
//! fixture it needs; it needs no sleep and no randomness, which is what makes a
//! failing test a real regression rather than a flake.

use std::fs;
use std::path::Path;

const GATE: &str = "#[cfg(any(test, feature = \"test-support\"))]";

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => panic!("cannot read {}: {error}", path.display()),
    }
}

/// The `[features]` section, with comments and blank lines removed.
fn feature_block() -> Vec<String> {
    let manifest = read(&manifest_dir().join("Cargo.toml"));
    let mut block = Vec::new();
    let mut in_features = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed == "[features]" {
            in_features = true;
            continue;
        }
        if !in_features {
            continue;
        }
        if trimmed.starts_with('[') {
            break;
        }
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            block.push(trimmed.to_owned());
        }
    }
    assert!(in_features, "Cargo.toml has no [features] section");
    block
}

#[test]
fn test_support_is_declared_and_is_not_a_default_feature() {
    let block = feature_block();
    assert!(
        block.iter().any(|line| line == "test-support = []"),
        "test-support is not declared: {block:?}"
    );
    let default = block
        .iter()
        .find(|line| line.starts_with("default"))
        .map(String::as_str);
    assert_eq!(
        default,
        Some("default = []"),
        "the default feature set must stay empty, or a shipped build could contain the double"
    );
}

#[test]
fn the_double_is_declared_only_under_the_test_support_gate() {
    let source = read(
        &manifest_dir()
            .join("src")
            .join("security")
            .join("keyring.rs"),
    );
    assert!(source.contains(GATE), "the gate is not present at all");

    for declaration in [
        "struct InMemoryState",
        "pub struct InMemoryKeyStore",
        "pub enum Fault",
        "impl InMemoryKeyStore",
        "impl Default for InMemoryKeyStore",
        "impl KeyStore for InMemoryKeyStore",
    ] {
        let position = match source.find(declaration) {
            Some(position) => position,
            None => panic!("{declaration} is missing"),
        };
        // The item's attribute block, read backwards: `#[...]` attributes and `///`
        // doc lines, in whichever order they appear, up to the first line that is
        // neither. The gate has to be in that block, so a declaration cannot sit next
        // to a `#[derive]` and escape the gate.
        let attributes: Vec<&str> = source[..position]
            .lines()
            .rev()
            .map(str::trim)
            .take_while(|line| line.starts_with("#[") || line.starts_with("///"))
            .collect();
        assert!(
            attributes.contains(&GATE),
            "{declaration} is not behind the test-support gate: {attributes:?}"
        );
    }
}
