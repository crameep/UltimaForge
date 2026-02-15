//! Custom error types for UltimaForge.
//!
//! This module provides a unified error handling system for the application,
//! using `thiserror` for ergonomic error definitions. Each subsystem has its
//! own specific error type, and `UltimaForgeError` provides a top-level error
//! that can wrap any of them.
//!
//! # Error Hierarchy
//!
//! - [`UltimaForgeError`] - Top-level application error
//!   - [`DownloadError`] - HTTP download failures
//!   - [`UpdateError`] - Update mechanism failures
//!   - [`InstallError`] - Installation process failures
//!   - [`LaunchError`] - Client launch failures
//!   - Module-specific errors from `config`, `manifest`, `signature`, `hash`
//!
//! # Example
//!
//! ```ignore
//! use ultimaforge_lib::error::{UltimaForgeError, DownloadError};
//!
//! fn download_manifest() -> Result<Vec<u8>, UltimaForgeError> {
//!     // ... download logic ...
//!     Err(DownloadError::NetworkError {
//!         url: "https://example.com/manifest.json".to_string(),
//!         message: "Connection refused".to_string(),
//!     }.into())
//! }
//! ```

use std::io;
use std::path::PathBuf;

/// Top-level error type for UltimaForge operations.
///
/// This enum wraps all subsystem-specific errors and provides a unified
/// interface for error handling throughout the application.
#[derive(Debug, thiserror::Error)]
pub enum UltimaForgeError {
    /// Configuration-related error.
    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    /// Manifest parsing or validation error.
    #[error("Manifest error: {0}")]
    Manifest(#[from] crate::manifest::ManifestError),

    /// Signature verification error.
    #[error("Signature error: {0}")]
    Signature(#[from] crate::signature::SignatureVerificationError),

    /// File hash verification error.
    #[error("Hash error: {0}")]
    Hash(#[from] crate::hash::HashError),

    /// Download operation error.
    #[error("Download error: {0}")]
    Download(#[from] DownloadError),

    /// Update mechanism error.
    #[error("Update error: {0}")]
    Update(#[from] UpdateError),

    /// Installation error.
    #[error("Installation error: {0}")]
    Install(#[from] InstallError),

    /// Client launch error.
    #[error("Launch error: {0}")]
    Launch(#[from] LaunchError),

    /// Generic I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic error with a message.
    #[error("{0}")]
    Other(String),
}

impl UltimaForgeError {
    /// Creates a new `Other` error from a message.
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }

    /// Returns true if this error is recoverable and the operation can be retried.
    pub fn is_recoverable(&self) -> bool {
        match self {
            Self::Download(e) => e.is_recoverable(),
            Self::Update(e) => e.is_recoverable(),
            Self::Io(_) => true, // I/O errors are often transient
            _ => false,
        }
    }

    /// Returns a user-friendly message suitable for display in the UI.
    pub fn user_message(&self) -> String {
        match self {
            Self::Config(e) => format!("Configuration problem: {}", e),
            Self::Manifest(e) => format!("Invalid update manifest: {}", e),
            Self::Signature(e) => "Update verification failed. The update server may be unavailable or compromised.".to_string(),
            Self::Hash(e) => format!("File integrity check failed: {}", e),
            Self::Download(e) => e.user_message(),
            Self::Update(e) => e.user_message(),
            Self::Install(e) => e.user_message(),
            Self::Launch(e) => e.user_message(),
            Self::Io(e) => format!("File system error: {}", e),
            Self::Json(e) => format!("Data format error: {}", e),
            Self::Other(msg) => msg.clone(),
        }
    }
}

/// Errors that can occur during HTTP download operations.
#[derive(Debug, thiserror::Error)]
pub enum DownloadError {
    /// Network error (connection refused, DNS failure, etc.).
    #[error("Network error for '{url}': {message}")]
    NetworkError {
        url: String,
        message: String,
    },

    /// HTTP error response (4xx or 5xx).
    #[error("HTTP error {status} for '{url}': {message}")]
    HttpError {
        url: String,
        status: u16,
        message: String,
    },

    /// Request timed out.
    #[error("Request timed out for '{url}'")]
    Timeout {
        url: String,
    },

    /// Failed to write downloaded data to disk.
    #[error("Failed to write to '{path}': {source}")]
    WriteError {
        path: String,
        #[source]
        source: io::Error,
    },

    /// Downloaded file hash doesn't match expected hash.
    #[error("Hash mismatch for '{path}': expected {expected}, got {actual}")]
    HashMismatch {
        path: String,
        expected: String,
        actual: String,
    },

    /// Download was interrupted.
    #[error("Download interrupted for '{url}'")]
    Interrupted {
        url: String,
    },

    /// Insufficient disk space.
    #[error("Insufficient disk space: need {required} bytes, have {available} bytes")]
    InsufficientSpace {
        required: u64,
        available: u64,
    },

    /// Invalid URL format.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// SSL/TLS error.
    #[error("SSL/TLS error for '{url}': {message}")]
    SslError {
        url: String,
        message: String,
    },
}

impl DownloadError {
    /// Returns true if this error is recoverable and the download can be retried.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::NetworkError { .. }
                | Self::Timeout { .. }
                | Self::Interrupted { .. }
                | Self::HttpError { status, .. } if *status >= 500
        )
    }

    /// Returns a user-friendly message suitable for display in the UI.
    pub fn user_message(&self) -> String {
        match self {
            Self::NetworkError { .. } => "Connection failed. Please check your internet connection.".to_string(),
            Self::HttpError { status, .. } if *status == 404 => "Update file not found on server. Please contact server support.".to_string(),
            Self::HttpError { status, .. } if *status >= 500 => "Server error. Please try again later.".to_string(),
            Self::HttpError { status, .. } => format!("Server returned error {}.", status),
            Self::Timeout { .. } => "Download timed out. Please try again.".to_string(),
            Self::WriteError { path, .. } => format!("Failed to save file to '{}'. Check disk space and permissions.", path),
            Self::HashMismatch { .. } => "Downloaded file is corrupted. Retrying download.".to_string(),
            Self::Interrupted { .. } => "Download was interrupted. Please try again.".to_string(),
            Self::InsufficientSpace { required, available } => {
                format!(
                    "Not enough disk space. Need {} MB, but only {} MB available.",
                    required / (1024 * 1024),
                    available / (1024 * 1024)
                )
            }
            Self::InvalidUrl(_) => "Invalid download URL. Please contact server support.".to_string(),
            Self::SslError { .. } => "Secure connection failed. Please check your network settings.".to_string(),
        }
    }
}

/// Errors that can occur during the update process.
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// Failed to fetch the manifest from the server.
    #[error("Failed to fetch manifest: {0}")]
    ManifestFetchFailed(String),

    /// Manifest signature is missing.
    #[error("Manifest signature is missing")]
    MissingSignature,

    /// Failed to backup files before update.
    #[error("Failed to backup file '{path}': {source}")]
    BackupFailed {
        path: String,
        #[source]
        source: io::Error,
    },

    /// Failed to apply update (move files from staging).
    #[error("Failed to apply update for '{path}': {source}")]
    ApplyFailed {
        path: String,
        #[source]
        source: io::Error,
    },

    /// Rollback was required and completed.
    #[error("Update failed, rolled back to previous version: {reason}")]
    RolledBack {
        reason: String,
    },

    /// Rollback failed, installation may be corrupted.
    #[error("Update failed and rollback failed: {reason}. Installation may be corrupted.")]
    RollbackFailed {
        reason: String,
    },

    /// File is locked by another process.
    #[error("File '{path}' is locked by another process")]
    FileLocked {
        path: String,
    },

    /// Update was cancelled by user.
    #[error("Update cancelled by user")]
    Cancelled,

    /// No update is needed (already up to date).
    #[error("Already up to date")]
    AlreadyUpToDate,

    /// Staging directory error.
    #[error("Staging error: {0}")]
    StagingError(String),

    /// Version downgrade attempted.
    #[error("Cannot downgrade from version {current} to {target}")]
    DowngradeAttempted {
        current: String,
        target: String,
    },
}

impl UpdateError {
    /// Returns true if this error is recoverable.
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::ManifestFetchFailed(_)
                | Self::FileLocked { .. }
                | Self::Cancelled
                | Self::AlreadyUpToDate
        )
    }

    /// Returns a user-friendly message suitable for display in the UI.
    pub fn user_message(&self) -> String {
        match self {
            Self::ManifestFetchFailed(_) => "Failed to check for updates. Please try again.".to_string(),
            Self::MissingSignature => "Update verification failed. Server may be misconfigured.".to_string(),
            Self::BackupFailed { path, .. } => format!("Failed to backup '{}'. Check disk space.", path),
            Self::ApplyFailed { path, .. } => format!("Failed to update '{}'. Check permissions.", path),
            Self::RolledBack { reason } => format!("Update failed, restored previous version: {}", reason),
            Self::RollbackFailed { reason } => format!("Critical error: {}. Please reinstall.", reason),
            Self::FileLocked { path } => format!("'{}' is in use. Close other programs and try again.", path),
            Self::Cancelled => "Update cancelled.".to_string(),
            Self::AlreadyUpToDate => "You already have the latest version.".to_string(),
            Self::StagingError(msg) => format!("Update preparation failed: {}", msg),
            Self::DowngradeAttempted { current, target } => {
                format!("Cannot downgrade from v{} to v{}.", current, target)
            }
        }
    }
}

/// Errors that can occur during installation.
#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    /// Invalid installation path.
    #[error("Invalid installation path: {reason}")]
    InvalidPath {
        path: PathBuf,
        reason: String,
    },

    /// Installation directory already exists and is not empty.
    #[error("Directory '{path}' is not empty")]
    DirectoryNotEmpty {
        path: PathBuf,
    },

    /// Failed to create installation directory.
    #[error("Failed to create directory '{path}': {source}")]
    CreateDirectoryFailed {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    /// Installation was cancelled.
    #[error("Installation cancelled by user")]
    Cancelled,

    /// Insufficient permissions to write to directory.
    #[error("Insufficient permissions to write to '{path}'")]
    PermissionDenied {
        path: PathBuf,
    },

    /// Installation path is on a read-only filesystem.
    #[error("Installation path '{path}' is on a read-only filesystem")]
    ReadOnlyFilesystem {
        path: PathBuf,
    },

    /// Previous installation exists but is corrupted.
    #[error("Existing installation at '{path}' is corrupted")]
    CorruptedInstallation {
        path: PathBuf,
    },

    /// Failed to save installation configuration.
    #[error("Failed to save configuration: {0}")]
    ConfigSaveFailed(String),
}

impl InstallError {
    /// Returns a user-friendly message suitable for display in the UI.
    pub fn user_message(&self) -> String {
        match self {
            Self::InvalidPath { path, reason } => {
                format!("Cannot install to '{}': {}", path.display(), reason)
            }
            Self::DirectoryNotEmpty { path } => {
                format!("'{}' is not empty. Choose an empty folder or a new location.", path.display())
            }
            Self::CreateDirectoryFailed { path, .. } => {
                format!("Cannot create folder '{}'. Check permissions.", path.display())
            }
            Self::Cancelled => "Installation cancelled.".to_string(),
            Self::PermissionDenied { path } => {
                format!("No permission to write to '{}'. Try running as administrator.", path.display())
            }
            Self::ReadOnlyFilesystem { path } => {
                format!("Cannot write to '{}'. The drive may be write-protected.", path.display())
            }
            Self::CorruptedInstallation { path } => {
                format!("Existing installation at '{}' is damaged. Delete it and reinstall.", path.display())
            }
            Self::ConfigSaveFailed(msg) => {
                format!("Failed to save settings: {}", msg)
            }
        }
    }
}

/// Errors that can occur when launching the game client.
#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    /// Client executable not found.
    #[error("Client executable not found at '{path}'")]
    ExecutableNotFound {
        path: PathBuf,
    },

    /// Failed to start the client process.
    #[error("Failed to start client: {source}")]
    ProcessSpawnFailed {
        #[source]
        source: io::Error,
    },

    /// Client executable exists but is not executable.
    #[error("Client at '{path}' is not executable")]
    NotExecutable {
        path: PathBuf,
    },

    /// Installation is not complete.
    #[error("Installation is not complete. Please run the installer first.")]
    InstallationIncomplete,

    /// Installation path is not configured.
    #[error("Installation path is not configured")]
    NoInstallPath,

    /// Client crashed immediately after launch.
    #[error("Client exited immediately with code {code}")]
    ClientCrashed {
        code: i32,
    },

    /// Antivirus or security software blocked the launch.
    #[error("Client launch was blocked. Check your antivirus settings.")]
    Blocked,
}

impl LaunchError {
    /// Returns a user-friendly message suitable for display in the UI.
    pub fn user_message(&self) -> String {
        match self {
            Self::ExecutableNotFound { path } => {
                format!("Game executable not found at '{}'. Try reinstalling.", path.display())
            }
            Self::ProcessSpawnFailed { .. } => {
                "Failed to start the game. Check your antivirus settings.".to_string()
            }
            Self::NotExecutable { path } => {
                format!("'{}' is not a valid executable.", path.display())
            }
            Self::InstallationIncomplete => {
                "Installation is incomplete. Please complete the installation first.".to_string()
            }
            Self::NoInstallPath => {
                "Game location is not set. Please configure the installation path.".to_string()
            }
            Self::ClientCrashed { code } => {
                format!("Game crashed with error code {}. Try reinstalling.", code)
            }
            Self::Blocked => {
                "Game launch was blocked. Add the game folder to your antivirus exclusions.".to_string()
            }
        }
    }
}

/// Result type alias for UltimaForge operations.
pub type Result<T> = std::result::Result<T, UltimaForgeError>;

/// Result type alias for download operations.
pub type DownloadResult<T> = std::result::Result<T, DownloadError>;

/// Result type alias for update operations.
pub type UpdateResult<T> = std::result::Result<T, UpdateError>;

/// Result type alias for installation operations.
pub type InstallResult<T> = std::result::Result<T, InstallError>;

/// Result type alias for launch operations.
pub type LaunchResult<T> = std::result::Result<T, LaunchError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_download_error_recoverable() {
        assert!(DownloadError::NetworkError {
            url: "test".to_string(),
            message: "failed".to_string(),
        }
        .is_recoverable());

        assert!(DownloadError::Timeout {
            url: "test".to_string(),
        }
        .is_recoverable());

        assert!(DownloadError::Interrupted {
            url: "test".to_string(),
        }
        .is_recoverable());

        // 500 errors are recoverable
        assert!(DownloadError::HttpError {
            url: "test".to_string(),
            status: 500,
            message: "server error".to_string(),
        }
        .is_recoverable());

        // 404 is not recoverable
        assert!(!DownloadError::HttpError {
            url: "test".to_string(),
            status: 404,
            message: "not found".to_string(),
        }
        .is_recoverable());

        assert!(!DownloadError::HashMismatch {
            path: "test".to_string(),
            expected: "a".to_string(),
            actual: "b".to_string(),
        }
        .is_recoverable());
    }

    #[test]
    fn test_update_error_recoverable() {
        assert!(UpdateError::ManifestFetchFailed("test".to_string()).is_recoverable());
        assert!(UpdateError::FileLocked {
            path: "test".to_string(),
        }
        .is_recoverable());
        assert!(UpdateError::Cancelled.is_recoverable());
        assert!(UpdateError::AlreadyUpToDate.is_recoverable());

        assert!(!UpdateError::RollbackFailed {
            reason: "test".to_string(),
        }
        .is_recoverable());
    }

    #[test]
    fn test_download_error_display() {
        let error = DownloadError::HashMismatch {
            path: "test.exe".to_string(),
            expected: "abc".to_string(),
            actual: "def".to_string(),
        };

        let display = format!("{}", error);
        assert!(display.contains("test.exe"));
        assert!(display.contains("abc"));
        assert!(display.contains("def"));
    }

    #[test]
    fn test_ultimaforge_error_from_download() {
        let download_err = DownloadError::Timeout {
            url: "https://example.com".to_string(),
        };

        let app_err: UltimaForgeError = download_err.into();
        assert!(matches!(app_err, UltimaForgeError::Download(_)));
    }

    #[test]
    fn test_ultimaforge_error_user_message() {
        let error = UltimaForgeError::Download(DownloadError::Timeout {
            url: "https://example.com".to_string(),
        });

        let msg = error.user_message();
        assert!(msg.contains("timed out"));
    }

    #[test]
    fn test_install_error_display() {
        let error = InstallError::DirectoryNotEmpty {
            path: PathBuf::from("/test/path"),
        };

        let display = format!("{}", error);
        assert!(display.contains("not empty"));
    }

    #[test]
    fn test_launch_error_display() {
        let error = LaunchError::ExecutableNotFound {
            path: PathBuf::from("client.exe"),
        };

        let display = format!("{}", error);
        assert!(display.contains("client.exe"));
        assert!(display.contains("not found"));
    }

    #[test]
    fn test_ultimaforge_error_is_recoverable() {
        let recoverable = UltimaForgeError::Download(DownloadError::Timeout {
            url: "test".to_string(),
        });
        assert!(recoverable.is_recoverable());

        let not_recoverable = UltimaForgeError::other("some error");
        assert!(!not_recoverable.is_recoverable());
    }

    #[test]
    fn test_error_other_constructor() {
        let error = UltimaForgeError::other("custom error message");
        assert!(matches!(error, UltimaForgeError::Other(_)));
        assert_eq!(error.user_message(), "custom error message");
    }

    #[test]
    fn test_insufficient_space_formatting() {
        let error = DownloadError::InsufficientSpace {
            required: 1024 * 1024 * 100, // 100 MB
            available: 1024 * 1024 * 50,  // 50 MB
        };

        let msg = error.user_message();
        assert!(msg.contains("100 MB"));
        assert!(msg.contains("50 MB"));
    }

    #[test]
    fn test_update_error_downgrade() {
        let error = UpdateError::DowngradeAttempted {
            current: "2.0.0".to_string(),
            target: "1.0.0".to_string(),
        };

        let display = format!("{}", error);
        assert!(display.contains("2.0.0"));
        assert!(display.contains("1.0.0"));
        assert!(display.contains("downgrade"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let app_err: UltimaForgeError = io_err.into();
        assert!(matches!(app_err, UltimaForgeError::Io(_)));
    }

    #[test]
    fn test_install_error_user_messages() {
        let cancelled = InstallError::Cancelled;
        assert_eq!(cancelled.user_message(), "Installation cancelled.");

        let permission_denied = InstallError::PermissionDenied {
            path: PathBuf::from("/restricted"),
        };
        assert!(permission_denied.user_message().contains("administrator"));
    }

    #[test]
    fn test_launch_error_user_messages() {
        let incomplete = LaunchError::InstallationIncomplete;
        assert!(incomplete.user_message().contains("incomplete"));

        let no_path = LaunchError::NoInstallPath;
        assert!(no_path.user_message().contains("not set"));
    }
}
