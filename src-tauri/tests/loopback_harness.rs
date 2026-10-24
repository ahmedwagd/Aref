//! Task 3.2 — the loopback mock harness, and the dependency it does not add.
//!
//! The harness lives in `tests/support/mod.rs` and uses only `std`: a `TcpListener` on
//! `127.0.0.1:0`, a scripted status line, headers and body, and a request log. This file
//! proves it works by driving one request through it, and proves the dependency decision
//! by reading the manifest.

mod support;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;

use support::{MockServer, Response};

fn manifest() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => panic!("cannot read {}: {error}", path.display()),
    }
}

fn send(server: &MockServer, request: &str) -> String {
    let mut stream = match TcpStream::connect(server.address()) {
        Ok(stream) => stream,
        Err(error) => panic!("cannot connect to the harness: {error}"),
    };
    if let Err(error) = stream.write_all(request.as_bytes()) {
        panic!("cannot write the request: {error}");
    }
    let mut response = String::new();
    if let Err(error) = stream.read_to_string(&mut response) {
        panic!("cannot read the response: {error}");
    }
    response
}

#[test]
fn one_request_travels_through_the_harness_and_is_recorded() {
    let server = MockServer::start([
        Response::json(200, r#"{"choices":[]}"#).with_header("Retry-After", "120")
    ]);

    assert!(server.port() > 0, "an ephemeral port was bound");
    assert!(server.address().ip().is_loopback());
    assert_eq!(server.base_url(), format!("http://{}/v1", server.address()));

    let body = r#"{"model":"model-a","messages":[]}"#;
    let response = send(
        &server,
        &format!(
            "POST /v1/chat/completions HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            server.address(),
            body.len()
        ),
    );

    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("Retry-After: 120\r\n"));
    assert!(response.contains(&format!("Content-Length: {}", r#"{"choices":[]}"#.len())));
    assert!(response.ends_with(r#"{"choices":[]}"#));

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].target, "/v1/chat/completions");
    assert_eq!(requests[0].version, "HTTP/1.1");
    assert_eq!(requests[0].header("content-type"), Some("application/json"));
    assert!(!requests[0].has_header("Authorization"));
    assert_eq!(requests[0].body, body);
    assert_eq!(server.request_count(), 1);
}

#[test]
fn the_script_is_answered_in_order() {
    let server = MockServer::start([Response::empty(401), Response::json(200, "second")]);

    let first = send(&server, "GET /one HTTP/1.1\r\nHost: x\r\n\r\n");
    let second = send(&server, "GET /two HTTP/1.1\r\nHost: x\r\n\r\n");

    assert!(first.starts_with("HTTP/1.1 401 Unauthorized"));
    assert!(second.starts_with("HTTP/1.1 200 OK"));
    let targets: Vec<String> = server
        .requests()
        .into_iter()
        .map(|request| request.target)
        .collect();
    assert_eq!(targets, vec!["/one", "/two"]);
}

#[test]
fn a_header_value_is_read_case_insensitively() {
    let server = MockServer::start([Response::empty(200)]);
    let _ = send(
        &server,
        "POST /k HTTP/1.1\r\nHost: x\r\nauthorization: Bearer fixture\r\nContent-Length: 0\r\n\r\n",
    );
    let requests = server.requests();
    assert_eq!(requests[0].header("Authorization"), Some("Bearer fixture"));
    assert!(requests[0].has_header("AUTHORIZATION"));
}

#[test]
fn the_manifest_declares_no_mock_server_dependency() {
    let manifest = manifest();
    for forbidden in ["wiremock", "mockito", "httpmock"] {
        assert!(
            !manifest.contains(forbidden),
            "{forbidden} is a dependency; the harness is the ~80-line std one (§10)"
        );
    }
}
