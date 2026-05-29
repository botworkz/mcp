use std::sync::Once;

use anyhow::{bail, Context};
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
use tokio::{net::TcpListener, process::Command};
use tokio_util::sync::CancellationToken;
use tracing::info;
use tracing_subscriber::EnvFilter;

pub const LOG_PREFIX: &str = "[mcp-glint]";
pub const REQUEST_SEPARATOR: &str = "─── request ────────────────────────────────────────";

#[derive(Debug, Clone)]
pub struct GlintServer {
    tool_router: ToolRouter<Self>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
struct GlintInput {
    paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GlintOutput {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}

impl GlintServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

impl Default for GlintServer {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_paths(paths: &[String]) -> anyhow::Result<()> {
    if paths.is_empty() {
        bail!("paths must be non-empty");
    }

    if paths.iter().any(|path| path.starts_with('-')) {
        bail!("paths must not start with '-'");
    }

    Ok(())
}

fn parse_json_stdout(stdout: &str) -> anyhow::Result<String> {
    let value: serde_json::Value =
        serde_json::from_str(stdout).context("glint stdout was not valid JSON")?;
    Ok(value.to_string())
}

async fn run_glint(args: &[&str], paths: &[String]) -> anyhow::Result<GlintOutput> {
    let glint_bin = std::env::var("GLINT_BIN").unwrap_or_else(|_| "glint".to_string());

    let output = Command::new(glint_bin)
        .args(args)
        .args(paths)
        .output()
        .await
        .context("failed to execute glint")?;

    Ok(GlintOutput {
        stdout: String::from_utf8(output.stdout).context("glint stdout was not valid UTF-8")?,
        stderr: String::from_utf8(output.stderr).context("glint stderr was not valid UTF-8")?,
        exit_code: output.status.code(),
    })
}

#[tool_router]
impl GlintServer {
    #[tool(
        name = "glint_check",
        description = "Run glint in check mode on the given paths and return its JSON report."
    )]
    async fn glint_check(
        &self,
        Parameters(GlintInput { paths }): Parameters<GlintInput>,
    ) -> Result<String, String> {
        info!("{LOG_PREFIX} tool glint_check called: {:?}", paths);
        validate_paths(&paths).map_err(|err| err.to_string())?;

        let output = run_glint(&[], &paths)
            .await
            .map_err(|err| err.to_string())?;

        if output.exit_code.unwrap_or_default() != 0 && !output.stderr.trim().is_empty() {
            return Err(format!(
                "glint_check failed with exit code {:?}: {}",
                output.exit_code,
                output.stderr.trim()
            ));
        }

        parse_json_stdout(&output.stdout).map_err(|err| err.to_string())
    }

    #[tool(
        name = "glint_fix",
        description = "Run glint in fix mode (--fix) on the given paths and return its JSON report of changes."
    )]
    async fn glint_fix(
        &self,
        Parameters(GlintInput { paths }): Parameters<GlintInput>,
    ) -> Result<String, String> {
        info!("{LOG_PREFIX} tool glint_fix called: {:?}", paths);
        validate_paths(&paths).map_err(|err| err.to_string())?;

        let output = run_glint(&["--fix"], &paths)
            .await
            .map_err(|err| err.to_string())?;

        if output.exit_code.unwrap_or_default() != 0 && !output.stderr.trim().is_empty() {
            return Err(format!(
                "glint_fix failed with exit code {:?}: {}",
                output.exit_code,
                output.stderr.trim()
            ));
        }

        if output.stdout.trim().is_empty() {
            return Ok("no changes".to_string());
        }

        parse_json_stdout(&output.stdout).map_err(|err| err.to_string())
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for GlintServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            server_info: Implementation {
                name: "mcp-glint".to_string(),
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
    let service: StreamableHttpService<GlintServer, LocalSessionManager> =
        StreamableHttpService::new(
            || Ok(GlintServer::new()),
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
    use super::{run_glint, validate_paths};

    #[test]
    fn path_validation_rejects_leading_dash() {
        let err = validate_paths(&["-bad".to_string()]).expect_err("should reject leading dash");
        assert!(err.to_string().contains("must not start with '-'"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_glint_captures_stdout_with_fake_binary() {
        unsafe {
            std::env::set_var("GLINT_BIN", "/bin/echo");
        }

        let output = run_glint(&["--fix"], &["/tmp/sample".to_string()])
            .await
            .expect("run_glint should succeed");

        assert_eq!(output.stdout.trim(), "--fix /tmp/sample");
        assert!(output.stderr.is_empty());
        assert_eq!(output.exit_code, Some(0));

        unsafe {
            std::env::remove_var("GLINT_BIN");
        }
    }
}
