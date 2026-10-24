//! Task 3.19 — the four layering rules of `design.md` §3.2, each as a test — plus the
//! no-logging-backend rule of `design.md` §3.4 / settled `D-3` that task 3.18 checks.
//!
//! A documented convention is a rule nobody can fail. Each rule below is a scan of the
//! tree, so a future module that breaks it breaks the build rather than being discovered in
//! review. The scans are structural rather than clever: a substring on the production
//! source, an allow-list of paths, and a positive control so a scan cannot pass because it
//! found nothing to look at.
//!
//! Rules 3 and 5 (services never import `commands`; only `src/ipc/` calls `invoke`/`listen`)
//! and rules 4 and 6 (`reqwest` only under `providers/`; only `security::keyring` writes a
//! secret) are all here.

use std::fs;
use std::path::{Path, PathBuf};

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

fn sources_with(directory: &Path, extensions: &[&str], found: &mut Vec<PathBuf>) {
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
            sources_with(&path, extensions, found);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extensions.contains(&extension))
        {
            found.push(path);
        }
    }
}

fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    sources_with(directory, &["rs"], &mut found);
    found
}

/// A path relative to the crate, with forward slashes so the assertions read the same on
/// every platform.
fn relative(path: &Path) -> String {
    path.strip_prefix(manifest_dir())
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

/// Rule 3 — `commands → services → infrastructure`, never back.
#[test]
fn services_never_import_commands() {
    let mut scanned = 0;
    let mut offenders = Vec::new();
    for module in ["providers", "security", "storage"] {
        for path in rust_sources(&manifest_dir().join("src").join(module)) {
            scanned += 1;
            if read(&path).contains("crate::commands") {
                offenders.push(relative(&path));
            }
        }
    }
    assert!(scanned >= 8, "the scan found too few files: {scanned}");
    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "a service imports the command layer"
    );
}

/// Rule 4 — the HTTP client stays inside `providers/`, so the future detector cannot
/// inherit one without breaking this test.
#[test]
fn an_http_client_is_named_only_under_providers() {
    let mut scanned = 0;
    let mut offenders = Vec::new();
    for path in rust_sources(&manifest_dir().join("src")) {
        scanned += 1;
        if read(&path).contains("reqwest") && !relative(&path).starts_with("src/providers/") {
            offenders.push(relative(&path));
        }
    }
    assert!(scanned >= 12, "the scan found too few files: {scanned}");
    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "an HTTP client is named outside providers/"
    );
}

/// Rule 5 — only `src/ipc/` calls `invoke` or listens to an event (`AGENTS.md` §10).
#[test]
fn only_the_ipc_module_calls_invoke_or_listen() {
    let frontend = repository_root().join("src");
    let mut found = Vec::new();
    sources_with(&frontend, &["ts", "tsx"], &mut found);
    assert!(
        found.len() >= 8,
        "the scan found too few frontend files: {}",
        found.len()
    );

    let mut offenders = Vec::new();
    let mut positives = 0;
    for path in found {
        let text = read(&path);
        let relative_path = path
            .strip_prefix(repository_root())
            .unwrap_or(&path)
            .display()
            .to_string()
            .replace('\\', "/");
        for needle in ["invoke", "listen"] {
            if text.contains(needle) {
                if relative_path.starts_with("src/ipc/") {
                    positives += 1;
                } else {
                    offenders.push(format!("{relative_path}: {needle}"));
                }
            }
        }
    }
    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "a module outside src/ipc/ reaches Tauri directly"
    );
    assert!(
        positives >= 2,
        "neither `invoke` nor `listen` was found in src/ipc/, so the scan proves nothing"
    );
}

/// Rule 6 — only `security::keyring` touches the OS credential store, and `KeyStore::write`
/// has exactly one caller (`providers::keys`).
#[test]
fn only_the_keyring_module_writes_a_secret() {
    let os_store_api = [
        "keyring::Entry",
        "set_password",
        "get_password",
        "delete_credential",
    ];
    let mut store_offenders = Vec::new();
    let mut write_callers = Vec::new();

    for path in rust_sources(&manifest_dir().join("src")) {
        let text = read(&path);
        let relative_path = relative(&path);
        if os_store_api.iter().any(|needle| text.contains(needle))
            && relative_path != "src/security/keyring.rs"
        {
            store_offenders.push(relative_path.clone());
        }
        if text.contains(".write(") {
            write_callers.push(relative_path);
        }
    }

    assert_eq!(
        store_offenders,
        Vec::<String>::new(),
        "a module other than security::keyring names the OS store API"
    );
    // `security/keyring.rs` implements the trait and exercises its own double; the one
    // production caller is the key service. Anything else is a second write path.
    write_callers.sort();
    assert_eq!(
        write_callers,
        vec![
            "src/providers/keys.rs".to_owned(),
            "src/security/keyring.rs".to_owned()
        ],
        "KeyStore::write gained a caller outside providers::keys"
    );

    let keys = read(&manifest_dir().join("src").join("providers").join("keys.rs"));
    assert!(
        keys.matches(".write(").count() == 2,
        "providers/keys.rs should hold exactly one production call to `write` \
         (plus its own gated fixture); found {}",
        keys.matches(".write(").count()
    );
}

/// Every file of the crate that is worth scanning: the manifest, the lockfile, the sources,
/// the tests and the capabilities. `target/` is skipped — it holds build output, not code.
fn crate_text_files(directory: &Path, found: &mut Vec<PathBuf>) {
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
        if path.file_name().is_some_and(|name| name == "target") {
            continue;
        }
        if path.is_dir() {
            crate_text_files(&path, found);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| ["rs", "toml", "json", "lock", "sql"].contains(&extension))
        {
            found.push(path);
        }
    }
}

/// `D-3` — no logging backend in slices 1–3, so the `log::` records a later change adds
/// stay inert until that change picks a sink and audits it.
///
/// Two facts: no backend crate is named anywhere in the crate (including the lockfile), and
/// the manifest's direct dependencies hold no logging facade — which is what makes "the
/// records reach no sink" true, because without a facade there are no records to sink.
#[test]
fn no_logging_backend_is_installed() {
    let mut files = Vec::new();
    crate_text_files(manifest_dir(), &mut files);
    assert!(
        files.len() >= 20,
        "the scan found too few files: {}",
        files.len()
    );

    let mut offenders = Vec::new();
    for path in files {
        // The scan's own file necessarily writes the needles down; like the mask-constant
        // scan, it excludes the declaration site and not the claim.
        if relative(&path) == "tests/provider_architecture.rs" {
            continue;
        }
        let text = read(&path);
        for needle in [
            "env_logger",
            "fern",
            "tracing_subscriber",
            "tracing-subscriber",
            "tauri-plugin-log",
        ] {
            if text.contains(needle) {
                offenders.push(format!("{}: {needle}", relative(&path)));
            }
        }
    }
    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "a logging backend is present, and this slice installs none"
    );

    let manifest = read(&manifest_dir().join("Cargo.toml"));
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .unwrap_or_default()
        .split("\n[")
        .next()
        .unwrap_or_default();
    for line in dependencies.lines() {
        let name = line.split('=').next().unwrap_or_default().trim();
        for facade in ["log", "tracing", "simplelog", "fern", "env_logger"] {
            assert_ne!(name, facade, "{facade} is a direct dependency");
        }
    }
}
