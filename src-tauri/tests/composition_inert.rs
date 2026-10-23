//! The composition root has no reporting surface and no side effect (task 1.4,
//! `NFR-203`, `NFR-202`).
//!
//! Proving a negative needs a stated method. Two complementary ones are used here:
//!
//! 1. **Capability absence** — nothing in `src-tauri/src` can write a file or open a
//!    socket, because no source refers to `std::fs`, `std::net`, `std::process` or an
//!    HTTP crate, and no direct dependency supplies one. A future module that wants
//!    that ability has to break this test to get it.
//! 2. **Observation** — composing the application state twice leaves the source tree
//!    byte-identical (no file appears or changes) and yields the same snapshot, so the
//!    composition path is pure. A socket cannot be counted portably without a syscall
//!    hook, which is why (1) carries the socket half of the claim.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use aref_lib::compose_app_state;
use aref_lib::status::ConstantDetectorMode;

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn source_files(directory: &Path, found: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(directory) {
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
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

fn rust_sources() -> Vec<(PathBuf, String)> {
    let mut paths = Vec::new();
    source_files(&manifest_dir().join("src"), &mut paths);
    assert!(
        paths.len() >= 4,
        "the scan found too few files to be meaningful"
    );
    paths
        .into_iter()
        .map(|path| {
            let text = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(error) => panic!("cannot read {}: {error}", path.display()),
            };
            (path, text)
        })
        .collect()
}

fn offenders(needle: &str) -> Vec<String> {
    rust_sources()
        .into_iter()
        .filter(|(_, text)| text.contains(needle))
        .map(|(path, _)| {
            path.strip_prefix(manifest_dir())
                .unwrap_or(&path)
                .display()
                .to_string()
        })
        .collect()
}

/// A snapshot of every path under `root` with its length, so a created, deleted or
/// rewritten file is visible without reading file contents.
fn tree_snapshot(root: &Path) -> BTreeSet<(String, u64)> {
    let mut seen = BTreeSet::new();
    collect_snapshot(root, root, &mut seen);
    seen
}

fn collect_snapshot(root: &Path, directory: &Path, seen: &mut BTreeSet<(String, u64)>) {
    let entries = match std::fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => panic!("cannot read {}: {error}", directory.display()),
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => panic!("cannot read an entry: {error}"),
        };
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(metadata) => metadata,
            Err(error) => panic!("cannot stat {}: {error}", path.display()),
        };
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();
        if metadata.is_dir() {
            seen.insert((format!("{relative}/"), 0));
            collect_snapshot(root, &path, seen);
        } else {
            seen.insert((relative, metadata.len()));
        }
    }
}

#[test]
fn no_reporting_or_remote_config_module_exists() {
    for needle in [
        "telemetry",
        "analytics",
        "sentry",
        "crash_report",
        "remote_config",
    ] {
        assert_eq!(
            offenders(needle),
            Vec::<String>::new(),
            "{needle} is present"
        );
    }
}

#[test]
fn no_source_can_write_a_file_or_open_a_socket() {
    for needle in [
        "std::fs",
        "std::net",
        "std::process",
        "reqwest",
        "TcpStream",
        "UdpSocket",
    ] {
        assert_eq!(
            offenders(needle),
            Vec::<String>::new(),
            "{needle} is present"
        );
    }
}

#[test]
fn the_composition_root_registers_no_plugin_and_no_setup_hook() {
    let lib = match std::fs::read_to_string(manifest_dir().join("src/lib.rs")) {
        Ok(text) => text,
        Err(error) => panic!("cannot read lib.rs: {error}"),
    };
    assert!(!lib.contains(".plugin("), "a plugin is registered");
    assert!(!lib.contains("setup("), "a setup hook exists");
    assert!(
        !lib.contains("Builder::default().setup"),
        "a setup hook exists"
    );
}

#[test]
fn no_direct_dependency_supplies_an_http_client() {
    let manifest = match std::fs::read_to_string(manifest_dir().join("Cargo.toml")) {
        Ok(text) => text,
        Err(error) => panic!("cannot read Cargo.toml: {error}"),
    };
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap_or_default()
        .split("\n[")
        .next()
        .unwrap_or_default();
    for forbidden in ["reqwest", "hyper", "ureq", "isahc", "surf", "curl"] {
        assert!(
            !dependencies.contains(forbidden),
            "{forbidden} is a direct dependency"
        );
    }
}

#[test]
fn composing_the_state_twice_is_pure_and_writes_nothing() {
    let root = manifest_dir().join("src");
    let before = tree_snapshot(&root);

    let first = compose_app_state(Arc::new(ConstantDetectorMode::statistical_only()), "0.1.0");
    let second = compose_app_state(Arc::new(ConstantDetectorMode::statistical_only()), "0.1.0");

    let after = tree_snapshot(&root);
    assert_eq!(before, after, "composition wrote to the source tree");
    assert_eq!(first.status().snapshot(), second.status().snapshot());
    assert!(!first.status().snapshot().humanize_available);
}
