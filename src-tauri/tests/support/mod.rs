//! A loopback HTTP/1.1 mock server, with no dependency beyond `std` (task 3.2).
//!
//! `design.md` §10 asks for a ~80-line `TcpListener` harness rather than `wiremock`: the
//! tests need a scripted status line, a couple of headers, a body and a request log, and
//! that is less machinery than a mock-server dependency would be. `wiremock` stays the
//! documented alternative if chunked encoding or TLS is ever needed — that would be a
//! dependency request, not a decision taken here.
//!
//! Determinism: the listener binds `127.0.0.1:0`, so the port is never fixed and two test
//! binaries cannot collide. Nothing resolves a name, nothing leaves loopback, and the
//! script is answered in order, so a failing test is always a real regression.
//!
//! Each connection consumes one scripted response and produces one [`RecordedRequest`].
//! The serving thread ends when the script is exhausted, which closes the port; a test
//! that sends fewer requests than it scripted leaves the thread blocked on `accept`, which
//! the test process's exit reclaims.

#![allow(dead_code)]
// Each test binary that declares `mod support;` compiles its own copy, so a binary that
// uses only part of the harness would otherwise warn about the rest.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use aref_lib::providers::client::{ChatRequest, ChatResponse, Transport, TransportError};
use aref_lib::providers::config::ProbeTarget;
use aref_lib::providers::events::{EventSink, KeyInvalid, RateLimited};
use aref_lib::security::keyring::{KeyId, KeyStore, KeyStoreError};
use aref_lib::security::redact::Secret;
use aref_lib::storage::providers_repo::ProviderConfig;

/// One scripted answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    /// When set, `Content-Length` announces this many bytes while only `body` is written, so
    /// the client observes a stream that was cut after the status line (task 3.14).
    pub declared_length: Option<usize>,
    /// When set, the server reads the request and then closes without writing anything, after
    /// pausing this long: the client's timeout fires first. Keep the pause well above the
    /// client's timeout so the test is not a race.
    pub hang: Option<Duration>,
}

impl Response {
    /// A JSON response with the content type a provider would send.
    pub fn json(status: u16, body: impl Into<String>) -> Self {
        Self {
            status,
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: body.into(),
            declared_length: None,
            hang: None,
        }
    }

    /// A response with no body, for a status-only case.
    pub fn empty(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: String::new(),
            declared_length: None,
            hang: None,
        }
    }

    /// A 200 whose body is cut short of its announced length.
    pub fn cut(body: impl Into<String>) -> Self {
        let body = body.into();
        Self {
            status: 200,
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            declared_length: Some(body.len() + 64),
            body,
            hang: None,
        }
    }

    /// Accepts the request and never answers, so the client's own timeout decides.
    pub fn hang() -> Self {
        Self {
            status: 200,
            headers: Vec::new(),
            body: String::new(),
            declared_length: None,
            hang: Some(Duration::from_millis(300)),
        }
    }

    /// Adds a header, which is how a `Retry-After` or a `Location` is scripted.
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

/// One request the server received.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedRequest {
    pub method: String,
    pub target: String,
    pub version: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl RecordedRequest {
    /// A header value, matched case-insensitively, because HTTP field names are.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Whether a header is present at all — the shape "a keyless provider sends no
    /// `Authorization`" is asserted with this.
    pub fn has_header(&self, name: &str) -> bool {
        self.header(name).is_some()
    }
}

/// A loopback mock server with a fixed script.
pub struct MockServer {
    address: SocketAddr,
    requests: Arc<Mutex<Vec<RecordedRequest>>>,
    /// Detached on drop: joining could block forever on a test that failed early.
    _thread: JoinHandle<()>,
}

impl MockServer {
    /// Binds `127.0.0.1:0` and starts serving `responses` in order.
    pub fn start(responses: impl IntoIterator<Item = Response>) -> Self {
        let script: Vec<Response> = responses.into_iter().collect();
        let listener = match TcpListener::bind(("127.0.0.1", 0)) {
            Ok(listener) => listener,
            Err(error) => panic!("cannot bind a loopback listener: {error}"),
        };
        let address = match listener.local_addr() {
            Ok(address) => address,
            Err(error) => panic!("cannot read the bound address: {error}"),
        };
        let requests = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&requests);

        let thread = thread::spawn(move || {
            for response in script {
                let (stream, _peer) = match listener.accept() {
                    Ok(accepted) => accepted,
                    Err(_) => return,
                };
                let mut writer = stream;
                let reader = match writer.try_clone() {
                    Ok(handle) => BufReader::new(handle),
                    Err(_) => return,
                };
                let request = match read_request(reader) {
                    Ok(request) => request,
                    Err(_) => return,
                };
                // Recorded before the answer is written, so a test that awaited the client
                // can assert the log without racing the serving thread.
                match recorded.lock() {
                    Ok(mut requests) => requests.push(request),
                    Err(poison) => poison.into_inner().push(request),
                }
                if let Some(pause) = response.hang {
                    thread::sleep(pause);
                    continue;
                }
                if write_response(&mut writer, &response).is_err() {
                    return;
                }
            }
        });

        Self {
            address,
            requests,
            _thread: thread,
        }
    }

    /// The bound loopback address, scripted with the ephemeral port.
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn port(&self) -> u16 {
        self.address.port()
    }

    /// `http://127.0.0.1:<port>/v1`, the shape a provider entry's `base_url` has (`R-6`).
    pub fn base_url(&self) -> String {
        format!("http://{}/v1", self.address)
    }

    /// Every request received so far, in arrival order.
    pub fn requests(&self) -> Vec<RecordedRequest> {
        match self.requests.lock() {
            Ok(requests) => requests.clone(),
            Err(poison) => poison.into_inner().clone(),
        }
    }

    pub fn request_count(&self) -> usize {
        match self.requests.lock() {
            Ok(requests) => requests.len(),
            Err(poison) => poison.into_inner().len(),
        }
    }
}

/// Reads one request from a connection.
fn read_request(mut reader: BufReader<TcpStream>) -> std::io::Result<RecordedRequest> {
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_owned();
    let target = parts.next().unwrap_or_default().to_owned();
    let version = parts.next().unwrap_or_default().to_owned();

    let mut headers = Vec::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            break;
        }
        let trimmed = header.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some((name, value)) = trimmed.split_once(':') {
            headers.push((name.trim().to_owned(), value.trim().to_owned()));
        }
    }

    let length = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-length"))
        .and_then(|(_, value)| value.parse::<usize>().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; length];
    if length > 0 {
        reader.read_exact(&mut body)?;
    }

    Ok(RecordedRequest {
        method,
        target,
        version,
        headers,
        body: String::from_utf8_lossy(&body).into_owned(),
    })
}

fn write_response(stream: &mut TcpStream, response: &Response) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status,
        reason(response.status)
    );
    for (name, value) in &response.headers {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    let declared = response.declared_length.unwrap_or(response.body.len());
    head.push_str(&format!("Content-Length: {declared}\r\n"));
    head.push_str("Connection: close\r\n\r\n");
    stream.write_all(head.as_bytes())?;
    // A `cut` response writes fewer bytes than it announced, then closes, which is what the
    // client observes as an interrupted stream.
    stream.write_all(response.body.as_bytes())?;
    stream.flush()
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        302 => "Found",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "Unknown",
    }
}

/// A loopback address with nothing listening on it: a deterministic refused connection.
///
/// An ephemeral port that is bound and immediately released is refused as soon as the
/// listener closes. A *fixed* low port is not a safe substitute: on this host a connect to
/// `127.0.0.1:1` takes about two seconds to be refused (a filtering layer intervenes on low
/// ports), which fires any timeout the caller armed and turns a refusal into a timeout.
pub fn closed_port() -> SocketAddr {
    let listener = match TcpListener::bind(("127.0.0.1", 0)) {
        Ok(listener) => listener,
        Err(error) => panic!("cannot bind a loopback listener: {error}"),
    };
    let address = match listener.local_addr() {
        Ok(address) => address,
        Err(error) => panic!("cannot read the bound address: {error}"),
    };
    drop(listener);
    address
}

static TEMP_SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// A temp directory that removes itself, for tests that need a real database file.
pub struct TempDir {
    directory: PathBuf,
}

impl TempDir {
    pub fn new(label: &str) -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        let directory =
            std::env::temp_dir().join(format!("aref-slice3-{label}-{nanos}-{sequence}"));
        if let Err(error) = std::fs::create_dir_all(&directory) {
            panic!("cannot create {}: {error}", directory.display());
        }
        Self { directory }
    }

    pub fn database(&self) -> PathBuf {
        self.directory.join(aref_lib::storage::db::DATABASE_FILE)
    }

    /// Opens a pool on this directory's database, creating the file and applying migrations.
    pub async fn open(&self) -> sqlx::SqlitePool {
        use std::str::FromStr;

        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

        let options = SqliteConnectOptions::from_str(aref_lib::storage::db::DATABASE_URL)
            .map(|options| options.filename(self.database()))
            .unwrap_or_else(|error| panic!("bad connection string: {error}"))
            .create_if_missing(true)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap_or_else(|error| panic!("cannot open {}: {error}", self.database().display()));
        aref_lib::storage::db::migrate(&pool, &self.database())
            .await
            .unwrap_or_else(|error| panic!("cannot migrate: {error}"));
        pool
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}

/// A credential store that accepts writes, yields nothing on read, and records deletes.
///
/// `Secret` has no accessor outside the crate, so a double built in `tests/` cannot
/// round-trip a value — by design. This one is enough for the paths that never read:
/// `remove_provider` deletes credits before rows, and a keyless local provider reads nothing.
#[derive(Default)]
pub struct RecordingKeyStore {
    deletes: Mutex<Vec<String>>,
}

impl RecordingKeyStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn deleted(&self) -> Vec<String> {
        self.deletes
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .clone()
    }
}

impl KeyStore for RecordingKeyStore {
    fn write(&self, _id: &KeyId, _secret: &Secret) -> Result<(), KeyStoreError> {
        Ok(())
    }

    fn read(&self, _id: &KeyId) -> Result<Option<Secret>, KeyStoreError> {
        Ok(None)
    }

    fn delete(&self, id: &KeyId) -> Result<(), KeyStoreError> {
        self.deletes
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .push(id.to_string());
        Ok(())
    }
}

/// A transport that answers nothing and counts every attempt.
///
/// The mechanical form of "nothing reached the network": the count is the assertion.
#[derive(Default)]
pub struct CountingTransport {
    sends: AtomicU32,
    probes: AtomicU32,
}

impl CountingTransport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sends(&self) -> u32 {
        self.sends.load(Ordering::Relaxed)
    }

    pub fn probes(&self) -> u32 {
        self.probes.load(Ordering::Relaxed)
    }
}

impl Transport for CountingTransport {
    fn send(
        &self,
        _provider: &ProviderConfig,
        _credential: Option<&Secret>,
        _request: &ChatRequest,
    ) -> aref_lib::providers::client::TransportFuture<'_, Result<ChatResponse, TransportError>>
    {
        self.sends.fetch_add(1, Ordering::Relaxed);
        Box::pin(std::future::ready(Err(TransportError::Dropped)))
    }

    fn probe(
        &self,
        _target: &ProbeTarget,
    ) -> aref_lib::providers::client::TransportFuture<'_, Result<bool, TransportError>> {
        self.probes.fetch_add(1, Ordering::Relaxed);
        Box::pin(std::future::ready(Ok(false)))
    }
}

/// An event sink that records nothing: the pool needs one, and these tests assert outcomes
/// rather than events (the payloads are asserted in `providers::pool`'s unit tests).
#[derive(Default)]
pub struct SilentSink;

impl EventSink for SilentSink {
    fn key_invalid(&self, _event: &KeyInvalid) {}
    fn rate_limited(&self, _event: &RateLimited) {}
}
