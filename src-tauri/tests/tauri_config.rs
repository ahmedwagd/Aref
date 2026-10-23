//! The bundle is Windows-only, the window geometry is the approved baseline, and no
//! signing step exists (task 1.3, `NFR-307`, `NFR-308`, `AGENTS.md` §3.8/§3.9).

use std::path::{Path, PathBuf};

use serde_json::Value;

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn config() -> Value {
    let path = manifest_dir().join("tauri.conf.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => panic!("cannot read tauri.conf.json: {error}"),
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => panic!("tauri.conf.json is not valid JSON: {error}"),
    }
}

fn window<'a>(config: &'a Value, key: &str) -> &'a Value {
    let value = config
        .pointer("/app/windows/0")
        .and_then(|windows| windows.get(key));
    match value {
        Some(value) => value,
        None => panic!("app.windows[0].{key} is missing"),
    }
}

fn collect_keys<'a>(value: &'a Value, keys: &mut Vec<&'a str>) {
    match value {
        Value::Object(object) => {
            for (key, inner) in object {
                keys.push(key.as_str());
                collect_keys(inner, keys);
            }
        }
        Value::Array(entries) => entries.iter().for_each(|entry| collect_keys(entry, keys)),
        _ => {}
    }
}

#[test]
fn the_bundle_targets_windows_only() {
    let config = config();
    let targets = match config.pointer("/bundle/targets") {
        Some(Value::Array(targets)) => targets,
        _ => panic!("bundle.targets must be an array"),
    };
    let names: Vec<&str> = targets.iter().filter_map(Value::as_str).collect();
    assert_eq!(names, vec!["nsis"], "only the Windows installer target");
    assert!(!names.iter().any(|name| name.contains("dmg")));
    assert!(!names.iter().any(|name| name.contains("appimage")));
    assert!(!names.iter().any(|name| name.contains("deb")));
}

#[test]
fn the_window_matches_the_approved_geometry() {
    let config = config();
    assert_eq!(window(&config, "width").as_u64(), Some(1240));
    assert_eq!(window(&config, "height").as_u64(), Some(820));
    assert_eq!(window(&config, "minWidth").as_u64(), Some(1000));
    assert_eq!(window(&config, "minHeight").as_u64(), Some(680));
    assert_eq!(window(&config, "center").as_bool(), Some(true));
}

#[test]
fn no_signing_or_certificate_step_is_configured() {
    let config = config();
    let mut keys = Vec::new();
    collect_keys(&config, &mut keys);
    let forbidden = [
        "sign",
        "signing",
        "signingIdentity",
        "certificate",
        "authenticode",
    ];
    for key in keys {
        let lowered = key.to_lowercase();
        assert!(
            !forbidden.iter().any(|needle| lowered.contains(needle)),
            "signing-related key `{key}` is present"
        );
    }
}

#[test]
fn no_workflow_declares_a_non_windows_job() {
    let workflows = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.github/workflows");
    if !workflows.exists() {
        // No workflow file exists, which satisfies the check trivially and keeps the
        // assertion meaningful the day one is added.
        return;
    }
    let entries = match std::fs::read_dir(&workflows) {
        Ok(entries) => entries,
        Err(error) => panic!("cannot read workflows: {error}"),
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => panic!("cannot read a workflow entry: {error}"),
        };
        let path = entry.path();
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(_) => continue,
        };
        for line in text.lines() {
            if let Some(rest) = line.trim().strip_prefix("runs-on:") {
                assert!(
                    rest.to_lowercase().contains("windows"),
                    "{} declares a non-Windows runner: {line}",
                    path.display()
                );
            }
        }
    }
}
