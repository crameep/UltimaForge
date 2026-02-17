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
use std::path::Path;

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
    pub fn validate(&self) -> Result<(), ManifestError> {
        // Check for path traversal attacks
        if self.path.contains("..") || self.path.starts_with('/') || self.path.starts_with('\\') {
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
        assert!(matches!(result, Err(ManifestError::TotalSizeMismatch { .. })));
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
}
