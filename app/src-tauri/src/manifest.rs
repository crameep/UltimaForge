//! Manifest schema definition and parsing for UltimaForge.
//!
//! This module defines the manifest format used to describe the files
//! that make up a UO client installation. The manifest is signed by
//! the server owner and verified before trusting its contents.
//!
//! # Security
//!
//! - ALWAYS verify the manifest signature before parsing
//! - Never trust manifest contents until signature is verified
//! - Use [`crate::signature::verify_manifest`] before calling [`Manifest::parse`]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

/// Errors that can occur during manifest parsing and validation.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// The manifest JSON is malformed or invalid.
    #[error("Invalid manifest JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    /// A required field is missing from the manifest.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// The manifest version format is invalid.
    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    /// A file path in the manifest is invalid (e.g., path traversal attempt).
    #[error("Invalid file path: {0}")]
    InvalidPath(String),

    /// A path escapes its intended containment directory.
    #[error("Path containment violation: '{target}' escapes base directory '{base}'")]
    PathContainment { base: String, target: String },

    /// Failed to canonicalize a path (e.g., path doesn't exist).
    #[error("Failed to canonicalize path '{path}': {reason}")]
    CanonicalizationFailed { path: String, reason: String },

    /// A SHA-256 hash in the manifest is invalid.
    #[error("Invalid SHA-256 hash for file '{path}': {reason}")]
    InvalidHash { path: String, reason: String },

    /// A file size in the manifest is invalid.
    #[error("Invalid file size for file '{path}': {reason}")]
    InvalidSize { path: String, reason: String },

    /// The total_size doesn't match the sum of individual file sizes.
    #[error("Total size mismatch: declared {declared}, calculated {calculated}")]
    TotalSizeMismatch { declared: u64, calculated: u64 },

    /// The client_executable is not in the files list.
    #[error("Client executable '{0}' not found in files list")]
    ExecutableNotInFiles(String),
}

/// Checks if a path is a safe relative path that doesn't escape the base directory.
///
/// Uses `std::path::Component` iteration for robust validation instead of
/// string-based checks which can be bypassed with mixed path separators.
///
/// # Security
///
/// This function prevents path traversal attacks by rejecting:
/// - Absolute paths (`/etc/passwd`, `C:\Windows`)
/// - Windows drive prefixes (`C:`, `D:`)
/// - UNC paths (`\\server\share`)
/// - Parent directory traversal (`..`, `foo/../bar`)
///
/// # Arguments
///
/// * `path` - The path to validate
///
/// # Returns
///
/// `true` if the path is safe (relative with no escaping), `false` otherwise.
///
/// # Examples
///
/// ```
/// use std::path::Path;
/// use ultimaforge_lib::manifest::is_safe_relative_path;
///
/// // Safe paths
/// assert!(is_safe_relative_path(Path::new("client.exe")));
/// assert!(is_safe_relative_path(Path::new("data/maps/map0.mul")));
/// assert!(is_safe_relative_path(Path::new("./config.ini")));
///
/// // Unsafe paths
/// assert!(!is_safe_relative_path(Path::new("../../../etc/passwd")));
/// assert!(!is_safe_relative_path(Path::new("/etc/passwd")));
/// assert!(!is_safe_relative_path(Path::new("C:\\Windows\\System32")));
/// assert!(!is_safe_relative_path(Path::new("\\\\server\\share")));
/// ```
pub fn is_safe_relative_path(path: &Path) -> bool {
    // Reject absolute paths early
    if path.is_absolute() {
        return false;
    }

    // Check each component for dangerous elements
    for component in path.components() {
        match component {
            // Windows drive prefix (C:, D:) or UNC path (\\server\share)
            Component::Prefix(_) => return false,
            // Root directory (/) - indicates absolute path
            Component::RootDir => return false,
            // Parent directory traversal (..) - could escape base directory
            Component::ParentDir => return false,
            // Current directory (.) - safe, just skip
            Component::CurDir => continue,
            // Normal path segment (file/directory name) - safe
            Component::Normal(_) => continue,
        }
    }

    // Empty path is not safe (nothing to operate on)
    if path.as_os_str().is_empty() {
        return false;
    }

    true
}

/// Validates that a target path, when joined to a base directory, stays within
/// that base directory after canonicalization.
///
/// This function performs post-join validation to prevent path traversal attacks
/// that might bypass simple string-based checks. It uses filesystem canonicalization
/// to resolve symlinks and normalize paths before checking containment.
///
/// # Security
///
/// This function provides defense-in-depth against path traversal:
/// - Resolves symlinks that might escape the base directory
/// - Normalizes `..` components that survived initial validation
/// - Handles platform-specific path quirks (Windows junctions, etc.)
///
/// # Arguments
///
/// * `base` - The base directory that should contain the target. Must exist.
/// * `target` - The relative path to validate. When joined with base, must stay within base.
///
/// # Returns
///
/// * `Ok(PathBuf)` - The canonicalized target path if it stays within base
/// * `Err(ManifestError)` - If the path escapes base or canonicalization fails
///
/// # Errors
///
/// * `ManifestError::CanonicalizationFailed` - If base or joined path cannot be canonicalized
///   (e.g., the path doesn't exist on disk)
/// * `ManifestError::PathContainment` - If the canonicalized target escapes the base directory
///
/// # Examples
///
/// ```ignore
/// use std::path::Path;
///
/// let base = Path::new("/install/dir");
/// let target = Path::new("data/file.txt");
///
/// // Valid: stays within base
/// let result = validate_path_containment(base, target)?;
/// assert!(result.starts_with("/install/dir"));
///
/// // Invalid: escapes base (would error)
/// let malicious = Path::new("../../../etc/passwd");
/// assert!(validate_path_containment(base, malicious).is_err());
/// ```
///
/// # Note
///
/// This function requires the paths to exist on the filesystem for canonicalization.
/// Use `is_safe_relative_path` for pre-validation before creating files, and this
/// function for post-validation after files are written.
pub fn validate_path_containment(base: &Path, target: &Path) -> Result<PathBuf, ManifestError> {
    // Join the base and target paths
    let joined = base.join(target);

    // Canonicalize the base directory to get its absolute, normalized form
    let canonical_base =
        base.canonicalize()
            .map_err(|e| ManifestError::CanonicalizationFailed {
                path: base.display().to_string(),
                reason: e.to_string(),
            })?;

    // Canonicalize the joined path to resolve symlinks and normalize ..
    let canonical_target =
        joined
            .canonicalize()
            .map_err(|e| ManifestError::CanonicalizationFailed {
                path: joined.display().to_string(),
                reason: e.to_string(),
            })?;

    // Verify the canonical target is still within the canonical base
    if !canonical_target.starts_with(&canonical_base) {
        return Err(ManifestError::PathContainment {
            base: canonical_base.display().to_string(),
            target: canonical_target.display().to_string(),
        });
    }

    Ok(canonical_target)
}

/// Represents a single file entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    /// Relative path to the file within the installation directory.
    pub path: String,

    /// SHA-256 hash of the file contents (hex-encoded, lowercase).
    pub sha256: String,

    /// Size of the file in bytes.
    pub size: u64,

    /// Whether this file is required for the client to function.
    /// Optional files may be skipped if they fail to download.
    #[serde(default = "default_required")]
    pub required: bool,
}

fn default_required() -> bool {
    true
}

impl FileEntry {
    /// Creates a new file entry.
    pub fn new(path: impl Into<String>, sha256: impl Into<String>, size: u64) -> Self {
        Self {
            path: path.into(),
            sha256: sha256.into(),
            size,
            required: true,
        }
    }

    /// Sets whether this file is required.
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Validates the file entry.
    ///
    /// Uses Component-based path validation to prevent path traversal attacks.
    /// This is more secure than string-based checks which can be bypassed with
    /// mixed path separators (e.g., `foo/bar\..\secret`).
    pub fn validate(&self) -> Result<(), ManifestError> {
        // Check for path traversal attacks using Component-based validation
        // This rejects:
        // - Absolute paths (/etc/passwd, C:\Windows)
        // - Windows drive prefixes (C:, D:)
        // - UNC paths (\\server\share)
        // - Parent directory traversal (.., foo/../bar)
        let path = Path::new(&self.path);
        if !is_safe_relative_path(path) {
            return Err(ManifestError::InvalidPath(self.path.clone()));
        }

        // Validate SHA-256 hash format (64 hex characters)
        if self.sha256.len() != 64 {
            return Err(ManifestError::InvalidHash {
                path: self.path.clone(),
                reason: format!("expected 64 hex characters, got {}", self.sha256.len()),
            });
        }

        if !self.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ManifestError::InvalidHash {
                path: self.path.clone(),
                reason: "contains non-hexadecimal characters".to_string(),
            });
        }

        Ok(())
    }

    /// Returns the blob URL for this file given a base URL.
    ///
    /// Files are stored in content-addressed storage using their SHA-256 hash.
    pub fn blob_url(&self, base_url: &str) -> String {
        format!("{}/files/{}", base_url.trim_end_matches('/'), self.sha256)
    }
}

/// The main manifest structure describing a UO client distribution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    /// Version string for this release (e.g., "1.0.0").
    pub version: String,

    /// ISO 8601 timestamp when this manifest was created.
    pub timestamp: String,

    /// Relative path to the client executable within the installation.
    pub client_executable: String,

    /// Command-line arguments to pass when launching the client.
    #[serde(default)]
    pub client_args: Vec<String>,

    /// List of all files in this distribution.
    pub files: Vec<FileEntry>,

    /// Total size of all files in bytes.
    pub total_size: u64,

    /// Optional URL to patch notes for this version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch_notes_url: Option<String>,
}

impl Manifest {
    /// Parses a manifest from JSON bytes.
    ///
    /// # Security Warning
    ///
    /// ALWAYS verify the manifest signature before calling this function.
    /// Use [`crate::signature::verify_manifest`] first.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use ultimaforge_lib::{signature, manifest::Manifest};
    ///
    /// // First verify signature
    /// signature::verify_manifest(manifest_bytes, signature_bytes, public_key)?;
    ///
    /// // Only then parse
    /// let manifest = Manifest::parse(manifest_bytes)?;
    /// ```
    pub fn parse(json_bytes: &[u8]) -> Result<Self, ManifestError> {
        let manifest: Self = serde_json::from_slice(json_bytes)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Parses a manifest from a JSON string.
    ///
    /// # Security Warning
    ///
    /// ALWAYS verify the manifest signature before calling this function.
    pub fn parse_str(json_str: &str) -> Result<Self, ManifestError> {
        let manifest: Self = serde_json::from_str(json_str)?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Serializes the manifest to JSON bytes.
    pub fn to_json(&self) -> Result<Vec<u8>, ManifestError> {
        Ok(serde_json::to_vec_pretty(self)?)
    }

    /// Serializes the manifest to a JSON string.
    pub fn to_json_string(&self) -> Result<String, ManifestError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Validates the manifest structure and contents.
    pub fn validate(&self) -> Result<(), ManifestError> {
        // Validate version format (semver-like: major.minor.patch)
        self.validate_version()?;

        // Validate all file entries
        for file in &self.files {
            file.validate()?;
        }

        // Validate client_executable is in the files list
        if !self.files.iter().any(|f| f.path == self.client_executable) {
            return Err(ManifestError::ExecutableNotInFiles(
                self.client_executable.clone(),
            ));
        }

        // Validate total_size matches sum of file sizes
        let calculated_size: u64 = self.files.iter().map(|f| f.size).sum();
        if calculated_size != self.total_size {
            return Err(ManifestError::TotalSizeMismatch {
                declared: self.total_size,
                calculated: calculated_size,
            });
        }

        Ok(())
    }

    /// Validates the version string format.
    fn validate_version(&self) -> Result<(), ManifestError> {
        if self.version.is_empty() {
            return Err(ManifestError::InvalidVersion(
                "version cannot be empty".to_string(),
            ));
        }

        // Basic semver validation: at least one digit
        if !self.version.chars().any(|c| c.is_ascii_digit()) {
            return Err(ManifestError::InvalidVersion(format!(
                "'{}' must contain at least one digit",
                self.version
            )));
        }

        Ok(())
    }

    /// Returns the number of files in the manifest.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Returns the number of required files in the manifest.
    pub fn required_file_count(&self) -> usize {
        self.files.iter().filter(|f| f.required).count()
    }

    /// Returns an iterator over all file entries.
    pub fn iter_files(&self) -> impl Iterator<Item = &FileEntry> {
        self.files.iter()
    }

    /// Returns an iterator over only required file entries.
    pub fn iter_required_files(&self) -> impl Iterator<Item = &FileEntry> {
        self.files.iter().filter(|f| f.required)
    }

    /// Finds a file entry by its path.
    pub fn find_file(&self, path: &str) -> Option<&FileEntry> {
        self.files.iter().find(|f| f.path == path)
    }

    /// Finds a file entry by its SHA-256 hash.
    pub fn find_file_by_hash(&self, hash: &str) -> Option<&FileEntry> {
        self.files.iter().find(|f| f.sha256 == hash)
    }

    /// Returns the client executable file entry.
    pub fn client_executable_entry(&self) -> Option<&FileEntry> {
        self.find_file(&self.client_executable)
    }

    /// Computes which files need to be downloaded/updated given a map of
    /// local file paths to their SHA-256 hashes.
    ///
    /// Returns a list of files that either don't exist locally or have
    /// different hashes.
    pub fn files_to_update<'a>(
        &'a self,
        local_hashes: &HashMap<String, String>,
    ) -> Vec<&'a FileEntry> {
        self.files
            .iter()
            .filter(|file| {
                match local_hashes.get(&file.path) {
                    Some(local_hash) => local_hash != &file.sha256,
                    None => true, // File doesn't exist locally
                }
            })
            .collect()
    }

    /// Computes the total download size for files that need updating.
    pub fn update_size(&self, local_hashes: &HashMap<String, String>) -> u64 {
        self.files_to_update(local_hashes)
            .iter()
            .map(|f| f.size)
            .sum()
    }

    /// Returns human-readable size string (KB, MB, GB).
    pub fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }
}

/// Builder for creating manifests programmatically.
#[derive(Debug, Default)]
pub struct ManifestBuilder {
    version: Option<String>,
    timestamp: Option<String>,
    client_executable: Option<String>,
    client_args: Vec<String>,
    files: Vec<FileEntry>,
    patch_notes_url: Option<String>,
}

impl ManifestBuilder {
    /// Creates a new manifest builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the version string.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Sets the timestamp.
    pub fn timestamp(mut self, timestamp: impl Into<String>) -> Self {
        self.timestamp = Some(timestamp.into());
        self
    }

    /// Sets the client executable path.
    pub fn client_executable(mut self, path: impl Into<String>) -> Self {
        self.client_executable = Some(path.into());
        self
    }

    /// Sets the client arguments.
    pub fn client_args(mut self, args: Vec<String>) -> Self {
        self.client_args = args;
        self
    }

    /// Adds a client argument.
    pub fn add_client_arg(mut self, arg: impl Into<String>) -> Self {
        self.client_args.push(arg.into());
        self
    }

    /// Adds a file entry.
    pub fn add_file(mut self, file: FileEntry) -> Self {
        self.files.push(file);
        self
    }

    /// Adds multiple file entries.
    pub fn add_files(mut self, files: impl IntoIterator<Item = FileEntry>) -> Self {
        self.files.extend(files);
        self
    }

    /// Sets the patch notes URL.
    pub fn patch_notes_url(mut self, url: impl Into<String>) -> Self {
        self.patch_notes_url = Some(url.into());
        self
    }

    /// Builds the manifest.
    ///
    /// Returns an error if required fields are missing.
    pub fn build(self) -> Result<Manifest, ManifestError> {
        let version = self
            .version
            .ok_or_else(|| ManifestError::MissingField("version".to_string()))?;

        let timestamp = self
            .timestamp
            .ok_or_else(|| ManifestError::MissingField("timestamp".to_string()))?;

        let client_executable = self
            .client_executable
            .ok_or_else(|| ManifestError::MissingField("client_executable".to_string()))?;

        let total_size: u64 = self.files.iter().map(|f| f.size).sum();

        let manifest = Manifest {
            version,
            timestamp,
            client_executable,
            client_args: self.client_args,
            files: self.files,
            total_size,
            patch_notes_url: self.patch_notes_url,
        };

        manifest.validate()?;
        Ok(manifest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // ==================== is_safe_relative_path tests ====================

    #[test]
    fn test_is_safe_relative_path_accepts_simple_filename() {
        assert!(is_safe_relative_path(Path::new("client.exe")));
        assert!(is_safe_relative_path(Path::new("readme.txt")));
        assert!(is_safe_relative_path(Path::new("data.mul")));
    }

    #[test]
    fn test_is_safe_relative_path_accepts_subdirectory_paths() {
        assert!(is_safe_relative_path(Path::new("data/map0.mul")));
        assert!(is_safe_relative_path(Path::new("data/maps/world.map")));
        assert!(is_safe_relative_path(Path::new(
            "assets/textures/grass.png"
        )));
    }

    #[test]
    fn test_is_safe_relative_path_accepts_current_dir() {
        assert!(is_safe_relative_path(Path::new("./client.exe")));
        assert!(is_safe_relative_path(Path::new("./data/map0.mul")));
    }

    #[test]
    fn test_is_safe_relative_path_rejects_parent_traversal() {
        assert!(!is_safe_relative_path(Path::new("..")));
        assert!(!is_safe_relative_path(Path::new("../secret")));
        assert!(!is_safe_relative_path(Path::new("../../../etc/passwd")));
        assert!(!is_safe_relative_path(Path::new(
            "data/../../../etc/passwd"
        )));
    }

    #[test]
    fn test_is_safe_relative_path_rejects_hidden_parent_traversal() {
        // Traversal embedded in the middle of a path
        assert!(!is_safe_relative_path(Path::new("foo/../bar")));
        assert!(!is_safe_relative_path(Path::new(
            "data/subdir/../../../etc/passwd"
        )));
    }

    #[test]
    fn test_is_safe_relative_path_rejects_absolute_unix() {
        assert!(!is_safe_relative_path(Path::new("/etc/passwd")));
        assert!(!is_safe_relative_path(Path::new("/usr/bin/bash")));
        assert!(!is_safe_relative_path(Path::new("/home/user/.bashrc")));
    }

    #[test]
    fn test_is_safe_relative_path_rejects_absolute_windows() {
        // Note: On non-Windows platforms, these may not be detected as absolute
        // but the Component::Prefix check will catch the drive letter
        assert!(!is_safe_relative_path(Path::new("C:\\Windows\\System32")));
        assert!(!is_safe_relative_path(Path::new(
            "D:\\Program Files\\app.exe"
        )));
    }

    #[test]
    fn test_is_safe_relative_path_rejects_unc_paths() {
        // UNC paths (Windows network shares)
        assert!(!is_safe_relative_path(Path::new("\\\\server\\share")));
        assert!(!is_safe_relative_path(Path::new(
            "\\\\server\\share\\file.txt"
        )));
    }

    #[test]
    fn test_is_safe_relative_path_rejects_backslash_root() {
        // Paths starting with backslash (absolute on Windows)
        assert!(!is_safe_relative_path(Path::new("\\Windows\\System32")));
    }

    #[test]
    fn test_is_safe_relative_path_rejects_mixed_separator_traversal() {
        // Mixed forward/back slashes with traversal
        assert!(!is_safe_relative_path(Path::new("foo\\..\\bar")));
        assert!(!is_safe_relative_path(Path::new("data/..\\secret")));
    }

    #[test]
    fn test_is_safe_relative_path_rejects_empty_path() {
        assert!(!is_safe_relative_path(Path::new("")));
    }

    #[test]
    fn test_is_safe_relative_path_accepts_windows_subdirectory_format() {
        // Relative paths with backslashes (common in Windows manifests)
        assert!(is_safe_relative_path(Path::new("data\\maps\\map0.mul")));
        assert!(is_safe_relative_path(Path::new(
            "assets\\textures\\grass.png"
        )));
    }

    // ==================== Original manifest tests continue below ====================

    /// Creates a valid test manifest JSON string.
    fn valid_manifest_json() -> String {
        r#"{
            "version": "1.0.0",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "client_args": ["-connect", "server.example.com"],
            "files": [
                {
                    "path": "client.exe",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "size": 12345678,
                    "required": true
                },
                {
                    "path": "data/map0.mul",
                    "sha256": "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3",
                    "size": 87654321,
                    "required": true
                }
            ],
            "total_size": 99999999,
            "patch_notes_url": "patchnotes.md"
        }"#
        .to_string()
    }

    #[test]
    fn test_parse_valid_manifest() {
        let json = valid_manifest_json();
        let manifest = Manifest::parse_str(&json).expect("Should parse valid manifest");

        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.timestamp, "2026-02-15T00:00:00Z");
        assert_eq!(manifest.client_executable, "client.exe");
        assert_eq!(manifest.client_args, vec!["-connect", "server.example.com"]);
        assert_eq!(manifest.files.len(), 2);
        assert_eq!(manifest.total_size, 99999999);
        assert_eq!(manifest.patch_notes_url, Some("patchnotes.md".to_string()));
    }

    #[test]
    fn test_parse_minimal_manifest() {
        let json = r#"{
            "version": "1.0.0",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "files": [
                {
                    "path": "client.exe",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "size": 1000
                }
            ],
            "total_size": 1000
        }"#;

        let manifest = Manifest::parse_str(json).expect("Should parse minimal manifest");

        assert_eq!(manifest.version, "1.0.0");
        assert!(manifest.client_args.is_empty());
        assert!(manifest.patch_notes_url.is_none());
        // Default required value should be true
        assert!(manifest.files[0].required);
    }

    #[test]
    fn test_parse_invalid_json() {
        let invalid_json = "{ not valid json }";
        let result = Manifest::parse_str(invalid_json);

        assert!(matches!(result, Err(ManifestError::InvalidJson(_))));
    }

    #[test]
    fn test_parse_missing_field() {
        let json = r#"{
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "files": [],
            "total_size": 0
        }"#;

        let result = Manifest::parse_str(json);
        assert!(matches!(result, Err(ManifestError::InvalidJson(_))));
    }

    #[test]
    fn test_validate_path_traversal() {
        let json = r#"{
            "version": "1.0.0",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "files": [
                {
                    "path": "../../../etc/passwd",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "size": 1000
                }
            ],
            "total_size": 1000
        }"#;

        let result = Manifest::parse_str(json);
        assert!(matches!(result, Err(ManifestError::InvalidPath(_))));
    }

    #[test]
    fn test_validate_absolute_path() {
        let json = r#"{
            "version": "1.0.0",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "files": [
                {
                    "path": "/etc/passwd",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "size": 1000
                }
            ],
            "total_size": 1000
        }"#;

        let result = Manifest::parse_str(json);
        assert!(matches!(result, Err(ManifestError::InvalidPath(_))));
    }

    #[test]
    fn test_validate_invalid_hash_length() {
        let json = r#"{
            "version": "1.0.0",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "files": [
                {
                    "path": "client.exe",
                    "sha256": "tooshort",
                    "size": 1000
                }
            ],
            "total_size": 1000
        }"#;

        let result = Manifest::parse_str(json);
        assert!(matches!(result, Err(ManifestError::InvalidHash { .. })));
    }

    #[test]
    fn test_validate_invalid_hash_characters() {
        let json = r#"{
            "version": "1.0.0",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "files": [
                {
                    "path": "client.exe",
                    "sha256": "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
                    "size": 1000
                }
            ],
            "total_size": 1000
        }"#;

        let result = Manifest::parse_str(json);
        assert!(matches!(result, Err(ManifestError::InvalidHash { .. })));
    }

    #[test]
    fn test_validate_total_size_mismatch() {
        let json = r#"{
            "version": "1.0.0",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "files": [
                {
                    "path": "client.exe",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "size": 1000
                }
            ],
            "total_size": 9999
        }"#;

        let result = Manifest::parse_str(json);
        assert!(matches!(
            result,
            Err(ManifestError::TotalSizeMismatch { .. })
        ));
    }

    #[test]
    fn test_validate_executable_not_in_files() {
        let json = r#"{
            "version": "1.0.0",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "missing.exe",
            "files": [
                {
                    "path": "client.exe",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "size": 1000
                }
            ],
            "total_size": 1000
        }"#;

        let result = Manifest::parse_str(json);
        assert!(matches!(
            result,
            Err(ManifestError::ExecutableNotInFiles(_))
        ));
    }

    #[test]
    fn test_validate_empty_version() {
        let json = r#"{
            "version": "",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "files": [
                {
                    "path": "client.exe",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "size": 1000
                }
            ],
            "total_size": 1000
        }"#;

        let result = Manifest::parse_str(json);
        assert!(matches!(result, Err(ManifestError::InvalidVersion(_))));
    }

    #[test]
    fn test_file_entry_blob_url() {
        let file = FileEntry::new(
            "client.exe",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            1000,
        );

        assert_eq!(
            file.blob_url("https://updates.example.com"),
            "https://updates.example.com/files/e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // Should handle trailing slash
        assert_eq!(
            file.blob_url("https://updates.example.com/"),
            "https://updates.example.com/files/e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_manifest_file_count() {
        let json = valid_manifest_json();
        let manifest = Manifest::parse_str(&json).unwrap();

        assert_eq!(manifest.file_count(), 2);
        assert_eq!(manifest.required_file_count(), 2);
    }

    #[test]
    fn test_manifest_find_file() {
        let json = valid_manifest_json();
        let manifest = Manifest::parse_str(&json).unwrap();

        let file = manifest.find_file("client.exe");
        assert!(file.is_some());
        assert_eq!(file.unwrap().size, 12345678);

        let missing = manifest.find_file("nonexistent.exe");
        assert!(missing.is_none());
    }

    #[test]
    fn test_manifest_find_file_by_hash() {
        let json = valid_manifest_json();
        let manifest = Manifest::parse_str(&json).unwrap();

        let file = manifest
            .find_file_by_hash("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert!(file.is_some());
        assert_eq!(file.unwrap().path, "client.exe");
    }

    #[test]
    fn test_manifest_files_to_update() {
        let json = valid_manifest_json();
        let manifest = Manifest::parse_str(&json).unwrap();

        // All files need update when local is empty
        let empty_local: HashMap<String, String> = HashMap::new();
        let updates = manifest.files_to_update(&empty_local);
        assert_eq!(updates.len(), 2);

        // One file up to date, one needs update
        let mut partial_local = HashMap::new();
        partial_local.insert(
            "client.exe".to_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        );
        let updates = manifest.files_to_update(&partial_local);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].path, "data/map0.mul");

        // All files up to date
        let mut full_local = partial_local.clone();
        full_local.insert(
            "data/map0.mul".to_string(),
            "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3".to_string(),
        );
        let updates = manifest.files_to_update(&full_local);
        assert!(updates.is_empty());
    }

    #[test]
    fn test_manifest_update_size() {
        let json = valid_manifest_json();
        let manifest = Manifest::parse_str(&json).unwrap();

        let empty_local: HashMap<String, String> = HashMap::new();
        assert_eq!(manifest.update_size(&empty_local), 99999999);

        let mut partial_local = HashMap::new();
        partial_local.insert(
            "client.exe".to_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        );
        assert_eq!(manifest.update_size(&partial_local), 87654321);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(Manifest::format_size(500), "500 bytes");
        assert_eq!(Manifest::format_size(1024), "1.00 KB");
        assert_eq!(Manifest::format_size(1536), "1.50 KB");
        assert_eq!(Manifest::format_size(1048576), "1.00 MB");
        assert_eq!(Manifest::format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_manifest_builder() {
        let manifest = ManifestBuilder::new()
            .version("2.0.0")
            .timestamp("2026-02-15T12:00:00Z")
            .client_executable("game.exe")
            .add_client_arg("-windowed")
            .add_file(FileEntry::new(
                "game.exe",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                5000,
            ))
            .patch_notes_url("notes.md")
            .build()
            .expect("Should build valid manifest");

        assert_eq!(manifest.version, "2.0.0");
        assert_eq!(manifest.client_executable, "game.exe");
        assert_eq!(manifest.client_args, vec!["-windowed"]);
        assert_eq!(manifest.total_size, 5000); // Auto-calculated
        assert_eq!(manifest.patch_notes_url, Some("notes.md".to_string()));
    }

    #[test]
    fn test_manifest_builder_missing_version() {
        let result = ManifestBuilder::new()
            .timestamp("2026-02-15T12:00:00Z")
            .client_executable("game.exe")
            .add_file(FileEntry::new(
                "game.exe",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                5000,
            ))
            .build();

        assert!(matches!(result, Err(ManifestError::MissingField(_))));
    }

    #[test]
    fn test_manifest_serialization_roundtrip() {
        let original = ManifestBuilder::new()
            .version("1.5.0")
            .timestamp("2026-02-15T12:00:00Z")
            .client_executable("client.exe")
            .add_file(FileEntry::new(
                "client.exe",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                1000,
            ))
            .build()
            .unwrap();

        let json = original.to_json_string().unwrap();
        let parsed = Manifest::parse_str(&json).unwrap();

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_file_entry_with_required() {
        let file = FileEntry::new(
            "optional.dat",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            100,
        )
        .with_required(false);

        assert!(!file.required);
    }

    #[test]
    fn test_manifest_iter_required_files() {
        let manifest = ManifestBuilder::new()
            .version("1.0.0")
            .timestamp("2026-02-15T12:00:00Z")
            .client_executable("client.exe")
            .add_file(FileEntry::new(
                "client.exe",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                1000,
            ))
            .add_file(
                FileEntry::new(
                    "optional.dat",
                    "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3",
                    500,
                )
                .with_required(false),
            )
            .build()
            .unwrap();

        assert_eq!(manifest.file_count(), 2);
        assert_eq!(manifest.required_file_count(), 1);

        let required: Vec<_> = manifest.iter_required_files().collect();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0].path, "client.exe");
    }

    #[test]
    fn test_client_executable_entry() {
        let json = valid_manifest_json();
        let manifest = Manifest::parse_str(&json).unwrap();

        let exe = manifest.client_executable_entry();
        assert!(exe.is_some());
        assert_eq!(exe.unwrap().path, "client.exe");
    }

    #[test]
    fn test_parse_bytes() {
        let json = valid_manifest_json();
        let bytes = json.as_bytes();

        let manifest = Manifest::parse(bytes).expect("Should parse from bytes");
        assert_eq!(manifest.version, "1.0.0");
    }

    #[test]
    fn test_windows_path_separator() {
        // Windows-style paths should also be rejected if they're absolute
        let json = r#"{
            "version": "1.0.0",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "files": [
                {
                    "path": "\\Windows\\System32\\cmd.exe",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "size": 1000
                }
            ],
            "total_size": 1000
        }"#;

        let result = Manifest::parse_str(json);
        assert!(matches!(result, Err(ManifestError::InvalidPath(_))));
    }

    #[test]
    fn test_subdirectory_paths_allowed() {
        // Subdirectory paths should be allowed
        let json = r#"{
            "version": "1.0.0",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "files": [
                {
                    "path": "client.exe",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "size": 500
                },
                {
                    "path": "data/maps/map0.mul",
                    "sha256": "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3",
                    "size": 500
                }
            ],
            "total_size": 1000
        }"#;

        let manifest = Manifest::parse_str(json).expect("Subdirectory paths should be allowed");
        assert_eq!(manifest.files.len(), 2);
    }

    // ==================== validate_path_containment tests ====================

    #[test]
    fn test_path_containment_valid_file_in_base() {
        // Create a temporary directory structure for testing
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path();

        // Create a file inside the base directory
        let file_path = base.join("test_file.txt");
        std::fs::write(&file_path, "test content").unwrap();

        // validate_path_containment should succeed for a file inside base
        let result = validate_path_containment(base, Path::new("test_file.txt"));
        assert!(result.is_ok());

        let canonical = result.unwrap();
        assert!(canonical.starts_with(base.canonicalize().unwrap()));
    }

    #[test]
    fn test_path_containment_valid_nested_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path();

        // Create nested directory structure
        let nested = base.join("data").join("maps");
        std::fs::create_dir_all(&nested).unwrap();
        let file_path = nested.join("map0.mul");
        std::fs::write(&file_path, "map data").unwrap();

        // validate_path_containment should succeed for nested paths
        let result = validate_path_containment(base, Path::new("data/maps/map0.mul"));
        assert!(result.is_ok());

        let canonical = result.unwrap();
        assert!(canonical.starts_with(base.canonicalize().unwrap()));
    }

    #[test]
    fn test_path_containment_rejects_nonexistent_target() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path();

        // Attempting to validate a non-existent file should fail
        let result = validate_path_containment(base, Path::new("nonexistent.txt"));
        assert!(matches!(
            result,
            Err(ManifestError::CanonicalizationFailed { .. })
        ));
    }

    #[test]
    fn test_path_containment_rejects_nonexistent_base() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path().join("nonexistent_base");

        // Attempting to validate with non-existent base should fail
        let result = validate_path_containment(&base, Path::new("file.txt"));
        assert!(matches!(
            result,
            Err(ManifestError::CanonicalizationFailed { .. })
        ));
    }

    #[test]
    fn test_path_containment_rejects_parent_traversal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path().join("install");
        std::fs::create_dir_all(&base).unwrap();

        // Create a file outside the base directory
        let outside_file = temp_dir.path().join("secret.txt");
        std::fs::write(&outside_file, "secret").unwrap();

        // Attempting to access ../secret.txt should fail
        let result = validate_path_containment(&base, Path::new("../secret.txt"));

        // This should either fail to canonicalize (if .. is not resolved)
        // or fail containment check (if it resolves to outside base)
        assert!(result.is_err());

        // Verify it's either a canonicalization error or containment error
        match result {
            Err(ManifestError::PathContainment { .. }) => {
                // Expected: path resolved but escaped containment
            }
            Err(ManifestError::CanonicalizationFailed { .. }) => {
                // Also acceptable: canonicalization failed
            }
            _ => panic!("Expected PathContainment or CanonicalizationFailed error"),
        }
    }

    #[test]
    fn test_path_containment_rejects_deep_traversal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&base).unwrap();

        // Create a file at the temp root
        let outside_file = temp_dir.path().join("outside.txt");
        std::fs::write(&outside_file, "outside").unwrap();

        // Attempting to access ../../../outside.txt should fail
        let result = validate_path_containment(&base, Path::new("../../../outside.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_path_containment_rejects_hidden_traversal() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path();

        // Create directory structure: base/data and base/secret
        let data_dir = base.join("data");
        let secret_dir = base.join("secret");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::create_dir_all(&secret_dir).unwrap();

        let secret_file = secret_dir.join("password.txt");
        std::fs::write(&secret_file, "hunter2").unwrap();

        // Even though data/../secret/password.txt stays "within" base by string,
        // this test validates the canonicalization handles it correctly
        let result = validate_path_containment(base, Path::new("data/../secret/password.txt"));

        // This SHOULD succeed because data/../secret/password.txt resolves to
        // base/secret/password.txt which is still within base. This is a valid path.
        assert!(result.is_ok());

        // But importantly, it should resolve to the correct canonical path
        let canonical = result.unwrap();
        assert!(canonical.ends_with("password.txt"));
        assert!(canonical.starts_with(base.canonicalize().unwrap()));
    }

    #[test]
    fn test_path_containment_current_dir_prefix() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path();

        // Create a file
        let file_path = base.join("file.txt");
        std::fs::write(&file_path, "content").unwrap();

        // ./file.txt should be valid
        let result = validate_path_containment(base, Path::new("./file.txt"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_containment_error_messages() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path().join("install");
        std::fs::create_dir_all(&base).unwrap();

        // Create file outside base
        let outside = temp_dir.path().join("outside.txt");
        std::fs::write(&outside, "outside").unwrap();

        // Try to escape
        let result = validate_path_containment(&base, Path::new("../outside.txt"));

        // Verify error contains useful information
        match result {
            Err(ManifestError::PathContainment { base: b, target: t }) => {
                // Error should contain paths for debugging
                assert!(!b.is_empty());
                assert!(!t.is_empty());
            }
            Err(ManifestError::CanonicalizationFailed { path, reason }) => {
                assert!(!path.is_empty());
                assert!(!reason.is_empty());
            }
            _ => panic!("Expected PathContainment or CanonicalizationFailed"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_path_containment_symlink_escape_attempt() {
        use std::os::unix::fs::symlink;

        let temp_dir = tempfile::tempdir().unwrap();
        let base = temp_dir.path().join("install");
        std::fs::create_dir_all(&base).unwrap();

        // Create a sensitive file outside base
        let outside = temp_dir.path().join("secret.txt");
        std::fs::write(&outside, "secret data").unwrap();

        // Create a symlink inside base that points outside
        let symlink_path = base.join("link");
        symlink(&outside, &symlink_path).unwrap();

        // The symlink exists inside base, but resolves outside
        // validate_path_containment should detect this via canonicalization
        let result = validate_path_containment(&base, Path::new("link"));

        // This should fail because the canonical path is outside base
        assert!(matches!(result, Err(ManifestError::PathContainment { .. })));
    }

    // ==================== Cross-Platform Path Validation Tests ====================
    //
    // These tests ensure path validation works correctly across Windows and Unix
    // platforms, covering all attack vectors mentioned in the security spec:
    // - Windows drive paths (C:\x, D:\x)
    // - UNC paths (\\server\share)
    // - Unix absolute paths (/abs)
    // - Backslash parent traversal (..\x)
    // - Mixed separator traversal (foo/..\bar)
    // - Extended Windows paths (\\?\)
    // - Device paths (CON, NUL)

    mod cross_platform_path_tests {
        use super::*;
        use std::path::Path;

        // === Windows Drive Path Tests ===

        #[test]
        fn test_rejects_windows_drive_c() {
            // C:\x - Windows C: drive path
            assert!(!is_safe_relative_path(Path::new("C:\\x")));
            assert!(!is_safe_relative_path(Path::new("C:\\Windows")));
            assert!(!is_safe_relative_path(Path::new(
                "C:\\Program Files\\app.exe"
            )));
        }

        #[test]
        fn test_rejects_windows_drive_various_letters() {
            // Various drive letters
            assert!(!is_safe_relative_path(Path::new("D:\\data")));
            assert!(!is_safe_relative_path(Path::new("E:\\backup")));
            assert!(!is_safe_relative_path(Path::new("Z:\\network\\share")));
        }

        #[test]
        fn test_rejects_windows_drive_lowercase() {
            // Lowercase drive letters (should also be rejected)
            assert!(!is_safe_relative_path(Path::new("c:\\users")));
            assert!(!is_safe_relative_path(Path::new("d:\\files")));
        }

        #[test]
        fn test_rejects_windows_drive_with_forward_slash() {
            // Windows drive with forward slashes (mixed format)
            assert!(!is_safe_relative_path(Path::new("C:/Windows/System32")));
            assert!(!is_safe_relative_path(Path::new("D:/Program Files")));
        }

        // === UNC Path Tests ===

        #[test]
        fn test_rejects_unc_server_share() {
            // \\server\share - UNC network paths
            assert!(!is_safe_relative_path(Path::new("\\\\server\\share")));
            assert!(!is_safe_relative_path(Path::new(
                "\\\\server\\share\\file.txt"
            )));
            assert!(!is_safe_relative_path(Path::new("\\\\192.168.1.1\\c$")));
        }

        #[test]
        fn test_rejects_unc_with_forward_slashes() {
            // UNC-like paths with forward slashes
            assert!(!is_safe_relative_path(Path::new("//server/share")));
            assert!(!is_safe_relative_path(Path::new("//localhost/c$")));
        }

        #[test]
        fn test_rejects_unc_admin_shares() {
            // Windows administrative shares
            assert!(!is_safe_relative_path(Path::new("\\\\server\\c$")));
            assert!(!is_safe_relative_path(Path::new("\\\\server\\admin$")));
            assert!(!is_safe_relative_path(Path::new("\\\\server\\ipc$")));
        }

        // === Unix Absolute Path Tests ===

        #[test]
        fn test_rejects_unix_absolute_paths() {
            // /abs - Unix absolute paths
            assert!(!is_safe_relative_path(Path::new("/abs")));
            assert!(!is_safe_relative_path(Path::new("/etc/passwd")));
            assert!(!is_safe_relative_path(Path::new("/var/log/auth.log")));
            assert!(!is_safe_relative_path(Path::new("/home/user/.ssh/id_rsa")));
        }

        #[test]
        fn test_rejects_unix_root() {
            // Just root
            assert!(!is_safe_relative_path(Path::new("/")));
        }

        #[test]
        fn test_rejects_unix_tmp() {
            // Common attack targets
            assert!(!is_safe_relative_path(Path::new("/tmp/malware")));
            assert!(!is_safe_relative_path(Path::new("/dev/null")));
            assert!(!is_safe_relative_path(Path::new("/proc/self/environ")));
        }

        // === Backslash Parent Traversal Tests ===

        #[test]
        fn test_rejects_backslash_parent_traversal() {
            // ..\x - backslash parent directory traversal
            assert!(!is_safe_relative_path(Path::new("..\\x")));
            assert!(!is_safe_relative_path(Path::new("..\\secret.txt")));
            assert!(!is_safe_relative_path(Path::new("..\\..\\..\\etc\\passwd")));
        }

        #[test]
        fn test_rejects_backslash_traversal_deep() {
            // Deep backslash traversal
            assert!(!is_safe_relative_path(Path::new(
                "..\\..\\..\\..\\Windows\\System32"
            )));
            assert!(!is_safe_relative_path(Path::new(
                "data\\..\\..\\..\\secret"
            )));
        }

        #[test]
        fn test_rejects_backslash_traversal_from_subdir() {
            // Traversal starting from apparent subdirectory
            assert!(!is_safe_relative_path(Path::new("subdir\\..\\..\\secret")));
            assert!(!is_safe_relative_path(Path::new(
                "a\\b\\..\\..\\..\\outside"
            )));
        }

        // === Mixed Separator Traversal Tests ===

        #[test]
        fn test_rejects_mixed_forward_back_traversal() {
            // foo/..\bar - mixed separator traversal
            assert!(!is_safe_relative_path(Path::new("foo/..\\bar")));
            assert!(!is_safe_relative_path(Path::new("foo\\../bar")));
        }

        #[test]
        fn test_rejects_complex_mixed_traversal() {
            // Complex mixed separator traversal attempts
            assert!(!is_safe_relative_path(Path::new("a/b\\..\\../c")));
            assert!(!is_safe_relative_path(Path::new(
                "data\\maps/..\\..\\secret"
            )));
            assert!(!is_safe_relative_path(Path::new(
                "foo/bar\\..\\..\\..\\etc\\passwd"
            )));
        }

        #[test]
        fn test_rejects_alternating_separators_with_traversal() {
            // Alternating separators with parent traversal
            assert!(!is_safe_relative_path(Path::new("a\\b/c\\..\\..\\..\\x")));
        }

        // === Extended Windows Path Tests ===

        #[test]
        fn test_rejects_extended_length_paths() {
            // \\?\ extended-length path prefix (Windows)
            assert!(!is_safe_relative_path(Path::new("\\\\?\\C:\\Windows")));
            assert!(!is_safe_relative_path(Path::new(
                "\\\\?\\UNC\\server\\share"
            )));
        }

        #[test]
        fn test_rejects_device_namespace_paths() {
            // \\.\ device namespace (Windows)
            assert!(!is_safe_relative_path(Path::new("\\\\.\\PhysicalDrive0")));
            assert!(!is_safe_relative_path(Path::new("\\\\.\\COM1")));
        }

        // === Windows Device Name Tests ===

        #[test]
        fn test_accepts_device_like_names_in_safe_context() {
            // Device-like names as regular filenames should be allowed
            // (These are just filenames, not actual device references)
            // Note: Windows treats CON, NUL etc. specially but as path components
            // they should be validated based on structure, not reserved names
            assert!(is_safe_relative_path(Path::new("data/CON.txt")));
            assert!(is_safe_relative_path(Path::new("NUL_file.dat")));
            assert!(is_safe_relative_path(Path::new("files/COM1_backup.log")));
        }

        // === Edge Cases ===

        #[test]
        fn test_rejects_dot_dot_variations() {
            // Various representations of parent directory
            assert!(!is_safe_relative_path(Path::new("..")));
            assert!(!is_safe_relative_path(Path::new("../")));
            assert!(!is_safe_relative_path(Path::new("..\\")));
            assert!(!is_safe_relative_path(Path::new("../.")));
        }

        #[test]
        fn test_accepts_dots_in_filenames() {
            // Dots in filenames should be fine (not traversal)
            assert!(is_safe_relative_path(Path::new("file..txt")));
            assert!(is_safe_relative_path(Path::new("...hidden")));
            assert!(is_safe_relative_path(Path::new("data/file...ext")));
        }

        #[test]
        fn test_accepts_dot_files() {
            // Unix-style hidden files (starting with .)
            assert!(is_safe_relative_path(Path::new(".hidden")));
            assert!(is_safe_relative_path(Path::new(".config/settings.json")));
            assert!(is_safe_relative_path(Path::new("data/.gitignore")));
        }

        #[test]
        fn test_rejects_trailing_dot_dot() {
            // Path ending with ..
            assert!(!is_safe_relative_path(Path::new("foo/..")));
            assert!(!is_safe_relative_path(Path::new("a/b/c/..")));
        }

        #[test]
        fn test_accepts_complex_valid_paths() {
            // Valid complex paths that should be allowed
            assert!(is_safe_relative_path(Path::new("data/maps/world-map.mul")));
            assert!(is_safe_relative_path(Path::new(
                "assets/textures/grass_01.png"
            )));
            assert!(is_safe_relative_path(Path::new("client/v2.5.0/client.exe")));
            assert!(is_safe_relative_path(Path::new("./relative/path/file.txt")));
        }

        #[test]
        fn test_accepts_deep_nesting() {
            // Deeply nested but valid paths
            assert!(is_safe_relative_path(Path::new(
                "a/b/c/d/e/f/g/h/i/j/file.txt"
            )));
            assert!(is_safe_relative_path(Path::new(
                "level1\\level2\\level3\\level4\\file.dat"
            )));
        }

        #[test]
        fn test_spaces_in_paths_allowed() {
            // Spaces in path components are valid filenames
            // " .." with a space is a valid filename, not parent traversal
            assert!(is_safe_relative_path(Path::new("foo/ ../bar")));
            assert!(is_safe_relative_path(Path::new("my files/data.txt")));
            assert!(is_safe_relative_path(Path::new("Program Files/app.exe")));
        }

        #[test]
        fn test_handles_unicode_paths() {
            // Unicode characters in paths should be allowed
            assert!(is_safe_relative_path(Path::new("données/fichier.txt")));
            assert!(is_safe_relative_path(Path::new("日本語/ファイル.dat")));
            assert!(is_safe_relative_path(Path::new("данные/файл.txt")));
        }

        #[test]
        fn test_rejects_unicode_with_traversal() {
            // Unicode paths with traversal should still be rejected
            assert!(!is_safe_relative_path(Path::new("données/../secret")));
            assert!(!is_safe_relative_path(Path::new("../日本語/file")));
        }
    }

    // ==================== Manifest Integration Path Tests ====================
    //
    // These tests verify that malicious paths are properly rejected when
    // parsing manifests, ensuring the path validation is correctly wired up.
    //
    // Note: JSON requires backslashes to be escaped as \\, so these tests
    // use double backslashes in the JSON strings.

    mod manifest_path_integration_tests {
        use super::*;

        fn make_manifest_json_with_path(path: &str) -> String {
            format!(
                r#"{{
                    "version": "1.0.0",
                    "timestamp": "2026-02-15T00:00:00Z",
                    "client_executable": "client.exe",
                    "files": [
                        {{
                            "path": "client.exe",
                            "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                            "size": 500
                        }},
                        {{
                            "path": "{}",
                            "sha256": "a665a45920422f9d417e4867efdc4fb8a04a1f3fff1fa07e998e86f7f7a27ae3",
                            "size": 500
                        }}
                    ],
                    "total_size": 1000
                }}"#,
                path
            )
        }

        #[test]
        fn test_manifest_rejects_windows_drive_path() {
            // JSON-escaped: C:\\Windows\\System32\\cmd.exe -> C:\Windows\System32\cmd.exe
            let json = make_manifest_json_with_path("C:\\\\Windows\\\\System32\\\\cmd.exe");
            let result = Manifest::parse_str(&json);
            assert!(matches!(result, Err(ManifestError::InvalidPath(_))));
        }

        #[test]
        fn test_manifest_rejects_unc_path() {
            // JSON-escaped: \\\\server\\share\\file.txt -> \\server\share\file.txt
            let json = make_manifest_json_with_path("\\\\\\\\server\\\\share\\\\file.txt");
            let result = Manifest::parse_str(&json);
            assert!(matches!(result, Err(ManifestError::InvalidPath(_))));
        }

        #[test]
        fn test_manifest_rejects_unix_absolute() {
            // Forward slashes don't need escaping in JSON
            let json = make_manifest_json_with_path("/etc/passwd");
            let result = Manifest::parse_str(&json);
            assert!(matches!(result, Err(ManifestError::InvalidPath(_))));
        }

        #[test]
        fn test_manifest_rejects_backslash_traversal() {
            // JSON-escaped: ..\\secret.txt -> ..\secret.txt
            let json = make_manifest_json_with_path("..\\\\secret.txt");
            let result = Manifest::parse_str(&json);
            assert!(matches!(result, Err(ManifestError::InvalidPath(_))));
        }

        #[test]
        fn test_manifest_rejects_mixed_separator_traversal() {
            // JSON-escaped: data/..\\..\\secret -> data/..\..\ secret
            let json = make_manifest_json_with_path("data/..\\\\..\\\\secret");
            let result = Manifest::parse_str(&json);
            assert!(matches!(result, Err(ManifestError::InvalidPath(_))));
        }

        #[test]
        fn test_manifest_rejects_deep_traversal() {
            // JSON-escaped deep backslash traversal
            let json = make_manifest_json_with_path(
                "..\\\\..\\\\..\\\\..\\\\Windows\\\\System32\\\\calc.exe",
            );
            let result = Manifest::parse_str(&json);
            assert!(matches!(result, Err(ManifestError::InvalidPath(_))));
        }

        #[test]
        fn test_manifest_rejects_hidden_traversal_in_subdir() {
            // Forward slash traversal (no JSON escaping needed)
            let json = make_manifest_json_with_path("data/subdir/../../../etc/passwd");
            let result = Manifest::parse_str(&json);
            assert!(matches!(result, Err(ManifestError::InvalidPath(_))));
        }

        #[test]
        fn test_manifest_accepts_valid_nested_path() {
            // Forward slash relative path (valid)
            let json = make_manifest_json_with_path("data/maps/map0.mul");
            let result = Manifest::parse_str(&json);
            assert!(result.is_ok());
        }

        #[test]
        fn test_manifest_accepts_windows_relative_backslash() {
            // JSON-escaped: data\\textures\\grass.png -> data\textures\grass.png
            let json = make_manifest_json_with_path("data\\\\textures\\\\grass.png");
            let result = Manifest::parse_str(&json);
            assert!(result.is_ok());
        }

        #[test]
        fn test_manifest_accepts_current_dir_prefix() {
            // Current directory prefix (valid)
            let json = make_manifest_json_with_path("./config/settings.ini");
            let result = Manifest::parse_str(&json);
            assert!(result.is_ok());
        }
    }
}
