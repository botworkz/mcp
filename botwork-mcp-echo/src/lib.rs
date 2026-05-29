use std::sync::Once;

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
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
    transport::streamable_http_server::{
        session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
    },
    ServerHandler,
};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_subscriber::EnvFilter;

pub const LOG_PREFIX: &str = "[mcp-echo]";
pub const REQUEST_SEPARATOR: &str = "─── request ────────────────────────────────────────";

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

pub fn echo_message(message: String) -> String {
    message
}

#[tool_router]
impl EchoServer {
    #[tool(name = "echo", description = "Return the input message unchanged.")]
    async fn echo(&self, Parameters(EchoInput { message }): Parameters<EchoInput>) -> String {
        info!("{LOG_PREFIX} tool echo called: {:?}", message);
        echo_message(message)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for EchoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "mcp-echo".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                ..Default::default()
            },
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
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
    use super::echo_message;

    #[test]
    fn echo_returns_message_unchanged() {
        let message = "hello from unit test".to_string();
        assert_eq!(echo_message(message.clone()), message);
    }
}
