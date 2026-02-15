//! UltimaForge Host Server
//!
//! Simple static file server for hosting update artifacts.
//! Features:
//! - Static file serving with directory traversal protection
//! - Health check endpoint
//! - Validation endpoint for update folder structure
//! - Request logging

use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::get,
    Router,
};
use clap::Parser;
use serde::Serialize;
use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::info;

/// UltimaForge Host Server - Static file server for update artifacts
#[derive(Parser)]
#[command(name = "host-server")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Directory to serve files from
    #[arg(short, long, default_value = "./updates")]
    dir: PathBuf,

    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Host address to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
}

/// Shared application state
struct AppState {
    serve_dir: PathBuf,
    port: u16,
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

/// Server info response for root endpoint
#[derive(Serialize)]
struct InfoResponse {
    name: &'static str,
    version: &'static str,
    serve_dir: String,
    endpoints: EndpointsInfo,
}

#[derive(Serialize)]
struct EndpointsInfo {
    health: &'static str,
    validate: &'static str,
    manifest: &'static str,
    files: &'static str,
}

#[tokio::main]
async fn main() {
    // Initialize tracing for structured logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Validate serve directory exists
    if !cli.dir.exists() {
        eprintln!("Error: Directory does not exist: {:?}", cli.dir);
        eprintln!("Create the directory or specify a different path with --dir");
        std::process::exit(1);
    }

    let serve_dir = cli.dir.canonicalize().unwrap_or_else(|_| cli.dir.clone());

    info!("Starting UltimaForge Host Server");
    info!("Serving files from: {:?}", serve_dir);
    info!("Listening on: {}:{}", cli.host, cli.port);

    let state = Arc::new(AppState {
        serve_dir: serve_dir.clone(),
        port: cli.port,
    });

    // Build the router
    let app = Router::new()
        // API endpoints
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/validate", get(validate_handler))
        // Static file serving for update artifacts
        .nest_service("/files", ServeDir::new(serve_dir.join("files")))
        .route("/manifest.json", get(manifest_handler))
        .route("/manifest.sig", get(signature_handler))
        // Add request tracing
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Parse address and start server
    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port)
        .parse()
        .expect("Invalid address");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("Server ready at http://{}", addr);

    axum::serve(listener, app).await.unwrap();
}

/// Root endpoint - server info and setup instructions
async fn root_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Html(format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>UltimaForge Host Server</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; max-width: 800px; margin: 40px auto; padding: 20px; }}
        h1 {{ color: #333; }}
        code {{ background: #f4f4f4; padding: 2px 6px; border-radius: 3px; }}
        pre {{ background: #f4f4f4; padding: 15px; border-radius: 5px; overflow-x: auto; }}
        .status {{ color: #22c55e; }}
        .endpoint {{ margin: 10px 0; }}
    </style>
</head>
<body>
    <h1>🎮 UltimaForge Host Server</h1>
    <p class="status">✓ Server is running</p>

    <h2>Configuration</h2>
    <ul>
        <li>Serving from: <code>{serve_dir}</code></li>
        <li>Port: <code>{port}</code></li>
    </ul>

    <h2>Endpoints</h2>
    <div class="endpoint"><code>GET /health</code> - Health check (JSON)</div>
    <div class="endpoint"><code>GET /validate</code> - Validate update folder structure</div>
    <div class="endpoint"><code>GET /manifest.json</code> - Download manifest</div>
    <div class="endpoint"><code>GET /manifest.sig</code> - Download manifest signature</div>
    <div class="endpoint"><code>GET /files/{{sha256}}</code> - Download content-addressed file</div>

    <h2>Player Update URL</h2>
    <p>Configure your launcher with this base URL:</p>
    <pre>http://your-server-ip:{port}</pre>

    <h2>Setup Instructions</h2>
    <ol>
        <li>Use <code>publish-cli</code> to generate update artifacts</li>
        <li>Copy artifacts to <code>{serve_dir}</code></li>
        <li>Verify with <code>GET /validate</code></li>
        <li>Distribute the launcher to players</li>
    </ol>
</body>
</html>"#,
        serve_dir = state.serve_dir.display(),
        port = state.port,
    ))
}

/// Health check endpoint
async fn health_handler() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Validate update folder structure
async fn validate_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // TODO: Implement full validation in subtask-6-3
    // For now, check basic file existence
    let manifest_exists = state.serve_dir.join("manifest.json").exists();
    let signature_exists = state.serve_dir.join("manifest.sig").exists();
    let files_dir_exists = state.serve_dir.join("files").is_dir();

    #[derive(Serialize)]
    struct ValidationResult {
        valid: bool,
        manifest_exists: bool,
        signature_exists: bool,
        files_dir_exists: bool,
        message: String,
    }

    let valid = manifest_exists && signature_exists && files_dir_exists;
    let message = if valid {
        "Update folder structure is valid".to_string()
    } else {
        let mut missing = Vec::new();
        if !manifest_exists {
            missing.push("manifest.json");
        }
        if !signature_exists {
            missing.push("manifest.sig");
        }
        if !files_dir_exists {
            missing.push("files/");
        }
        format!("Missing required files: {}", missing.join(", "))
    };

    Json(ValidationResult {
        valid,
        manifest_exists,
        signature_exists,
        files_dir_exists,
        message,
    })
}

/// Serve manifest.json
async fn manifest_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let path = state.serve_dir.join("manifest.json");
    match tokio::fs::read_to_string(&path).await {
        Ok(content) => (
            StatusCode::OK,
            [("content-type", "application/json")],
            content,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "manifest.json not found").into_response(),
    }
}

/// Serve manifest.sig
async fn signature_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let path = state.serve_dir.join("manifest.sig");
    match tokio::fs::read(&path).await {
        Ok(content) => (
            StatusCode::OK,
            [("content-type", "application/octet-stream")],
            content,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "manifest.sig not found").into_response(),
    }
}
