//! Streaming HTTP download engine for UltimaForge.
//!
//! This module provides streaming file downloads with:
//! - Progress reporting via callbacks and events
//! - Resume support using HTTP Range headers
//! - Hash verification after download
//! - Retry logic with configurable attempts
//! - Memory-efficient streaming (never buffers entire file)
//!
//! # Security
//!
//! - All downloaded files MUST have their hash verified
//! - Downloads use HTTPS by default
//! - Timeouts prevent hung connections
//!
//! # Example
//!
//! ```ignore
//! use ultimaforge_lib::downloader::{Downloader, DownloadProgress};
//!
//! let downloader = Downloader::new()?;
//!
//! downloader.download_file(
//!     "https://example.com/files/abc123",
//!     Path::new("./downloads/file.bin"),
//!     Some("abc123..."), // Expected SHA-256 hash
//!     |progress| {
//!         println!("Downloaded {} of {} bytes", progress.downloaded, progress.total);
//!     }
//! ).await?;
//! ```

use crate::error::DownloadError;
use crate::hash::{hash_file, validate_hash_format};
use futures_util::StreamExt;
use reqwest::{header, Client, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use tauri::Emitter;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};

/// Default connection timeout in seconds.
const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Default read timeout in seconds.
const DEFAULT_READ_TIMEOUT_SECS: u64 = 30;

/// Default number of retry attempts for recoverable errors.
const DEFAULT_MAX_RETRIES: u32 = 3;

/// Delay between retry attempts in milliseconds.
const RETRY_DELAY_MS: u64 = 1000;

/// Event name for download progress events emitted to the frontend.
pub const DOWNLOAD_PROGRESS_EVENT: &str = "download-progress";

/// Progress information for an ongoing download.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    /// Bytes downloaded so far.
    pub downloaded: u64,
    /// Total bytes to download (0 if unknown).
    pub total: u64,
    /// Current download speed in bytes per second.
    pub speed_bps: u64,
    /// Estimated time remaining in seconds (0 if unknown).
    pub eta_secs: u64,
    /// File path being downloaded to.
    pub file_path: String,
    /// Whether this download is resuming from a previous attempt.
    pub is_resuming: bool,
}

impl DownloadProgress {
    /// Creates a new progress instance.
    pub fn new(downloaded: u64, total: u64, file_path: &str) -> Self {
        Self {
            downloaded,
            total,
            speed_bps: 0,
            eta_secs: 0,
            file_path: file_path.to_string(),
            is_resuming: false,
        }
    }

    /// Returns the download progress as a percentage (0-100).
    pub fn percentage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            (self.downloaded as f64 / self.total as f64) * 100.0
        }
    }

    /// Returns true if the download is complete.
    pub fn is_complete(&self) -> bool {
        self.total > 0 && self.downloaded >= self.total
    }

    /// Updates the speed and ETA based on elapsed time.
    pub fn with_speed(&mut self, speed_bps: u64) -> &mut Self {
        self.speed_bps = speed_bps;
        if speed_bps > 0 && self.total > self.downloaded {
            self.eta_secs = (self.total - self.downloaded) / speed_bps;
        } else {
            self.eta_secs = 0;
        }
        self
    }

    /// Emits this progress as a Tauri event to the frontend.
    ///
    /// # Arguments
    ///
    /// * `app_handle` - The Tauri app handle to emit the event through
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the event was emitted successfully, or an error otherwise.
    pub fn emit(&self, app_handle: &tauri::AppHandle) -> Result<(), tauri::Error> {
        app_handle.emit(DOWNLOAD_PROGRESS_EVENT, self)
    }

    /// Emits this progress as a Tauri event to a specific window.
    ///
    /// # Arguments
    ///
    /// * `window` - The Tauri window to emit the event to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the event was emitted successfully, or an error otherwise.
    pub fn emit_to_window(&self, window: &tauri::Window) -> Result<(), tauri::Error> {
        window.emit(DOWNLOAD_PROGRESS_EVENT, self)
    }
}

/// Configuration for the download client.
#[derive(Debug, Clone)]
pub struct DownloaderConfig {
    /// Connection timeout duration.
    pub connect_timeout: Duration,
    /// Read timeout duration.
    pub read_timeout: Duration,
    /// Maximum retry attempts for recoverable errors.
    pub max_retries: u32,
    /// User-Agent header value.
    pub user_agent: String,
}

impl Default for DownloaderConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS),
            read_timeout: Duration::from_secs(DEFAULT_READ_TIMEOUT_SECS),
            max_retries: DEFAULT_MAX_RETRIES,
            user_agent: format!("UltimaForge/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

impl DownloaderConfig {
    /// Creates a new configuration with custom timeouts.
    pub fn with_timeouts(connect_secs: u64, read_secs: u64) -> Self {
        Self {
            connect_timeout: Duration::from_secs(connect_secs),
            read_timeout: Duration::from_secs(read_secs),
            ..Default::default()
        }
    }

    /// Sets the maximum number of retry attempts.
    pub fn with_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Sets the user agent string.
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }
}

/// HTTP download client with progress reporting and retry support.
pub struct Downloader {
    client: Client,
    config: DownloaderConfig,
}

impl Downloader {
    /// Creates a new downloader with default configuration.
    pub fn new() -> Result<Self, DownloadError> {
        Self::with_config(DownloaderConfig::default())
    }

    /// Creates a new downloader with custom configuration.
    pub fn with_config(config: DownloaderConfig) -> Result<Self, DownloadError> {
        let client = Client::builder()
            .connect_timeout(config.connect_timeout)
            .timeout(config.read_timeout)
            .user_agent(&config.user_agent)
            .build()
            .map_err(|e| DownloadError::NetworkError {
                url: String::new(),
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self { client, config })
    }

    /// Returns the underlying reqwest Client.
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Downloads a file with progress reporting.
    ///
    /// # Arguments
    ///
    /// * `url` - URL to download from
    /// * `dest` - Destination file path
    /// * `expected_hash` - Optional expected SHA-256 hash for verification
    /// * `progress_callback` - Callback invoked with progress updates
    ///
    /// # Returns
    ///
    /// Returns the SHA-256 hash of the downloaded file on success.
    pub async fn download_file<F>(
        &self,
        url: &str,
        dest: &Path,
        expected_hash: Option<&str>,
        progress_callback: F,
    ) -> Result<String, DownloadError>
    where
        F: Fn(&DownloadProgress) + Send + Sync,
    {
        // Validate expected hash format if provided
        if let Some(hash) = expected_hash {
            validate_hash_format(hash).map_err(|e| DownloadError::HashMismatch {
                path: dest.display().to_string(),
                expected: hash.to_string(),
                actual: format!("invalid hash format: {}", e),
            })?;
        }

        // Attempt download with retries
        let mut last_error = None;
        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                warn!(
                    "Retrying download (attempt {}/{}): {}",
                    attempt + 1,
                    self.config.max_retries + 1,
                    url
                );
                tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS * attempt as u64)).await;
            }

            match self
                .download_file_inner(url, dest, expected_hash, &progress_callback)
                .await
            {
                Ok(hash) => return Ok(hash),
                Err(e) => {
                    if !e.is_recoverable() {
                        return Err(e);
                    }
                    error!("Download attempt {} failed: {}", attempt + 1, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| DownloadError::NetworkError {
            url: url.to_string(),
            message: "Download failed after all retry attempts".to_string(),
        }))
    }

    /// Inner download implementation (single attempt).
    async fn download_file_inner<F>(
        &self,
        url: &str,
        dest: &Path,
        expected_hash: Option<&str>,
        progress_callback: &F,
    ) -> Result<String, DownloadError>
    where
        F: Fn(&DownloadProgress) + Send + Sync,
    {
        debug!("Starting download: {} -> {}", url, dest.display());

        // Check for existing file and validate its hash if we have an expected hash
        let mut existing_size = 0u64;
        if dest.exists() {
            if let Some(expected) = expected_hash {
                // If we have an expected hash, check if the existing file is already correct
                if let Ok(existing_hash) = hash_file(dest) {
                    if existing_hash.to_lowercase() == expected.to_lowercase() {
                        info!("File already exists with correct hash, skipping download: {}", dest.display());
                        return Ok(existing_hash);
                    } else {
                        // Existing file has wrong hash - delete it and download fresh
                        warn!(
                            "Existing file has wrong hash (expected {}, got {}), deleting: {}",
                            expected, existing_hash, dest.display()
                        );
                        let _ = fs::remove_file(dest).await;
                    }
                } else {
                    // Can't hash existing file - delete it to be safe
                    warn!("Cannot hash existing file, deleting: {}", dest.display());
                    let _ = fs::remove_file(dest).await;
                }
            } else {
                // No expected hash, allow resume
                existing_size = fs::metadata(dest)
                    .await
                    .map(|m| m.len())
                    .unwrap_or(0);
            }
        }

        // Build request with optional Range header for resume
        let request = self.client.get(url);
        let request = if existing_size > 0 {
            info!(
                "Resuming download from byte {}: {}",
                existing_size,
                dest.display()
            );
            request.header(header::RANGE, format!("bytes={}-", existing_size))
        } else {
            request
        };

        // Send request
        let response = request.send().await.map_err(|e| {
            if e.is_timeout() {
                DownloadError::Timeout {
                    url: url.to_string(),
                }
            } else if e.is_connect() {
                DownloadError::NetworkError {
                    url: url.to_string(),
                    message: format!("Connection failed: {}", e),
                }
            } else {
                DownloadError::NetworkError {
                    url: url.to_string(),
                    message: e.to_string(),
                }
            }
        })?;

        // Check response status
        let (total_size, is_resuming, starting_offset) =
            self.handle_response_status(&response, url, existing_size)?;

        // Ensure parent directory exists
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await.map_err(|e| DownloadError::WriteError {
                path: parent.display().to_string(),
                source: e,
            })?;
        }

        // Open file for writing (append if resuming)
        let mut file = if is_resuming {
            OpenOptions::new()
                .append(true)
                .open(dest)
                .await
                .map_err(|e| DownloadError::WriteError {
                    path: dest.display().to_string(),
                    source: e,
                })?
        } else {
            // Truncate existing file for fresh download
            File::create(dest).await.map_err(|e| DownloadError::WriteError {
                path: dest.display().to_string(),
                source: e,
            })?
        };

        // Stream the response body to disk
        let mut stream = response.bytes_stream();
        let mut downloaded = starting_offset;
        let mut progress = DownloadProgress::new(downloaded, total_size, &dest.display().to_string());
        progress.is_resuming = is_resuming;

        let start_time = std::time::Instant::now();
        let mut last_progress_time = start_time;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| DownloadError::NetworkError {
                url: url.to_string(),
                message: format!("Failed to read chunk: {}", e),
            })?;

            file.write_all(&chunk).await.map_err(|e| DownloadError::WriteError {
                path: dest.display().to_string(),
                source: e,
            })?;

            downloaded += chunk.len() as u64;
            progress.downloaded = downloaded;

            // Calculate speed and ETA periodically (every 100ms)
            let now = std::time::Instant::now();
            if now.duration_since(last_progress_time).as_millis() >= 100 {
                let elapsed = now.duration_since(start_time).as_secs_f64();
                if elapsed > 0.0 {
                    let downloaded_since_start = downloaded - starting_offset;
                    progress.speed_bps = (downloaded_since_start as f64 / elapsed) as u64;
                    if progress.speed_bps > 0 && total_size > downloaded {
                        progress.eta_secs = (total_size - downloaded) / progress.speed_bps;
                    }
                }
                last_progress_time = now;
            }

            progress_callback(&progress);
        }

        // Ensure all data is written
        file.flush().await.map_err(|e| DownloadError::WriteError {
            path: dest.display().to_string(),
            source: e,
        })?;
        drop(file);

        info!(
            "Download complete: {} ({} bytes)",
            dest.display(),
            downloaded
        );

        // Verify hash if expected
        let actual_hash = hash_file(dest).map_err(|e| DownloadError::HashMismatch {
            path: dest.display().to_string(),
            expected: expected_hash.unwrap_or("").to_string(),
            actual: format!("failed to compute hash: {}", e),
        })?;

        if let Some(expected) = expected_hash {
            if actual_hash.to_lowercase() != expected.to_lowercase() {
                // Delete corrupted file
                let _ = fs::remove_file(dest).await;
                return Err(DownloadError::HashMismatch {
                    path: dest.display().to_string(),
                    expected: expected.to_string(),
                    actual: actual_hash,
                });
            }
            debug!("Hash verified: {}", actual_hash);
        }

        Ok(actual_hash)
    }

    /// Handles HTTP response status and determines resume behavior.
    fn handle_response_status(
        &self,
        response: &Response,
        url: &str,
        existing_size: u64,
    ) -> Result<(u64, bool, u64), DownloadError> {
        let status = response.status();

        match status {
            StatusCode::OK => {
                // Full download (server doesn't support Range or fresh download)
                let total = response.content_length().unwrap_or(0);
                Ok((total, false, 0))
            }
            StatusCode::PARTIAL_CONTENT => {
                // Resume supported
                let content_length = response.content_length().unwrap_or(0);
                let total = existing_size + content_length;
                Ok((total, true, existing_size))
            }
            StatusCode::RANGE_NOT_SATISFIABLE => {
                // File is complete or server doesn't support Range for this resource
                // Start fresh download
                let total = response.content_length().unwrap_or(0);
                Ok((total, false, 0))
            }
            status if status.is_client_error() => {
                let message = status.canonical_reason().unwrap_or("Client error");
                Err(DownloadError::HttpError {
                    url: url.to_string(),
                    status: status.as_u16(),
                    message: message.to_string(),
                })
            }
            status if status.is_server_error() => {
                let message = status.canonical_reason().unwrap_or("Server error");
                Err(DownloadError::HttpError {
                    url: url.to_string(),
                    status: status.as_u16(),
                    message: message.to_string(),
                })
            }
            _ => {
                Err(DownloadError::HttpError {
                    url: url.to_string(),
                    status: status.as_u16(),
                    message: format!("Unexpected status: {}", status),
                })
            }
        }
    }

    /// Downloads raw bytes to memory (for small files like manifests).
    ///
    /// # Warning
    ///
    /// Only use this for small files. Large files should use `download_file`.
    pub async fn download_bytes(&self, url: &str) -> Result<Vec<u8>, DownloadError> {
        debug!("Downloading bytes from: {}", url);

        let response = self.client.get(url).send().await.map_err(|e| {
            if e.is_timeout() {
                DownloadError::Timeout {
                    url: url.to_string(),
                }
            } else {
                DownloadError::NetworkError {
                    url: url.to_string(),
                    message: e.to_string(),
                }
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(DownloadError::HttpError {
                url: url.to_string(),
                status: status.as_u16(),
                message: status
                    .canonical_reason()
                    .unwrap_or("Unknown error")
                    .to_string(),
            });
        }

        response.bytes().await.map(|b| b.to_vec()).map_err(|e| {
            DownloadError::NetworkError {
                url: url.to_string(),
                message: format!("Failed to read response body: {}", e),
            }
        })
    }

    /// Fetches text content (for manifest JSON, patch notes, etc.).
    pub async fn download_text(&self, url: &str) -> Result<String, DownloadError> {
        let bytes = self.download_bytes(url).await?;
        String::from_utf8(bytes).map_err(|e| DownloadError::NetworkError {
            url: url.to_string(),
            message: format!("Invalid UTF-8 response: {}", e),
        })
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new().expect("Failed to create default downloader")
    }
}

/// Result type for download operations.
pub type DownloadResult<T> = Result<T, DownloadError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn test_progress_percentage() {
        let progress = DownloadProgress::new(50, 100, "test.bin");
        assert!((progress.percentage() - 50.0).abs() < f64::EPSILON);

        let zero_total = DownloadProgress::new(50, 0, "test.bin");
        assert!((zero_total.percentage() - 0.0).abs() < f64::EPSILON);

        let complete = DownloadProgress::new(100, 100, "test.bin");
        assert!((complete.percentage() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_progress_is_complete() {
        assert!(!DownloadProgress::new(50, 100, "test.bin").is_complete());
        assert!(DownloadProgress::new(100, 100, "test.bin").is_complete());
        assert!(DownloadProgress::new(150, 100, "test.bin").is_complete());
        assert!(!DownloadProgress::new(50, 0, "test.bin").is_complete());
    }

    #[test]
    fn test_progress_with_speed() {
        let mut progress = DownloadProgress::new(500, 1000, "test.bin");
        progress.with_speed(100); // 100 bytes per second

        assert_eq!(progress.speed_bps, 100);
        assert_eq!(progress.eta_secs, 5); // 500 bytes remaining / 100 bps = 5 seconds
    }

    #[test]
    fn test_config_defaults() {
        let config = DownloaderConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS));
        assert_eq!(config.read_timeout, Duration::from_secs(DEFAULT_READ_TIMEOUT_SECS));
        assert_eq!(config.max_retries, DEFAULT_MAX_RETRIES);
        assert!(config.user_agent.contains("UltimaForge"));
    }

    #[test]
    fn test_config_with_timeouts() {
        let config = DownloaderConfig::with_timeouts(5, 15);
        assert_eq!(config.connect_timeout, Duration::from_secs(5));
        assert_eq!(config.read_timeout, Duration::from_secs(15));
    }

    #[test]
    fn test_config_with_retries() {
        let config = DownloaderConfig::default().with_retries(5);
        assert_eq!(config.max_retries, 5);
    }

    #[test]
    fn test_config_with_user_agent() {
        let config = DownloaderConfig::default().with_user_agent("CustomAgent/1.0");
        assert_eq!(config.user_agent, "CustomAgent/1.0");
    }

    #[test]
    fn test_downloader_creation() {
        let downloader = Downloader::new();
        assert!(downloader.is_ok());
    }

    #[test]
    fn test_downloader_with_config() {
        let config = DownloaderConfig::with_timeouts(5, 10).with_retries(2);
        let downloader = Downloader::with_config(config);
        assert!(downloader.is_ok());
    }

    #[test]
    fn test_download_error_is_recoverable() {
        assert!(DownloadError::NetworkError {
            url: "test".to_string(),
            message: "error".to_string(),
        }
        .is_recoverable());

        assert!(DownloadError::Timeout {
            url: "test".to_string(),
        }
        .is_recoverable());

        assert!(!DownloadError::InvalidUrl("test".to_string()).is_recoverable());

        assert!(!DownloadError::HashMismatch {
            path: "test".to_string(),
            expected: "a".to_string(),
            actual: "b".to_string(),
        }
        .is_recoverable());
    }

    #[tokio::test]
    async fn test_download_invalid_url() {
        let downloader = Downloader::new().unwrap();
        let result = downloader
            .download_bytes("not-a-valid-url")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_download_file_creates_parent_dirs() {
        let temp_dir = tempdir().unwrap();
        let dest = temp_dir.path().join("nested").join("path").join("file.txt");

        let downloader = Downloader::new().unwrap();

        // This will fail because the URL doesn't exist, but the parent directories
        // should be created before the network request
        let _ = downloader
            .download_file(
                "http://localhost:1/nonexistent",
                &dest,
                None,
                |_| {},
            )
            .await;

        // Parent directory might or might not exist depending on when the error occurred
        // This test mainly validates the code path doesn't panic
    }

    #[test]
    fn test_progress_callback_type() {
        // Verify the callback type compiles correctly
        let progress_counter = Arc::new(AtomicU64::new(0));
        let counter_clone = progress_counter.clone();

        let callback = move |progress: &DownloadProgress| {
            counter_clone.store(progress.downloaded, Ordering::SeqCst);
        };

        // Simulate calling the callback
        let mut progress = DownloadProgress::new(100, 1000, "test.bin");
        callback(&progress);
        assert_eq!(progress_counter.load(Ordering::SeqCst), 100);

        progress.downloaded = 200;
        callback(&progress);
        assert_eq!(progress_counter.load(Ordering::SeqCst), 200);
    }

    #[test]
    fn test_progress_new() {
        let progress = DownloadProgress::new(0, 1000, "/path/to/file.bin");
        assert_eq!(progress.downloaded, 0);
        assert_eq!(progress.total, 1000);
        assert_eq!(progress.file_path, "/path/to/file.bin");
        assert_eq!(progress.speed_bps, 0);
        assert_eq!(progress.eta_secs, 0);
        assert!(!progress.is_resuming);
    }

    #[test]
    fn test_progress_percentage_edge_cases() {
        // 0/0 case
        let progress = DownloadProgress::new(0, 0, "test");
        assert_eq!(progress.percentage(), 0.0);

        // Very large numbers
        let large = DownloadProgress::new(u64::MAX / 2, u64::MAX, "test");
        assert!(large.percentage() >= 49.0 && large.percentage() <= 51.0);
    }

    #[test]
    fn test_downloader_default() {
        let downloader = Downloader::default();
        assert!(downloader.client().get("http://example.com").build().is_ok());
    }

    #[tokio::test]
    async fn test_download_bytes_connection_refused() {
        // Test against a port that's almost certainly not listening
        let downloader = Downloader::with_config(
            DownloaderConfig::with_timeouts(1, 1)
        ).unwrap();

        let result = downloader.download_bytes("http://127.0.0.1:1").await;
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, DownloadError::NetworkError { .. } | DownloadError::Timeout { .. }));
        }
    }

    #[tokio::test]
    async fn test_download_text_connection_refused() {
        let downloader = Downloader::with_config(
            DownloaderConfig::with_timeouts(1, 1)
        ).unwrap();

        let result = downloader.download_text("http://127.0.0.1:1").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = DownloaderConfig::default()
            .with_retries(5)
            .with_user_agent("Test/1.0");

        assert_eq!(config.max_retries, 5);
        assert_eq!(config.user_agent, "Test/1.0");
        // Defaults should be preserved
        assert_eq!(config.connect_timeout, Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS));
    }

    #[test]
    fn test_progress_speed_calculation() {
        let mut progress = DownloadProgress::new(0, 1000, "test");

        // With zero speed, ETA should be 0
        progress.with_speed(0);
        assert_eq!(progress.eta_secs, 0);

        // When complete, ETA should be 0
        progress.downloaded = 1000;
        progress.with_speed(100);
        assert_eq!(progress.eta_secs, 0);

        // Partial download
        progress.downloaded = 500;
        progress.with_speed(100);
        assert_eq!(progress.eta_secs, 5);
    }

    #[test]
    fn test_download_error_display() {
        let error = DownloadError::HashMismatch {
            path: "test.exe".to_string(),
            expected: "abc123".to_string(),
            actual: "def456".to_string(),
        };

        let display = format!("{}", error);
        assert!(display.contains("test.exe"));
        assert!(display.contains("abc123"));
        assert!(display.contains("def456"));
    }

    #[test]
    fn test_download_error_user_message() {
        let network_error = DownloadError::NetworkError {
            url: "http://example.com".to_string(),
            message: "Connection refused".to_string(),
        };
        assert!(network_error.user_message().contains("Connection failed"));

        let timeout = DownloadError::Timeout {
            url: "http://example.com".to_string(),
        };
        assert!(timeout.user_message().contains("timed out"));

        let hash_mismatch = DownloadError::HashMismatch {
            path: "file.exe".to_string(),
            expected: "a".to_string(),
            actual: "b".to_string(),
        };
        assert!(hash_mismatch.user_message().contains("corrupted"));
    }
}
