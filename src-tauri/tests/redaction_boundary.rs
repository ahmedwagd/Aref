//! Task 2.5 — `Secret` is the only type that holds key material, and it cannot leak.
//!
//! Two methods, because the claims are of two kinds. `Debug` is asserted at runtime,
//! since it is behaviour. The absence of `Clone`, of `Serialize`, and of a string
//! accessor is asserted against the source: those are capability absences, the same way
//! `tests/composition_inert.rs` proves the composition root cannot write a file.

use std::fs;
use std::path::{Path, PathBuf};

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
fn debug_prints_only_the_redaction_marker() {
    let secret = Secret::new("fixture-secret-a");
    assert_eq!(format!("{secret:?}"), "<redacted>");
    assert_eq!(format!("{secret:#?}"), "<redacted>");
}

#[test]
fn secret_has_no_string_accessor() {
    let source = read(
        &manifest_dir()
            .join("src")
            .join("security")
            .join("redact.rs"),
    );
    assert!(
        !source.contains("fn as_str"),
        "Secret grew an as_str accessor"
    );
    assert!(
        !source.contains("impl Deref"),
        "Secret grew a Deref, which formats itself implicitly"
    );
    assert!(
        !source.contains("impl fmt::Display for Secret"),
        "Secret grew a Display, which `{{}}` renders unredacted"
    );
}

#[test]
fn secret_is_neither_cloneable_nor_serializable() {
    let source = read(
        &manifest_dir()
            .join("src")
            .join("security")
            .join("redact.rs"),
    );
    assert!(
        !source.contains("impl Serialize"),
        "Secret grew a Serialize impl"
    );
    for line in source.lines() {
        let trimmed = line.trim();
        let declares_secret = (trimmed.starts_with("impl") || trimmed.starts_with("#[derive"))
            && trimmed.contains("Secret");
        if declares_secret {
            assert!(!trimmed.contains("Clone"), "Secret is cloneable: {trimmed}");
            assert!(
                !trimmed.contains("Serialize"),
                "Secret is serializable: {trimmed}"
            );
        }
    }
}

#[test]
fn no_dto_field_holds_a_secret() {
    // `crate::security` is the only module allowed to name `Secret` from the command
    // layer's perspective: no request and no response field may be one (design.md §4.1).
    let mut paths = Vec::new();
    rust_sources(&manifest_dir().join("src").join("commands"), &mut paths);
    assert!(
        paths.len() >= 4,
        "the scan found too few command files to be meaningful"
    );
    let mut offenders: Vec<String> = Vec::new();
    for path in paths {
        if read(&path).contains("Secret") {
            offenders.push(
                path.strip_prefix(manifest_dir())
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
        }
    }
    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "the command layer names Secret, so a DTO field could carry key material"
    );
}

#[test]
fn the_secret_type_is_reachable_only_through_the_security_module() {
    // `Secret` is declared once. A second declaration would be a second place a raw key
    // can live, and the keyring trait would be pointed at the wrong one.
    let mut paths = Vec::new();
    rust_sources(&manifest_dir().join("src"), &mut paths);
    let declarations = paths
        .iter()
        .filter(|path| read(path).contains("pub struct Secret"))
        .count();
    assert_eq!(declarations, 1, "Secret is declared more than once");
}
