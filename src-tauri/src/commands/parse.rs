//! Parsing turns a raw `InvokeBody` into a typed request.
//!
//! Typed command arguments cannot serve `NFR-306`: Tauri deserializes them before the
//! handler runs and rejects with a plain string, which is not an error envelope. So the
//! six field-taking commands accept the raw body and parse it here (design.md §4.5).
//!
//! Two distinct failures, deliberately kept apart:
//! - a **missing or blank required field** is `E_INPUT_EMPTY { field }` (`D-1`);
//! - a **wrongly typed field or a non-object body** is `E_INTERNAL`.

use serde::de::DeserializeOwned;
use tauri::ipc::InvokeBody;

use crate::error::AppError;

/// Deserialize a command body into its DTO.
///
/// Every DTO field is `Option<T>`, so this never rejects an absent field — presence is
/// validated afterwards. Only a body that is not JSON, not an object, or a field of the
/// wrong type fails here.
pub fn parse_body<T: DeserializeOwned>(body: &InvokeBody) -> Result<T, AppError> {
    match body {
        InvokeBody::Json(value) => {
            serde_json::from_value(value.clone()).map_err(|_| AppError::internal())
        }
        // A raw body carries no field names, so it cannot be validated field by field.
        InvokeBody::Raw(_) => Err(AppError::internal()),
    }
}

/// `parse → validate → one service call`, in one place, so a handler stays three lines
/// (design.md §3.2 rule 2).
pub fn handle_body<T, R>(
    body: &InvokeBody,
    run: impl FnOnce(T) -> Result<R, AppError>,
) -> Result<R, AppError>
where
    T: DeserializeOwned,
{
    let request: T = parse_body(body)?;
    run(request)
}

/// A required text field. `field` is the DTO field name, which is what the UI receives
/// in `params.field` so it can attach the message to the right input (design.md §4.6).
///
/// The value is returned unchanged: trimming here would silently alter a secret.
pub fn required_text(field: &'static str, value: Option<&str>) -> Result<String, AppError> {
    match value {
        Some(text) if !text.trim().is_empty() => Ok(text.to_owned()),
        _ => Err(AppError::InputEmpty { field }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Debug, Deserialize)]
    struct SampleRequest {
        provider_id: Option<String>,
        label: Option<String>,
        secret: Option<String>,
    }

    impl SampleRequest {
        const REQUIRED: [&'static str; 3] = ["provider_id", "label", "secret"];

        fn validate(&self) -> Result<(), AppError> {
            required_text("provider_id", self.provider_id.as_deref())?;
            required_text("label", self.label.as_deref())?;
            required_text("secret", self.secret.as_deref())?;
            Ok(())
        }
    }

    fn valid_body() -> InvokeBody {
        InvokeBody::Json(json!({
            "provider_id": "provider-1",
            "label": "personal",
            "secret": "fixture-secret-a"
        }))
    }

    #[test]
    fn a_malformed_body_returns_the_same_code_twice_and_the_next_valid_call_succeeds() {
        let malformed = InvokeBody::Json(json!(42));
        let run = |body: &InvokeBody| {
            handle_body::<SampleRequest, ()>(body, |request| {
                request.validate()?;
                Ok(())
            })
        };

        let first = run(&malformed);
        let second = run(&malformed);
        assert_eq!(
            first.as_ref().err().map(AppError::code),
            Some("E_INTERNAL"),
            "first malformed call"
        );
        assert_eq!(
            second.as_ref().err().map(AppError::code),
            Some("E_INTERNAL"),
            "second malformed call"
        );

        // The same handler still answers a valid call: no state was poisoned.
        assert!(run(&valid_body()).is_ok());
    }

    #[test]
    fn a_wrongly_typed_field_is_internal_not_input_empty() {
        let wrong_type = InvokeBody::Json(json!({
            "provider_id": 7,
            "label": "personal",
            "secret": "fixture-secret-a"
        }));
        let result = parse_body::<SampleRequest>(&wrong_type);
        assert_eq!(result.err().map(|error| error.code()), Some("E_INTERNAL"));
    }

    #[test]
    fn each_missing_field_names_itself() {
        for field in SampleRequest::REQUIRED {
            let mut body = json!({
                "provider_id": "provider-1",
                "label": "personal",
                "secret": "fixture-secret-a"
            });
            let object = match body.as_object_mut() {
                Some(object) => object,
                None => panic!("fixture is not an object"),
            };
            object.remove(field);

            let request = match parse_body::<SampleRequest>(&InvokeBody::Json(body)) {
                Ok(request) => request,
                Err(error) => panic!("parse failed for {field}: {}", error.code()),
            };
            let error = match request.validate() {
                Ok(()) => panic!("{field} was expected to be required"),
                Err(error) => error,
            };
            assert_eq!(error.code(), "E_INPUT_EMPTY");
            assert_eq!(
                error.params().get("field"),
                Some(&crate::error::ParamValue::Text(field.to_owned())),
                "{field} must travel in params.field"
            );
        }
    }

    #[test]
    fn a_blank_field_is_empty_and_the_value_is_not_trimmed() {
        let blank = required_text("label", Some("   "));
        assert_eq!(blank.err().map(|error| error.code()), Some("E_INPUT_EMPTY"));

        let padded = json!({
            "provider_id": "provider-1",
            "label": " personal ",
            "secret": "fixture-secret-a"
        });
        let request = match parse_body::<SampleRequest>(&InvokeBody::Json(padded)) {
            Ok(request) => request,
            Err(error) => panic!("parse failed: {}", error.code()),
        };
        assert_eq!(request.label.as_deref(), Some(" personal "));
    }

    #[test]
    fn a_raw_body_is_internal() {
        let raw = InvokeBody::Raw(vec![0xff, 0x00]);
        assert_eq!(
            parse_body::<SampleRequest>(&raw)
                .err()
                .map(|error| error.code()),
            Some("E_INTERNAL")
        );
    }
}
