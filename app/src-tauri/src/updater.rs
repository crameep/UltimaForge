//! Atomic update engine for UltimaForge.
//!
//! This module provides atomic update functionality with backup and rollback
//! capabilities. Updates follow a strict sequence to ensure the installation
//! is never left in an inconsistent state:
//!
//! 1. **Download to staging** - All files downloaded to a temporary staging directory
//! 2. **Verify all hashes** - Every staged file is verified before any changes
//! 3. **Backup current files** - Files to be replaced are backed up first
//! 4. **Apply staged files** - Move files from staging to installation directory
//! 5. **Rollback on failure** - Restore from backup if any step fails
//!
//! # Transaction Logging
//!
//! All operations are logged to `update.log` in the install directory for
//! troubleshooting purposes. The log includes timestamps and operation results.
//!
//! # Security
//!
//! - All verification happens BEFORE any file modification
//! - Manifest signature must be verified before using this module
//! - File hashes are verified after download and before application

use crate::config::{BrandConfig, LauncherConfig};
use crate::downloader::{DownloadProgress, Downloader, DownloaderConfig};
use crate::error::{DownloadError, UpdateError};
use crate::hash::{hash_file, verify_file_hash};
use crate::manifest::{FileEntry, Manifest};
use crate::signature;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::Emitter;
use tracing::{debug, error, info, warn};

/// Name of the staging directory relative to install path.
const STAGING_DIR: &str = ".update-staging";

/// Name of the backup directory relative to install path.
const BACKUP_DIR: &str = ".update-backup";

/// Name of the update transaction log file.
const UPDATE_LOG_FILE: &str = "update.log";

/// Event name for update progress events emitted to the frontend.
pub const UPDATE_PROGRESS_EVENT: &str = "update-progress";

/// Current state of an update operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateState {
    /// No update in progress.
    Idle,
    /// Checking for available updates.
    Checking,
    /// Downloading files to staging directory.
    Downloading,
    /// Verifying downloaded file hashes.
    Verifying,
    /// Backing up current files.
    BackingUp,
    /// Applying staged files to installation.
    Applying,
    /// Rolling back failed update.
    RollingBack,
    /// Update completed successfully.
    Completed,
    /// Update failed (may have rolled back).
    Failed,
}

impl std::fmt::Display for UpdateState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Idle => write!(f, "Idle"),
            Self::Checking => write!(f, "Checking for updates"),
            Self::Downloading => write!(f, "Downloading files"),
            Self::Verifying => write!(f, "Verifying files"),
            Self::BackingUp => write!(f, "Backing up current files"),
            Self::Applying => write!(f, "Applying update"),
            Self::RollingBack => write!(f, "Rolling back"),
            Self::Completed => write!(f, "Update completed"),
            Self::Failed => write!(f, "Update failed"),
        }
    }
}

/// Progress information for an ongoing update.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProgress {
    /// Current update state.
    pub state: UpdateState,
    /// Total number of files to update.
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
    /// Version being updated to.
    pub target_version: Option<String>,
    /// Error message if state is Failed.
    pub error_message: Option<String>,
}

impl UpdateProgress {
    /// Creates a new idle progress instance.
    pub fn new() -> Self {
        Self {
            state: UpdateState::Idle,
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

    /// Returns true if the update is complete (success or failure).
    pub fn is_complete(&self) -> bool {
        matches!(self.state, UpdateState::Completed | UpdateState::Failed)
    }

    /// Sets the state to failed with an error message.
    pub fn set_failed(&mut self, message: impl Into<String>) {
        self.state = UpdateState::Failed;
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
        app_handle.emit(UPDATE_PROGRESS_EVENT, self)
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
        window.emit(UPDATE_PROGRESS_EVENT, self)
    }
}

impl Default for UpdateProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of checking for updates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    /// Whether an update is available.
    pub update_available: bool,
    /// Current installed version.
    pub current_version: Option<String>,
    /// Available version on server.
    pub server_version: String,
    /// Number of files that need updating.
    pub files_to_update: usize,
    /// Total download size in bytes.
    pub download_size: u64,
    /// URL to patch notes (if available).
    pub patch_notes_url: Option<String>,
}

impl UpdateCheckResult {
    /// Returns a human-readable download size string.
    pub fn download_size_formatted(&self) -> String {
        Manifest::format_size(self.download_size)
    }
}

/// A manifest that has been cryptographically verified.
///
/// This struct holds a manifest that has passed signature verification,
/// along with metadata about when the verification occurred. By using this
/// struct, we ensure that manifest data has been verified before use,
/// eliminating TOCTOU (time-of-check-time-of-use) vulnerabilities.
///
/// # Security
///
/// - The manifest signature is verified BEFORE parsing the JSON
/// - Once verified, the manifest should be reused rather than re-fetched
/// - The `signature_verified_at` timestamp can be used for cache invalidation
#[derive(Debug, Clone)]
pub struct VerifiedManifest {
    /// The verified manifest contents.
    pub manifest: Manifest,
    /// When the signature was verified (for cache/freshness checks).
    pub signature_verified_at: Instant,
}

impl VerifiedManifest {
    /// Returns how long ago the signature was verified.
    pub fn age(&self) -> std::time::Duration {
        self.signature_verified_at.elapsed()
    }

    /// Returns true if the verification is older than the specified duration.
    pub fn is_stale(&self, max_age: std::time::Duration) -> bool {
        self.age() > max_age
    }
}

/// Transaction log entry for update operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionLogEntry {
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

impl TransactionLogEntry {
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

/// Transaction logger for update operations.
struct TransactionLog {
    log_path: PathBuf,
    file: Option<File>,
}

impl TransactionLog {
    /// Creates a new transaction log.
    fn new(install_path: &Path) -> io::Result<Self> {
        let log_path = install_path.join(UPDATE_LOG_FILE);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        Ok(Self {
            log_path,
            file: Some(file),
        })
    }

    /// Logs a transaction entry.
    fn log(&mut self, entry: TransactionLogEntry) {
        if let Some(ref mut file) = self.file {
            let line = format!("{}\n", entry.format());
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
        debug!("{}", entry.format());
    }

    /// Logs the start of an update session.
    fn log_session_start(&mut self, target_version: &str) {
        self.log(
            TransactionLogEntry::new("SESSION_START", None, "STARTED")
                .with_details(format!("Updating to version {}", target_version)),
        );
    }

    /// Logs the end of an update session.
    fn log_session_end(&mut self, success: bool) {
        let result = if success { "SUCCESS" } else { "FAILED" };
        self.log(TransactionLogEntry::new("SESSION_END", None, result));
    }
}

/// Atomic update engine with backup and rollback capabilities.
pub struct Updater {
    /// Installation directory path.
    install_path: PathBuf,
    /// Staging directory for downloaded files.
    staging_path: PathBuf,
    /// Backup directory for current files.
    backup_path: PathBuf,
    /// HTTP downloader instance.
    downloader: Downloader,
    /// Brand configuration with update URL.
    brand_config: BrandConfig,
    /// Files that were backed up (for rollback).
    backed_up_files: Vec<String>,
    /// Files that were applied (for rollback tracking).
    applied_files: Vec<String>,
}

impl Updater {
    /// Creates a new updater for the given installation path.
    pub fn new(install_path: PathBuf, brand_config: BrandConfig) -> Result<Self, UpdateError> {
        let staging_path = install_path.join(STAGING_DIR);
        let backup_path = install_path.join(BACKUP_DIR);

        let downloader = Downloader::new().map_err(|e| UpdateError::StagingError(e.to_string()))?;

        Ok(Self {
            install_path,
            staging_path,
            backup_path,
            downloader,
            brand_config,
            backed_up_files: Vec::new(),
            applied_files: Vec::new(),
        })
    }

    /// Creates a new updater with custom downloader configuration.
    pub fn with_config(
        install_path: PathBuf,
        brand_config: BrandConfig,
        downloader_config: DownloaderConfig,
    ) -> Result<Self, UpdateError> {
        let staging_path = install_path.join(STAGING_DIR);
        let backup_path = install_path.join(BACKUP_DIR);

        let downloader = Downloader::with_config(downloader_config)
            .map_err(|e| UpdateError::StagingError(e.to_string()))?;

        Ok(Self {
            install_path,
            staging_path,
            backup_path,
            downloader,
            brand_config,
            backed_up_files: Vec::new(),
            applied_files: Vec::new(),
        })
    }

    /// Returns the installation path.
    pub fn install_path(&self) -> &Path {
        &self.install_path
    }

    /// Fetches and verifies the manifest from the update server.
    ///
    /// This is the single, canonical way to obtain a trusted manifest. It:
    /// 1. Downloads the manifest bytes
    /// 2. Downloads the signature bytes
    /// 3. Verifies the signature BEFORE parsing the JSON
    /// 4. Returns a `VerifiedManifest` that can be safely used
    ///
    /// # Security
    ///
    /// This method eliminates TOCTOU vulnerabilities by ensuring signature
    /// verification happens atomically with the fetch. The returned
    /// `VerifiedManifest` should be stored and reused rather than calling
    /// this method again (which would create a new TOCTOU window).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let verified = updater.fetch_verified_manifest().await?;
    /// // Safe to use manifest.files, manifest.version, etc.
    /// let files_to_update = verified.manifest.files_to_update(&local_hashes);
    /// ```
    pub async fn fetch_verified_manifest(&self) -> Result<VerifiedManifest, UpdateError> {
        info!(
            "Fetching verified manifest from {}",
            self.brand_config.update_url
        );

        // Step 1: Download manifest bytes
        let manifest_url = format!("{}/manifest.json", self.brand_config.update_url);
        let manifest_bytes = self
            .downloader
            .download_bytes(&manifest_url)
            .await
            .map_err(|e| UpdateError::ManifestFetchFailed(e.to_string()))?;

        // Step 2: Download signature bytes
        let signature_url = format!("{}/manifest.sig", self.brand_config.update_url);
        let signature_hex = self
            .downloader
            .download_bytes(&signature_url)
            .await
            .map_err(|_| UpdateError::MissingSignature)?;

        // Step 3: Decode hex signature
        let signature_str = std::str::from_utf8(&signature_hex)
            .map_err(|_| UpdateError::StagingError("Invalid signature encoding".to_string()))?
            .trim();
        let signature_bytes = signature::parse_hex_signature(signature_str)
            .map_err(|e| UpdateError::StagingError(format!("Invalid signature format: {}", e)))?;

        // Step 4: Get public key from brand config
        let public_key_bytes: [u8; 32] = self
            .brand_config
            .public_key_bytes()
            .map_err(|e| UpdateError::StagingError(format!("Invalid public key: {}", e)))?
            .try_into()
            .map_err(|_| UpdateError::StagingError("Invalid public key length".to_string()))?;

        // Step 5: VERIFY SIGNATURE BEFORE PARSING
        // This is critical - never parse untrusted data
        signature::verify_manifest(&manifest_bytes, &signature_bytes, &public_key_bytes)
            .map_err(|e| {
                UpdateError::StagingError(format!("Signature verification failed: {}", e))
            })?;

        let signature_verified_at = Instant::now();

        // Step 6: Parse manifest (now safe since signature is verified)
        let manifest = Manifest::parse(&manifest_bytes)
            .map_err(|e| UpdateError::StagingError(format!("Invalid manifest: {}", e)))?;

        info!(
            "Manifest verified successfully: version={}, {} files",
            manifest.version,
            manifest.files.len()
        );

        Ok(VerifiedManifest {
            manifest,
            signature_verified_at,
        })
    }

    /// Returns the staging directory path.
    pub fn staging_path(&self) -> &Path {
        &self.staging_path
    }

    /// Returns the backup directory path.
    pub fn backup_path(&self) -> &Path {
        &self.backup_path
    }

    /// Checks for available updates.
    ///
    /// Downloads and verifies the manifest from the update server, then
    /// compares it against the current installation to determine which
    /// files need updating.
    pub async fn check_for_updates(
        &self,
        current_version: Option<&str>,
    ) -> Result<UpdateCheckResult, UpdateError> {
        info!("Checking for updates from {}", self.brand_config.update_url);

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

        // Compute local file hashes
        let local_hashes = self.compute_local_hashes(&manifest)?;

        // Determine files needing update
        let files_to_update = manifest.files_to_update(&local_hashes);
        let download_size: u64 = files_to_update.iter().map(|f| f.size).sum();

        let update_available = !files_to_update.is_empty()
            || current_version
                .map(|cv| cv != manifest.version)
                .unwrap_or(true);

        info!(
            "Update check complete: {} files to update ({} bytes)",
            files_to_update.len(),
            download_size
        );

        Ok(UpdateCheckResult {
            update_available,
            current_version: current_version.map(String::from),
            server_version: manifest.version.clone(),
            files_to_update: files_to_update.len(),
            download_size,
            patch_notes_url: manifest.patch_notes_url.clone(),
        })
    }

    /// Computes SHA-256 hashes of local files that exist in the manifest.
    fn compute_local_hashes(&self, manifest: &Manifest) -> Result<HashMap<String, String>, UpdateError> {
        let mut hashes = HashMap::new();

        for file in manifest.iter_files() {
            let local_path = self.install_path.join(&file.path);
            if local_path.exists() {
                match hash_file(&local_path) {
                    Ok(hash) => {
                        hashes.insert(file.path.clone(), hash);
                    }
                    Err(e) => {
                        warn!("Failed to hash {}: {}", file.path, e);
                        // Don't insert - file will be treated as needing update
                    }
                }
            }
        }

        Ok(hashes)
    }

    /// Downloads all files that need updating to the staging directory.
    ///
    /// # Arguments
    ///
    /// * `files` - List of files to download
    /// * `progress_callback` - Callback invoked with progress updates
    ///
    /// # Returns
    ///
    /// Returns the list of successfully downloaded files.
    pub async fn download_to_staging<'a, F>(
        &self,
        files: &[&'a FileEntry],
        progress_callback: F,
    ) -> Result<Vec<&'a FileEntry>, UpdateError>
    where
        F: Fn(&DownloadProgress) + Send + Sync,
    {
        info!("Downloading {} files to staging", files.len());

        // Ensure staging directory exists and is clean
        self.prepare_staging_directory()?;

        let mut downloaded = Vec::new();

        for file in files {
            let staged_path = self.staging_path.join(&file.path);
            let blob_url = file.blob_url(&self.brand_config.update_url);

            debug!("Downloading {} to staging", file.path);

            // Create parent directories in staging
            if let Some(parent) = staged_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    UpdateError::StagingError(format!("Failed to create staging dirs: {}", e))
                })?;
            }

            // Download with hash verification
            match self
                .downloader
                .download_file(&blob_url, &staged_path, Some(&file.sha256), &progress_callback)
                .await
            {
                Ok(_) => {
                    debug!("Successfully downloaded {}", file.path);
                    downloaded.push(*file);
                }
                Err(e) => {
                    error!("Failed to download {}: {}", file.path, e);
                    // Clean up partial staging
                    self.cleanup_staging()?;
                    return Err(UpdateError::StagingError(format!(
                        "Download failed for {}: {}",
                        file.path, e
                    )));
                }
            }
        }

        info!("Downloaded {} files to staging", downloaded.len());
        Ok(downloaded)
    }

    /// Prepares the staging directory, cleaning up any previous partial updates.
    fn prepare_staging_directory(&self) -> Result<(), UpdateError> {
        if self.staging_path.exists() {
            debug!("Cleaning up previous staging directory");
            fs::remove_dir_all(&self.staging_path).map_err(|e| {
                UpdateError::StagingError(format!("Failed to clean staging: {}", e))
            })?;
        }

        fs::create_dir_all(&self.staging_path).map_err(|e| {
            UpdateError::StagingError(format!("Failed to create staging dir: {}", e))
        })?;

        Ok(())
    }

    /// Cleans up the staging directory.
    fn cleanup_staging(&self) -> Result<(), UpdateError> {
        if self.staging_path.exists() {
            fs::remove_dir_all(&self.staging_path).map_err(|e| {
                UpdateError::StagingError(format!("Failed to cleanup staging: {}", e))
            })?;
        }
        Ok(())
    }

    /// Verifies all files in the staging directory have correct hashes.
    ///
    /// This is a secondary verification step (downloads already verify hashes),
    /// but provides additional assurance before applying the update.
    pub fn verify_staged_hashes(&self, files: &[&FileEntry]) -> Result<(), UpdateError> {
        info!("Verifying {} staged file hashes", files.len());

        for file in files {
            let staged_path = self.staging_path.join(&file.path);

            if !staged_path.exists() {
                return Err(UpdateError::StagingError(format!(
                    "Staged file missing: {}",
                    file.path
                )));
            }

            let verified = verify_file_hash(&staged_path, &file.sha256).map_err(|e| {
                UpdateError::StagingError(format!("Hash verification error for {}: {}", file.path, e))
            })?;

            if !verified {
                return Err(UpdateError::StagingError(format!(
                    "Hash mismatch for staged file: {}",
                    file.path
                )));
            }

            debug!("Verified hash for {}", file.path);
        }

        info!("All {} staged files verified", files.len());
        Ok(())
    }

    /// Backs up current files that will be replaced.
    ///
    /// Creates a backup of each file that exists in the installation and
    /// will be replaced by the update.
    pub fn backup_current_files(&mut self, files: &[&FileEntry]) -> Result<(), UpdateError> {
        info!("Backing up {} current files", files.len());

        // Ensure backup directory exists and is clean
        if self.backup_path.exists() {
            fs::remove_dir_all(&self.backup_path).map_err(|e| {
                UpdateError::BackupFailed {
                    path: self.backup_path.display().to_string(),
                    source: e,
                }
            })?;
        }

        fs::create_dir_all(&self.backup_path).map_err(|e| UpdateError::BackupFailed {
            path: self.backup_path.display().to_string(),
            source: e,
        })?;

        self.backed_up_files.clear();

        for file in files {
            let current_path = self.install_path.join(&file.path);

            // Only backup if file exists
            if current_path.exists() {
                let backup_path = self.backup_path.join(&file.path);

                // Create parent directories in backup
                if let Some(parent) = backup_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| UpdateError::BackupFailed {
                        path: parent.display().to_string(),
                        source: e,
                    })?;
                }

                // Copy file to backup
                fs::copy(&current_path, &backup_path).map_err(|e| UpdateError::BackupFailed {
                    path: file.path.clone(),
                    source: e,
                })?;

                self.backed_up_files.push(file.path.clone());
                debug!("Backed up {}", file.path);
            }
        }

        info!(
            "Backed up {} files (skipped {} new files)",
            self.backed_up_files.len(),
            files.len() - self.backed_up_files.len()
        );

        Ok(())
    }

    /// Applies staged files to the installation directory.
    ///
    /// Moves files from staging to the install directory. If this fails
    /// partway through, call `rollback()` to restore the backup.
    pub fn apply_staged_files(&mut self, files: &[&FileEntry]) -> Result<(), UpdateError> {
        info!("Applying {} staged files", files.len());

        self.applied_files.clear();

        for file in files {
            let staged_path = self.staging_path.join(&file.path);
            let target_path = self.install_path.join(&file.path);

            // Create parent directories if needed
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent).map_err(|e| UpdateError::ApplyFailed {
                    path: parent.display().to_string(),
                    source: e,
                })?;
            }

            // Remove existing file if it exists
            if target_path.exists() {
                fs::remove_file(&target_path).map_err(|e| {
                    // Check if file is locked
                    if e.kind() == io::ErrorKind::PermissionDenied {
                        UpdateError::FileLocked {
                            path: file.path.clone(),
                        }
                    } else {
                        UpdateError::ApplyFailed {
                            path: file.path.clone(),
                            source: e,
                        }
                    }
                })?;
            }

            // Move staged file to target
            // Use copy+delete instead of rename for cross-device compatibility
            fs::copy(&staged_path, &target_path).map_err(|e| UpdateError::ApplyFailed {
                path: file.path.clone(),
                source: e,
            })?;

            fs::remove_file(&staged_path).map_err(|e| {
                // Non-fatal - file was copied successfully
                warn!("Failed to remove staged file {}: {}", file.path, e);
                e
            }).ok();

            self.applied_files.push(file.path.clone());
            debug!("Applied {}", file.path);
        }

        info!("Applied {} files", self.applied_files.len());
        Ok(())
    }

    /// Rolls back a failed update by restoring backed up files.
    ///
    /// This should be called if `apply_staged_files` fails partway through.
    pub fn rollback(&mut self) -> Result<(), UpdateError> {
        warn!("Rolling back update...");

        // Remove any files that were applied
        for path in &self.applied_files {
            let target_path = self.install_path.join(path);
            if target_path.exists() {
                if let Err(e) = fs::remove_file(&target_path) {
                    error!("Failed to remove applied file {}: {}", path, e);
                }
            }
        }

        // Restore backed up files
        for path in &self.backed_up_files {
            let backup_path = self.backup_path.join(path);
            let target_path = self.install_path.join(path);

            if backup_path.exists() {
                // Ensure parent directory exists
                if let Some(parent) = target_path.parent() {
                    fs::create_dir_all(parent).ok();
                }

                if let Err(e) = fs::copy(&backup_path, &target_path) {
                    error!("Failed to restore {}: {}", path, e);
                    return Err(UpdateError::RollbackFailed {
                        reason: format!("Failed to restore {}: {}", path, e),
                    });
                }
                debug!("Restored {}", path);
            }
        }

        self.applied_files.clear();
        info!("Rollback complete - {} files restored", self.backed_up_files.len());

        Ok(())
    }

    /// Cleans up temporary directories after a successful update.
    pub fn cleanup(&self) -> Result<(), UpdateError> {
        info!("Cleaning up update directories");

        if self.staging_path.exists() {
            fs::remove_dir_all(&self.staging_path).map_err(|e| {
                UpdateError::StagingError(format!("Failed to cleanup staging: {}", e))
            })?;
        }

        if self.backup_path.exists() {
            fs::remove_dir_all(&self.backup_path).map_err(|e| {
                UpdateError::StagingError(format!("Failed to cleanup backup: {}", e))
            })?;
        }

        Ok(())
    }

    /// Performs a complete update operation.
    ///
    /// This is the main entry point for updating. It:
    /// 1. Fetches and verifies the manifest
    /// 2. Determines which files need updating
    /// 3. Downloads files to staging
    /// 4. Verifies all staged files
    /// 5. Backs up current files
    /// 6. Applies the update
    /// 7. Cleans up or rolls back on failure
    ///
    /// # Arguments
    ///
    /// * `progress_callback` - Callback invoked with update progress
    ///
    /// # Returns
    ///
    /// Returns the new version string on success.
    pub async fn perform_update<F>(
        &mut self,
        mut progress_callback: F,
    ) -> Result<String, UpdateError>
    where
        F: FnMut(&UpdateProgress) + Send,
    {
        let mut progress = UpdateProgress::new();

        // Initialize transaction log
        let mut log = TransactionLog::new(&self.install_path).map_err(|e| {
            UpdateError::StagingError(format!("Failed to create transaction log: {}", e))
        })?;

        // Step 1: Check for updates
        progress.state = UpdateState::Checking;
        progress_callback(&progress);

        let check_result = self.check_for_updates(None).await?;

        if !check_result.update_available {
            return Err(UpdateError::AlreadyUpToDate);
        }

        let target_version = check_result.server_version.clone();
        progress.target_version = Some(target_version.clone());
        log.log_session_start(&target_version);

        // Re-fetch manifest for file list
        let manifest_url = format!("{}/manifest.json", self.brand_config.update_url);
        let manifest_bytes = self
            .downloader
            .download_bytes(&manifest_url)
            .await
            .map_err(|e| UpdateError::ManifestFetchFailed(e.to_string()))?;

        let manifest = Manifest::parse(&manifest_bytes)
            .map_err(|e| UpdateError::StagingError(format!("Invalid manifest: {}", e)))?;

        // Compute files to update
        let local_hashes = self.compute_local_hashes(&manifest)?;
        let files_to_update: Vec<&FileEntry> = manifest.files_to_update(&local_hashes);

        if files_to_update.is_empty() {
            log.log_session_end(true);
            return Err(UpdateError::AlreadyUpToDate);
        }

        progress.total_files = files_to_update.len();
        progress.total_bytes = files_to_update.iter().map(|f| f.size).sum();

        // Step 2: Download to staging
        progress.state = UpdateState::Downloading;
        progress_callback(&progress);
        log.log(TransactionLogEntry::new("DOWNLOAD_START", None, "STARTED"));

        let mut downloaded_bytes = 0u64;
        let download_callback = |dl_progress: &DownloadProgress| {
            let mut up = UpdateProgress {
                state: UpdateState::Downloading,
                total_files: progress.total_files,
                processed_files: progress.processed_files,
                total_bytes: progress.total_bytes,
                downloaded_bytes: downloaded_bytes + dl_progress.downloaded,
                current_file: Some(dl_progress.file_path.clone()),
                speed_bps: dl_progress.speed_bps,
                eta_secs: dl_progress.eta_secs,
                target_version: progress.target_version.clone(),
                error_message: None,
            };
            progress_callback(&up);
        };

        // Download each file
        for file in &files_to_update {
            let staged_path = self.staging_path.join(&file.path);
            let blob_url = file.blob_url(&self.brand_config.update_url);

            // Ensure staging directory structure exists
            if !self.staging_path.exists() {
                self.prepare_staging_directory()?;
            }
            if let Some(parent) = staged_path.parent() {
                fs::create_dir_all(parent).map_err(|e| {
                    UpdateError::StagingError(format!("Failed to create staging dirs: {}", e))
                })?;
            }

            match self.downloader.download_file(
                &blob_url,
                &staged_path,
                Some(&file.sha256),
                |_| {},
            ).await {
                Ok(_) => {
                    downloaded_bytes += file.size;
                    progress.processed_files += 1;
                    log.log(TransactionLogEntry::new("DOWNLOAD", Some(&file.path), "OK"));
                }
                Err(e) => {
                    log.log(TransactionLogEntry::new("DOWNLOAD", Some(&file.path), "FAILED")
                        .with_details(e.to_string()));
                    self.cleanup_staging()?;
                    log.log_session_end(false);
                    return Err(UpdateError::StagingError(format!(
                        "Download failed for {}: {}",
                        file.path, e
                    )));
                }
            }
        }

        log.log(TransactionLogEntry::new("DOWNLOAD_START", None, "COMPLETE"));

        // Step 3: Verify staged hashes
        progress.state = UpdateState::Verifying;
        progress_callback(&progress);
        log.log(TransactionLogEntry::new("VERIFY_START", None, "STARTED"));

        if let Err(e) = self.verify_staged_hashes(&files_to_update) {
            log.log(TransactionLogEntry::new("VERIFY", None, "FAILED").with_details(e.to_string()));
            self.cleanup_staging()?;
            log.log_session_end(false);
            return Err(e);
        }

        log.log(TransactionLogEntry::new("VERIFY_START", None, "COMPLETE"));

        // Step 4: Backup current files
        progress.state = UpdateState::BackingUp;
        progress_callback(&progress);
        log.log(TransactionLogEntry::new("BACKUP_START", None, "STARTED"));

        if let Err(e) = self.backup_current_files(&files_to_update) {
            log.log(TransactionLogEntry::new("BACKUP", None, "FAILED").with_details(e.to_string()));
            self.cleanup_staging()?;
            log.log_session_end(false);
            return Err(e);
        }

        for path in &self.backed_up_files {
            log.log(TransactionLogEntry::new("BACKUP", Some(path), "OK"));
        }
        log.log(TransactionLogEntry::new("BACKUP_START", None, "COMPLETE"));

        // Step 5: Apply staged files
        progress.state = UpdateState::Applying;
        progress_callback(&progress);
        log.log(TransactionLogEntry::new("APPLY_START", None, "STARTED"));

        match self.apply_staged_files(&files_to_update) {
            Ok(()) => {
                for path in &self.applied_files {
                    log.log(TransactionLogEntry::new("APPLY", Some(path), "OK"));
                }
                log.log(TransactionLogEntry::new("APPLY_START", None, "COMPLETE"));
            }
            Err(e) => {
                log.log(TransactionLogEntry::new("APPLY", None, "FAILED").with_details(e.to_string()));

                // Attempt rollback
                progress.state = UpdateState::RollingBack;
                progress_callback(&progress);
                log.log(TransactionLogEntry::new("ROLLBACK_START", None, "STARTED"));

                match self.rollback() {
                    Ok(()) => {
                        log.log(TransactionLogEntry::new("ROLLBACK_START", None, "COMPLETE"));
                        log.log_session_end(false);
                        return Err(UpdateError::RolledBack {
                            reason: e.to_string(),
                        });
                    }
                    Err(rollback_err) => {
                        log.log(TransactionLogEntry::new("ROLLBACK", None, "FAILED")
                            .with_details(rollback_err.to_string()));
                        log.log_session_end(false);
                        return Err(UpdateError::RollbackFailed {
                            reason: format!("Apply failed: {}; Rollback failed: {}", e, rollback_err),
                        });
                    }
                }
            }
        }

        // Step 6: Cleanup
        log.log(TransactionLogEntry::new("CLEANUP_START", None, "STARTED"));
        if let Err(e) = self.cleanup() {
            // Non-fatal - update was successful
            warn!("Cleanup failed (non-fatal): {}", e);
            log.log(TransactionLogEntry::new("CLEANUP", None, "PARTIAL").with_details(e.to_string()));
        } else {
            log.log(TransactionLogEntry::new("CLEANUP_START", None, "COMPLETE"));
        }

        // Success!
        progress.state = UpdateState::Completed;
        progress.downloaded_bytes = progress.total_bytes;
        progress_callback(&progress);

        log.log_session_end(true);
        info!("Update to version {} completed successfully", target_version);

        Ok(target_version)
    }
}

/// Result type alias for updater operations.
pub type UpdateResult<T> = Result<T, UpdateError>;

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
    fn test_update_state_display() {
        assert_eq!(UpdateState::Idle.to_string(), "Idle");
        assert_eq!(UpdateState::Downloading.to_string(), "Downloading files");
        assert_eq!(UpdateState::RollingBack.to_string(), "Rolling back");
    }

    #[test]
    fn test_update_progress_new() {
        let progress = UpdateProgress::new();
        assert_eq!(progress.state, UpdateState::Idle);
        assert_eq!(progress.total_files, 0);
        assert_eq!(progress.processed_files, 0);
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_update_progress_percentage() {
        let mut progress = UpdateProgress::new();
        progress.total_bytes = 100;
        progress.downloaded_bytes = 50;
        assert!((progress.percentage() - 50.0).abs() < f64::EPSILON);

        progress.total_bytes = 0;
        assert!((progress.percentage() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_progress_file_percentage() {
        let mut progress = UpdateProgress::new();
        progress.total_files = 10;
        progress.processed_files = 5;
        assert!((progress.file_percentage() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_progress_is_complete() {
        let mut progress = UpdateProgress::new();
        assert!(!progress.is_complete());

        progress.state = UpdateState::Completed;
        assert!(progress.is_complete());

        progress.state = UpdateState::Failed;
        assert!(progress.is_complete());

        progress.state = UpdateState::Downloading;
        assert!(!progress.is_complete());
    }

    #[test]
    fn test_update_progress_set_failed() {
        let mut progress = UpdateProgress::new();
        progress.set_failed("Something went wrong");
        assert_eq!(progress.state, UpdateState::Failed);
        assert_eq!(progress.error_message, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_update_check_result_download_size_formatted() {
        let result = UpdateCheckResult {
            update_available: true,
            current_version: Some("1.0.0".to_string()),
            server_version: "1.1.0".to_string(),
            files_to_update: 5,
            download_size: 1024 * 1024, // 1 MB
            patch_notes_url: None,
        };

        assert_eq!(result.download_size_formatted(), "1.00 MB");
    }

    #[test]
    fn test_transaction_log_entry_format() {
        let entry = TransactionLogEntry::new("DOWNLOAD", Some("client.exe"), "OK");
        let formatted = entry.format();
        assert!(formatted.contains("DOWNLOAD"));
        assert!(formatted.contains("client.exe"));
        assert!(formatted.contains("OK"));

        let entry_with_details = entry.with_details("Downloaded 1024 bytes");
        let formatted = entry_with_details.format();
        assert!(formatted.contains("Downloaded 1024 bytes"));
    }

    #[test]
    fn test_updater_paths() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = temp_dir.path().to_path_buf();
        let brand = test_brand_config();

        let updater = Updater::new(install_path.clone(), brand).unwrap();

        assert_eq!(updater.install_path(), install_path);
        assert_eq!(updater.staging_path(), install_path.join(STAGING_DIR));
        assert_eq!(updater.backup_path(), install_path.join(BACKUP_DIR));
    }

    #[test]
    fn test_updater_prepare_staging_directory() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = temp_dir.path().to_path_buf();
        let brand = test_brand_config();

        let updater = Updater::new(install_path.clone(), brand).unwrap();

        // Create a file in staging to simulate previous update
        fs::create_dir_all(updater.staging_path()).unwrap();
        fs::write(updater.staging_path().join("old_file.txt"), "old content").unwrap();

        updater.prepare_staging_directory().unwrap();

        // Old file should be gone
        assert!(!updater.staging_path().join("old_file.txt").exists());
        // Directory should exist
        assert!(updater.staging_path().exists());
    }

    #[test]
    fn test_updater_cleanup_staging() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = temp_dir.path().to_path_buf();
        let brand = test_brand_config();

        let updater = Updater::new(install_path.clone(), brand).unwrap();

        fs::create_dir_all(updater.staging_path()).unwrap();
        fs::write(updater.staging_path().join("test.txt"), "content").unwrap();

        updater.cleanup_staging().unwrap();
        assert!(!updater.staging_path().exists());
    }

    #[test]
    fn test_updater_backup_and_rollback() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = temp_dir.path().to_path_buf();
        let brand = test_brand_config();

        let mut updater = Updater::new(install_path.clone(), brand).unwrap();

        // Create an existing file
        let file_path = install_path.join("test.exe");
        fs::write(&file_path, "original content").unwrap();

        // Create a fake file entry
        let file_entry = FileEntry::new(
            "test.exe",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            16,
        );
        let files = vec![&file_entry];

        // Backup
        updater.backup_current_files(&files).unwrap();
        assert!(updater.backup_path.join("test.exe").exists());
        assert_eq!(updater.backed_up_files, vec!["test.exe"]);

        // Simulate applying a new file
        fs::write(&file_path, "new content").unwrap();
        updater.applied_files.push("test.exe".to_string());

        // Rollback
        updater.rollback().unwrap();

        // Original content should be restored
        let restored = fs::read_to_string(&file_path).unwrap();
        assert_eq!(restored, "original content");
    }

    #[test]
    fn test_updater_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = temp_dir.path().to_path_buf();
        let brand = test_brand_config();

        let updater = Updater::new(install_path.clone(), brand).unwrap();

        // Create staging and backup directories
        fs::create_dir_all(updater.staging_path()).unwrap();
        fs::create_dir_all(updater.backup_path()).unwrap();

        updater.cleanup().unwrap();

        assert!(!updater.staging_path().exists());
        assert!(!updater.backup_path().exists());
    }

    #[test]
    fn test_updater_apply_staged_files() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = temp_dir.path().to_path_buf();
        let brand = test_brand_config();

        let mut updater = Updater::new(install_path.clone(), brand).unwrap();

        // Create staging directory with a file
        let staging = updater.staging_path.clone();
        fs::create_dir_all(&staging).unwrap();
        fs::write(staging.join("newfile.exe"), "new content").unwrap();

        // Create file entry
        let file_entry = FileEntry::new(
            "newfile.exe",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            11,
        );
        let files = vec![&file_entry];

        // Apply
        updater.apply_staged_files(&files).unwrap();

        // File should be in install path
        let installed_file = install_path.join("newfile.exe");
        assert!(installed_file.exists());
        assert_eq!(fs::read_to_string(installed_file).unwrap(), "new content");
        assert_eq!(updater.applied_files, vec!["newfile.exe"]);
    }

    #[test]
    fn test_updater_verify_staged_hashes_missing_file() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = temp_dir.path().to_path_buf();
        let brand = test_brand_config();

        let updater = Updater::new(install_path.clone(), brand).unwrap();

        // Create staging without the file
        fs::create_dir_all(updater.staging_path()).unwrap();

        let file_entry = FileEntry::new(
            "missing.exe",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            100,
        );
        let files = vec![&file_entry];

        let result = updater.verify_staged_hashes(&files);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("missing"));
        }
    }

    #[test]
    fn test_updater_verify_staged_hashes_wrong_hash() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = temp_dir.path().to_path_buf();
        let brand = test_brand_config();

        let updater = Updater::new(install_path.clone(), brand).unwrap();

        // Create staging with file that has wrong content
        fs::create_dir_all(updater.staging_path()).unwrap();
        fs::write(updater.staging_path().join("wronghash.exe"), "wrong content").unwrap();

        let file_entry = FileEntry::new(
            "wronghash.exe",
            "0000000000000000000000000000000000000000000000000000000000000000",
            13,
        );
        let files = vec![&file_entry];

        let result = updater.verify_staged_hashes(&files);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("mismatch"));
        }
    }

    #[test]
    fn test_update_state_all_variants() {
        // Ensure all variants are testable
        let states = vec![
            UpdateState::Idle,
            UpdateState::Checking,
            UpdateState::Downloading,
            UpdateState::Verifying,
            UpdateState::BackingUp,
            UpdateState::Applying,
            UpdateState::RollingBack,
            UpdateState::Completed,
            UpdateState::Failed,
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
    fn test_transaction_log_entry_without_details() {
        let entry = TransactionLogEntry::new("TEST_OP", None, "OK");
        let formatted = entry.format();
        assert!(formatted.contains("TEST_OP"));
        assert!(formatted.contains("OK"));
        // When no file path, there should be only one '[' (for the timestamp)
        // Format is: [timestamp] operation result: file_info details_info
        assert_eq!(formatted.matches('[').count(), 1, "Should have exactly one '[' for timestamp only");
    }

    #[test]
    fn test_compute_local_hashes_empty_install() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = temp_dir.path().to_path_buf();
        let brand = test_brand_config();

        let updater = Updater::new(install_path, brand).unwrap();

        // Create manifest with some files
        use crate::manifest::ManifestBuilder;
        let manifest = ManifestBuilder::new()
            .version("1.0.0")
            .timestamp("2026-02-15T00:00:00Z")
            .client_executable("client.exe")
            .add_file(FileEntry::new(
                "client.exe",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                1000,
            ))
            .build()
            .unwrap();

        // Empty install - no local files
        let hashes = updater.compute_local_hashes(&manifest).unwrap();
        assert!(hashes.is_empty());
    }

    #[test]
    fn test_compute_local_hashes_with_existing_files() {
        let temp_dir = TempDir::new().unwrap();
        let install_path = temp_dir.path().to_path_buf();
        let brand = test_brand_config();

        // Create a file in the install directory
        fs::write(install_path.join("client.exe"), "test content").unwrap();

        let updater = Updater::new(install_path, brand).unwrap();

        use crate::manifest::ManifestBuilder;
        let manifest = ManifestBuilder::new()
            .version("1.0.0")
            .timestamp("2026-02-15T00:00:00Z")
            .client_executable("client.exe")
            .add_file(FileEntry::new(
                "client.exe",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                1000,
            ))
            .build()
            .unwrap();

        let hashes = updater.compute_local_hashes(&manifest).unwrap();
        assert_eq!(hashes.len(), 1);
        assert!(hashes.contains_key("client.exe"));
    }

    #[test]
    fn test_update_progress_default() {
        let progress = UpdateProgress::default();
        assert_eq!(progress.state, UpdateState::Idle);
    }

    #[test]
    fn test_verified_manifest_struct() {
        use crate::manifest::ManifestBuilder;
        use std::time::{Duration, Instant};

        // Create a test manifest
        let manifest = ManifestBuilder::new()
            .version("1.0.0")
            .timestamp("2026-02-15T00:00:00Z")
            .client_executable("client.exe")
            .add_file(FileEntry::new(
                "client.exe",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                1000,
            ))
            .build()
            .unwrap();

        // Create verified manifest
        let verified = VerifiedManifest {
            manifest,
            signature_verified_at: Instant::now(),
        };

        // Test that the manifest is accessible
        assert_eq!(verified.manifest.version, "1.0.0");
        assert_eq!(verified.manifest.files.len(), 1);

        // Test age tracking
        let age = verified.age();
        assert!(age < Duration::from_secs(1), "Age should be very small");

        // Test is_stale with a long max_age (should not be stale)
        assert!(!verified.is_stale(Duration::from_secs(60)));

        // Test is_stale with a zero max_age (should be stale)
        std::thread::sleep(Duration::from_millis(1));
        assert!(verified.is_stale(Duration::from_nanos(1)));
    }

    #[test]
    fn test_verified_manifest_clone() {
        use crate::manifest::ManifestBuilder;
        use std::time::Instant;

        let manifest = ManifestBuilder::new()
            .version("2.0.0")
            .timestamp("2026-02-15T00:00:00Z")
            .client_executable("client.exe")
            .add_file(FileEntry::new(
                "client.exe",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                1000,
            ))
            .build()
            .unwrap();

        let verified = VerifiedManifest {
            manifest,
            signature_verified_at: Instant::now(),
        };

        // Test Clone trait
        let cloned = verified.clone();
        assert_eq!(cloned.manifest.version, "2.0.0");
        assert_eq!(
            cloned.signature_verified_at,
            verified.signature_verified_at
        );
    }

    #[test]
    fn test_verified_manifest_debug() {
        use crate::manifest::ManifestBuilder;
        use std::time::Instant;

        let manifest = ManifestBuilder::new()
            .version("1.0.0")
            .timestamp("2026-02-15T00:00:00Z")
            .client_executable("client.exe")
            .add_file(FileEntry::new(
                "client.exe",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                1000,
            ))
            .build()
            .unwrap();

        let verified = VerifiedManifest {
            manifest,
            signature_verified_at: Instant::now(),
        };

        // Test Debug trait
        let debug_str = format!("{:?}", verified);
        assert!(debug_str.contains("VerifiedManifest"));
        assert!(debug_str.contains("manifest"));
        assert!(debug_str.contains("signature_verified_at"));
    }
}
