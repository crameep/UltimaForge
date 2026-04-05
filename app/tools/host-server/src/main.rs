//! UltimaForge Host Server
//!
//! Simple static file server for hosting update artifacts.
//! Features:
//! - Static file serving with directory traversal protection
//! - Health check endpoint
//! - Validation endpoint for update folder structure
//! - Request logging with method, path, status, and duration

mod brand;
mod health;
mod routes;
mod validation;

use axum::{routing::get, Router};
use clap::Parser;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tower_http::{
    classify::ServerErrorsFailureClass,
    services::ServeDir,
    trace::{DefaultMakeSpan, DefaultOnRequest, DefaultOnResponse, TraceLayer},
};
use tracing::{info, Level, Span};

use health::health_handler;
use routes::{
    launcher_update_handler, manifest_handler, root_handler, signature_handler, AppState,
};
use validation::validate_handler;

/// UltimaForge Host Server - Static file server for update artifacts
#[derive(Parser)]
#[command(name = "host-server")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Directory to serve files from
    #[arg(short, long, default_value = "./updates")]
    dir: std::path::PathBuf,

    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Host address to bind to
    #[arg(long, default_value = "0.0.0.0")]
    host: String,
}

#[tokio::main]
async fn main() {
    // Initialize tracing for structured logging
    // Default to INFO level for request logging, can be overridden with RUST_LOG env var
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,tower_http=debug")),
        )
        .init();

    let cli = Cli::parse();

    // Validate serve directory exists
    if !cli.dir.exists() {
        eprintln!("Error: Directory does not exist: {:?}", cli.dir);
        eprintln!("Create the directory or specify a different path with --dir");
        std::process::exit(1);
    }

    let serve_dir = cli.dir.canonicalize().unwrap_or_else(|_| cli.dir.clone());

    // Load brand configuration for custom server name
    let brand = brand::BrandConfig::load();
    let server_name = brand
        .as_ref()
        .map(|b| b.product.display_name.as_str())
        .unwrap_or("UltimaForge Host Server");

    info!("Starting {}", server_name);
    if let Some(ref brand) = brand {
        if let Some(ref description) = brand.product.description {
            info!("{}", description);
        }
    }
    info!("Serving files from: {:?}", serve_dir);
    info!("Listening on: {}:{}", cli.host, cli.port);

    let state = Arc::new(AppState {
        serve_dir: serve_dir.clone(),
        port: cli.port,
    });

    // Build the router with request tracing
    // Logs: method, path, status code, and duration for each request
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
        .on_request(DefaultOnRequest::new().level(Level::INFO))
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .latency_unit(tower_http::LatencyUnit::Millis),
        )
        .on_failure(
            |error: ServerErrorsFailureClass, latency: Duration, _span: &Span| {
                tracing::error!(
                    error = %error,
                    latency_ms = latency.as_millis(),
                    "request failed"
                );
            },
        );

    let app = Router::new()
        // API endpoints
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/validate", get(validate_handler))
        // Static file serving for update artifacts
        .nest_service("/files", ServeDir::new(serve_dir.join("files")))
        .nest_service(
            "/launcher/files",
            ServeDir::new(serve_dir.join("launcher/files")),
        )
        .route(
            "/launcher/:target/:arch/:current_version",
            get(launcher_update_handler),
        )
        .route("/manifest.json", get(manifest_handler))
        .route("/manifest.sig", get(signature_handler))
        // Add request tracing (logs method, path, status, duration)
        .layer(trace_layer)
        .with_state(state);

    // Parse address and start server
    let addr: SocketAddr = format!("{}:{}", cli.host, cli.port)
        .parse()
        .expect("Invalid address");

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!(
                error = %e,
                address = %addr,
                "Failed to bind to address"
            );
            eprintln!("Error: Failed to bind to {}:{} - {}", cli.host, cli.port, e);
            eprintln!("Check if another process is using this port or if you have permission to bind to this address.");
            std::process::exit(1);
        }
    };

    info!("Server ready at http://{}", addr);

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(error = %e, "Server error");
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
