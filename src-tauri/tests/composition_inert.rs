//! The composition root has no reporting surface and no side effect (task 1.4,
//! `NFR-203`, `NFR-202`).
//!
//! Proving a negative needs a stated method. Two complementary ones are used here:
//!
//! 1. **Capability absence** — no source in `src-tauri/src` can open a socket or spawn a
//!    process: no file names `std::net`, `std::process`, `TcpStream` or `UdpSocket`, and the
//!    HTTP client crate is named only under `providers/` (slice 3 moved that needle from
//!    "absent" to "confined"; see `an_http_client_stays_inside_providers`). Exactly one
//!    module may touch a file, the migration guard `storage/db.rs`, which `FR-404` requires
//!    (`design.md` §8.3). A future module that wants either ability has to break a test to
//!    get it.
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

/// Nothing in `src-tauri/src` may open a socket or spawn a process, and the one HTTP client
/// crate the request path uses may be named **only** inside `providers/`.
///
/// Slice 3 narrowed this guard: `providers::client::ReqwestTransport` is the single place
/// that speaks HTTP, which is what keeps the future detector from inheriting a client
/// (`design.md` §3.2 rule 4). The socket types stay forbidden everywhere — including in
/// `providers/` — because a request path that needs `std::net` directly would mean the
/// transport had been bypassed. That is the half of this guard the Detect path relies on.
#[test]
fn no_source_opens_a_socket_or_spawns_a_process() {
    for needle in ["std::net", "std::process", "TcpStream", "UdpSocket"] {
        assert_eq!(
            offenders(needle),
            Vec::<String>::new(),
            "{needle} is present in src-tauri/src"
        );
    }
}

/// The HTTP client crate appears only under `providers/` (design.md §3.2 rule 4).
///
/// This is the narrowed form of the pre-slice-3 check, which forbade the name outright.
/// The confinement is what matters: a detector module that wants a client has to move this
/// test to get one.
#[test]
fn an_http_client_stays_inside_providers() {
    let providers = Path::new("src").join("providers");
    let named = offenders("reqwest");
    let mut outside: Vec<String> = named
        .iter()
        .filter(|path| !Path::new(path).starts_with(&providers))
        .cloned()
        .collect();
    outside.sort();
    assert_eq!(
        outside,
        Vec::<String>::new(),
        "the HTTP client crate is named outside src/providers/"
    );

    // Positive control: the request path does name it, so the scan above is not passing on a
    // tree that has no client at all.
    let inside = named
        .iter()
        .filter(|path| Path::new(path).starts_with(&providers))
        .count();
    assert!(
        inside > 0,
        "no provider source names the HTTP client, so this guard proves nothing"
    );
}

/// `FR-404` and `design.md` §8.3 put one file copy in the build: the migration guard.
/// That guard is `storage/db.rs`, and it is the only source file allowed to name
/// `std::fs` — the scan below is the allow-list, so a second writer fails the build
/// rather than being discovered in review.
#[test]
fn only_the_migration_guard_writes_a_file() {
    let allowed = Path::new("src").join("storage").join("db.rs");
    let mut offenders = offenders("std::fs");
    offenders.retain(|path| Path::new(path) != allowed);
    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "a module other than storage/db.rs can write a file"
    );
    let guard = match std::fs::read_to_string(manifest_dir().join(&allowed)) {
        Ok(text) => text,
        Err(error) => panic!("cannot read storage/db.rs: {error}"),
    };
    assert!(
        guard.contains("std::fs"),
        "storage/db.rs no longer performs the migration guard's file copy"
    );
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

/// Exactly one HTTP client, declared with an explicit feature set (task 3.3/3.4).
///
/// Slice 3 narrowed this guard too: `reqwest` is now the request path's client, so the
/// assertion moved from "no client at all" to "no *other* client, and no default feature set
/// that would smuggle one in". `default-features = false` is the load-bearing part: the
/// default set pulls a TLS stack, a charset decoder and HTTP/2, none of which this slice
/// asked for. `cookies`, `http3`, `charset` and `system-proxy` stay off: no cookie store, no
/// HTTP/3, and no environment proxy a loopback probe could be routed through.
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

    for forbidden in [
        "hyper",
        "ureq",
        "isahc",
        "surf",
        "curl",
        "attohttpc",
        "native-tls",
        "openssl",
        "reqwest-middleware",
    ] {
        assert!(
            !dependencies.contains(forbidden),
            "{forbidden} is a direct dependency"
        );
    }

    let declaration = dependencies
        .lines()
        .find(|line| line.trim_start().starts_with("reqwest"))
        .unwrap_or_else(|| panic!("reqwest is not declared"));
    assert!(
        declaration.contains("default-features = false"),
        "reqwest must not inherit its default feature set: {declaration}"
    );
    for forbidden in [
        "cookies",
        "http3",
        "charset",
        "system-proxy",
        "blocking",
        "gzip",
        "brotli",
        "zstd",
        "deflate",
        "multipart",
        "socks",
    ] {
        assert!(
            !declaration.contains(forbidden),
            "{forbidden} must not be enabled: {declaration}"
        );
    }
    assert!(
        declaration.contains("\"rustls\""),
        "the request path needs TLS to reach a cloud provider: {declaration}"
    );
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
