//! Route handlers for UltimaForge Host Server
//!
//! Contains handlers for the root info endpoint, manifest serving, and validation.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
};
use serde::Serialize;
use std::{path::PathBuf, sync::Arc};

/// Shared application state
pub struct AppState {
    /// Directory being served
    pub serve_dir: PathBuf,
    /// Port the server is running on
    pub port: u16,
}

/// Server info response for API consumers
#[derive(Debug, Clone, Serialize)]
pub struct InfoResponse {
    /// Server name
    pub name: &'static str,
    /// Server version
    pub version: &'static str,
    /// Directory being served
    pub serve_dir: String,
    /// Available API endpoints
    pub endpoints: EndpointsInfo,
}

/// API endpoint information
#[derive(Debug, Clone, Serialize)]
pub struct EndpointsInfo {
    /// Health check endpoint
    pub health: &'static str,
    /// Validation endpoint
    pub validate: &'static str,
    /// Manifest download endpoint
    pub manifest: &'static str,
    /// Content-addressed file downloads
    pub files: &'static str,
    /// Launcher update metadata endpoint
    pub launcher_update: &'static str,
    /// Launcher update file downloads
    pub launcher_files: &'static str,
}

impl EndpointsInfo {
    /// Create the default endpoints info
    pub fn default_endpoints() -> Self {
        Self {
            health: "GET /health",
            validate: "GET /validate",
            manifest: "GET /manifest.json",
            files: "GET /files/{sha256}",
            launcher_update: "GET /launcher/{target}/{arch}/{current_version}",
            launcher_files: "GET /launcher/files/{filename}",
        }
    }
}

impl InfoResponse {
    /// Create an info response from app state
    pub fn from_state(state: &AppState) -> Self {
        Self {
            name: "UltimaForge Host Server",
            version: env!("CARGO_PKG_VERSION"),
            serve_dir: state.serve_dir.display().to_string(),
            endpoints: EndpointsInfo::default_endpoints(),
        }
    }
}

/// Root endpoint - server info and setup instructions (HTML)
///
/// Returns an HTML page with:
/// - Server status
/// - Configuration details
/// - Available endpoints
/// - Setup instructions for server owners
pub async fn root_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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
    <h1>UltimaForge Host Server</h1>
    <p class="status">Server is running</p>

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
    <div class="endpoint"><code>GET /launcher/{{target}}/{{arch}}/{{current_version}}</code> - Launcher update metadata</div>
    <div class="endpoint"><code>GET /launcher/files/{{filename}}</code> - Launcher update files</div>

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
    <h2>Launcher Self-Update</h2>
    <ol>
        <li>Place launcher update metadata in <code>{serve_dir}/launcher</code></li>
        <li>Place launcher binaries in <code>{serve_dir}/launcher/files</code></li>
        <li>Point Tauri updater endpoint to <code>/launcher/&lt;target&gt;/&lt;arch&gt;/&lt;current_version&gt;</code></li>
    </ol>
</body>
</html>"#,
        serve_dir = state.serve_dir.display(),
        port = state.port,
    ))
}

/// Info endpoint - JSON API for server information
///
/// Returns JSON response:
/// ```json
/// {
///   "name": "UltimaForge Host Server",
///   "version": "0.1.0",
///   "serve_dir": "/path/to/updates",
///   "endpoints": { ... }
/// }
/// ```
#[allow(dead_code)]
pub async fn info_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(InfoResponse::from_state(&state))
}

/// Serve manifest.json
pub async fn manifest_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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
pub async fn signature_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
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

/// Serve launcher update metadata.
///
/// Looks for a platform-specific JSON file first:
///   launcher/{target}-{arch}.json
/// If not found, falls back to:
///   launcher/latest.json
pub async fn launcher_update_handler(
    Path((target, arch, _current_version)): Path<(String, String, String)>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let platform_key = format!("{}-{}", target, arch);
    let launcher_dir = state.serve_dir.join("launcher");
    let platform_path = launcher_dir.join(format!("{}.json", platform_key));
    let fallback_path = launcher_dir.join("latest.json");

    let read_path = if tokio::fs::metadata(&platform_path).await.is_ok() {
        platform_path
    } else {
        fallback_path
    };

    match tokio::fs::read_to_string(&read_path).await {
        Ok(content) => (
            StatusCode::OK,
            [("content-type", "application/json")],
            content,
        )
            .into_response(),
        Err(_) => (
            StatusCode::NOT_FOUND,
            "launcher update metadata not found",
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoints_info_default() {
        let endpoints = EndpointsInfo::default_endpoints();
        assert_eq!(endpoints.health, "GET /health");
        assert_eq!(endpoints.validate, "GET /validate");
        assert_eq!(endpoints.manifest, "GET /manifest.json");
        assert_eq!(endpoints.files, "GET /files/{sha256}");
        assert_eq!(
            endpoints.launcher_update,
            "GET /launcher/{target}/{arch}/{current_version}"
        );
        assert_eq!(endpoints.launcher_files, "GET /launcher/files/{filename}");
    }

    #[test]
    fn test_info_response_serialization() {
        let state = AppState {
            serve_dir: PathBuf::from("/test/updates"),
            port: 8080,
        };
        let info = InfoResponse::from_state(&state);

        assert_eq!(info.name, "UltimaForge Host Server");
        assert!(!info.version.is_empty());
        assert!(info.serve_dir.contains("updates"));

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("UltimaForge Host Server"));
    }
}
