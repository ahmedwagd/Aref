//! The non-database halves of `NFR-201` and `FR-313`: nothing outside the keyring can
//! hold a key, and the fixture secret is not shaped like one.
//!
//! The database half — scanning the file bytes after a save and a list — runs in
//! `src/providers/keys.rs`, next to the fixture that writes them. What is left is
//! negative, so it is asserted against the source the way this repository asserts
//! capability absence (`tests/composition_inert.rs`): a manifest entry and a browser
//! storage call are both things a future change would have to add.

use std::fs;
use std::path::{Path, PathBuf};

const FIXTURE: &str = "fixture-secret-a";

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn repository_root() -> PathBuf {
    match manifest_dir().parent() {
        Some(root) => root.to_path_buf(),
        None => panic!("the crate has no parent directory"),
    }
}

fn read(path: &Path) -> String {
    match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) => panic!("cannot read {}: {error}", path.display()),
    }
}

fn source_files(directory: &Path, found: &mut Vec<PathBuf>) {
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
            source_files(&path, found);
        } else if path
            .extension()
            .is_some_and(|extension| extension == "rs" || extension == "ts" || extension == "tsx")
        {
            found.push(path);
        }
    }
}

#[test]
fn no_fallback_store_is_available_to_write_a_key_to() {
    // `FR-313`: an unavailable keyring blocks storage. The mechanical form is that
    // there is no second store to fall back to — `design.md` §8.2 keeps
    // `tauri-plugin-store` unregistered and not a dependency.
    let manifest = read(&manifest_dir().join("Cargo.toml"));
    assert!(
        !manifest.contains("tauri-plugin-store"),
        "a plaintext settings store is a dependency"
    );
    let package = read(&repository_root().join("package.json"));
    assert!(
        !package.contains("@tauri-apps/plugin-store"),
        "a plaintext settings store is a frontend dependency"
    );
}

#[test]
fn no_browser_storage_call_exists_in_either_side() {
    // `NFR-201` names `localStorage` explicitly. Nothing writes it today, and the
    // providers screen (`4.x`) is the only place that could: it holds a key for the
    // lifetime of one submit and never a cache (`design.md` §9).
    let mut files = Vec::new();
    source_files(&manifest_dir().join("src"), &mut files);
    source_files(&repository_root().join("src"), &mut files);
    assert!(files.len() >= 8, "the scan found too few files");

    let mut offenders: Vec<String> = Vec::new();
    for path in files {
        let text = read(&path);
        for needle in ["localStorage", "sessionStorage", "indexedDB"] {
            if text.contains(needle) {
                offenders.push(format!("{}: {needle}", path.display()));
            }
        }
    }
    assert_eq!(offenders, Vec::<String>::new());
}

#[test]
fn the_fixture_is_not_key_shaped() {
    // `NFR-207`: a fixture that looked like a real credential would be a false positive
    // for every secret scanner pointed at this repository.
    assert!(FIXTURE.starts_with("fixture-secret-"));
    for prefix in ["sk-", "sk_", "pk-", "xoxb-", "ghp_", "AKIA"] {
        assert!(
            !FIXTURE.starts_with(prefix),
            "{FIXTURE} starts with {prefix}"
        );
    }
    assert_eq!(FIXTURE.len(), 16);
    assert!(FIXTURE
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-'));
}
