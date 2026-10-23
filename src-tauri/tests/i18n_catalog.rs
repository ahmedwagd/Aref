//! `src/i18n/` is the contract between the core and the UI: every `message_key` the
//! core can emit must exist in both catalogs (task 1.7, `NFR-401`).

use std::collections::BTreeSet;
use std::path::PathBuf;

use aref_lib::error::AppError;
use serde_json::{Map, Value};

fn i18n_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/i18n")
}

fn read_json(name: &str) -> Value {
    let path = i18n_dir().join(name);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => panic!("cannot read {name}: {error}"),
    };
    match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => panic!("{name} is not valid JSON: {error}"),
    }
}

fn object<'a>(value: &'a Value, name: &str) -> &'a Map<String, Value> {
    match value.as_object() {
        Some(object) => object,
        None => panic!("{name} must be a JSON object"),
    }
}

#[test]
fn error_keys_json_lists_exactly_the_catalog() -> Result<(), String> {
    let listed: BTreeSet<String> = match read_json("error-keys.json") {
        Value::Array(entries) => entries
            .iter()
            .map(|entry| match entry.as_str() {
                Some(key) => Ok(key.to_owned()),
                None => Err("error-keys.json holds a non-string entry".to_owned()),
            })
            .collect::<Result<_, _>>()?,
        _ => return Err("error-keys.json must be a JSON array".to_owned()),
    };

    let catalogued: BTreeSet<String> = AppError::catalog()
        .iter()
        .map(|(_, message_key)| (*message_key).to_owned())
        .collect();

    assert_eq!(
        listed, catalogued,
        "error-keys.json and AppError must agree"
    );
    Ok(())
}

#[test]
fn both_catalogs_carry_every_catalog_key() {
    for locale in ["ar.json", "en.json"] {
        let value = read_json(locale);
        let catalog = object(&value, locale);
        for (_, message_key) in AppError::catalog() {
            let entry = catalog.get(*message_key);
            let text = entry.and_then(Value::as_str).unwrap_or("");
            assert!(!text.trim().is_empty(), "{locale} is missing {message_key}");
        }
    }
}

#[test]
fn catalogs_have_key_parity() {
    let ar = read_json("ar.json");
    let en = read_json("en.json");
    let ar_keys: BTreeSet<&String> = object(&ar, "ar.json").keys().collect();
    let en_keys: BTreeSet<&String> = object(&en, "en.json").keys().collect();
    assert_eq!(ar_keys, en_keys, "ar and en key sets must match");
}

#[test]
fn error_input_empty_reads_without_interpolating_field() {
    // D-1: `params.field` only decides which input carries the message.
    for locale in ["ar.json", "en.json"] {
        let value = read_json(locale);
        let catalog = object(&value, locale);
        let message = catalog
            .get("error.input.empty")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(!message.contains('{'), "{locale} has a placeholder");
        assert!(!message.contains("${"), "{locale} has a template token");
        assert!(message.ends_with('.'), "{locale} reads as a fragment");
    }
}
