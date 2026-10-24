//! Task 3.1 — the provider constants live in one file, and no call site repeats a literal.
//!
//! Two claims, two kinds of assertion. The **values** are asserted against the requirement
//! that fixes them (`G-13` for the curve, `FR-304` for the probe targets). The
//! **absence of literals elsewhere** is asserted by scanning the production sources, which
//! is how this repository tests a capability absence (`tests/composition_inert.rs`).
//!
//! The scan stops each file at its first `#[cfg(test)]` marker. A unit test that asserts a
//! known instant with `Duration::from_secs(1_700_000_000)` is not a call site of a provider
//! constant: it is a fixture, and the constants' purpose — one place to revise — is not
//! threatened by it. `AGENTS.md` §14.1 forbids reinterpreting a check, so the narrower scan
//! is stated here rather than hidden: every non-test line of every file under
//! `src-tauri/src` is scanned.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use aref_lib::providers::config::{
    ProbeTarget, BACKOFF_BASE, BACKOFF_CAP, KEYRING_SERVICE, MAX_TRANSPORT_ATTEMPTS, PROBE_TARGETS,
    PROBE_TIMEOUT, PROVIDER_CONNECT_TIMEOUT, PROVIDER_REQUEST_TIMEOUT,
};
use aref_lib::storage::providers_repo::LocalFlavor;

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

/// Every production source file, as `(relative path, text without the test module)`.
fn production_sources() -> Vec<(String, String)> {
    let mut paths = Vec::new();
    rust_sources(&manifest_dir().join("src"), &mut paths);
    assert!(
        paths.len() >= 12,
        "the scan found too few files to be meaningful: {}",
        paths.len()
    );
    paths
        .into_iter()
        .map(|path| {
            let text = read(&path);
            let production = match text.find("#[cfg(test)]") {
                Some(position) => text[..position].to_owned(),
                None => text,
            };
            let relative = path
                .strip_prefix(manifest_dir())
                .unwrap_or(&path)
                .display()
                .to_string();
            (relative, production)
        })
        .collect()
}

fn config_source() -> String {
    read(
        &manifest_dir()
            .join("src")
            .join("providers")
            .join("config.rs"),
    )
}

#[test]
fn the_curve_matches_g13() {
    assert_eq!(BACKOFF_BASE, Duration::from_secs(1), "G-13: base 1 s");
    assert_eq!(BACKOFF_CAP, Duration::from_secs(30), "G-13: cap 30 s");
}

/// Read through a function so the comparisons below are runtime checks rather than
/// constant assertions: the point is to document the relationships a later edit must keep.
fn attempt_budget_is_positive(budget: u32) -> bool {
    budget > 0
}

fn connect_fits_inside_the_request(connect: Duration, request: Duration) -> bool {
    connect <= request
}

fn probe_timeout_is_usable(timeout: Duration) -> bool {
    !timeout.is_zero()
}

#[test]
fn the_engineering_defaults_leave_a_working_request_path() {
    assert!(attempt_budget_is_positive(MAX_TRANSPORT_ATTEMPTS));
    assert!(connect_fits_inside_the_request(
        PROVIDER_CONNECT_TIMEOUT,
        PROVIDER_REQUEST_TIMEOUT
    ));
    assert!(probe_timeout_is_usable(PROBE_TIMEOUT));
}

#[test]
fn the_two_probe_targets_are_the_two_loopback_literals() {
    assert_eq!(PROBE_TARGETS.len(), 2, "FR-304 names exactly two ports");

    let expected: [ProbeTarget; 2] = [
        ProbeTarget {
            flavor: LocalFlavor::Ollama,
            base_url: "http://127.0.0.1:11434/v1",
            port: 11434,
        },
        ProbeTarget {
            flavor: LocalFlavor::LmStudio,
            base_url: "http://127.0.0.1:1234/v1",
            port: 1234,
        },
    ];
    assert_eq!(PROBE_TARGETS, expected);

    // Every target is a numeric loopback address: no `localhost`, no name resolution, no
    // path through which another host could enter (NFR-206, design.md §7.2).
    for target in PROBE_TARGETS {
        let host = target
            .base_url
            .split("://")
            .nth(1)
            .and_then(|rest| rest.split('/').next())
            .and_then(|authority| authority.split(':').next())
            .unwrap_or_default();
        assert_eq!(host, "127.0.0.1", "{}", target.base_url);
        assert!(!target.base_url.contains("localhost"));
        assert!(
            target.base_url.ends_with(&format!(":{}/v1", target.port)),
            "{} does not carry its port",
            target.base_url
        );
    }
}

#[test]
fn no_duration_conversion_literal_appears_outside_the_config_file() {
    let mut offenders = Vec::new();
    for (path, text) in production_sources() {
        if path.ends_with(&format!("providers{}config.rs", std::path::MAIN_SEPARATOR)) {
            continue;
        }
        if text.contains("Duration::from_secs(") {
            offenders.push(path);
        }
    }
    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "a duration literal is repeated outside providers/config.rs"
    );
    // Positive control: the constant file itself does use the conversion, so the scan
    // above is not passing on a tree that never mentions it.
    assert!(config_source().contains("Duration::from_secs("));
}

#[test]
fn no_probe_port_literal_appears_outside_the_config_file() {
    let mut offenders = Vec::new();
    for (path, text) in production_sources() {
        if path.ends_with(&format!("providers{}config.rs", std::path::MAIN_SEPARATOR)) {
            continue;
        }
        for needle in ["11434", "1234"] {
            if text.contains(needle) {
                offenders.push(format!("{path}: {needle}"));
            }
        }
    }
    assert_eq!(
        offenders,
        Vec::<String>::new(),
        "a probe port is repeated outside providers/config.rs"
    );
    assert!(config_source().contains("11434") && config_source().contains("1234"));
}

#[test]
fn the_keyring_service_is_one_definition_reached_through_the_config_module() {
    // Two paths, one value: `providers::config` re-exports it so a provider call site does
    // not repeat the string, while `security::keyring` keeps the definition beside the
    // entry-name derivation that uses it (design.md §3.2 rule 1: infrastructure must not
    // import services).
    assert_eq!(KEYRING_SERVICE, "aref");
    assert_eq!(
        aref_lib::security::keyring::KEYRING_SERVICE,
        KEYRING_SERVICE,
        "the re-export and the definition must be the same constant"
    );

    let mut offenders = Vec::new();
    for (path, text) in production_sources() {
        if text.contains("\"aref\"") {
            offenders.push(path);
        }
    }
    assert_eq!(
        offenders,
        vec![format!(
            "src{}security{}keyring.rs",
            std::path::MAIN_SEPARATOR,
            std::path::MAIN_SEPARATOR
        )],
        "the service name is written down more than once"
    );
}

#[test]
fn the_manifest_declares_no_dependency_the_retry_policy_forbids() {
    // 3.5 forbids `rand` (jitter is SplitMix64 in `providers/clock.rs`) and 3.7 forbids
    // `httpdate` (settled `D-2`: delta-seconds only, so a date parser has no caller).
    let manifest = read(&manifest_dir().join("Cargo.toml"));
    for forbidden in ["rand", "httpdate"] {
        assert!(
            !manifest.contains(forbidden),
            "{forbidden} is a dependency; see tasks 3.5 and 3.7"
        );
    }
}

#[test]
fn every_named_constant_the_task_lists_is_declared_in_the_config_file() {
    let source = config_source();
    for name in [
        "BACKOFF_BASE",
        "BACKOFF_CAP",
        "PROBE_TARGETS",
        "MAX_TRANSPORT_ATTEMPTS",
        "PROVIDER_CONNECT_TIMEOUT",
        "PROVIDER_REQUEST_TIMEOUT",
        "PROBE_TIMEOUT",
    ] {
        assert!(
            source.contains(&format!("pub const {name}")),
            "{name} is not declared in providers/config.rs"
        );
    }
    assert!(
        source.contains("pub use crate::security::keyring::KEYRING_SERVICE"),
        "KEYRING_SERVICE is not reachable through providers/config.rs"
    );
}
