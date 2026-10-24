//! Tasks 3.3, 3.4 (client half), 3.11 (socket half) and 3.14 — the real `ReqwestTransport`
//! against the loopback harness.
//!
//! Every destination here is `127.0.0.1` on an ephemeral port, every body is a fixture, and
//! no test contacts a provider: the harness is the only server any of these tests talks to.

mod support;

use std::time::Duration;

use aref_lib::providers::client::{ChatRequest, ReqwestTransport, Transport, TransportError};
use aref_lib::providers::config::ProbeTarget;
use aref_lib::security::redact::Secret;
use aref_lib::storage::providers_repo::{LocalFlavor, ProviderConfig, ProviderKind};
use support::{closed_port, MockServer, Response};

const KEY: &str = "fixture-secret-a";

fn cloud(base_url: &str) -> ProviderConfig {
    ProviderConfig::new(
        "provider-1",
        ProviderKind::OpenAiCompatible,
        None,
        base_url,
        "model-a",
        "Primary",
        true,
    )
}

fn keyless_local(base_url: &str) -> ProviderConfig {
    ProviderConfig::new(
        "local",
        ProviderKind::Local,
        Some(LocalFlavor::Ollama),
        base_url,
        "model-b",
        "Ollama",
        true,
    )
}

fn completion(text: &str) -> String {
    serde_json::json!({
        "choices": [{
            "message": { "role": "assistant", "content": text },
            "finish_reason": "stop"
        }]
    })
    .to_string()
}

/// A probe target pointed at the harness, with a leaked `'static` URL.
///
/// `ProbeTarget` is a `const`-shaped type (`base_url: &'static str`), so a test that points
/// one at an ephemeral port has to leak that URL. Small and bounded: one string per test.
fn target(server: &MockServer) -> ProbeTarget {
    let base_url: &'static str = Box::leak(server.base_url().into_boxed_str());
    ProbeTarget {
        flavor: LocalFlavor::Ollama,
        base_url,
        port: server.port(),
    }
}

fn json_body(text: &str) -> serde_json::Value {
    match serde_json::from_str(text) {
        Ok(value) => value,
        Err(error) => panic!("the request body is not JSON: {error}"),
    }
}

// ------------------------------------------------------------------ 3.3 the request

#[tokio::test]
async fn a_request_carries_the_model_from_the_entry_and_no_credential_when_keyless() {
    let server = MockServer::start([Response::json(200, completion("rewritten"))]);
    let transport = ReqwestTransport::new();

    let response = transport
        .send(
            &cloud(&server.base_url()),
            None,
            &ChatRequest::user("fixture text"),
        )
        .await;

    let response = match response {
        Ok(response) => response,
        Err(error) => panic!("the harness should have answered: {error:?}"),
    };
    assert_eq!(response.status, 200);
    assert!(response.body.contains("rewritten"));

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].target, "/v1/chat/completions");
    // G-09: no models route, ever.
    assert!(!requests[0].target.contains("/models"));
    // FR-303: a keyless provider sends no Authorization header at all, not an empty one.
    assert!(!requests[0].has_header("Authorization"));

    let body = json_body(&requests[0].body);
    assert_eq!(body["model"], serde_json::json!("model-a"));
    assert_eq!(
        body["messages"][0]["content"],
        serde_json::json!("fixture text")
    );
    assert_eq!(body["stream"], serde_json::json!(false));
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.target.contains("/models"))
            .count(),
        0,
        "no model-discovery request was issued"
    );
}

#[tokio::test]
async fn only_the_chat_completions_path_is_ever_requested() {
    // The same provider answered twice: the URL is the one format string in `client.rs`.
    let server = MockServer::start([
        Response::json(200, completion("first")),
        Response::json(200, completion("second")),
    ]);
    let transport = ReqwestTransport::new();
    let provider = cloud(&server.base_url());

    let _ = transport
        .send(&provider, None, &ChatRequest::user("a"))
        .await;
    let _ = transport
        .send(&provider, None, &ChatRequest::user("b"))
        .await;

    let targets: Vec<String> = server
        .requests()
        .into_iter()
        .map(|request| request.target)
        .collect();
    assert_eq!(
        targets,
        vec!["/v1/chat/completions", "/v1/chat/completions"]
    );
}

#[tokio::test]
async fn a_configured_key_is_sent_as_a_bearer_header() {
    let server = MockServer::start([Response::json(200, completion("ok"))]);
    let transport = ReqwestTransport::new();
    let secret = Secret::new(KEY);

    let _ = transport
        .send(
            &cloud(&server.base_url()),
            Some(&secret),
            &ChatRequest::user("x"),
        )
        .await;

    let authorization = server.requests()[0]
        .header("authorization")
        .map(str::to_owned);
    assert_eq!(authorization.as_deref(), Some("Bearer fixture-secret-a"));
}

// ------------------------------------------------------------ 3.4 lazy construction

#[test]
fn constructing_the_transport_builds_no_client() {
    let transport = ReqwestTransport::new();
    assert_eq!(
        transport.client_builds(),
        0,
        "a Client at construction would be a socket-builder in the composition root"
    );
}

#[tokio::test]
async fn the_first_request_builds_one_client_and_the_second_reuses_it() {
    let server = MockServer::start([
        Response::json(200, completion("first")),
        Response::json(200, completion("second")),
    ]);
    let transport = ReqwestTransport::new();
    let provider = cloud(&server.base_url());

    let _ = transport
        .send(&provider, None, &ChatRequest::user("a"))
        .await;
    assert_eq!(transport.client_builds(), 1, "one client, on first use");

    let _ = transport
        .send(&provider, None, &ChatRequest::user("b"))
        .await;
    assert_eq!(transport.client_builds(), 1, "reused, not rebuilt");
}

// ------------------------------------------------------- 3.11 the probe, over a socket

#[tokio::test]
async fn the_probe_is_a_credential_free_get_to_the_models_route() {
    let server = MockServer::start([Response::json(200, "{}")]);
    let transport = ReqwestTransport::new();

    let reachable = transport.probe(&target(&server)).await;

    assert_eq!(reachable, Ok(true));
    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].target, "/v1/models");
    assert!(!requests[0].has_header("Authorization"));
    assert_eq!(
        server.request_count(),
        1,
        "no model list was requested twice"
    );
}

#[tokio::test]
async fn a_redirect_is_not_followed() {
    // Two responses scripted: if the 302 were followed, the second would be served too.
    let server = MockServer::start([
        Response::empty(302).with_header("Location", "http://127.0.0.1:1/v1/models"),
        Response::json(200, "{}"),
    ]);
    let transport = ReqwestTransport::new();

    let reachable = transport.probe(&target(&server)).await;

    assert_eq!(reachable, Ok(false), "a 3xx is not a hit");
    assert_eq!(
        server.request_count(),
        1,
        "the redirect target was never requested"
    );
}

#[tokio::test]
async fn a_probe_that_hangs_hits_its_own_short_timeout() {
    let server = MockServer::start([Response::hang()]);
    let transport = ReqwestTransport::with_timeouts(
        Duration::from_millis(200),
        Duration::from_millis(200),
        Duration::from_millis(50),
    );

    let outcome = transport.probe(&target(&server)).await;

    assert_eq!(outcome, Err(TransportError::Timeout));
}

// ------------------------------------------- 3.14 classification at the transport edge

#[tokio::test]
async fn a_refused_connection_is_a_drop() {
    let address = closed_port();
    // Generous budgets: a fresh client's first request costs real setup time, and a refusal
    // must reach the classifier as a connect error rather than as an expired timeout.
    let transport = ReqwestTransport::with_timeouts(
        Duration::from_secs(2),
        Duration::from_secs(5),
        Duration::from_secs(1),
    );

    let outcome = transport
        .send(
            &cloud(&format!("http://{address}/v1")),
            None,
            &ChatRequest::user("x"),
        )
        .await;

    assert_eq!(outcome, Err(TransportError::Dropped));
}

#[tokio::test]
async fn a_request_timeout_is_a_timeout() {
    let server = MockServer::start([Response::hang()]);
    let transport = ReqwestTransport::with_timeouts(
        Duration::from_millis(200),
        Duration::from_millis(50),
        Duration::from_millis(50),
    );

    let outcome = transport
        .send(&cloud(&server.base_url()), None, &ChatRequest::user("x"))
        .await;

    assert_eq!(outcome, Err(TransportError::Timeout));
}

#[tokio::test]
async fn a_body_cut_after_a_2xx_is_an_interrupted_stream() {
    let server = MockServer::start([Response::cut("{\"choices\":[")]);
    let transport = ReqwestTransport::new();

    let outcome = transport
        .send(&cloud(&server.base_url()), None, &ChatRequest::user("x"))
        .await;

    assert_eq!(
        outcome,
        Err(TransportError::Interrupted),
        "the status arrived, the rest did not"
    );
}

#[tokio::test]
async fn a_keyless_local_provider_reaches_its_configured_base_url() {
    // FR-303 end to end: no credential, and the request still goes to the entry's URL.
    let server = MockServer::start([Response::json(200, completion("local"))]);
    let transport = ReqwestTransport::new();

    let outcome = transport
        .send(
            &keyless_local(&server.base_url()),
            None,
            &ChatRequest::user("x"),
        )
        .await;

    assert!(matches!(outcome, Ok(response) if response.status == 200));
    assert!(!server.requests()[0].has_header("Authorization"));
    assert_eq!(server.requests()[0].target, "/v1/chat/completions");
}
