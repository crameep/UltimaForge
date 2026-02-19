//! Health check module for UltimaForge Host Server
//!
//! Provides health check endpoint and system status information.

use axum::{response::IntoResponse, Json};
use serde::Serialize;
use std::time::Instant;

/// Health check response
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    /// Health status: "ok" or "degraded"
    pub status: &'static str,
    /// Server version from Cargo.toml
    pub version: &'static str,
    /// Uptime in seconds (if tracked)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<u64>,
}

impl HealthResponse {
    /// Create a healthy response
    pub fn ok() -> Self {
        Self {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
            uptime_seconds: None,
        }
    }

    /// Create a healthy response with uptime tracking
    pub fn ok_with_uptime(start_time: Instant) -> Self {
        Self {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
            uptime_seconds: Some(start_time.elapsed().as_secs()),
        }
    }

    /// Create a degraded health response
    #[allow(dead_code)]
    pub fn degraded() -> Self {
        Self {
            status: "degraded",
            version: env!("CARGO_PKG_VERSION"),
            uptime_seconds: None,
        }
    }
}

/// Health check endpoint handler
///
/// Returns JSON response with server health status:
/// ```json
/// {
///   "status": "ok",
///   "version": "0.1.0"
/// }
/// ```
pub async fn health_handler() -> impl IntoResponse {
    Json(HealthResponse::ok())
}

/// Health check handler with uptime tracking
#[allow(dead_code)]
pub async fn health_handler_with_uptime(start_time: Instant) -> impl IntoResponse {
    Json(HealthResponse::ok_with_uptime(start_time))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_ok() {
        let response = HealthResponse::ok();
        assert_eq!(response.status, "ok");
        assert!(!response.version.is_empty());
        assert!(response.uptime_seconds.is_none());
    }

    #[test]
    fn test_health_response_with_uptime() {
        let start = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let response = HealthResponse::ok_with_uptime(start);
        assert_eq!(response.status, "ok");
        assert!(response.uptime_seconds.is_some());
    }

    #[test]
    fn test_health_response_degraded() {
        let response = HealthResponse::degraded();
        assert_eq!(response.status, "degraded");
    }

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse::ok();
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"version\""));
        // Should not contain uptime if None
        assert!(!json.contains("uptime_seconds"));
    }

    #[test]
    fn test_health_response_with_uptime_serialization() {
        let response = HealthResponse::ok_with_uptime(Instant::now());
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("uptime_seconds"));
    }
}
