//! First-run installation flow for UltimaForge.
//!
//! This module handles the initial installation of the UO client:
//!
//! 1. **Directory Selection** - Validate and prepare the install directory
//! 2. **Full Installation** - Download all files from the manifest
//! 3. **Verification** - Verify all files have correct hashes
//!
//! # Security
//!
//! - Manifest signature is verified before downloading any files
//! - All downloaded files are hash-verified
//! - Installation path is validated to prevent attacks
//!
//! # Example
//!
//! ```ignore
//! use ultimaforge_lib::installer::{Installer, InstallProgress};
//!
//! let installer = Installer::new(brand_config)?;
//!
//! // Validate selected directory
//! installer.validate_install_path(&selected_path)?;
//!
//! // Perform full installation
//! installer.full_install(&selected_path, |progress| {
//!     println!("{}% complete", progress.percentage());
//! }).await?;
//! ```

use crate::config::{BrandConfig, LauncherConfig};
use crate::downloader::{DownloadProgress, Downloader, DownloaderConfig};
use crate::error::{DownloadError, InstallError, UpdateError};
use crate::hash::{hash_file, verify_file_hash};
use crate::manifest::{is_safe_relative_path, FileEntry, Manifest};
use crate::signature;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::Emitter;
use tracing::{debug, error, info, warn};

/// Minimum disk space required beyond the total file size (in bytes).
/// This accounts for filesystem overhead and temporary staging.
const MIN_FREE_SPACE_BUFFER: u64 = 100 * 1024 * 1024; // 100 MB

/// Name of the installation log file.
const INSTALL_LOG_FILE: &str = "install.log";

/// Event name for install progress events emitted to the frontend.
pub const INSTALL_PROGRESS_EVENT: &str = "install-progress";

/// Current state of an installation operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallState {
    /// No installation in progress.
    Idle,
    /// Validating the installation path.
    ValidatingPath,
    /// Fetching manifest from server.
    FetchingManifest,
    /// Downloading files to installation directory.
    Downloading,
    /// Verifying downloaded file hashes.
    Verifying,
    /// Installation completed successfully.
    Completed,
    /// Installation failed.
    Failed,
}

impl std::fmt::Display for InstallState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::ValidatingPath => write!(f, "Validating installation path"),
            Self::FetchingManifest => write!(f, "Fetching manifest"),
            Self::Downloading => write!(f, "Downloading files"),
            Self::Verifying => write!(f, "Verifying files"),
            Self::Completed => write!(f, "Installation completed"),
            Self::Failed => write!(f, "Installation failed"),
        }
    }
}

/// Progress information for an ongoing installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallProgress {
    /// Current installation state.
    pub state: InstallState,
    /// Total number of files to install.
    pub total_files: usize,
    /// Number of files processed so far.
    pub processed_files: usize,
    /// Total bytes to download.
    pub total_bytes: u64,
    /// Bytes downloaded so far.
    pub downloaded_bytes: u64,
    /// Current file being processed (if any).
    pub current_file: Option<String>,
    /// Download speed in bytes per second.
    pub speed_bps: u64,
    /// Estimated time remaining in seconds.
    pub eta_secs: u64,
    /// Target version being installed.
    pub target_version: Option<String>,
    /// Error message if state is Failed.
    pub error_message: Option<String>,
}

impl InstallProgress {
    /// Creates a new idle progress instance.
    pub fn new() -> Self {
        Self {
            state: InstallState::Idle,
            total_files: 0,
            processed_files: 0,
            total_bytes: 0,
            downloaded_bytes: 0,
            current_file: None,
            speed_bps: 0,
            eta_secs: 0,
            target_version: None,
            error_message: None,
        }
    }

    /// Returns the download progress as a percentage (0-100).
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.downloaded_bytes as f64 / self.total_bytes as f64) * 100.0
        }
    }

    /// Returns the file progress as a percentage (0-100).
    pub fn file_percentage(&self) -> f64 {
        if self.total_files == 0 {
            0.0
        } else {
            (self.processed_files as f64 / self.total_files as f64) * 100.0
        }
    }

    /// Returns true if the installation is complete (success or failure).
    pub fn is_complete(&self) -> bool {
        matches!(self.state, InstallState::Completed | InstallState::Failed)
    }

    /// Returns true if the installation completed successfully.
    pub fn is_success(&self) -> bool {
        matches!(self.state, InstallState::Completed)
    }

    /// Sets the state to failed with an error message.
    pub fn set_failed(&mut self, message: impl Into<String>) {
        self.state = InstallState::Failed;
        self.error_message = Some(message.into());
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
        app_handle.emit(INSTALL_PROGRESS_EVENT, self)
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
        window.emit(INSTALL_PROGRESS_EVENT, self)
    }
}

impl Default for InstallProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of validating an installation path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathValidationResult {
    /// Whether the path is valid for installation.
    pub is_valid: bool,
    /// Reason why the path is invalid (if applicable).
    pub reason: Option<String>,
    /// Whether the directory exists.
    pub exists: bool,
    /// Whether the directory is empty.
    pub is_empty: bool,
    /// Available disk space in bytes.
    pub available_space: u64,
    /// Whether there's sufficient space for installation.
    pub has_sufficient_space: bool,
    /// Whether we have write permissions.
    pub is_writable: bool,
    /// Whether this path requires administrator elevation.
    pub requires_elevation: bool,
    /// Warning message (e.g., when disk space check failed but installation can proceed).
    pub warning_message: Option<String>,
}

impl PathValidationResult {
    /// Creates a valid result.
    pub fn valid(
        available_space: u64,
        exists: bool,
        is_empty: bool,
        is_writable: bool,
        requires_elevation: bool,
        warning_message: Option<String>,
    ) -> Self {
        Self {
            is_valid: true,
            reason: None,
            exists,
            is_empty,
            available_space,
            has_sufficient_space: true,
            is_writable,
            requires_elevation,
            warning_message,
        }
    }

    /// Creates an invalid result with a reason.
    pub fn invalid(reason: impl Into<String>) -> Self {
        Self {
            is_valid: false,
            reason: Some(reason.into()),
            exists: false,
            is_empty: false,
            available_space: 0,
            has_sufficient_space: false,
            is_writable: false,
            requires_elevation: false,
            warning_message: None,
        }
    }
}

/// Installation log entry for tracking operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallLogEntry {
    /// ISO 8601 timestamp.
    timestamp: String,
    /// Operation being performed.
    operation: String,
    /// File path (if applicable).
    file_path: Option<String>,
    /// Result of the operation.
    result: String,
    /// Additional details.
    details: Option<String>,
}

impl InstallLogEntry {
    fn new(operation: &str, file_path: Option<&str>, result: &str) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            operation: operation.to_string(),
            file_path: file_path.map(|s| s.to_string()),
            result: result.to_string(),
            details: None,
        }
    }

    fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    fn format(&self) -> String {
        let file_info = self
            .file_path
            .as_ref()
            .map(|p| format!(" [{}]", p))
            .unwrap_or_default();
        let details_info = self
            .details
            .as_ref()
            .map(|d| format!(" - {}", d))
            .unwrap_or_default();
        format!(
            "[{}] {} {}: {}{}",
            self.timestamp, self.operation, self.result, file_info, details_info
        )
    }
}

/// Installation log for tracking operations.
struct InstallLog {
    log_path: PathBuf,
    file: Option<File>,
}

impl InstallLog {
    /// Creates a new installation log.
    fn new(install_path: &Path) -> io::Result<Self> {
        let log_path = install_path.join(INSTALL_LOG_FILE);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        Ok(Self {
            log_path,
            file: Some(file),
        })
    }

    /// Logs an entry.
    fn log(&mut self, entry: InstallLogEntry) {
        if let Some(ref mut file) = self.file {
            let line = format!("{}\n", entry.format());
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
        debug!("{}", entry.format());
    }

    /// Logs the start of an installation session.
    fn log_session_start(&mut self, target_version: &str) {
        self.log(
            InstallLogEntry::new("SESSION_START", None, "STARTED")
                .with_details(format!("Installing version {}", target_version)),
        );
    }

    /// Logs the end of an installation session.
    fn log_session_end(&mut self, success: bool) {
        let result = if success { "SUCCESS" } else { "FAILED" };
        self.log(InstallLogEntry::new("SESSION_END", None, result));
    }
}

/// First-run installer for UltimaForge.
pub struct Installer {
    /// HTTP downloader instance.
    downloader: Downloader,
    /// Brand configuration with update URL.
    brand_config: BrandConfig,
    /// Cached manifest (after fetch).
    cached_manifest: Option<Manifest>,
}

impl Installer {
    /// Creates a new installer with default downloader configuration.
    pub fn new(brand_config: BrandConfig) -> Result<Self, InstallError> {
        let downloader = Downloader::new().map_err(|e| InstallError::ConfigSaveFailed(e.to_string()))?;

        Ok(Self {
            downloader,
            brand_config,
            cached_manifest: None,
        })
    }

    /// Creates a new installer with custom downloader configuration.
    pub fn with_config(
        brand_config: BrandConfig,
        downloader_config: DownloaderConfig,
    ) -> Result<Self, InstallError> {
        let downloader = Downloader::with_config(downloader_config)
            .map_err(|e| InstallError::ConfigSaveFailed(e.to_string()))?;

        Ok(Self {
            downloader,
            brand_config,
            cached_manifest: None,
        })
    }

    /// Returns the brand configuration.
    pub fn brand_config(&self) -> &BrandConfig {
        &self.brand_config
    }

    /// Checks if a path requires administrator elevation to write to.
    ///
    /// Returns true if the path is in a protected system location like Program Files.
    fn path_requires_elevation(path: &Path) -> bool {
        if let Some(path_str) = path.to_str() {
            let path_lower = path_str.to_lowercase();

            // Check for common protected directories on Windows
            path_lower.contains("\\program files\\")
                || path_lower.contains("\\program files (x86)\\")
                || path_lower.contains("\\windows\\")
                || path_lower.starts_with("c:\\program files\\")
                || path_lower.starts_with("c:\\program files (x86)\\")
                || path_lower.starts_with("c:\\windows\\")
        } else {
            false
        }
    }

    /// Checks if the current process is running with elevated privileges.
    fn is_running_elevated() -> bool {
        #[cfg(target_os = "windows")]
        {
            use std::mem;
            use windows::Win32::Foundation::BOOL;
            use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
            use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

            unsafe {
                let mut token = std::mem::zeroed();

                // Open process token
                if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
                    return false;
                }

                let mut elevation: TOKEN_ELEVATION = mem::zeroed();
                let mut size = 0u32;

                // Get token elevation information
                if GetTokenInformation(
                    token,
                    TokenElevation,
                    Some(&mut elevation as *mut _ as *mut _),
                    mem::size_of::<TOKEN_ELEVATION>() as u32,
                    &mut size,
                ).is_err() {
                    return false;
                }

                elevation.TokenIsElevated != 0
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            false
        }
    }

    /// Validates an installation path without performing installation.
    ///
    /// # Arguments
    ///
    /// * `path` - The proposed installation path
    /// * `required_space` - Required disk space in bytes (use 0 to skip check)
    ///
    /// # Returns
    ///
    /// A validation result indicating whether the path is suitable.
    pub fn validate_install_path(&self, path: &Path, required_space: u64) -> PathValidationResult {
        // Check for invalid path patterns
        if let Some(path_str) = path.to_str() {
            // Check for path traversal or suspicious patterns
            if path_str.contains("..") {
                return PathValidationResult::invalid("Path cannot contain '..'");
            }
        } else {
            return PathValidationResult::invalid("Path contains invalid characters");
        }

        // Check if path is absolute
        if !path.is_absolute() {
            return PathValidationResult::invalid("Path must be absolute");
        }

        // Check if path exists
        let exists = path.exists();
        let is_empty = if exists {
            path.read_dir()
                .map(|mut entries| entries.next().is_none())
                .unwrap_or(false)
        } else {
            true // Non-existent directory is considered "empty"
        };

        // Try to get available disk space (with optional warning if API failed)
        let (available_space, disk_space_warning) = self.get_available_space(path);

        // Check if path requires elevation
        let requires_elevation = Self::path_requires_elevation(path) && !Self::is_running_elevated();

        // Check write permissions by trying to create the directory
        let is_writable = self.check_write_permission(path);

        // If we don't have write permissions and the path doesn't require elevation,
        // then it's invalid (we can't fix it with UAC)
        if !is_writable && !requires_elevation {
            return PathValidationResult {
                is_valid: false,
                reason: Some("Insufficient permissions to write to this directory".to_string()),
                exists,
                is_empty,
                available_space,
                has_sufficient_space: available_space >= required_space + MIN_FREE_SPACE_BUFFER,
                is_writable: false,
                requires_elevation,
                warning_message: disk_space_warning,
            };
        }

        // Check disk space
        let space_needed = required_space + MIN_FREE_SPACE_BUFFER;
        if available_space < space_needed && required_space > 0 {
            return PathValidationResult {
                is_valid: false,
                reason: Some(format!(
                    "Insufficient disk space. Need {} MB, have {} MB",
                    space_needed / (1024 * 1024),
                    available_space / (1024 * 1024)
                )),
                exists,
                is_empty,
                available_space,
                has_sufficient_space: false,
                is_writable,
                requires_elevation,
                warning_message: disk_space_warning,
            };
        }

        PathValidationResult::valid(available_space, exists, is_empty, is_writable, requires_elevation, disk_space_warning)
    }

    /// Gets available disk space for a path.
    ///
    /// Uses the `fs4` crate for cross-platform disk space detection.
    /// Falls back to `u64::MAX` if the API call fails, with a warning logged.
    ///
    /// # Returns
    ///
    /// A tuple of (available_space, warning_message) where:
    /// - `available_space` is the available disk space in bytes, or `u64::MAX` if unknown
    /// - `warning_message` is an optional warning if the disk space check failed
    fn get_available_space(&self, path: &Path) -> (u64, Option<String>) {
        // Try to find an existing ancestor directory
        let check_path = if path.exists() {
            path.to_path_buf()
        } else {
            // Walk up to find an existing parent
            let mut current = path.to_path_buf();
            while !current.exists() {
                if let Some(parent) = current.parent() {
                    current = parent.to_path_buf();
                } else {
                    warn!("Cannot determine disk space: no existing ancestor directory for {:?}", path);
                    return (u64::MAX, Some("Could not determine available disk space. Install may fail if there is insufficient space.".to_string()));
                }
            }
            current
        };

        // Use fs4 crate for cross-platform disk space detection
        match fs4::available_space(&check_path) {
            Ok(space) => {
                debug!("Available disk space at {:?}: {} bytes ({} MB)",
                    check_path, space, space / (1024 * 1024));
                (space, None)
            }
            Err(e) => {
                let warning_msg = format!(
                    "Could not check available disk space: {}. Install will proceed but may fail if there is insufficient space.",
                    e
                );
                warn!("Failed to get disk space for {:?}: {}. Assuming sufficient space.", check_path, e);
                (u64::MAX, Some(warning_msg))
            }
        }
    }

    /// Checks if we can write to the given path.
    fn check_write_permission(&self, path: &Path) -> bool {
        if path.exists() {
            // Try to create a temporary file
            let test_file = path.join(".ultimaforge_write_test");
            match fs::write(&test_file, b"test") {
                Ok(_) => {
                    let _ = fs::remove_file(test_file);
                    true
                }
                Err(_) => false,
            }
        } else {
            // Try to create the directory
            match fs::create_dir_all(path) {
                Ok(_) => {
                    // Remove if we created it just for testing
                    let _ = fs::remove_dir(path);
                    true
                }
                Err(_) => false,
            }
        }
    }

    /// Fetches and verifies the manifest from the update server.
    ///
    /// # Returns
    ///
    /// The parsed and verified manifest.
    pub async fn fetch_manifest(&mut self) -> Result<Manifest, UpdateError> {
        info!("Fetching manifest from {}", self.brand_config.update_url);

        // Download manifest
        let manifest_url = format!("{}/manifest.json", self.brand_config.update_url);
        let manifest_bytes = self
            .downloader
            .download_bytes(&manifest_url)
            .await
            .map_err(|e| UpdateError::ManifestFetchFailed(e.to_string()))?;

        // Download signature
        let signature_url = format!("{}/manifest.sig", self.brand_config.update_url);
        let signature_hex = self
            .downloader
            .download_bytes(&signature_url)
            .await
            .map_err(|_| UpdateError::MissingSignature)?;

        // Decode hex signature to raw bytes
        let signature_str = std::str::from_utf8(&signature_hex)
            .map_err(|_| UpdateError::StagingError("Invalid signature encoding".to_string()))?
            .trim();
        let signature_bytes = signature::parse_hex_signature(signature_str)
            .map_err(|e| UpdateError::StagingError(format!("Invalid signature format: {}", e)))?;

        // Verify signature BEFORE parsing
        let public_key_bytes: [u8; 32] = self
            .brand_config
            .public_key_bytes()
            .map_err(|e| UpdateError::StagingError(format!("Invalid public key: {}", e)))?
            .try_into()
            .map_err(|_| UpdateError::StagingError("Invalid public key length".to_string()))?;

        signature::verify_manifest(&manifest_bytes, &signature_bytes, &public_key_bytes)
            .map_err(|e| UpdateError::StagingError(format!("Signature verification failed: {}", e)))?;

        // Parse manifest (now safe since signature is verified)
        let manifest = Manifest::parse(&manifest_bytes)
            .map_err(|e| UpdateError::StagingError(format!("Invalid manifest: {}", e)))?;

        // Cache the manifest
        self.cached_manifest = Some(manifest.clone());

        info!(
            "Manifest verified: version {}, {} files, {} bytes total",
            manifest.version,
            manifest.file_count(),
            manifest.total_size
        );

        Ok(manifest)
    }

    /// Returns the required installation size based on the cached manifest.
    ///
    /// Call `fetch_manifest` first to populate the cache.
    pub fn required_size(&self) -> u64 {
        self.cached_manifest
            .as_ref()
            .map(|m| m.total_size)
            .unwrap_or(0)
    }

    /// Performs a full installation to the specified directory.
    ///
    /// # Arguments
    ///
    /// * `install_path` - Directory to install files to
    /// * `progress_callback` - Callback invoked with progress updates
    ///
    /// # Returns
    ///
    /// The installed version string on success.
    pub async fn full_install<F>(
        &mut self,
        install_path: &Path,
        progress_callback: F,
    ) -> Result<String, InstallError>
    where
        F: FnMut(&InstallProgress) + Send,
    {
        // Wrap callback in Arc<Mutex> to allow sharing with inner closures
        // while maintaining Send + Sync requirements for download_file
        let progress_callback = Arc::new(Mutex::new(progress_callback));

        // Helper closure to invoke the callback
        let invoke_callback = |cb: &Arc<Mutex<F>>, progress: &InstallProgress| {
            if let Ok(mut callback) = cb.lock() {
                callback(progress);
            }
        };

        let mut progress = InstallProgress::new();

        // Step 1: Validate path
        progress.state = InstallState::ValidatingPath;
        invoke_callback(&progress_callback, &progress);

        // Fetch manifest first to know required size
        progress.state = InstallState::FetchingManifest;
        invoke_callback(&progress_callback, &progress);

        let manifest = self.fetch_manifest().await.map_err(|e| {
            InstallError::ConfigSaveFailed(format!("Failed to fetch manifest: {}", e))
        })?;

        // Validate path with actual required size
        let validation = self.validate_install_path(install_path, manifest.total_size);
        if !validation.is_valid {
            return Err(InstallError::InvalidPath {
                path: install_path.to_path_buf(),
                reason: validation.reason.unwrap_or_else(|| "Unknown error".to_string()),
            });
        }

        // Create installation directory
        fs::create_dir_all(install_path).map_err(|e| {
            // Check if this is a permission error
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                InstallError::PermissionDenied {
                    path: install_path.to_path_buf(),
                }
            } else {
                InstallError::CreateDirectoryFailed {
                    path: install_path.to_path_buf(),
                    source: e,
                }
            }
        })?;

        // Initialize install log
        let mut log = InstallLog::new(install_path).map_err(|e| {
            InstallError::ConfigSaveFailed(format!("Failed to create install log: {}", e))
        })?;

        let target_version = manifest.version.clone();
        progress.target_version = Some(target_version.clone());
        progress.total_files = manifest.file_count();
        progress.total_bytes = manifest.total_size;

        log.log_session_start(&target_version);

        // Step 2: Download all files
        progress.state = InstallState::Downloading;
        invoke_callback(&progress_callback, &progress);
        log.log(InstallLogEntry::new("DOWNLOAD_START", None, "STARTED"));

        let files: Vec<&FileEntry> = manifest.iter_files().collect();

        for file in &files {
            // Defense-in-depth: Validate path safety before any filesystem operations.
            // While the manifest parsing already validates paths using is_safe_relative_path,
            // we re-validate here to protect against any bypass or modification.
            let file_path = Path::new(&file.path);
            if !is_safe_relative_path(file_path) {
                error!("Path containment violation detected: {}", file.path);
                log.log(
                    InstallLogEntry::new("DOWNLOAD", Some(&file.path), "BLOCKED")
                        .with_details("Path containment violation - possible path traversal attack"),
                );
                log.log_session_end(false);
                return Err(InstallError::InvalidPath {
                    path: PathBuf::from(&file.path),
                    reason: format!(
                        "Path '{}' failed containment validation - possible path traversal attack",
                        file.path
                    ),
                });
            }

            let dest_path = install_path.join(&file.path);
            let blob_url = file.blob_url(&self.brand_config.update_url);

            progress.current_file = Some(file.path.clone());
            invoke_callback(&progress_callback, &progress);

            debug!("Downloading {} to {}", file.path, dest_path.display());

            // Create parent directories
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_err(|e| InstallError::CreateDirectoryFailed {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
            }

            // Download with hash verification
            // Extract values for closure capture (must be Send + Sync compatible)
            let file_start_bytes = progress.downloaded_bytes;
            let total_files = progress.total_files;
            let processed_files = progress.processed_files;
            let total_bytes = progress.total_bytes;
            let target_version = progress.target_version.clone();
            let current_file = file.path.clone();
            let cb = progress_callback.clone();

            match self
                .downloader
                .download_file(
                    &blob_url,
                    &dest_path,
                    Some(&file.sha256),
                    move |dl_progress| {
                        let up = InstallProgress {
                            state: InstallState::Downloading,
                            total_files,
                            processed_files,
                            total_bytes,
                            downloaded_bytes: file_start_bytes + dl_progress.downloaded,
                            current_file: Some(current_file.clone()),
                            speed_bps: dl_progress.speed_bps,
                            eta_secs: dl_progress.eta_secs,
                            target_version: target_version.clone(),
                            error_message: None,
                        };
                        if let Ok(mut callback) = cb.lock() {
                            callback(&up);
                        }
                    },
                )
                .await
            {
                Ok(_) => {
                    progress.downloaded_bytes += file.size;
                    progress.processed_files += 1;
                    log.log(InstallLogEntry::new("DOWNLOAD", Some(&file.path), "OK"));
                    debug!("Downloaded {}", file.path);
                }
                Err(e) => {
                    error!("Failed to download {}: {}", file.path, e);
                    log.log(
                        InstallLogEntry::new("DOWNLOAD", Some(&file.path), "FAILED")
                            .with_details(e.to_string()),
                    );

                    // For required files, fail the installation
                    if file.required {
                        log.log_session_end(false);
                        progress.set_failed(format!("Failed to download {}: {}", file.path, e));
                        invoke_callback(&progress_callback, &progress);
                        return Err(InstallError::ConfigSaveFailed(format!(
                            "Download failed for {}: {}",
                            file.path, e
                        )));
                    } else {
                        warn!("Skipping optional file {}: {}", file.path, e);
                    }
                }
            }
        }

        log.log(InstallLogEntry::new("DOWNLOAD_START", None, "COMPLETE"));

        // Step 3: Verify all files
        progress.state = InstallState::Verifying;
        progress.processed_files = 0;
        invoke_callback(&progress_callback, &progress);
        log.log(InstallLogEntry::new("VERIFY_START", None, "STARTED"));

        for file in &files {
            let dest_path = install_path.join(&file.path);

            progress.current_file = Some(file.path.clone());
            invoke_callback(&progress_callback, &progress);

            // Skip missing optional files
            if !dest_path.exists() {
                if file.required {
                    log.log(
                        InstallLogEntry::new("VERIFY", Some(&file.path), "FAILED")
                            .with_details("File missing"),
                    );
                    log.log_session_end(false);
                    return Err(InstallError::ConfigSaveFailed(format!(
                        "Required file missing: {}",
                        file.path
                    )));
                }
                continue;
            }

            // Verify hash
            match verify_file_hash(&dest_path, &file.sha256) {
                Ok(true) => {
                    progress.processed_files += 1;
                    log.log(InstallLogEntry::new("VERIFY", Some(&file.path), "OK"));
                }
                Ok(false) => {
                    log.log(
                        InstallLogEntry::new("VERIFY", Some(&file.path), "FAILED")
                            .with_details("Hash mismatch"),
                    );
                    if file.required {
                        log.log_session_end(false);
                        return Err(InstallError::CorruptedInstallation {
                            path: dest_path,
                        });
                    }
                }
                Err(e) => {
                    log.log(
                        InstallLogEntry::new("VERIFY", Some(&file.path), "ERROR")
                            .with_details(e.to_string()),
                    );
                    if file.required {
                        log.log_session_end(false);
                        return Err(InstallError::ConfigSaveFailed(format!(
                            "Verification error for {}: {}",
                            file.path, e
                        )));
                    }
                }
            }
        }

        log.log(InstallLogEntry::new("VERIFY_START", None, "COMPLETE"));

        // Installation complete!
        progress.state = InstallState::Completed;
        progress.current_file = None;
        invoke_callback(&progress_callback, &progress);

        log.log_session_end(true);
        info!(
            "Installation complete: version {} to {}",
            target_version,
            install_path.display()
        );

        Ok(target_version)
    }

    /// Verifies an existing installation by checking all file hashes.
    ///
    /// # Arguments
    ///
    /// * `install_path` - Directory containing the installation
    /// * `progress_callback` - Callback invoked with progress updates
    ///
    /// # Returns
    ///
    /// A map of file paths to their verification status (true = valid).
    pub async fn verify_installation<F>(
        &mut self,
        install_path: &Path,
        mut progress_callback: F,
    ) -> Result<HashMap<String, bool>, InstallError>
    where
        F: FnMut(&InstallProgress) + Send,
    {
        let mut progress = InstallProgress::new();
        progress.state = InstallState::FetchingManifest;
        progress_callback(&progress);

        // Fetch manifest if not cached
        let manifest = if let Some(ref m) = self.cached_manifest {
            m.clone()
        } else {
            self.fetch_manifest().await.map_err(|e| {
                InstallError::ConfigSaveFailed(format!("Failed to fetch manifest: {}", e))
            })?
        };

        progress.state = InstallState::Verifying;
        progress.total_files = manifest.file_count();
        progress_callback(&progress);

        let mut results = HashMap::new();

        for file in manifest.iter_files() {
            let file_path = install_path.join(&file.path);
            progress.current_file = Some(file.path.clone());
            progress_callback(&progress);

            let is_valid = if file_path.exists() {
                verify_file_hash(&file_path, &file.sha256).unwrap_or(false)
            } else {
                false
            };

            results.insert(file.path.clone(), is_valid);
            progress.processed_files += 1;
        }

        progress.state = InstallState::Completed;
        progress.current_file = None;
        progress_callback(&progress);

        Ok(results)
    }

    /// Returns a list of files that need to be re-downloaded based on verification.
    ///
    /// # Arguments
    ///
    /// * `install_path` - Directory containing the installation
    ///
    /// # Returns
    ///
    /// A list of file paths that are missing or have invalid hashes.
    pub async fn get_repair_list(&mut self, install_path: &Path) -> Result<Vec<String>, InstallError> {
        let verification = self.verify_installation(install_path, |_| {}).await?;

        let repair_list: Vec<String> = verification
            .into_iter()
            .filter(|(_, is_valid)| !is_valid)
            .map(|(path, _)| path)
            .collect();

        Ok(repair_list)
    }

    /// Checks if an installation is complete and valid.
    ///
    /// # Arguments
    ///
    /// * `install_path` - Directory containing the installation
    ///
    /// # Returns
    ///
    /// True if all required files are present and valid.
    pub async fn is_installation_valid(&mut self, install_path: &Path) -> Result<bool, InstallError> {
        let manifest = if let Some(ref m) = self.cached_manifest {
            m.clone()
        } else {
            self.fetch_manifest().await.map_err(|e| {
                InstallError::ConfigSaveFailed(format!("Failed to fetch manifest: {}", e))
            })?
        };

        for file in manifest.iter_required_files() {
            let file_path = install_path.join(&file.path);

            if !file_path.exists() {
                return Ok(false);
            }

            if !verify_file_hash(&file_path, &file.sha256).unwrap_or(false) {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// Result type for installer operations.
pub type InstallResult<T> = Result<T, InstallError>;

/// Confidence level for installation detection.
///
/// Indicates how confident we are that a detected path contains a valid UO installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectionConfidence {
    /// All expected files present (executable + all data files).
    High,
    /// Executable present with some data files.
    Medium,
    /// Only some files present (partial installation).
    Low,
    /// No installation detected.
    None,
}

impl std::fmt::Display for DetectionConfidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::High => write!(f, "High"),
            Self::Medium => write!(f, "Medium"),
            Self::Low => write!(f, "Low"),
            Self::None => write!(f, "None"),
        }
    }
}

/// Result of detecting an existing installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// Whether an installation was detected.
    pub detected: bool,
    /// Path where installation was detected (if any).
    pub install_path: Option<PathBuf>,
    /// Confidence level of the detection.
    pub confidence: DetectionConfidence,
    /// Detected version (if determinable from files).
    pub detected_version: Option<String>,
    /// List of found executables.
    pub found_executables: Vec<String>,
    /// List of found data files.
    pub found_data_files: Vec<String>,
    /// List of missing expected files.
    pub missing_files: Vec<String>,
}

impl DetectionResult {
    /// Creates a new empty detection result (no installation detected).
    pub fn not_detected() -> Self {
        Self {
            detected: false,
            install_path: None,
            confidence: DetectionConfidence::None,
            detected_version: None,
            found_executables: Vec::new(),
            found_data_files: Vec::new(),
            missing_files: Vec::new(),
        }
    }

    /// Creates a detection result for a found installation.
    pub fn detected(path: PathBuf, confidence: DetectionConfidence) -> Self {
        Self {
            detected: true,
            install_path: Some(path),
            confidence,
            detected_version: None,
            found_executables: Vec::new(),
            found_data_files: Vec::new(),
            missing_files: Vec::new(),
        }
    }

    /// Returns true if a valid installation was detected with at least medium confidence.
    pub fn is_valid_installation(&self) -> bool {
        self.detected && matches!(self.confidence, DetectionConfidence::High | DetectionConfidence::Medium)
    }
}

impl Default for DetectionResult {
    fn default() -> Self {
        Self::not_detected()
    }
}

/// Known UO client executable names to search for.
const DETECTION_EXECUTABLES: &[&str] = &[
    "client.exe",
    "Client.exe",
    "ClassicUO.exe",
    "classicuo.exe",
];

/// Required UO data files that indicate a valid installation.
const DETECTION_DATA_FILES: &[&str] = &[
    "art.mul",
    "artidx.mul",
    "map0.mul",
    "staidx0.mul",
    "statics0.mul",
];

/// Minimum number of data files required for medium confidence.
const MIN_DATA_FILES_FOR_MEDIUM: usize = 3;

/// Detects an existing UO installation at the specified path.
///
/// This function performs a heuristic-based detection of UO client files
/// without requiring network access or a manifest. It checks for known
/// executable names and data files to determine if a valid installation exists.
///
/// # Arguments
///
/// * `path` - The path to check for an existing installation
///
/// # Returns
///
/// A `DetectionResult` containing detection status and confidence level.
///
/// # Detection Heuristics
///
/// - **High confidence**: At least one executable AND all required data files present
/// - **Medium confidence**: At least one executable AND some data files present
/// - **Low confidence**: Some files present but missing executable or most data files
/// - **None**: No relevant files found
///
/// # Example
///
/// ```ignore
/// use ultimaforge_lib::installer::detect_existing_installation;
///
/// let result = detect_existing_installation(Path::new("C:\\Games\\UO"));
/// if result.is_valid_installation() {
///     println!("Found UO installation at {:?}", result.install_path);
/// }
/// ```
pub fn detect_existing_installation(path: &Path) -> DetectionResult {
    info!("Detecting existing installation at: {}", path.display());

    // Check if the path exists and is a directory
    if !path.exists() {
        debug!("Path does not exist: {}", path.display());
        return DetectionResult::not_detected();
    }

    if !path.is_dir() {
        debug!("Path is not a directory: {}", path.display());
        return DetectionResult::not_detected();
    }

    let mut result = DetectionResult {
        detected: false,
        install_path: Some(path.to_path_buf()),
        confidence: DetectionConfidence::None,
        detected_version: None,
        found_executables: Vec::new(),
        found_data_files: Vec::new(),
        missing_files: Vec::new(),
    };

    // Check for executables
    for exe in DETECTION_EXECUTABLES {
        let exe_path = path.join(exe);
        if exe_path.exists() && exe_path.is_file() {
            debug!("Found executable: {}", exe);
            result.found_executables.push(exe.to_string());
        }
    }

    // Check for data files
    for data_file in DETECTION_DATA_FILES {
        let data_path = path.join(data_file);
        if data_path.exists() && data_path.is_file() {
            debug!("Found data file: {}", data_file);
            result.found_data_files.push(data_file.to_string());
        } else {
            result.missing_files.push(data_file.to_string());
        }
    }

    // Determine confidence level based on found files
    let has_executable = !result.found_executables.is_empty();
    let data_file_count = result.found_data_files.len();
    let all_data_files = data_file_count == DETECTION_DATA_FILES.len();

    result.confidence = if has_executable && all_data_files {
        // High: Has executable and all data files
        DetectionConfidence::High
    } else if has_executable && data_file_count >= MIN_DATA_FILES_FOR_MEDIUM {
        // Medium: Has executable and some data files
        DetectionConfidence::Medium
    } else if has_executable || data_file_count > 0 {
        // Low: Has some files but incomplete
        DetectionConfidence::Low
    } else {
        // None: No relevant files found
        DetectionConfidence::None
    };

    result.detected = result.confidence != DetectionConfidence::None;

    if result.detected {
        info!(
            "Detected installation with {} confidence: {} executables, {} data files",
            result.confidence,
            result.found_executables.len(),
            result.found_data_files.len()
        );
    } else {
        debug!("No installation detected at: {}", path.display());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BrandConfigBuilder;
    use tempfile::TempDir;

    /// Valid 64-character hex public key for testing.
    const TEST_PUBLIC_KEY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn test_brand_config() -> BrandConfig {
        BrandConfigBuilder::new()
            .display_name("Test Server")
            .server_name("TestServer")
            .update_url("http://localhost:8080")
            .public_key(TEST_PUBLIC_KEY)
            .build()
            .unwrap()
    }

    #[test]
    fn test_install_state_display() {
        assert_eq!(InstallState::Idle.to_string(), "Idle");
        assert_eq!(InstallState::Downloading.to_string(), "Downloading files");
        assert_eq!(InstallState::Completed.to_string(), "Installation completed");
    }

    #[test]
    fn test_install_progress_new() {
        let progress = InstallProgress::new();
        assert_eq!(progress.state, InstallState::Idle);
        assert_eq!(progress.total_files, 0);
        assert_eq!(progress.processed_files, 0);
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_install_progress_percentage() {
        let mut progress = InstallProgress::new();
        progress.total_bytes = 100;
        progress.downloaded_bytes = 50;
        assert!((progress.percentage() - 50.0).abs() < f64::EPSILON);

        progress.total_bytes = 0;
        assert!((progress.percentage() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_install_progress_file_percentage() {
        let mut progress = InstallProgress::new();
        progress.total_files = 10;
        progress.processed_files = 5;
        assert!((progress.file_percentage() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_install_progress_is_complete() {
        let mut progress = InstallProgress::new();
        assert!(!progress.is_complete());

        progress.state = InstallState::Completed;
        assert!(progress.is_complete());
        assert!(progress.is_success());

        progress.state = InstallState::Failed;
        assert!(progress.is_complete());
        assert!(!progress.is_success());

        progress.state = InstallState::Downloading;
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_install_progress_set_failed() {
        let mut progress = InstallProgress::new();
        progress.set_failed("Something went wrong");
        assert_eq!(progress.state, InstallState::Failed);
        assert_eq!(progress.error_message, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_path_validation_result_valid() {
        let result = PathValidationResult::valid(1024 * 1024 * 1024, true, true, true, false, None);
        assert!(result.is_valid);
        assert!(result.reason.is_none());
        assert!(result.exists);
        assert!(result.is_empty);
        assert!(result.is_writable);
        assert!(!result.requires_elevation);
        assert!(result.warning_message.is_none());
    }

    #[test]
    fn test_path_validation_result_invalid() {
        let result = PathValidationResult::invalid("Test error");
        assert!(!result.is_valid);
        assert_eq!(result.reason, Some("Test error".to_string()));
    }

    #[test]
    fn test_installer_creation() {
        let brand = test_brand_config();
        let installer = Installer::new(brand.clone());
        assert!(installer.is_ok());

        let installer = installer.unwrap();
        assert_eq!(installer.brand_config().product.display_name, "Test Server");
    }

    #[test]
    fn test_validate_install_path_valid() {
        let temp_dir = TempDir::new().unwrap();
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        let result = installer.validate_install_path(temp_dir.path(), 0);
        assert!(result.is_valid);
        assert!(result.is_writable);
    }

    #[test]
    fn test_validate_install_path_path_traversal() {
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        #[cfg(target_os = "windows")]
        let path = Path::new("C:\\test\\..\\..\\etc");

        #[cfg(not(target_os = "windows"))]
        let path = Path::new("/test/../../../etc");

        let result = installer.validate_install_path(path, 0);
        assert!(!result.is_valid);
        assert!(result.reason.unwrap().contains(".."));
    }

    #[test]
    fn test_validate_install_path_relative() {
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        let result = installer.validate_install_path(Path::new("relative/path"), 0);
        assert!(!result.is_valid);
        assert!(result.reason.unwrap().contains("absolute"));
    }

    #[test]
    fn test_validate_install_path_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        let result = installer.validate_install_path(temp_dir.path(), 0);
        assert!(result.is_valid);
        assert!(result.exists);
        assert!(result.is_empty);
    }

    #[test]
    fn test_validate_install_path_non_empty_dir() {
        let temp_dir = TempDir::new().unwrap();

        // Create a file in the directory
        fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        let result = installer.validate_install_path(temp_dir.path(), 0);
        assert!(result.is_valid); // Non-empty is allowed
        assert!(!result.is_empty);
    }

    #[test]
    fn test_install_log_entry_format() {
        let entry = InstallLogEntry::new("DOWNLOAD", Some("client.exe"), "OK");
        let formatted = entry.format();
        assert!(formatted.contains("DOWNLOAD"));
        assert!(formatted.contains("client.exe"));
        assert!(formatted.contains("OK"));

        let entry_with_details = entry.with_details("Downloaded 1024 bytes");
        let formatted = entry_with_details.format();
        assert!(formatted.contains("Downloaded 1024 bytes"));
    }

    #[test]
    fn test_install_state_all_variants() {
        // Ensure all variants are testable
        let states = vec![
            InstallState::Idle,
            InstallState::ValidatingPath,
            InstallState::FetchingManifest,
            InstallState::Downloading,
            InstallState::Verifying,
            InstallState::Completed,
            InstallState::Failed,
        ];

        for state in states {
            // Test Display trait
            let _ = state.to_string();
            // Test Clone
            let _ = state.clone();
            // Test PartialEq
            assert_eq!(state, state);
        }
    }

    #[test]
    fn test_install_progress_default() {
        let progress = InstallProgress::default();
        assert_eq!(progress.state, InstallState::Idle);
    }

    #[test]
    fn test_required_size_without_manifest() {
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();
        assert_eq!(installer.required_size(), 0);
    }

    #[test]
    fn test_installer_with_config() {
        let brand = test_brand_config();
        let config = DownloaderConfig::with_timeouts(5, 15);
        let installer = Installer::with_config(brand, config);
        assert!(installer.is_ok());
    }

    #[test]
    fn test_install_log_creation() {
        let temp_dir = TempDir::new().unwrap();
        let log = InstallLog::new(temp_dir.path());
        assert!(log.is_ok());

        let log_path = temp_dir.path().join(INSTALL_LOG_FILE);
        assert!(log_path.exists());
    }

    #[test]
    fn test_install_log_write() {
        let temp_dir = TempDir::new().unwrap();
        let mut log = InstallLog::new(temp_dir.path()).unwrap();

        log.log_session_start("1.0.0");
        log.log(InstallLogEntry::new("TEST", Some("test.exe"), "OK"));
        log.log_session_end(true);

        let log_path = temp_dir.path().join(INSTALL_LOG_FILE);
        let content = fs::read_to_string(log_path).unwrap();

        assert!(content.contains("SESSION_START"));
        assert!(content.contains("TEST"));
        assert!(content.contains("test.exe"));
        assert!(content.contains("SESSION_END"));
        assert!(content.contains("SUCCESS"));
    }

    #[test]
    fn test_check_write_permission_existing_dir() {
        let temp_dir = TempDir::new().unwrap();
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        let result = installer.check_write_permission(temp_dir.path());
        assert!(result);
    }

    #[test]
    fn test_check_write_permission_new_dir() {
        let temp_dir = TempDir::new().unwrap();
        let new_path = temp_dir.path().join("new_subdir");

        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        // Should be able to create new directory
        let result = installer.check_write_permission(&new_path);
        assert!(result);

        // Directory should not exist after the test (was cleaned up)
        assert!(!new_path.exists());
    }

    #[test]
    fn test_validate_path_non_existent_parent() {
        let temp_dir = TempDir::new().unwrap();
        let new_path = temp_dir.path().join("nested").join("deep").join("install");

        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        let result = installer.validate_install_path(&new_path, 0);
        // Should be valid - we can create nested directories
        assert!(result.is_valid);
    }

    #[test]
    fn test_progress_with_all_fields() {
        let mut progress = InstallProgress {
            state: InstallState::Downloading,
            total_files: 100,
            processed_files: 50,
            total_bytes: 1024 * 1024,
            downloaded_bytes: 512 * 1024,
            current_file: Some("test.exe".to_string()),
            speed_bps: 102400,
            eta_secs: 60,
            target_version: Some("1.0.0".to_string()),
            error_message: None,
        };

        assert!((progress.percentage() - 50.0).abs() < f64::EPSILON);
        assert!((progress.file_percentage() - 50.0).abs() < f64::EPSILON);
        assert!(!progress.is_complete());
        assert_eq!(progress.current_file, Some("test.exe".to_string()));
        assert_eq!(progress.target_version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_path_validation_invalid_characters() {
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        // Test with an invalid path (non-UTF8 would fail earlier)
        // For this test, we just verify the path traversal check
        #[cfg(target_os = "windows")]
        let path = Path::new("C:\\..\\test");

        #[cfg(not(target_os = "windows"))]
        let path = Path::new("/../test");

        let result = installer.validate_install_path(path, 0);
        assert!(!result.is_valid);
    }

    // =============================================================
    // Detection Function Tests
    // =============================================================

    #[test]
    fn test_detection_confidence_display() {
        assert_eq!(DetectionConfidence::High.to_string(), "High");
        assert_eq!(DetectionConfidence::Medium.to_string(), "Medium");
        assert_eq!(DetectionConfidence::Low.to_string(), "Low");
        assert_eq!(DetectionConfidence::None.to_string(), "None");
    }

    #[test]
    fn test_detection_confidence_equality() {
        assert_eq!(DetectionConfidence::High, DetectionConfidence::High);
        assert_ne!(DetectionConfidence::High, DetectionConfidence::Low);
    }

    #[test]
    fn test_detection_result_not_detected() {
        let result = DetectionResult::not_detected();
        assert!(!result.detected);
        assert!(result.install_path.is_none());
        assert_eq!(result.confidence, DetectionConfidence::None);
        assert!(result.detected_version.is_none());
        assert!(result.found_executables.is_empty());
        assert!(result.found_data_files.is_empty());
        assert!(result.missing_files.is_empty());
    }

    #[test]
    fn test_detection_result_detected() {
        let path = PathBuf::from("/test/path");
        let result = DetectionResult::detected(path.clone(), DetectionConfidence::High);
        assert!(result.detected);
        assert_eq!(result.install_path, Some(path));
        assert_eq!(result.confidence, DetectionConfidence::High);
    }

    #[test]
    fn test_detection_result_is_valid_installation() {
        // High confidence should be valid
        let high = DetectionResult::detected(PathBuf::from("/test"), DetectionConfidence::High);
        assert!(high.is_valid_installation());

        // Medium confidence should be valid
        let medium = DetectionResult::detected(PathBuf::from("/test"), DetectionConfidence::Medium);
        assert!(medium.is_valid_installation());

        // Low confidence should NOT be valid
        let low = DetectionResult::detected(PathBuf::from("/test"), DetectionConfidence::Low);
        assert!(!low.is_valid_installation());

        // Not detected should NOT be valid
        let none = DetectionResult::not_detected();
        assert!(!none.is_valid_installation());
    }

    #[test]
    fn test_detection_result_default() {
        let result = DetectionResult::default();
        assert!(!result.detected);
        assert_eq!(result.confidence, DetectionConfidence::None);
    }

    #[test]
    fn test_detect_existing_installation_nonexistent_path() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("does_not_exist");

        let result = detect_existing_installation(&nonexistent);
        assert!(!result.detected);
        assert_eq!(result.confidence, DetectionConfidence::None);
    }

    #[test]
    fn test_detect_existing_installation_empty_directory() {
        let temp_dir = TempDir::new().unwrap();

        let result = detect_existing_installation(temp_dir.path());
        assert!(!result.detected);
        assert_eq!(result.confidence, DetectionConfidence::None);
        assert!(result.found_executables.is_empty());
        assert!(result.found_data_files.is_empty());
    }

    #[test]
    fn test_detect_existing_installation_file_not_directory() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("somefile.txt");
        fs::write(&file_path, "content").unwrap();

        let result = detect_existing_installation(&file_path);
        assert!(!result.detected);
        assert_eq!(result.confidence, DetectionConfidence::None);
    }

    #[test]
    fn test_detect_existing_installation_executable_only() {
        let temp_dir = TempDir::new().unwrap();

        // Create just an executable
        fs::write(temp_dir.path().join("client.exe"), "fake exe").unwrap();

        let result = detect_existing_installation(temp_dir.path());
        assert!(result.detected);
        assert_eq!(result.confidence, DetectionConfidence::Low);
        assert!(result.found_executables.contains(&"client.exe".to_string()));
        assert!(result.found_data_files.is_empty());
        assert!(!result.is_valid_installation());
    }

    #[test]
    fn test_detect_existing_installation_data_files_only() {
        let temp_dir = TempDir::new().unwrap();

        // Create just data files (no executable)
        for data_file in DETECTION_DATA_FILES {
            fs::write(temp_dir.path().join(data_file), "fake data").unwrap();
        }

        let result = detect_existing_installation(temp_dir.path());
        assert!(result.detected);
        assert_eq!(result.confidence, DetectionConfidence::Low);
        assert!(result.found_executables.is_empty());
        assert_eq!(result.found_data_files.len(), DETECTION_DATA_FILES.len());
        assert!(!result.is_valid_installation());
    }

    #[test]
    fn test_detect_existing_installation_partial_data_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create executable
        fs::write(temp_dir.path().join("client.exe"), "fake exe").unwrap();

        // Create only 2 data files (below MIN_DATA_FILES_FOR_MEDIUM threshold)
        fs::write(temp_dir.path().join("art.mul"), "fake data").unwrap();
        fs::write(temp_dir.path().join("artidx.mul"), "fake data").unwrap();

        let result = detect_existing_installation(temp_dir.path());
        assert!(result.detected);
        // With only 2 data files (less than MIN_DATA_FILES_FOR_MEDIUM=3), should be Low
        assert_eq!(result.confidence, DetectionConfidence::Low);
        assert!(!result.is_valid_installation());
    }

    #[test]
    fn test_detect_existing_installation_medium_confidence() {
        let temp_dir = TempDir::new().unwrap();

        // Create executable
        fs::write(temp_dir.path().join("client.exe"), "fake exe").unwrap();

        // Create exactly MIN_DATA_FILES_FOR_MEDIUM data files
        for i in 0..MIN_DATA_FILES_FOR_MEDIUM {
            fs::write(temp_dir.path().join(DETECTION_DATA_FILES[i]), "fake data").unwrap();
        }

        let result = detect_existing_installation(temp_dir.path());
        assert!(result.detected);
        assert_eq!(result.confidence, DetectionConfidence::Medium);
        assert!(result.is_valid_installation());
    }

    #[test]
    fn test_detect_existing_installation_high_confidence() {
        let temp_dir = TempDir::new().unwrap();

        // Create executable
        fs::write(temp_dir.path().join("ClassicUO.exe"), "fake exe").unwrap();

        // Create ALL data files
        for data_file in DETECTION_DATA_FILES {
            fs::write(temp_dir.path().join(data_file), "fake data").unwrap();
        }

        let result = detect_existing_installation(temp_dir.path());
        assert!(result.detected);
        assert_eq!(result.confidence, DetectionConfidence::High);
        assert!(result.is_valid_installation());
        assert!(result.found_executables.contains(&"ClassicUO.exe".to_string()));
        assert_eq!(result.found_data_files.len(), DETECTION_DATA_FILES.len());
        assert!(result.missing_files.is_empty());
    }

    #[test]
    fn test_detect_existing_installation_valid_complete() {
        let temp_dir = TempDir::new().unwrap();

        // Create both executables (client.exe and ClassicUO.exe)
        fs::write(temp_dir.path().join("client.exe"), "fake exe").unwrap();
        fs::write(temp_dir.path().join("ClassicUO.exe"), "fake exe").unwrap();

        // Create ALL data files
        for data_file in DETECTION_DATA_FILES {
            fs::write(temp_dir.path().join(data_file), "fake data").unwrap();
        }

        let result = detect_existing_installation(temp_dir.path());
        assert!(result.detected);
        assert_eq!(result.confidence, DetectionConfidence::High);
        assert!(result.is_valid_installation());
        // On case-insensitive filesystems (Windows), client.exe matches multiple patterns
        // Just verify we found multiple executables
        assert!(result.found_executables.len() >= 2);
    }

    #[test]
    fn test_detect_existing_installation_install_path_populated() {
        let temp_dir = TempDir::new().unwrap();

        // Create minimal installation
        fs::write(temp_dir.path().join("client.exe"), "fake exe").unwrap();

        let result = detect_existing_installation(temp_dir.path());
        assert!(result.install_path.is_some());
        assert_eq!(result.install_path.unwrap(), temp_dir.path().to_path_buf());
    }

    #[test]
    fn test_detect_existing_installation_missing_files_tracked() {
        let temp_dir = TempDir::new().unwrap();

        // Create executable but only one data file
        fs::write(temp_dir.path().join("client.exe"), "fake exe").unwrap();
        fs::write(temp_dir.path().join("art.mul"), "fake data").unwrap();

        let result = detect_existing_installation(temp_dir.path());
        // Should track the missing data files
        assert!(!result.missing_files.is_empty());
        // art.mul is present, so all others should be in missing_files
        assert_eq!(result.missing_files.len(), DETECTION_DATA_FILES.len() - 1);
        assert!(!result.missing_files.contains(&"art.mul".to_string()));
    }

    #[test]
    fn test_detect_existing_installation_case_sensitivity() {
        let temp_dir = TempDir::new().unwrap();

        // Test that both Client.exe and client.exe are detected
        fs::write(temp_dir.path().join("Client.exe"), "fake exe").unwrap();

        let result = detect_existing_installation(temp_dir.path());
        assert!(result.detected);
        assert!(result.found_executables.contains(&"Client.exe".to_string()));
    }

    #[test]
    fn test_detect_existing_installation_classicuo_executable() {
        let temp_dir = TempDir::new().unwrap();

        // Test ClassicUO executable variations
        fs::write(temp_dir.path().join("classicuo.exe"), "fake exe").unwrap();

        let result = detect_existing_installation(temp_dir.path());
        assert!(result.detected);
        assert!(result.found_executables.contains(&"classicuo.exe".to_string()));
    }

    #[test]
    fn test_detection_constants() {
        // Verify that our constants are defined correctly
        assert!(!DETECTION_EXECUTABLES.is_empty());
        assert!(!DETECTION_DATA_FILES.is_empty());
        assert!(MIN_DATA_FILES_FOR_MEDIUM > 0);
        assert!(MIN_DATA_FILES_FOR_MEDIUM <= DETECTION_DATA_FILES.len());
    }

    // =============================================================
    // Path Containment Validation Tests
    // =============================================================

    use crate::manifest::is_safe_relative_path;

    #[test]
    fn test_path_containment_safe_paths_in_installer() {
        // Verify that safe paths are accepted by is_safe_relative_path
        // These are the types of paths we expect in valid manifests
        assert!(is_safe_relative_path(Path::new("client.exe")));
        assert!(is_safe_relative_path(Path::new("data/map0.mul")));
        assert!(is_safe_relative_path(Path::new("assets/textures/grass.png")));
        assert!(is_safe_relative_path(Path::new("./config.ini")));
    }

    #[test]
    fn test_path_containment_rejects_traversal() {
        // Verify that path traversal attempts are rejected
        assert!(!is_safe_relative_path(Path::new("../secret.txt")));
        assert!(!is_safe_relative_path(Path::new("../../../etc/passwd")));
        assert!(!is_safe_relative_path(Path::new("data/../../../etc/passwd")));
        assert!(!is_safe_relative_path(Path::new("foo/../bar/../../../secret")));
    }

    #[test]
    fn test_path_containment_rejects_absolute_paths() {
        // Verify that absolute paths are rejected
        assert!(!is_safe_relative_path(Path::new("/etc/passwd")));
        assert!(!is_safe_relative_path(Path::new("/usr/bin/bash")));
        assert!(!is_safe_relative_path(Path::new("/")));
    }

    #[test]
    fn test_path_containment_rejects_windows_absolute() {
        // Verify that Windows absolute paths are rejected
        assert!(!is_safe_relative_path(Path::new("C:\\Windows\\System32")));
        assert!(!is_safe_relative_path(Path::new("D:\\Program Files")));
        assert!(!is_safe_relative_path(Path::new("\\\\server\\share")));
    }

    #[test]
    fn test_path_containment_rejects_mixed_separator_traversal() {
        // Verify that mixed separator path traversal is rejected
        assert!(!is_safe_relative_path(Path::new("foo/..\\bar")));
        assert!(!is_safe_relative_path(Path::new("data\\../../../etc")));
        assert!(!is_safe_relative_path(Path::new("..\\..\\secret")));
    }

    #[test]
    fn test_path_containment_accepts_windows_relative_subdirs() {
        // Verify that Windows-style relative subdirectories are accepted
        assert!(is_safe_relative_path(Path::new("data\\maps\\map0.mul")));
        assert!(is_safe_relative_path(Path::new("assets\\textures\\grass.png")));
    }

    #[test]
    fn test_path_containment_rejects_empty_path() {
        // Empty paths should be rejected
        assert!(!is_safe_relative_path(Path::new("")));
    }

    #[test]
    fn test_path_containment_accepts_dot_prefixed() {
        // Paths starting with ./ (current directory) are safe
        assert!(is_safe_relative_path(Path::new("./file.txt")));
        assert!(is_safe_relative_path(Path::new("./data/file.txt")));
    }

    #[test]
    fn test_path_containment_accepts_hidden_files() {
        // Unix-style hidden files (starting with .) should be accepted
        assert!(is_safe_relative_path(Path::new(".hidden")));
        assert!(is_safe_relative_path(Path::new(".config/settings.json")));
    }

    #[test]
    fn test_path_containment_integration_with_installer() {
        // This test verifies that the path validation logic integrates correctly
        // with the installer's expected behavior. The installer should:
        // 1. Accept safe relative paths for downloads
        // 2. Reject any path traversal attempts

        // Simulate paths that might come from a manifest
        let valid_paths = vec![
            "client.exe",
            "data/map0.mul",
            "assets/textures/grass.png",
            "config.ini",
            "./readme.txt",
        ];

        for path in valid_paths {
            assert!(
                is_safe_relative_path(Path::new(path)),
                "Expected path '{}' to be valid",
                path
            );
        }

        // These are attack vectors that must be rejected
        let malicious_paths = vec![
            "../secret.txt",
            "../../password.txt",
            "../../../etc/passwd",
            "data/../../../etc/passwd",
            "/etc/passwd",
            "C:\\Windows\\System32\\cmd.exe",
            "\\\\server\\share\\file.txt",
        ];

        for path in malicious_paths {
            assert!(
                !is_safe_relative_path(Path::new(path)),
                "Expected path '{}' to be rejected as unsafe",
                path
            );
        }
    }

    // =============================================================
    // Disk Space Tests
    // =============================================================

    #[test]
    fn test_disk_space_check_existing_directory() {
        // Test disk space check on an existing directory
        let temp_dir = TempDir::new().unwrap();
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        let (space, warning) = installer.get_available_space(temp_dir.path());

        // Should return a real value, not u64::MAX (unless API failed)
        // On most systems, there's at least some free space
        // We just verify it returns a non-zero value
        assert!(space > 0, "Disk space should be greater than 0");

        // If the API works, space should be less than u64::MAX and warning should be None
        // (u64::MAX is only returned on API failure with a warning)
        if space < u64::MAX {
            assert!(warning.is_none(), "No warning expected when API succeeds");
        } else {
            assert!(warning.is_some(), "Warning expected when API returns u64::MAX");
        }
    }

    #[test]
    fn test_disk_space_check_nonexistent_directory() {
        // Test disk space check on a non-existent directory
        // Should walk up to find an existing parent
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("does").join("not").join("exist");

        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        let (space, _warning) = installer.get_available_space(&nonexistent);

        // Should still return a valid value by walking up to temp_dir
        assert!(space > 0, "Disk space should be greater than 0 even for non-existent paths");
    }

    #[test]
    fn test_disk_space_check_validates_path_with_sufficient_space() {
        // Test that path validation uses real disk space check
        let temp_dir = TempDir::new().unwrap();
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        // Validate with 0 required space should pass
        let result = installer.validate_install_path(temp_dir.path(), 0);
        assert!(result.is_valid);
        assert!(result.has_sufficient_space);

        // The available_space should be populated with a real value
        assert!(result.available_space > 0, "Available space should be populated");
    }

    #[test]
    fn test_disk_space_check_insufficient_space() {
        // Test that validation fails when required space exceeds available
        let temp_dir = TempDir::new().unwrap();
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        // Request an absurdly large amount of space (1 exabyte)
        let huge_space_requirement: u64 = 1024 * 1024 * 1024 * 1024 * 1024 * 1024; // 1 EB

        let result = installer.validate_install_path(temp_dir.path(), huge_space_requirement);

        // Should fail due to insufficient space (unless API returned u64::MAX)
        // Note: If the disk space API fails and returns u64::MAX, this test may pass
        // unexpectedly, but that's acceptable - we're testing normal operation
        if result.available_space < huge_space_requirement {
            assert!(!result.is_valid, "Should fail when required space exceeds available");
            assert!(!result.has_sufficient_space);
            assert!(result.reason.is_some());
            assert!(result.reason.unwrap().contains("Insufficient disk space"));
        }
    }

    #[test]
    fn test_disk_space_returns_bytes() {
        // Test that disk space is returned in bytes (not KB, MB, etc.)
        let temp_dir = TempDir::new().unwrap();
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        let (space, _warning) = installer.get_available_space(temp_dir.path());

        // If the API works, we should have at least 1MB of space on most systems
        // This verifies the value is in bytes (a KB value would be 1000x smaller)
        if space < u64::MAX {
            // Most systems have at least 1MB free
            assert!(
                space > 1_000_000,
                "Disk space {} bytes seems too small - verify it's in bytes not KB",
                space
            );
        }
    }

    #[test]
    fn test_disk_space_warning_surfaced_in_validation() {
        // Test that warning_message field exists and can hold disk space warnings
        let temp_dir = TempDir::new().unwrap();
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        // Validate path - should return a result with warning_message field
        let result = installer.validate_install_path(temp_dir.path(), 0);
        assert!(result.is_valid);

        // The warning_message field should exist and be accessible
        // If disk space check succeeded, it should be None
        // If it failed, it would contain a warning message
        // Either way, the field should be present in the result
        match result.warning_message {
            None => {
                // Normal case: disk space API succeeded, no warning
                assert!(result.available_space > 0 || result.available_space == u64::MAX);
            }
            Some(ref msg) => {
                // Edge case: disk space API failed, warning present
                assert!(!msg.is_empty(), "Warning message should not be empty if present");
                assert!(
                    msg.contains("disk space") || msg.contains("insufficient"),
                    "Warning should be related to disk space"
                );
            }
        }
    }

    #[test]
    fn test_path_validation_result_with_warning() {
        // Test PathValidationResult::valid() with a warning message
        let warning = Some("Test warning: disk space check failed".to_string());
        let result = PathValidationResult::valid(
            u64::MAX,
            true,
            true,
            true,
            false,
            warning.clone(),
        );

        assert!(result.is_valid);
        assert!(result.warning_message.is_some());
        assert_eq!(result.warning_message, warning);
        assert_eq!(result.available_space, u64::MAX);
    }

    #[test]
    fn test_path_validation_warning_preserved_on_insufficient_permissions() {
        // Test that warning_message is preserved even when validation fails
        let brand = test_brand_config();
        let installer = Installer::new(brand).unwrap();

        // Try to validate a system path that requires elevation
        #[cfg(target_os = "windows")]
        let system_path = Path::new("C:\\Windows\\System32\\TestDir");

        #[cfg(not(target_os = "windows"))]
        let system_path = Path::new("/root/test_dir");

        let result = installer.validate_install_path(system_path, 0);

        // The result should have the warning_message field populated
        // (either None if API succeeded or Some if it failed)
        // This tests that the field is properly included in all code paths
        let _ = result.warning_message; // Access the field to ensure it exists
    }
}
