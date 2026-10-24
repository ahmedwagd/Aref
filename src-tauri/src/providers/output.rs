//! Response validation: turning a 2xx body into text or a failed attempt
//! (`FR-309`; task 3.13).
//!
//! Four outcomes, and none of them returns text except the success one:
//!
//! | Marker in the body | Result |
//! |---|---|
//! | `finish_reason: "content_filter"` | `E_OUTPUT_REJECTED { reason: "refused" }` (`D-5`) |
//! | `finish_reason: "length"` | `E_OUTPUT_REJECTED { reason: "truncated" }` |
//! | no assistant content, or blank content | `E_OUTPUT_REJECTED { reason: "empty" }` |
//! | usable content | `Ok(content)` |
//!
//! **The marker is read before the content is examined.** A filtered response normally
//! arrives with null or empty content, so an empty-output check that ran first would claim
//! it and report `empty` for a response the protocol has already labelled as a refusal.
//! The label wins, and the content of a refused response is never returned — not as the
//! rewrite, not in an error, not in an event (`NFR-201`, `NFR-205`).
//!
//! What is deliberately not here: a free-text refusal carrying no protocol marker. A 200
//! with `finish_reason: "stop"` whose prose declines is indistinguishable from ordinary
//! output, and the settled definition in `design.md` §4.7 keeps that out of scope.
//! Wrong-language output is out of scope too: `G-15` is open.

use serde_json::Value;

use crate::error::{AppError, OutputRejection};

/// The one reason a 2xx body fails here, plus the `length` and `content_filter` markers.
const MARKER_CONTENT_FILTER: &str = "content_filter";
const MARKER_LENGTH: &str = "length";

/// Reads a chat-completions body.
///
/// The body is the transport's text; the status is not an argument because every caller
/// reaches this function only after a 2xx check.
pub fn validate(body: &str) -> Result<String, AppError> {
    if body.trim().is_empty() {
        return Err(rejected(OutputRejection::Empty));
    }

    // A body that is not JSON at all is a body that was cut: `E_STREAM_INTERRUPTED` is
    // the §7 row for "the response stream was interrupted", and a truncated body is the
    // one failure a 2xx status cannot rule out.
    let parsed: Value = serde_json::from_str(body).map_err(|_| AppError::StreamInterrupted)?;

    let choice = parsed.get("choices").and_then(|choices| choices.get(0));
    let finish_reason = choice
        .and_then(|choice| choice.get("finish_reason"))
        .and_then(Value::as_str);

    // `D-5`: the marker first, so a refusal carrying text is never mistaken for output
    // and never mistaken for empty.
    if finish_reason == Some(MARKER_CONTENT_FILTER) {
        return Err(rejected(OutputRejection::Refused));
    }
    if finish_reason == Some(MARKER_LENGTH) {
        return Err(rejected(OutputRejection::Truncated));
    }

    let content = choice
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    if content.trim().is_empty() {
        return Err(rejected(OutputRejection::Empty));
    }
    Ok(content.to_owned())
}

fn rejected(reason: OutputRejection) -> AppError {
    AppError::OutputRejected { reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The `reason` the UI receives, read from the serialized envelope.
    fn reason(error: &AppError) -> String {
        let envelope = match serde_json::to_value(error) {
            Ok(value) => value,
            Err(error) => panic!("the envelope did not serialize: {error}"),
        };
        match envelope
            .get("params")
            .and_then(|params| params.get("reason"))
            .and_then(Value::as_str)
        {
            Some(reason) => reason.to_owned(),
            None => panic!("no string reason in {envelope}"),
        }
    }

    fn expect_rejection(outcome: Result<String, AppError>) -> (String, String) {
        match outcome {
            Ok(text) => panic!("expected a rejection, got {text:?}"),
            Err(error) => {
                assert_eq!(error.code(), "E_OUTPUT_REJECTED");
                (error.code().to_owned(), reason(&error))
            }
        }
    }

    #[test]
    fn a_marked_refusal_is_refused_even_when_it_carries_content() {
        // The settled `D-5` fixture: 200, non-empty assistant content, `content_filter`.
        let body = json!({
            "choices": [{
                "message": { "role": "assistant", "content": "here is a rewrite I will not give" },
                "finish_reason": "content_filter"
            }]
        })
        .to_string();

        let (code, reason) = expect_rejection(validate(&body));
        assert_eq!(code, "E_OUTPUT_REJECTED");
        assert_eq!(reason, "refused", "the marker wins over the content");
    }

    #[test]
    fn a_marked_refusal_with_empty_content_is_still_refused_not_empty() {
        let body = json!({
            "choices": [{
                "message": { "role": "assistant", "content": "" },
                "finish_reason": "content_filter"
            }]
        })
        .to_string();
        assert_eq!(expect_rejection(validate(&body)).1, "refused");

        let null_content = json!({
            "choices": [{
                "message": { "role": "assistant", "content": null },
                "finish_reason": "content_filter"
            }]
        })
        .to_string();
        assert_eq!(expect_rejection(validate(&null_content)).1, "refused");
    }

    #[test]
    fn no_text_of_a_refused_response_is_returned_anywhere() {
        // `FR-309`: the refusal is not the rewrite, and no part of it leaves this
        // function — the error carries the enum value only.
        let body = json!({
            "choices": [{
                "message": { "role": "assistant", "content": "fixture refusal text" },
                "finish_reason": "content_filter"
            }]
        })
        .to_string();
        let error = match validate(&body) {
            Ok(text) => panic!("returned {text:?}"),
            Err(error) => error,
        };
        let envelope = match serde_json::to_value(&error) {
            Ok(value) => value.to_string(),
            Err(error) => panic!("the envelope did not serialize: {error}"),
        };
        assert!(!envelope.contains("fixture refusal text"));
        assert!(!envelope.contains("refusal"));
    }

    #[test]
    fn a_length_finish_reason_is_truncated() {
        let body = json!({
            "choices": [{
                "message": { "role": "assistant", "content": "a rewrite that stops mid" },
                "finish_reason": "length"
            }]
        })
        .to_string();
        assert_eq!(expect_rejection(validate(&body)).1, "truncated");
    }

    #[test]
    fn empty_bodies_and_absent_or_blank_content_are_empty() {
        for body in [
            "",
            "   ",
            "{}",
            r#"{"choices": []}"#,
            r#"{"choices": [{"message": {"content": ""}, "finish_reason": "stop"}]}"#,
            r#"{"choices": [{"message": {"content": "   "}, "finish_reason": "stop"}]}"#,
            r#"{"choices": [{"finish_reason": "stop"}]}"#,
            r#"{"choices": [{"message": {"content": null}, "finish_reason": "stop"}]}"#,
        ] {
            assert_eq!(
                expect_rejection(validate(body)).1,
                "empty",
                "body = {body:?}"
            );
        }
    }

    #[test]
    fn a_cut_body_is_an_interrupted_stream() {
        for body in [r#"{"choices": [{"message": {"content": "abc"#, "[1, 2"] {
            let error = match validate(body) {
                Ok(text) => panic!("returned {text:?}"),
                Err(error) => error,
            };
            assert_eq!(error.code(), "E_STREAM_INTERRUPTED");
        }
    }

    #[test]
    fn usable_content_is_returned_unchanged() {
        let body = json!({
            "choices": [{
                "message": { "role": "assistant", "content": "  نصّ مُعاد كتابته  " },
                "finish_reason": "stop"
            }]
        })
        .to_string();
        assert_eq!(
            validate(&body),
            Ok("  نصّ مُعاد كتابته  ".to_owned()),
            "the rewrite is handed over verbatim; trimming is the caller's decision"
        );
    }

    #[test]
    fn every_rejection_is_one_of_the_four_catalog_reasons() {
        // The four `E_OUTPUT_REJECTED` reasons come from SRS §7; this task exercises three
        // of them and never invents a fifth.
        let reasons: Vec<&str> = [
            OutputRejection::Empty,
            OutputRejection::Truncated,
            OutputRejection::Refused,
            OutputRejection::WrongLanguage,
        ]
        .iter()
        .map(|reason| reason.as_str())
        .collect();
        assert_eq!(reasons.len(), 4);
        for produced in ["empty", "truncated", "refused"] {
            assert!(reasons.contains(&produced), "{produced} is not catalogued");
        }
    }
}
