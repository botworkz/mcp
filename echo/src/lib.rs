// Workspace-wide policy: no `unsafe` in production code. This crate
// is a single-file rmcp server with no FFI surface, so the policy is
// trivially satisfied today; `forbid` (vs `deny`) locks that in —
// a future PR that wants `unsafe` has to first land a stand-alone
// commit removing this attribute, which gets reviewed on its own
// merits. Mirrors `botworkz/botwork`'s workspace-level
// `unsafe_code = "forbid"` lint.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::{Once, OnceLock};

use axum::{
    body::{to_bytes, Body},
    http::Request,
    http::StatusCode,
    middleware::{from_fn, Next},
    response::IntoResponse,
    response::Response,
    Router,
};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Json, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    ServerHandler,
};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// Trimmed contents of the workspace-root `/VERSION` file, baked in
/// at compile time. The path is relative to *this* source file:
/// `echo/src/lib.rs` → `../../VERSION`.
const VERSION: &str = include_str!("../../VERSION").trim_ascii();

pub const LOG_PREFIX: &str = "[mcp-echo]";
pub const REQUEST_SEPARATOR: &str = "─── request ────────────────────────────────────────";

/// Prefix marking a secret-bearing env var.
///
/// Keep in sync with `botwork-launcher::validate::is_sensitive_env` and
/// `botwork-config-broker::registry::SECRET_ENV_PREFIX`. Values under this
/// prefix are redacted before being returned over the wire.
const SECRET_ENV_PREFIX: &str = "BOTWORK_SECRET_";

/// Snapshot of the plugin's environment captured at process startup.
///
/// We capture once rather than re-reading per call so a tool response is a
/// faithful answer to "what config did the broker inject when the container
/// was spawned?", not "what env exists at the moment you happened to ask?".
/// Those answers will usually agree, but if they ever diverge the spawn-time
/// view is the one the smoke test cares about.
static STARTUP_ENV: OnceLock<Vec<(String, String)>> = OnceLock::new();

fn startup_env() -> &'static Vec<(String, String)> {
    STARTUP_ENV.get_or_init(capture_startup_env)
}

fn capture_startup_env() -> Vec<(String, String)> {
    // BTreeMap to get sorted, deduplicated output regardless of process env
    // ordering. The wire shape is `Vec<EnvEntry>` so callers see an ordered
    // sequence and can byte-compare two responses without map-ordering noise.
    let collected: BTreeMap<String, String> = std::env::vars().collect();
    collected.into_iter().collect()
}

/// One env entry on the echo response. Mirrors the `{name, value}` shape that
/// session-broker uses on the launcher wire so the smoke test can read the
/// two layers with the same code.
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
pub struct EnvEntry {
    pub name: String,
    pub value: String,
}

/// Structured response from the `echo` tool.
///
/// The shape is deliberately HTTP-echo-like: caller sent `message`, server
/// echoes it back together with self-identifying metadata and the env it was
/// started with. `BOTWORK_MCP_CONFIG` is the field the vm smoke test actually
/// asserts on; the rest of `env` is included because once you have a generic
/// "what did the plugin see?" tool it's strictly more useful than a special-
/// case `read_config`.
///
/// `env` is sorted by `name` so two equivalent invocations produce
/// byte-identical responses regardless of process environment ordering.
#[derive(Debug, Clone, PartialEq, Serialize, schemars::JsonSchema)]
pub struct EchoResponse {
    /// The input message, returned verbatim.
    pub message: String,
    /// Server's plugin identifier (`mcp-echo`).
    pub plugin: String,
    /// Crate version from `CARGO_PKG_VERSION`.
    pub version: String,
    /// Process env captured at startup, sorted by name.
    ///
    /// Values for any name starting with `BOTWORK_SECRET_` are replaced with
    /// `<redacted len=N>` so the assertion "the secret arrived under the
    /// expected name" is still possible while preserving the no-secrets-in-
    /// logs invariant the rest of the system enforces.
    pub env: Vec<EnvEntry>,
}

#[derive(Debug, Clone)]
pub struct EchoServer {
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct EchoInput {
    message: String,
}

impl EchoServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for EchoServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the structured echo response from a message and the captured env.
///
/// Pulled out of the tool handler so unit tests can exercise it directly
/// without spinning up the rmcp transport.
pub fn build_echo_response(message: String, env: &[(String, String)]) -> EchoResponse {
    EchoResponse {
        message,
        plugin: "mcp-echo".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        env: env
            .iter()
            .map(|(name, value)| EnvEntry {
                name: name.clone(),
                value: redact_if_secret(name, value),
            })
            .collect(),
    }
}

fn redact_if_secret(name: &str, value: &str) -> String {
    if name.starts_with(SECRET_ENV_PREFIX) {
        format!("<redacted len={}>", value.len())
    } else {
        value.to_string()
    }
}

#[tool_router]
impl EchoServer {
    #[tool(
        name = "echo",
        description = "Echo the input message and return server diagnostics: \
                       plugin name, version, and a sorted snapshot of the \
                       process env captured at startup. Values for any env \
                       name starting with BOTWORK_SECRET_ are redacted."
    )]
    async fn echo(
        &self,
        Parameters(EchoInput { message }): Parameters<EchoInput>,
    ) -> Json<EchoResponse> {
        info!(
            "{LOG_PREFIX} tool echo called: message_len={}",
            message.len()
        );
        Json(build_echo_response(message, startup_env()))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for EchoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("mcp-echo", env!("CARGO_PKG_VERSION")))
    }
}

pub async fn request_logging_middleware(request: Request<Body>, next: Next) -> Response {
    info!("{LOG_PREFIX} {REQUEST_SEPARATOR}");
    info!("{LOG_PREFIX} {} {}", request.method(), request.uri().path());

    for (name, value) in request.headers() {
        if let Ok(value) = value.to_str() {
            info!("{LOG_PREFIX}   {name}: {value}");
        } else {
            info!("{LOG_PREFIX}   {name}: <binary>");
        }
    }

    let (parts, body) = request.into_parts();
    let bytes = match to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(err) => {
            info!("{LOG_PREFIX} body: <binary 0 bytes>");
            info!("{LOG_PREFIX} body read error: {err}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    match std::str::from_utf8(&bytes) {
        Ok(body) => info!("{LOG_PREFIX} body: {body}"),
        Err(_) => info!("{LOG_PREFIX} body: <binary {} bytes>", bytes.len()),
    }

    let replay = Request::from_parts(parts, Body::from(bytes));
    next.run(replay).await
}

pub fn build_router() -> Router {
    let service: StreamableHttpService<EchoServer, LocalSessionManager> =
        StreamableHttpService::new(
            || Ok(EchoServer::new()),
            Default::default(),
            StreamableHttpServerConfig::default(),
        );

    Router::new()
        .nest_service("/mcp", service)
        .layer(from_fn(request_logging_middleware))
}

pub async fn serve_with_listener(
    listener: TcpListener,
    shutdown: CancellationToken,
) -> anyhow::Result<()> {
    axum::serve(listener, build_router())
        .with_graceful_shutdown(async move { shutdown.cancelled_owned().await })
        .await?;
    Ok(())
}

pub async fn run() -> anyhow::Result<()> {
    init_logging();
    // Materialise the startup-env snapshot eagerly so the very first request
    // (and the startup log line below) sees the same view, even if something
    // else in the process happens to set env vars before the first call.
    let _ = startup_env();
    tracing::info!(
        "botwork-mcp-echo {}",
        botwork_version::format_full(VERSION, botwork_version::GIT_SHA),
    );
    info!("{LOG_PREFIX} starting on 0.0.0.0:8000/mcp");

    let listener = TcpListener::bind("0.0.0.0:8000").await?;
    axum::serve(listener, build_router()).await?;
    Ok(())
}

pub fn init_logging() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        let env_filter = EnvFilter::new("info,hyper=warn,tower=warn,rmcp=warn,axum=warn");
        let _ = tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_target(false)
            .with_level(false)
            .without_time()
            .with_ansi(false)
            .try_init();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_pair(name: &str, value: &str) -> (String, String) {
        (name.to_string(), value.to_string())
    }

    #[test]
    fn echo_returns_message_in_response() {
        let response = build_echo_response("hello".to_string(), &[]);
        assert_eq!(response.message, "hello");
        assert_eq!(response.plugin, "mcp-echo");
        assert!(!response.version.is_empty());
        assert!(response.env.is_empty());
    }

    #[test]
    fn echo_preserves_env_order_from_caller() {
        // The startup-env snapshot is sorted before being passed in; this
        // test confirms the builder does not re-shuffle it. (Sorting itself
        // is covered by capture_startup_env_is_sorted.)
        let env = vec![
            env_pair("A_FIRST", "1"),
            env_pair("B_SECOND", "2"),
            env_pair("C_THIRD", "3"),
        ];
        let response = build_echo_response("m".to_string(), &env);
        let names: Vec<&str> = response.env.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["A_FIRST", "B_SECOND", "C_THIRD"]);
    }

    #[test]
    fn echo_redacts_botwork_secret_values() {
        let env = vec![
            env_pair("BOTWORK_MCP_CONFIG", r#"{"k":"v"}"#),
            env_pair("BOTWORK_SECRET_GITHUB_PAT", "ghp_xxxxxxxxxxxxxxxxxxxx"),
            env_pair("BOTWORK_SECRET_EMPTY", ""),
            env_pair("PLAIN_VAR", "plain"),
        ];
        let response = build_echo_response("m".to_string(), &env);

        let by_name: std::collections::HashMap<&str, &str> = response
            .env
            .iter()
            .map(|e| (e.name.as_str(), e.value.as_str()))
            .collect();

        // Non-secret values pass through verbatim — including the structured
        // config blob the smoke test cares about.
        assert_eq!(by_name["BOTWORK_MCP_CONFIG"], r#"{"k":"v"}"#);
        assert_eq!(by_name["PLAIN_VAR"], "plain");
        // Secret values redacted to a fixed shape that surfaces *only* the
        // length, so the smoke test can still distinguish present-vs-absent
        // and "right shape" without leaking the value.
        assert_eq!(by_name["BOTWORK_SECRET_GITHUB_PAT"], "<redacted len=24>");
        assert_eq!(by_name["BOTWORK_SECRET_EMPTY"], "<redacted len=0>");
    }

    #[test]
    fn echo_redaction_only_triggers_on_full_prefix() {
        // Defensive: a variable that merely contains the substring "BOTWORK_SECRET_"
        // somewhere in the name (not as a prefix) must NOT be redacted.
        let env = vec![
            env_pair("X_BOTWORK_SECRET_FOO", "not-a-secret"),
            env_pair("BOTWORK_SECRET", "no-trailing-underscore-still-not-prefix"),
        ];
        let response = build_echo_response("m".to_string(), &env);
        let by_name: std::collections::HashMap<&str, &str> = response
            .env
            .iter()
            .map(|e| (e.name.as_str(), e.value.as_str()))
            .collect();

        // First key does not start with the prefix → pass through.
        assert_eq!(by_name["X_BOTWORK_SECRET_FOO"], "not-a-secret");
        // Second key does not have the trailing underscore so it is not a
        // prefix match — pass through. (If we ever loosen the rule, this
        // test will catch it.)
        assert_eq!(
            by_name["BOTWORK_SECRET"],
            "no-trailing-underscore-still-not-prefix"
        );
    }

    #[test]
    fn capture_startup_env_is_sorted_and_unique() {
        // Sanity-check the snapshot path used in production. We can't mutate
        // the live process env safely from a test, but we *can* assert that
        // whatever it captures comes out sorted and without duplicates —
        // which is the contract the smoke test depends on.
        let snapshot = capture_startup_env();
        let mut names: Vec<&str> = snapshot.iter().map(|(n, _)| n.as_str()).collect();
        let sorted = {
            let mut s = names.clone();
            s.sort();
            s
        };
        assert_eq!(names, sorted, "env snapshot must be sorted by name");
        names.dedup();
        assert_eq!(
            names.len(),
            snapshot.len(),
            "env snapshot must contain no duplicate keys"
        );
    }
}
