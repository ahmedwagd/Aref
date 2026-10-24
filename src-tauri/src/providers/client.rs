//! The OpenAI-compatible request shape and the transport seam (`FR-301`, `FR-303`).
//!
//! Three things are fixed here, and each one closes a specific hole:
//!
//! * **`ChatRequest` has no `model` field** (`G-09`). The model is read from the provider
//!   entry and injected by [`body_json`], so the caller *cannot* choose one. There is no
//!   models route anywhere in this module: `/chat/completions` is the only URL it builds.
//! * **The `Authorization` header is formatted only here**, from a [`Secret`]. No other
//!   module turns key material into a header value, and a keyless provider yields no
//!   header at all rather than an empty one.
//! * **`Transport` is a trait**, so the pool is exercised by a scripted double and the
//!   real HTTP implementation stays the only thing that needs a network stack. That is
//!   also what keeps the HTTP client crate confined to one file (design.md §3.2 rule 4),
//!   which is the guard that stops the future detector from inheriting one.
//!
//! `ChatResponse` carries the `Retry-After` field's **text**, not a parsed duration:
//! `retry::parse_retry_after` owns the parse rule (settled `D-2`, delta-seconds only), and
//! a transport that parsed it would be a second source of truth for it.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, RETRY_AFTER};
use serde_json::json;

use crate::providers::config::{
    ProbeTarget, PROBE_TIMEOUT, PROVIDER_CONNECT_TIMEOUT, PROVIDER_REQUEST_TIMEOUT,
};
use crate::security::redact::Secret;
use crate::storage::providers_repo::ProviderConfig;

/// A future a [`Transport`] returns.
pub type TransportFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// One message of a chat-completions request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// The roles this product sends. No tool and no function role: v1 has no tool calling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

impl ChatRole {
    pub fn as_str(self) -> &'static str {
        match self {
            ChatRole::System => "system",
            ChatRole::User => "user",
            ChatRole::Assistant => "assistant",
        }
    }
}

/// What the caller asks for.
///
/// **No `model` field** (`G-09`): the model belongs to the provider entry, and a caller
/// that could set it could send a rewrite to a model the user did not configure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
}

impl ChatRequest {
    pub fn new(messages: Vec<ChatMessage>) -> Self {
        Self { messages }
    }

    /// The one-message request a rewrite uses.
    pub fn user(content: impl Into<String>) -> Self {
        Self::new(vec![ChatMessage {
            role: ChatRole::User,
            content: content.into(),
        }])
    }
}

/// A response as the transport saw it, before any classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatResponse {
    pub status: u16,
    /// The `Retry-After` field's text, unparsed (`D-2`).
    pub retry_after: Option<String>,
    pub body: String,
}

/// How a request failed before a status was available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// The connection was refused, or it dropped before a status arrived.
    Dropped,
    /// The request exceeded the configured timeout.
    Timeout,
    /// A 2xx status arrived but the body could not be read to the end.
    Interrupted,
}

/// One shot at one provider with one credential.
///
/// `credential` is `None` for a keyless local provider (`FR-303`), which is a normal
/// request rather than a degraded one.
pub trait Transport: Send + Sync {
    fn send(
        &self,
        provider: &ProviderConfig,
        credential: Option<&Secret>,
        request: &ChatRequest,
    ) -> TransportFuture<'_, Result<ChatResponse, TransportError>>;

    /// A liveness check of one loopback target (`FR-304`).
    ///
    /// `Ok(true)` means "answered with a 2xx". No model list is parsed, returned, or used
    /// (`G-09`): the flavour comes from the target's port, not from the body. A non-2xx is
    /// `Ok(false)` rather than an error — a local server that answers "not found" is not
    /// running an OpenAI-compatible API, which is an absence, not a transport failure.
    fn probe(&self, target: &ProbeTarget) -> TransportFuture<'_, Result<bool, TransportError>>;
}

/// The chat-completions URL of a provider entry.
///
/// `base_url` already ends with the API prefix (`/v1`), which the schema stores and `R-6`
/// records as a one-way door. A trailing separator is tolerated so a pasted value does not
/// produce `//`.
pub fn chat_url(base_url: &str) -> String {
    format!("{}/chat/completions", base_url.trim_end_matches('/'))
}

/// The JSON body of a chat-completions request.
///
/// This is the only place `provider.model_id` enters a request (`G-09`), and the only
/// place the messages are rendered. `stream` is explicitly false: this slice reads a whole
/// body, and `E_STREAM_INTERRUPTED` is about that body being cut rather than about
/// server-sent events.
pub fn body_json(provider: &ProviderConfig, request: &ChatRequest) -> String {
    let messages: Vec<serde_json::Value> = request
        .messages
        .iter()
        .map(|message| {
            json!({
                "role": message.role.as_str(),
                "content": message.content,
            })
        })
        .collect();

    let body = json!({
        "model": provider.model_id,
        "messages": messages,
        "stream": false,
    });

    // Serializing a `Value` cannot fail; the fallback keeps the signature infallible
    // instead of introducing an error path no caller could act on.
    serde_json::to_string(&body).unwrap_or_else(|_| String::from("{}"))
}

/// The `Authorization` header for a credential, or nothing for a keyless provider.
///
/// The one conversion from [`Secret`] to a header value in the crate: `Security`'s doc
/// comment names this function as one of the two legitimate readers, and
/// `tests/provider_architecture.rs` asserts the `Bearer` format appears in no other file.
pub fn authorization(credential: Option<&Secret>) -> Option<String> {
    credential.map(|secret| format!("Bearer {}", secret.expose_secret()))
}

/// The production transport: an OpenAI-compatible HTTP/1.1 client.
///
/// **Both clients are lazy** (task 3.4): the [`reqwest::Client`] is built inside the first
/// [`Transport::send`] or [`Transport::probe`], never at construction, so composing the
/// application state opens no socket. [`ReqwestTransport::client_builds`] is the counter the
/// architecture test reads, and it stays zero until a request actually runs — which is what
/// keeps `NFR-203`'s cold start a property of the type rather than a discipline.
///
/// The probe client is a second, separately configured client: [`no_proxy`] and
/// `redirect::Policy::none()` keep a "loopback" probe on loopback (`NFR-206`), and the 1 s
/// timeout keeps the discovery panel responsive. The chat client also refuses redirects —
/// a followed redirect would re-send the `Authorization` header to whichever host the
/// provider named.
///
/// [`no_proxy`]: reqwest::ClientBuilder::no_proxy
pub struct ReqwestTransport {
    chat: OnceLock<reqwest::Client>,
    probe: OnceLock<reqwest::Client>,
    builds: AtomicU32,
    connect_timeout: Duration,
    request_timeout: Duration,
    probe_timeout: Duration,
}

impl ReqwestTransport {
    pub fn new() -> Self {
        Self::with_timeouts(
            PROVIDER_CONNECT_TIMEOUT,
            PROVIDER_REQUEST_TIMEOUT,
            PROBE_TIMEOUT,
        )
    }

    /// The same transport with explicit timeouts.
    ///
    /// The timeouts are engineering defaults this record may revise (`design.md` §6.5), so
    /// a test can fail fast instead of waiting the production 120 s.
    pub fn with_timeouts(connect: Duration, request: Duration, probe: Duration) -> Self {
        Self {
            chat: OnceLock::new(),
            probe: OnceLock::new(),
            builds: AtomicU32::new(0),
            connect_timeout: connect,
            request_timeout: request,
            probe_timeout: probe,
        }
    }

    /// How many `Client`s this transport has constructed, across both roles.
    pub fn client_builds(&self) -> u32 {
        self.builds.load(Ordering::Relaxed)
    }

    fn client<'a>(
        &self,
        cell: &'a OnceLock<reqwest::Client>,
        probe: bool,
    ) -> Result<&'a reqwest::Client, TransportError> {
        if let Some(client) = cell.get() {
            return Ok(client);
        }
        let timeout = if probe {
            self.probe_timeout
        } else {
            self.request_timeout
        };
        let built = build_client(self.connect_timeout, timeout, probe)?;
        self.builds.fetch_add(1, Ordering::Relaxed);
        // A concurrent first call may have won the race; either client serves the request,
        // and `set` returning `Err` is not a failure to report.
        let _ = cell.set(built);
        cell.get().ok_or(TransportError::Dropped)
    }
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds one client.
///
/// Redirects are refused on both roles (see [`ReqwestTransport`]). `system-proxy` is not an
/// enabled feature, so no environment proxy is consulted by either client; the probe also
/// says so explicitly, because `NFR-206` must not depend on a feature that a later change
/// could switch on.
fn build_client(
    connect: Duration,
    timeout: Duration,
    probe: bool,
) -> Result<reqwest::Client, TransportError> {
    let mut builder = reqwest::Client::builder()
        .connect_timeout(connect)
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none());
    if probe {
        builder = builder.no_proxy();
    }
    // A builder failure is a local configuration problem, not a provider failure; it maps
    // to the "nothing was reachable" branch so the caller's retry policy stays the only
    // thing that decides how long to keep trying.
    builder.build().map_err(|_| TransportError::Dropped)
}

/// Classifies a failure that happened before a status arrived.
///
/// `is_connect` is checked **first**: a failure during the connect phase — refused,
/// unreachable, or the connect timeout expiring — is the network row of `design.md` §4.6,
/// and reqwest sets `is_timeout` for a connect timeout as well. The other order reports a
/// refused connection as `E_PROVIDER_TIMEOUT`, which is what an earlier revision did and
/// what the harness test for a refusal caught.
fn classify_request_error(error: &reqwest::Error) -> TransportError {
    if error.is_connect() {
        TransportError::Dropped
    } else if error.is_timeout() {
        TransportError::Timeout
    } else if error.is_body() || error.is_decode() {
        TransportError::Interrupted
    } else {
        TransportError::Dropped
    }
}

/// Classifies a failure while reading a 2xx body: the status was fine, the rest was cut.
///
/// A timeout during the body is still a timeout (`E_PROVIDER_TIMEOUT`); anything else is an
/// interrupted stream (`E_STREAM_INTERRUPTED`).
fn classify_body_error(error: &reqwest::Error) -> TransportError {
    if error.is_timeout() {
        TransportError::Timeout
    } else {
        TransportError::Interrupted
    }
}

impl Transport for ReqwestTransport {
    fn send(
        &self,
        provider: &ProviderConfig,
        credential: Option<&Secret>,
        request: &ChatRequest,
    ) -> TransportFuture<'_, Result<ChatResponse, TransportError>> {
        // `provider`, `credential` and `request` are borrowed for the duration of the
        // future, which is the caller's `.await`; nothing is cached.
        let url = chat_url(&provider.base_url);
        let body = body_json(provider, request);
        let header = authorization(credential);
        Box::pin(async move {
            let client = self.client(&self.chat, false)?;
            let mut builder = client
                .post(url)
                .header(CONTENT_TYPE, "application/json")
                .body(body);
            if let Some(header) = header {
                builder = builder.header(AUTHORIZATION, header);
            }
            let response = builder
                .send()
                .await
                .map_err(|error| classify_request_error(&error))?;
            let status = response.status().as_u16();
            let retry_after = response
                .headers()
                .get(RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            match response.text().await {
                Ok(body) => Ok(ChatResponse {
                    status,
                    retry_after,
                    body,
                }),
                Err(error) => Err(classify_body_error(&error)),
            }
        })
    }

    fn probe(&self, target: &ProbeTarget) -> TransportFuture<'_, Result<bool, TransportError>> {
        // Copied so the future borrows `self` only; `ProbeTarget` is two literals and a port.
        let target = *target;
        Box::pin(async move {
            let client = self.client(&self.probe, true)?;
            let url = format!("{}/models", target.base_url.trim_end_matches('/'));
            let response = client
                .get(url)
                .send()
                .await
                .map_err(|error| classify_request_error(&error))?;
            // Any 2xx is a hit; a 3xx is not followed (redirects are disabled) and a 4xx/5xx
            // means the port is not serving an OpenAI-compatible API.
            Ok(response.status().is_success())
        })
    }
}

#[cfg(test)]
pub(crate) mod doubles {
    //! The transport double the pool and selection tests drive (design.md §10).
    //!
    //! It records the provider and whether a credential was present — never the credential
    //! itself, because the record outlives the request and a `Secret` is not `Clone`.

    use std::collections::VecDeque;
    use std::sync::Mutex;

    use super::*;

    /// One scripted outcome.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) enum Scripted {
        Respond(ChatResponse),
        Fail(TransportError),
    }

    impl Scripted {
        /// A 200 with a chat-completions body carrying `text`.
        pub(crate) fn ok(text: &str) -> Self {
            Scripted::Respond(ChatResponse {
                status: 200,
                retry_after: None,
                body: ok_body(text),
            })
        }

        pub(crate) fn status(status: u16) -> Self {
            Scripted::Respond(ChatResponse {
                status,
                retry_after: None,
                body: String::new(),
            })
        }

        pub(crate) fn rate_limited(retry_after: Option<&str>) -> Self {
            Scripted::Respond(ChatResponse {
                status: 429,
                retry_after: retry_after.map(str::to_owned),
                body: String::new(),
            })
        }
    }

    /// The body a successful OpenAI-compatible response carries.
    pub(crate) fn ok_body(text: &str) -> String {
        json!({
            "choices": [{
                "message": { "role": "assistant", "content": text },
                "finish_reason": "stop"
            }]
        })
        .to_string()
    }

    /// What a call looked like from the transport's side.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct Call {
        pub provider_id: String,
        pub model_id: String,
        pub credentialed: bool,
        pub body: String,
    }

    /// A transport that answers from a script and records every call.
    pub(crate) struct ScriptedTransport {
        script: Mutex<VecDeque<Scripted>>,
        calls: Mutex<Vec<Call>>,
        probes: Mutex<Vec<ProbeTarget>>,
        probe_completions: Mutex<Vec<u16>>,
        probe_script: Mutex<VecDeque<Result<bool, TransportError>>>,
        slow_probe_port: Option<u16>,
    }

    impl ScriptedTransport {
        pub(crate) fn new(script: impl IntoIterator<Item = Scripted>) -> Self {
            Self {
                script: Mutex::new(script.into_iter().collect()),
                calls: Mutex::new(Vec::new()),
                probes: Mutex::new(Vec::new()),
                probe_completions: Mutex::new(Vec::new()),
                probe_script: Mutex::new(VecDeque::new()),
                slow_probe_port: None,
            }
        }

        /// Makes one target's probe yield before answering, so the other target finishes
        /// first: the ordering test needs an out-of-order arrival, not a lucky one.
        pub(crate) fn with_slow_probe(mut self, port: u16) -> Self {
            self.slow_probe_port = Some(port);
            self
        }

        /// Answers each probe in order; an exhausted script answers `Ok(false)`.
        pub(crate) fn with_probe_script(
            self,
            script: impl IntoIterator<Item = Result<bool, TransportError>>,
        ) -> Self {
            *self
                .probe_script
                .lock()
                .unwrap_or_else(|poison| poison.into_inner()) = script.into_iter().collect();
            self
        }

        pub(crate) fn calls(&self) -> Vec<Call> {
            self.calls
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .clone()
        }

        pub(crate) fn call_count(&self) -> usize {
            self.calls
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .len()
        }

        /// Every probe target, in the order the probes were entered.
        pub(crate) fn probes(&self) -> Vec<ProbeTarget> {
            self.probes
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .clone()
        }

        pub(crate) fn probe_count(&self) -> usize {
            self.probes
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .len()
        }

        /// The ports of the probes, in the order they *finished*.
        pub(crate) fn probe_completions(&self) -> Vec<u16> {
            self.probe_completions
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .clone()
        }
    }

    impl Transport for ScriptedTransport {
        fn send(
            &self,
            provider: &ProviderConfig,
            credential: Option<&Secret>,
            request: &ChatRequest,
        ) -> TransportFuture<'_, Result<ChatResponse, TransportError>> {
            let body = body_json(provider, request);
            self.calls
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .push(Call {
                    provider_id: provider.id.clone(),
                    model_id: provider.model_id.clone(),
                    credentialed: credential.is_some(),
                    body,
                });

            let next = self
                .script
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .pop_front();
            let outcome = match next {
                Some(Scripted::Respond(response)) => Ok(response),
                Some(Scripted::Fail(error)) => Err(error),
                None => panic!("the scripted transport ran out of responses"),
            };
            Box::pin(std::future::ready(outcome))
        }

        fn probe(&self, target: &ProbeTarget) -> TransportFuture<'_, Result<bool, TransportError>> {
            self.probes
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .push(*target);
            let result = self
                .probe_script
                .lock()
                .unwrap_or_else(|poison| poison.into_inner())
                .pop_front()
                .unwrap_or(Ok(false));
            let slow = self.slow_probe_port == Some(target.port);
            let completions = &self.probe_completions;
            let port = target.port;
            Box::pin(async move {
                if slow {
                    tokio::task::yield_now().await;
                }
                completions
                    .lock()
                    .unwrap_or_else(|poison| poison.into_inner())
                    .push(port);
                result
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::doubles::{ok_body, ScriptedTransport};
    use super::*;
    use crate::storage::providers_repo::{LocalFlavor, ProviderKind};

    fn provider(model_id: &str) -> ProviderConfig {
        ProviderConfig::new(
            "provider-1",
            ProviderKind::OpenAiCompatible,
            None,
            "http://127.0.0.1:1/v1",
            model_id,
            "Primary",
            true,
        )
    }

    fn value(text: &str) -> serde_json::Value {
        serde_json::from_str(text).unwrap_or_else(|error| panic!("not JSON: {error}"))
    }

    #[test]
    fn the_url_is_the_configured_chat_completions_endpoint() {
        assert_eq!(
            chat_url("http://127.0.0.1:11434/v1"),
            "http://127.0.0.1:11434/v1/chat/completions"
        );
        assert_eq!(
            chat_url("http://127.0.0.1:11434/v1/"),
            "http://127.0.0.1:11434/v1/chat/completions"
        );
        // No other route is reachable from this module: the one format string above is
        // the only URL a request can be sent to (`G-09`, `FR-301`).
        assert!(!chat_url("http://127.0.0.1:1/v1").contains("/models"));
    }

    #[test]
    fn the_body_carries_the_model_from_the_entry_not_from_the_request() {
        let request = ChatRequest::user("fixture text");
        let first = value(&body_json(&provider("model-a"), &request));
        let second = value(&body_json(&provider("model-b"), &request));

        assert_eq!(first["model"], json!("model-a"));
        assert_eq!(second["model"], json!("model-b"));
        // The same request produced both bodies: the caller supplied no model.
        assert_eq!(first["messages"], second["messages"]);
        assert_eq!(first["messages"][0]["role"], json!("user"));
        assert_eq!(first["messages"][0]["content"], json!("fixture text"));
        assert_eq!(first["stream"], json!(false));
    }

    #[test]
    fn a_keyless_provider_formats_no_authorization_header() {
        assert_eq!(authorization(None), None);
        let secret = Secret::new("fixture-secret-a");
        assert_eq!(
            authorization(Some(&secret)),
            Some("Bearer fixture-secret-a".to_owned())
        );
    }

    #[test]
    fn a_local_flavour_is_irrelevant_to_the_request_shape() {
        // `FR-301`/`FR-303`: one request shape, no per-provider branch. The only thing a
        // flavour changes is selection order, which is not this module's business.
        let local = ProviderConfig::new(
            "local",
            ProviderKind::Local,
            Some(LocalFlavor::Ollama),
            "http://127.0.0.1:11434/v1",
            "model-c",
            "Ollama",
            true,
        );
        let request = ChatRequest::user("fixture text");
        let from_cloud = value(&body_json(&provider("model-c"), &request));
        let from_local = value(&body_json(&local, &request));
        assert_eq!(from_cloud, from_local);
    }

    #[tokio::test]
    async fn the_scripted_transport_records_the_model_it_was_given() {
        let transport = ScriptedTransport::new([super::doubles::Scripted::Respond(ChatResponse {
            status: 200,
            retry_after: None,
            body: ok_body("rewrite"),
        })]);
        let secret = Secret::new("fixture-secret-a");
        let outcome = transport
            .send(
                &provider("model-a"),
                Some(&secret),
                &ChatRequest::user("fixture text"),
            )
            .await;
        assert!(matches!(outcome, Ok(response) if response.status == 200));

        let calls = transport.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].provider_id, "provider-1");
        assert_eq!(calls[0].model_id, "model-a");
        assert!(calls[0].credentialed);
        // The record holds the body, never the credential.
        assert!(!calls[0].body.contains("fixture-secret-a"));
        assert!(!format!("{:?}", calls[0]).contains("fixture-secret-a"));
    }
}
