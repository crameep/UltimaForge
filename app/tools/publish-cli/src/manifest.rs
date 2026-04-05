//! Manifest generation for UltimaForge publishing.
//!
//! This module provides functionality to generate a manifest from a source
//! directory by walking the directory tree, computing SHA-256 hashes for
//! each file, and producing a manifest.json file.
//!
//! # Usage
//!
//! ```ignore
//! use publish_cli::manifest::generate_manifest;
//!
//! let result = generate_manifest("./source", "./manifest.json", "1.0.0", "client.exe")?;
//! println!("Generated manifest with {} files", result.file_count);
//! ```

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};
use walkdir::WalkDir;

/// Errors that can occur during manifest generation.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// Failed to access the source directory.
    #[error("Failed to access source directory: {0}")]
    SourceDirAccessFailed(#[source] std::io::Error),

    /// Source directory does not exist.
    #[error("Source directory does not exist: {0}")]
    SourceDirNotFound(String),

    /// Failed to read a file for hashing.
    #[error("Failed to read file '{path}': {source}")]
    ReadFileFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Failed to write the manifest file.
    #[error("Failed to write manifest: {0}")]
    WriteManifestFailed(#[source] std::io::Error),

    /// Failed to serialize manifest to JSON.
    #[error("Failed to serialize manifest: {0}")]
    SerializeFailed(#[source] serde_json::Error),

    /// The client executable was not found in the source directory.
    #[error("Client executable not found: {0}")]
    ExecutableNotFound(String),

    /// Failed to walk the directory tree.
    #[error("Failed to walk directory: {0}")]
    #[allow(dead_code)]
    WalkDirFailed(#[source] walkdir::Error),

    /// Failed to create output directory.
    #[error("Failed to create output directory: {0}")]
    CreateDirFailed(#[source] std::io::Error),
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
    #[allow(dead_code)]
    pub fn with_required(mut self, required: bool) -> Self {
        self.required = required;
        self
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
    /// Serializes the manifest to a JSON string.
    pub fn to_json_string(&self) -> Result<String, ManifestError> {
        serde_json::to_string_pretty(self).map_err(ManifestError::SerializeFailed)
    }
}

/// Result of manifest generation.
pub struct ManifestResult {
    /// Path to the generated manifest file.
    pub manifest_path: String,
    /// Number of files in the manifest.
    pub file_count: usize,
    /// Total size of all files in bytes.
    pub total_size: u64,
    /// Version string of the manifest.
    pub version: String,
}

/// Computes the SHA-256 hash of a file.
///
/// # Arguments
///
/// * `file_path` - Path to the file to hash
///
/// # Returns
///
/// Returns the hex-encoded SHA-256 hash.
pub fn compute_file_hash(file_path: &Path) -> Result<String, ManifestError> {
    let mut file = File::open(file_path).map_err(|e| ManifestError::ReadFileFailed {
        path: file_path.display().to_string(),
        source: e,
    })?;

    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| ManifestError::ReadFileFailed {
                path: file_path.display().to_string(),
                source: e,
            })?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Generates a manifest from a source directory.
///
/// This function walks the source directory, computes SHA-256 hashes for
/// each file, and generates a manifest.json file with all file entries.
///
/// # Arguments
///
/// * `source_dir` - Path to the source directory containing files
/// * `output_path` - Path where manifest.json will be written
/// * `version` - Version string for the manifest
/// * `executable` - Relative path to the client executable (e.g., "client.exe")
///
/// # Returns
///
/// Returns `ManifestResult` with details about the generated manifest.
///
/// # Example
///
/// ```ignore
/// use publish_cli::manifest::generate_manifest;
///
/// let result = generate_manifest(
///     "./uo-client",
///     "./manifest.json",
///     "1.0.0",
///     "client.exe"
/// )?;
/// println!("Generated manifest at: {}", result.manifest_path);
/// ```
pub fn generate_manifest(
    source_dir: &str,
    output_path: &str,
    version: &str,
    executable: &str,
) -> Result<ManifestResult, ManifestError> {
    let source_path = Path::new(source_dir);
    let output = Path::new(output_path);

    // Validate source directory exists
    if !source_path.exists() {
        return Err(ManifestError::SourceDirNotFound(source_dir.to_string()));
    }

    if !source_path.is_dir() {
        return Err(ManifestError::SourceDirAccessFailed(io::Error::new(
            io::ErrorKind::NotADirectory,
            "Source path is not a directory",
        )));
    }

    info!("Scanning source directory: {}", source_dir);

    // Collect all files with their hashes
    let mut files: Vec<FileEntry> = Vec::new();
    let mut executable_found = false;

    for entry in WalkDir::new(source_path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip directories
        if !path.is_file() {
            continue;
        }

        // Compute relative path from source directory
        let relative_path = path.strip_prefix(source_path).map_err(|_| {
            ManifestError::SourceDirAccessFailed(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Failed to compute relative path",
            ))
        })?;

        // Normalize path separators to forward slashes
        let relative_path_str = relative_path.to_string_lossy().replace('\\', "/");

        // Check if this is the executable
        if relative_path_str == executable {
            executable_found = true;
        }

        if is_excluded_path(&relative_path_str) {
            warn!("Excluding file from manifest: {}", relative_path_str);
            continue;
        }

        // Get file metadata for size
        let metadata = fs::metadata(path).map_err(|e| ManifestError::ReadFileFailed {
            path: relative_path_str.clone(),
            source: e,
        })?;
        let size = metadata.len();

        // Compute SHA-256 hash
        debug!("Hashing file: {}", relative_path_str);
        let sha256 = compute_file_hash(path)?;

        files.push(FileEntry::new(relative_path_str.clone(), sha256, size));
        debug!("  Size: {} bytes", size);
    }

    // Validate executable was found
    if !executable_found {
        return Err(ManifestError::ExecutableNotFound(executable.to_string()));
    }

    // Sort files by path for consistent output
    files.sort_by(|a, b| a.path.cmp(&b.path));

    // Calculate total size
    let total_size: u64 = files.iter().map(|f| f.size).sum();

    // Generate timestamp
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    // Create manifest
    let manifest = Manifest {
        version: version.to_string(),
        timestamp,
        client_executable: executable.to_string(),
        client_args: Vec::new(),
        files: files.clone(),
        total_size,
        patch_notes_url: None,
    };

    // Create output directory if needed
    if let Some(parent) = output.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(ManifestError::CreateDirFailed)?;
            info!("Created output directory: {}", parent.display());
        }
    }

    // Write manifest to file
    let json = manifest.to_json_string()?;
    fs::write(output, &json).map_err(ManifestError::WriteManifestFailed)?;

    info!("Wrote manifest to: {}", output_path);
    info!("  Files: {}", files.len());
    info!("  Total size: {} bytes", total_size);
    info!("  Version: {}", version);

    Ok(ManifestResult {
        manifest_path: output_path.to_string(),
        file_count: files.len(),
        total_size,
        version: version.to_string(),
    })
}

fn is_excluded_path(relative_path: &str) -> bool {
    let normalized = relative_path.replace('\\', "/");
    let lower = normalized.to_ascii_lowercase();

    const EXCLUDED_FILES: &[&str] = &[
        "settings.json",
        "data/client/player-map-markers.csv",
        "data/client/usermarkers.usr",
        "data/profiles/lastcharacter.json",
        "login.cfg",
        "unchained.exe.log",
        "thumbs.db",
    ];

    const EXCLUDED_DIR_PREFIXES: &[&str] = &[
        "data/client/journallogs/",
        "data/client/screenshots/",
        "data/profiles/",
        "data/plugins/razor/profiles/",
        "data/plugins/razor/.logs/",
        "data/plugins/razorenhanced/profiles/",
        "data/plugins/razorenhanced/scripts/",
        "data/plugins/razorenhanced/backup/",
        "data/plugins/razorenhanced/.beads/",
        "logs/",
        "macros/",
        "temp/",
    ];

    if EXCLUDED_FILES.iter().any(|f| *f == lower) {
        return true;
    }

    EXCLUDED_DIR_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
}

/// Formats a byte count as a human-readable string.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_compute_file_hash() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, b"hello world").unwrap();

        let hash = compute_file_hash(&file_path).unwrap();

        // SHA-256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_compute_empty_file_hash() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("empty.txt");
        fs::write(&file_path, b"").unwrap();

        let hash = compute_file_hash(&file_path).unwrap();

        // SHA-256 of empty string
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_generate_manifest_basic() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        fs::create_dir(&source_dir).unwrap();

        // Create test files
        fs::write(source_dir.join("client.exe"), b"fake executable content").unwrap();
        fs::write(source_dir.join("readme.txt"), b"readme content").unwrap();

        let output_path = temp_dir.path().join("manifest.json");

        let result = generate_manifest(
            source_dir.to_str().unwrap(),
            output_path.to_str().unwrap(),
            "1.0.0",
            "client.exe",
        )
        .unwrap();

        assert_eq!(result.file_count, 2);
        assert_eq!(result.version, "1.0.0");
        assert!(output_path.exists());

        // Verify manifest contents
        let manifest_json = fs::read_to_string(&output_path).unwrap();
        let manifest: Manifest = serde_json::from_str(&manifest_json).unwrap();

        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.client_executable, "client.exe");
        assert_eq!(manifest.files.len(), 2);
        assert!(manifest.files.iter().any(|f| f.path == "client.exe"));
        assert!(manifest.files.iter().any(|f| f.path == "readme.txt"));
    }

    #[test]
    fn test_generate_manifest_subdirectories() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let data_dir = source_dir.join("data");
        fs::create_dir_all(&data_dir).unwrap();

        // Create test files with subdirectory
        fs::write(source_dir.join("client.exe"), b"executable").unwrap();
        fs::write(data_dir.join("map0.mul"), b"map data").unwrap();

        let output_path = temp_dir.path().join("manifest.json");

        let result = generate_manifest(
            source_dir.to_str().unwrap(),
            output_path.to_str().unwrap(),
            "2.0.0",
            "client.exe",
        )
        .unwrap();

        assert_eq!(result.file_count, 2);

        // Verify paths use forward slashes
        let manifest_json = fs::read_to_string(&output_path).unwrap();
        let manifest: Manifest = serde_json::from_str(&manifest_json).unwrap();

        assert!(manifest.files.iter().any(|f| f.path == "data/map0.mul"));
    }

    #[test]
    fn test_generate_manifest_excludes_settings_json() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        fs::create_dir(&source_dir).unwrap();

        fs::write(source_dir.join("client.exe"), b"executable").unwrap();
        fs::write(source_dir.join("settings.json"), b"user settings").unwrap();

        let output_path = temp_dir.path().join("manifest.json");

        let result = generate_manifest(
            source_dir.to_str().unwrap(),
            output_path.to_str().unwrap(),
            "1.0.0",
            "client.exe",
        )
        .unwrap();

        assert_eq!(result.file_count, 1);

        let manifest_json = fs::read_to_string(&output_path).unwrap();
        let manifest: Manifest = serde_json::from_str(&manifest_json).unwrap();

        assert!(manifest.files.iter().any(|f| f.path == "client.exe"));
        assert!(!manifest.files.iter().any(|f| f.path == "settings.json"));
    }

    #[test]
    fn test_generate_manifest_excludes_profile_and_log_dirs() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let razor_profiles = source_dir.join("Data/Plugins/Razor/Profiles");
        let logs_dir = source_dir.join("Logs");
        fs::create_dir_all(&razor_profiles).unwrap();
        fs::create_dir_all(&logs_dir).unwrap();

        fs::write(source_dir.join("client.exe"), b"executable").unwrap();
        fs::write(razor_profiles.join("chars.lst"), b"chars").unwrap();
        fs::write(logs_dir.join("launcher.log"), b"log").unwrap();

        let output_path = temp_dir.path().join("manifest.json");

        let result = generate_manifest(
            source_dir.to_str().unwrap(),
            output_path.to_str().unwrap(),
            "1.0.0",
            "client.exe",
        )
        .unwrap();

        assert_eq!(result.file_count, 1);

        let manifest_json = fs::read_to_string(&output_path).unwrap();
        let manifest: Manifest = serde_json::from_str(&manifest_json).unwrap();

        assert!(manifest.files.iter().any(|f| f.path == "client.exe"));
        assert!(!manifest
            .files
            .iter()
            .any(|f| f.path == "Data/Plugins/Razor/Profiles/chars.lst"));
        assert!(!manifest.files.iter().any(|f| f.path == "Logs/launcher.log"));
    }

    #[test]
    fn test_generate_manifest_source_not_found() {
        let result = generate_manifest(
            "/nonexistent/path",
            "./manifest.json",
            "1.0.0",
            "client.exe",
        );

        assert!(matches!(result, Err(ManifestError::SourceDirNotFound(_))));
    }

    #[test]
    fn test_generate_manifest_executable_not_found() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        fs::create_dir(&source_dir).unwrap();

        // Create test file that is NOT the executable
        fs::write(source_dir.join("readme.txt"), b"readme content").unwrap();

        let output_path = temp_dir.path().join("manifest.json");

        let result = generate_manifest(
            source_dir.to_str().unwrap(),
            output_path.to_str().unwrap(),
            "1.0.0",
            "missing.exe",
        );

        assert!(matches!(result, Err(ManifestError::ExecutableNotFound(_))));
    }

    #[test]
    fn test_generate_manifest_total_size() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        fs::create_dir(&source_dir).unwrap();

        // Create files with known sizes
        let content1 = b"1234567890"; // 10 bytes
        let content2 = b"abcdefghij"; // 10 bytes
        fs::write(source_dir.join("client.exe"), content1).unwrap();
        fs::write(source_dir.join("file2.dat"), content2).unwrap();

        let output_path = temp_dir.path().join("manifest.json");

        let result = generate_manifest(
            source_dir.to_str().unwrap(),
            output_path.to_str().unwrap(),
            "1.0.0",
            "client.exe",
        )
        .unwrap();

        assert_eq!(result.total_size, 20);

        let manifest_json = fs::read_to_string(&output_path).unwrap();
        let manifest: Manifest = serde_json::from_str(&manifest_json).unwrap();
        assert_eq!(manifest.total_size, 20);
    }

    #[test]
    fn test_generate_manifest_creates_output_dir() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        fs::create_dir(&source_dir).unwrap();
        fs::write(source_dir.join("client.exe"), b"content").unwrap();

        // Output in nested directory that doesn't exist
        let output_path = temp_dir.path().join("nested/dir/manifest.json");

        let result = generate_manifest(
            source_dir.to_str().unwrap(),
            output_path.to_str().unwrap(),
            "1.0.0",
            "client.exe",
        );

        assert!(result.is_ok());
        assert!(output_path.exists());
    }

    #[test]
    fn test_file_entry_creation() {
        let entry = FileEntry::new("test.txt", "abc123", 1000);

        assert_eq!(entry.path, "test.txt");
        assert_eq!(entry.sha256, "abc123");
        assert_eq!(entry.size, 1000);
        assert!(entry.required);
    }

    #[test]
    fn test_file_entry_optional() {
        let entry = FileEntry::new("optional.txt", "abc123", 500).with_required(false);

        assert!(!entry.required);
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 bytes");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_manifest_serialization() {
        let manifest = Manifest {
            version: "1.0.0".to_string(),
            timestamp: "2026-02-15T00:00:00Z".to_string(),
            client_executable: "client.exe".to_string(),
            client_args: vec!["-windowed".to_string()],
            files: vec![FileEntry::new(
                "client.exe",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                1000,
            )],
            total_size: 1000,
            patch_notes_url: Some("notes.md".to_string()),
        };

        let json = manifest.to_json_string().unwrap();
        let parsed: Manifest = serde_json::from_str(&json).unwrap();

        assert_eq!(manifest, parsed);
    }

    #[test]
    fn test_files_sorted_by_path() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        fs::create_dir(&source_dir).unwrap();

        // Create files in non-alphabetical order
        fs::write(source_dir.join("zzz.txt"), b"z").unwrap();
        fs::write(source_dir.join("client.exe"), b"exe").unwrap();
        fs::write(source_dir.join("aaa.txt"), b"a").unwrap();

        let output_path = temp_dir.path().join("manifest.json");

        generate_manifest(
            source_dir.to_str().unwrap(),
            output_path.to_str().unwrap(),
            "1.0.0",
            "client.exe",
        )
        .unwrap();

        let manifest_json = fs::read_to_string(&output_path).unwrap();
        let manifest: Manifest = serde_json::from_str(&manifest_json).unwrap();

        // Files should be sorted alphabetically
        let paths: Vec<_> = manifest.files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["aaa.txt", "client.exe", "zzz.txt"]);
    }
}
