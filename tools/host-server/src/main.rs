//! UltimaForge Host Server
//!
//! Simple static file server for hosting update artifacts.
//! Features:
//! - Static file serving with directory traversal protection
//! - Health check endpoint
//! - Validation endpoint for update folder structure
//! - Request logging

mod health;
mod routes;

use axum::{routing::get, Router};
use clap::Parser;
use std::{net::SocketAddr, sync::Arc};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::info;

use health::health_handler;
use routes::{manifest_handler, root_handler, signature_handler, validate_handler, AppState};

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
