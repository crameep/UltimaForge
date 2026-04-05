//! Update folder validation for UltimaForge publishing.
//!
//! This module provides functionality to validate an update folder structure:
//! - Verifies the manifest signature using a public key
//! - Checks that all file blobs referenced in the manifest exist
//! - Reports validation status with detailed summary
//!
//! # Directory Structure
//!
//! Expected update folder structure:
//! ```text
//! updates/
//!   manifest.json   - The manifest file
//!   manifest.sig    - Ed25519 signature of manifest.json
//!   files/          - Content-addressed blob storage
//!     {sha256}      - Blob files named by their hash
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use publish_cli::validate::validate_update_folder;
//!
//! let result = validate_update_folder("./updates", "./public.key")?;
//! println!("Validation passed: {} files verified", result.files_verified);
//! ```

use ed25519_dalek::{Signature, VerifyingKey};
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::keygen;
use crate::manifest::Manifest;
use crate::sign;

/// Errors that can occur during validation.
#[derive(Debug, Error)]
pub enum ValidateError {
    /// The update directory does not exist.
    #[error("Update directory does not exist: {0}")]
    DirNotFound(String),

    /// manifest.json is missing.
    #[error("manifest.json not found in: {0}")]
    ManifestNotFound(String),

    /// manifest.sig is missing.
    #[error("manifest.sig not found in: {0}")]
    SignatureNotFound(String),

    /// files/ directory is missing.
    #[error("files/ directory not found in: {0}")]
    FilesDirNotFound(String),

    /// Failed to read public key.
    #[error("Failed to read public key: {0}")]
    ReadPublicKeyFailed(#[source] keygen::KeygenError),

    /// Public key file does not exist.
    #[error("Public key file not found: {0}")]
    PublicKeyNotFound(String),

    /// Failed to read manifest file.
    #[error("Failed to read manifest: {0}")]
    ReadManifestFailed(#[source] std::io::Error),

    /// Failed to read signature file.
    #[error("Failed to read signature: {0}")]
    ReadSignatureFailed(#[source] sign::SignError),

    /// Manifest JSON is invalid.
    #[error("Invalid manifest JSON: {0}")]
    InvalidManifestJson(#[source] serde_json::Error),

    /// Invalid public key format.
    #[error("Invalid public key format: {0}")]
    InvalidPublicKey(String),

    /// Invalid signature format.
    #[error("Invalid signature format: {0}")]
    #[allow(dead_code)]
    InvalidSignature(String),

    /// Signature verification failed.
    #[error("Signature verification failed: manifest may have been tampered with")]
    SignatureVerificationFailed,

    /// A file blob is missing.
    #[error("Missing blob for file '{path}': expected {hash}")]
    #[allow(dead_code)]
    MissingBlob { path: String, hash: String },
}

/// Information about a validated file.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ValidatedFile {
    /// Relative path of the file.
    pub path: String,
    /// SHA-256 hash of the file.
    pub sha256: String,
    /// Size of the file in bytes.
    pub size: u64,
    /// Whether the blob exists.
    pub blob_exists: bool,
}

/// Result of validation.
#[allow(dead_code)]
pub struct ValidateResult {
    /// Path to the update directory.
    pub dir_path: String,
    /// Version from the manifest.
    pub version: String,
    /// Whether the signature is valid.
    pub signature_valid: bool,
    /// Number of files in the manifest.
    pub file_count: usize,
    /// Number of files verified (blob exists).
    pub files_verified: usize,
    /// Number of missing blobs.
    pub missing_blobs: usize,
    /// Total size of all files in bytes.
    pub total_size: u64,
    /// List of all validated files.
    pub files: Vec<ValidatedFile>,
    /// List of missing blob paths.
    pub missing_blob_paths: Vec<String>,
}

/// Validates an update folder structure.
///
/// This function performs the following checks:
/// 1. Verifies that manifest.json, manifest.sig, and files/ exist
/// 2. Verifies the Ed25519 signature of the manifest
/// 3. Checks that all file blobs referenced in the manifest exist
///
/// # Arguments
///
/// * `update_dir` - Path to the update folder containing manifest and files
/// * `public_key_path` - Path to the public key file (hex-encoded)
///
/// # Returns
///
/// Returns `ValidateResult` with validation details on success.
/// Returns an error if signature verification fails or required files are missing.
///
/// # Example
///
/// ```ignore
/// use publish_cli::validate::validate_update_folder;
///
/// let result = validate_update_folder("./updates", "./keys/public.key")?;
/// if result.missing_blobs == 0 {
///     println!("Validation passed!");
/// }
/// ```
pub fn validate_update_folder(
    update_dir: &str,
    public_key_path: &str,
) -> Result<ValidateResult, ValidateError> {
    let update_path = Path::new(update_dir);
    let key_path = Path::new(public_key_path);

    // Validate update directory exists
    if !update_path.exists() {
        return Err(ValidateError::DirNotFound(update_dir.to_string()));
    }

    info!("Validating update folder: {}", update_dir);

    // Check required files exist
    let manifest_path = update_path.join("manifest.json");
    let signature_path = update_path.join("manifest.sig");
    let files_dir = update_path.join("files");

    if !manifest_path.exists() {
        return Err(ValidateError::ManifestNotFound(update_dir.to_string()));
    }
    debug!("Found manifest.json");

    if !signature_path.exists() {
        return Err(ValidateError::SignatureNotFound(update_dir.to_string()));
    }
    debug!("Found manifest.sig");

    if !files_dir.exists() || !files_dir.is_dir() {
        return Err(ValidateError::FilesDirNotFound(update_dir.to_string()));
    }
    debug!("Found files/ directory");

    // Validate public key exists
    if !key_path.exists() {
        return Err(ValidateError::PublicKeyNotFound(
            public_key_path.to_string(),
        ));
    }

    // Read public key
    let public_key_bytes =
        keygen::read_public_key(public_key_path).map_err(ValidateError::ReadPublicKeyFailed)?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|e| ValidateError::InvalidPublicKey(e.to_string()))?;
    info!("Loaded public key from: {}", public_key_path);

    // Read manifest content
    let manifest_content = fs::read(&manifest_path).map_err(ValidateError::ReadManifestFailed)?;
    debug!("Read manifest: {} bytes", manifest_content.len());

    // Read signature
    let signature_bytes = sign::read_signature(signature_path.to_str().unwrap())
        .map_err(ValidateError::ReadSignatureFailed)?;
    let signature = Signature::from_bytes(&signature_bytes);
    debug!("Read signature");

    // Verify signature
    verifying_key
        .verify_strict(&manifest_content, &signature)
        .map_err(|_| ValidateError::SignatureVerificationFailed)?;
    info!("✓ Signature verified successfully");

    // Parse manifest
    let manifest: Manifest =
        serde_json::from_slice(&manifest_content).map_err(ValidateError::InvalidManifestJson)?;
    debug!("Parsed manifest: version {}", manifest.version);

    // Check all file blobs exist
    let mut files: Vec<ValidatedFile> = Vec::new();
    let mut files_verified: usize = 0;
    let mut missing_blobs: usize = 0;
    let mut missing_blob_paths: Vec<String> = Vec::new();

    for file_entry in &manifest.files {
        let blob_path = files_dir.join(&file_entry.sha256);
        let blob_exists = blob_path.exists();

        if blob_exists {
            files_verified += 1;
            debug!(
                "✓ Blob exists: {} -> {}",
                file_entry.path, file_entry.sha256
            );
        } else {
            missing_blobs += 1;
            missing_blob_paths.push(file_entry.path.clone());
            warn!(
                "✗ Missing blob: {} -> {}",
                file_entry.path, file_entry.sha256
            );
        }

        files.push(ValidatedFile {
            path: file_entry.path.clone(),
            sha256: file_entry.sha256.clone(),
            size: file_entry.size,
            blob_exists,
        });
    }

    info!("Validation complete:");
    info!("  Version: {}", manifest.version);
    info!("  Files: {}", manifest.files.len());
    info!("  Verified: {}", files_verified);
    info!("  Missing: {}", missing_blobs);
    info!("  Total size: {} bytes", manifest.total_size);

    Ok(ValidateResult {
        dir_path: update_dir.to_string(),
        version: manifest.version,
        signature_valid: true,
        file_count: manifest.files.len(),
        files_verified,
        missing_blobs,
        total_size: manifest.total_size,
        files,
        missing_blob_paths,
    })
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
    use crate::blob::create_blobs;
    use crate::keygen::generate_keypair;
    use crate::manifest::generate_manifest;
    use crate::sign::sign_manifest;
    use std::fs;
    use tempfile::tempdir;

    /// Helper to set up a complete valid update folder.
    fn setup_valid_update_folder(temp_dir: &tempfile::TempDir) -> (String, String) {
        let keys_dir = temp_dir.path().join("keys");
        let source_dir = temp_dir.path().join("source");
        let update_dir = temp_dir.path().join("updates");

        // Create source files
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("client.exe"), b"fake executable").unwrap();
        fs::write(source_dir.join("readme.txt"), b"readme content").unwrap();

        // Generate keypair
        let keygen_result = generate_keypair(keys_dir.to_str().unwrap(), false).unwrap();

        // Create update directory structure
        fs::create_dir_all(&update_dir).unwrap();

        // Generate manifest
        let manifest_path = update_dir.join("manifest.json");
        generate_manifest(
            source_dir.to_str().unwrap(),
            manifest_path.to_str().unwrap(),
            "1.0.0",
            "client.exe",
        )
        .unwrap();

        // Sign manifest
        let sig_path = update_dir.join("manifest.sig");
        sign_manifest(
            manifest_path.to_str().unwrap(),
            &keygen_result.private_key_path,
            sig_path.to_str().unwrap(),
        )
        .unwrap();

        // Create blobs
        let files_dir = update_dir.join("files");
        create_blobs(source_dir.to_str().unwrap(), files_dir.to_str().unwrap()).unwrap();

        (
            update_dir.to_str().unwrap().to_string(),
            keygen_result.public_key_path,
        )
    }

    #[test]
    fn test_validate_valid_folder() {
        let temp_dir = tempdir().unwrap();
        let (update_dir, public_key_path) = setup_valid_update_folder(&temp_dir);

        let result = validate_update_folder(&update_dir, &public_key_path);
        assert!(result.is_ok(), "Validation should pass: {:?}", result.err());

        let validation = result.unwrap();
        assert!(validation.signature_valid);
        assert_eq!(validation.version, "1.0.0");
        assert_eq!(validation.file_count, 2);
        assert_eq!(validation.files_verified, 2);
        assert_eq!(validation.missing_blobs, 0);
        assert!(validation.missing_blob_paths.is_empty());
    }

    #[test]
    fn test_validate_missing_blob() {
        let temp_dir = tempdir().unwrap();
        let (update_dir, public_key_path) = setup_valid_update_folder(&temp_dir);

        // Remove one blob file
        let files_dir = Path::new(&update_dir).join("files");
        let blobs: Vec<_> = fs::read_dir(&files_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        if let Some(blob) = blobs.first() {
            fs::remove_file(blob.path()).unwrap();
        }

        let result = validate_update_folder(&update_dir, &public_key_path);
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert!(validation.signature_valid);
        assert_eq!(validation.missing_blobs, 1);
        assert!(!validation.missing_blob_paths.is_empty());
    }

    #[test]
    fn test_validate_dir_not_found() {
        let result = validate_update_folder("/nonexistent/path", "./public.key");
        assert!(matches!(result, Err(ValidateError::DirNotFound(_))));
    }

    #[test]
    fn test_validate_manifest_not_found() {
        let temp_dir = tempdir().unwrap();
        let update_dir = temp_dir.path().join("updates");
        fs::create_dir_all(&update_dir).unwrap();

        // Create just the files dir and sig, but no manifest
        fs::create_dir_all(update_dir.join("files")).unwrap();
        fs::write(update_dir.join("manifest.sig"), "0".repeat(128)).unwrap();

        let result = validate_update_folder(update_dir.to_str().unwrap(), "./public.key");
        assert!(matches!(result, Err(ValidateError::ManifestNotFound(_))));
    }

    #[test]
    fn test_validate_signature_not_found() {
        let temp_dir = tempdir().unwrap();
        let update_dir = temp_dir.path().join("updates");
        fs::create_dir_all(&update_dir).unwrap();

        // Create manifest but no signature
        fs::write(update_dir.join("manifest.json"), "{}").unwrap();
        fs::create_dir_all(update_dir.join("files")).unwrap();

        let result = validate_update_folder(update_dir.to_str().unwrap(), "./public.key");
        assert!(matches!(result, Err(ValidateError::SignatureNotFound(_))));
    }

    #[test]
    fn test_validate_files_dir_not_found() {
        let temp_dir = tempdir().unwrap();
        let update_dir = temp_dir.path().join("updates");
        fs::create_dir_all(&update_dir).unwrap();

        // Create manifest and sig, but no files dir
        fs::write(update_dir.join("manifest.json"), "{}").unwrap();
        fs::write(update_dir.join("manifest.sig"), "0".repeat(128)).unwrap();

        let result = validate_update_folder(update_dir.to_str().unwrap(), "./public.key");
        assert!(matches!(result, Err(ValidateError::FilesDirNotFound(_))));
    }

    #[test]
    fn test_validate_public_key_not_found() {
        let temp_dir = tempdir().unwrap();
        let update_dir = temp_dir.path().join("updates");
        fs::create_dir_all(&update_dir).unwrap();
        fs::create_dir_all(update_dir.join("files")).unwrap();
        fs::write(update_dir.join("manifest.json"), "{}").unwrap();
        fs::write(update_dir.join("manifest.sig"), "0".repeat(128)).unwrap();

        let result =
            validate_update_folder(update_dir.to_str().unwrap(), "/nonexistent/public.key");
        assert!(matches!(result, Err(ValidateError::PublicKeyNotFound(_))));
    }

    #[test]
    fn test_validate_tampered_manifest() {
        let temp_dir = tempdir().unwrap();
        let (update_dir, public_key_path) = setup_valid_update_folder(&temp_dir);

        // Tamper with the manifest after signing
        let manifest_path = Path::new(&update_dir).join("manifest.json");
        let mut content = fs::read_to_string(&manifest_path).unwrap();
        content = content.replace("1.0.0", "2.0.0");
        fs::write(&manifest_path, content).unwrap();

        let result = validate_update_folder(&update_dir, &public_key_path);
        assert!(matches!(
            result,
            Err(ValidateError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn test_validate_wrong_key() {
        let temp_dir = tempdir().unwrap();
        let (update_dir, _) = setup_valid_update_folder(&temp_dir);

        // Generate a different keypair
        let other_keys_dir = temp_dir.path().join("other_keys");
        let other_keygen = generate_keypair(other_keys_dir.to_str().unwrap(), false).unwrap();

        // Try to validate with wrong key
        let result = validate_update_folder(&update_dir, &other_keygen.public_key_path);
        assert!(matches!(
            result,
            Err(ValidateError::SignatureVerificationFailed)
        ));
    }

    #[test]
    fn test_validated_file_info() {
        let temp_dir = tempdir().unwrap();
        let (update_dir, public_key_path) = setup_valid_update_folder(&temp_dir);

        let result = validate_update_folder(&update_dir, &public_key_path).unwrap();

        // Check that files have correct info
        for file in &result.files {
            assert!(!file.path.is_empty());
            assert_eq!(file.sha256.len(), 64); // SHA-256 hex is 64 chars
            assert!(file.blob_exists);
        }
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 bytes");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }
}
