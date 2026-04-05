//! Manifest signing for UltimaForge publishing.
//!
//! This module provides Ed25519 signature generation for manifests.
//! The signature file allows launchers to verify that update manifests
//! were created by the server owner.
//!
//! # File Formats
//!
//! - Input: `manifest.json` - JSON manifest file
//! - Output: `manifest.sig` - Raw 64-byte Ed25519 signature (hex-encoded, 128 chars)
//!
//! # Usage
//!
//! ```ignore
//! use publish_cli::sign::sign_manifest;
//!
//! let result = sign_manifest("./manifest.json", "./private.key", "./manifest.sig")?;
//! println!("Signature: {}", result.signature_hex);
//! ```

use ed25519_dalek::Signer;
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::info;

use crate::keygen;

/// Errors that can occur during manifest signing.
#[derive(Debug, Error)]
pub enum SignError {
    /// Failed to read the manifest file.
    #[error("Failed to read manifest file: {0}")]
    ReadManifestFailed(#[source] std::io::Error),

    /// Manifest file does not exist.
    #[error("Manifest file not found: {0}")]
    ManifestNotFound(String),

    /// Failed to read the private key.
    #[error("Failed to read private key: {0}")]
    ReadKeyFailed(#[source] keygen::KeygenError),

    /// Private key file does not exist.
    #[error("Private key file not found: {0}")]
    KeyNotFound(String),

    /// Failed to write the signature file.
    #[error("Failed to write signature file: {0}")]
    WriteSignatureFailed(#[source] std::io::Error),

    /// Failed to create output directory.
    #[error("Failed to create output directory: {0}")]
    CreateDirFailed(#[source] std::io::Error),
}

/// Result of signing operation.
pub struct SignResult {
    /// Path to the generated signature file.
    pub signature_path: String,
    /// Hex-encoded signature (for display).
    pub signature_hex: String,
    /// Size of the manifest that was signed.
    pub manifest_size: u64,
}

/// Signs a manifest file with a private key and writes the signature to a file.
///
/// The signature is computed over the raw bytes of the manifest file.
/// This ensures the exact content (including whitespace and formatting)
/// can be verified.
///
/// # Arguments
///
/// * `manifest_path` - Path to the manifest.json file to sign
/// * `key_path` - Path to the private key file (hex-encoded Ed25519 seed)
/// * `output_path` - Path where the signature file will be written
///
/// # Returns
///
/// Returns `SignResult` with path and hex signature on success.
///
/// # Example
///
/// ```ignore
/// use publish_cli::sign::sign_manifest;
///
/// let result = sign_manifest("./manifest.json", "./keys/private.key", "./manifest.sig")?;
/// println!("Created signature: {}", result.signature_path);
/// ```
pub fn sign_manifest(
    manifest_path: &str,
    key_path: &str,
    output_path: &str,
) -> Result<SignResult, SignError> {
    let manifest_file = Path::new(manifest_path);
    let key_file = Path::new(key_path);
    let output_file = Path::new(output_path);

    // Validate manifest exists
    if !manifest_file.exists() {
        return Err(SignError::ManifestNotFound(manifest_path.to_string()));
    }

    // Validate key exists
    if !key_file.exists() {
        return Err(SignError::KeyNotFound(key_path.to_string()));
    }

    // Read manifest content
    let manifest_content = fs::read(manifest_file).map_err(SignError::ReadManifestFailed)?;
    let manifest_size = manifest_content.len() as u64;
    info!("Read manifest: {} ({} bytes)", manifest_path, manifest_size);

    // Read private key
    let signing_key = keygen::read_private_key(key_path).map_err(SignError::ReadKeyFailed)?;
    info!("Loaded private key from: {}", key_path);

    // Sign the manifest content
    let signature = signing_key.sign(&manifest_content);
    let signature_hex = hex::encode(signature.to_bytes());
    info!("Generated Ed25519 signature");

    // Create output directory if needed
    if let Some(parent) = output_file.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(SignError::CreateDirFailed)?;
            info!("Created output directory: {}", parent.display());
        }
    }

    // Write signature file (hex-encoded)
    fs::write(output_file, &signature_hex).map_err(SignError::WriteSignatureFailed)?;
    info!("Wrote signature to: {}", output_path);

    Ok(SignResult {
        signature_path: output_path.to_string(),
        signature_hex,
        manifest_size,
    })
}

/// Reads a signature from a file and returns the raw bytes.
///
/// # Arguments
///
/// * `path` - Path to the signature file (hex-encoded)
///
/// # Returns
///
/// Returns the 64-byte signature on success.
pub fn read_signature(path: &str) -> Result<[u8; 64], SignError> {
    let hex_content = fs::read_to_string(path).map_err(SignError::ReadManifestFailed)?;

    let bytes = hex::decode(hex_content.trim()).map_err(|_| {
        SignError::ReadManifestFailed(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid hex encoding in signature file",
        ))
    })?;

    if bytes.len() != 64 {
        return Err(SignError::ReadManifestFailed(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Invalid signature length: expected 64 bytes, got {}",
                bytes.len()
            ),
        )));
    }

    let sig_bytes: [u8; 64] = bytes.try_into().expect("already validated length");

    Ok(sig_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keygen::generate_keypair;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_sign_manifest_basic() {
        let temp_dir = tempdir().unwrap();
        let keys_dir = temp_dir.path().join("keys");

        // Generate keypair
        let keygen_result = generate_keypair(keys_dir.to_str().unwrap(), false).unwrap();

        // Create a test manifest
        let manifest_path = temp_dir.path().join("manifest.json");
        let manifest_content = r#"{
            "version": "1.0.0",
            "files": []
        }"#;
        fs::write(&manifest_path, manifest_content).unwrap();

        // Sign the manifest
        let sig_path = temp_dir.path().join("manifest.sig");
        let result = sign_manifest(
            manifest_path.to_str().unwrap(),
            &keygen_result.private_key_path,
            sig_path.to_str().unwrap(),
        );

        assert!(result.is_ok(), "Signing should succeed: {:?}", result.err());

        let sign_result = result.unwrap();
        assert!(Path::new(&sign_result.signature_path).exists());
        assert_eq!(sign_result.signature_hex.len(), 128); // 64 bytes = 128 hex chars
        assert_eq!(sign_result.manifest_size, manifest_content.len() as u64);
    }

    #[test]
    fn test_signature_verifies() {
        let temp_dir = tempdir().unwrap();
        let keys_dir = temp_dir.path().join("keys");

        // Generate keypair
        let keygen_result = generate_keypair(keys_dir.to_str().unwrap(), false).unwrap();

        // Create a test manifest
        let manifest_path = temp_dir.path().join("manifest.json");
        let manifest_content = b"test manifest content";
        fs::write(&manifest_path, manifest_content).unwrap();

        // Sign the manifest
        let sig_path = temp_dir.path().join("manifest.sig");
        let sign_result = sign_manifest(
            manifest_path.to_str().unwrap(),
            &keygen_result.private_key_path,
            sig_path.to_str().unwrap(),
        )
        .unwrap();

        // Read signature and verify with public key
        let sig_bytes = read_signature(sig_path.to_str().unwrap()).unwrap();
        let signature = Signature::from_bytes(&sig_bytes);

        let public_key_bytes = keygen::read_public_key(&keygen_result.public_key_path).unwrap();
        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes).unwrap();

        // Verify the signature
        assert!(verifying_key.verify(manifest_content, &signature).is_ok());
    }

    #[test]
    fn test_signature_fails_for_modified_content() {
        let temp_dir = tempdir().unwrap();
        let keys_dir = temp_dir.path().join("keys");

        // Generate keypair
        let keygen_result = generate_keypair(keys_dir.to_str().unwrap(), false).unwrap();

        // Create a test manifest
        let manifest_path = temp_dir.path().join("manifest.json");
        let manifest_content = b"original content";
        fs::write(&manifest_path, manifest_content).unwrap();

        // Sign the manifest
        let sig_path = temp_dir.path().join("manifest.sig");
        sign_manifest(
            manifest_path.to_str().unwrap(),
            &keygen_result.private_key_path,
            sig_path.to_str().unwrap(),
        )
        .unwrap();

        // Read signature
        let sig_bytes = read_signature(sig_path.to_str().unwrap()).unwrap();
        let signature = Signature::from_bytes(&sig_bytes);

        let public_key_bytes = keygen::read_public_key(&keygen_result.public_key_path).unwrap();
        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes).unwrap();

        // Try to verify with modified content - should fail
        let modified_content = b"modified content";
        assert!(verifying_key.verify(modified_content, &signature).is_err());
    }

    #[test]
    fn test_sign_manifest_not_found() {
        let result = sign_manifest(
            "/nonexistent/manifest.json",
            "/some/key.key",
            "/output/manifest.sig",
        );

        assert!(matches!(result, Err(SignError::ManifestNotFound(_))));
    }

    #[test]
    fn test_sign_key_not_found() {
        let temp_dir = tempdir().unwrap();

        // Create manifest but no key
        let manifest_path = temp_dir.path().join("manifest.json");
        fs::write(&manifest_path, "{}").unwrap();

        let result = sign_manifest(
            manifest_path.to_str().unwrap(),
            "/nonexistent/private.key",
            "./manifest.sig",
        );

        assert!(matches!(result, Err(SignError::KeyNotFound(_))));
    }

    #[test]
    fn test_sign_creates_output_directory() {
        let temp_dir = tempdir().unwrap();
        let keys_dir = temp_dir.path().join("keys");

        // Generate keypair
        let keygen_result = generate_keypair(keys_dir.to_str().unwrap(), false).unwrap();

        // Create manifest
        let manifest_path = temp_dir.path().join("manifest.json");
        fs::write(&manifest_path, "{}").unwrap();

        // Output to nested directory that doesn't exist
        let sig_path = temp_dir.path().join("nested/dir/manifest.sig");
        assert!(!sig_path.parent().unwrap().exists());

        let result = sign_manifest(
            manifest_path.to_str().unwrap(),
            &keygen_result.private_key_path,
            sig_path.to_str().unwrap(),
        );

        assert!(result.is_ok());
        assert!(sig_path.exists());
    }

    #[test]
    fn test_read_signature() {
        let temp_dir = tempdir().unwrap();

        // Create a valid hex signature (64 bytes = 128 hex chars)
        let sig_path = temp_dir.path().join("test.sig");
        let sig_hex = "0".repeat(128);
        fs::write(&sig_path, &sig_hex).unwrap();

        let result = read_signature(sig_path.to_str().unwrap());
        assert!(result.is_ok());

        let sig_bytes = result.unwrap();
        assert_eq!(sig_bytes.len(), 64);
    }

    #[test]
    fn test_read_signature_invalid_length() {
        let temp_dir = tempdir().unwrap();

        // Create a signature with wrong length
        let sig_path = temp_dir.path().join("test.sig");
        fs::write(&sig_path, "0123456789abcdef").unwrap(); // Only 8 bytes

        let result = read_signature(sig_path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_read_signature_invalid_hex() {
        let temp_dir = tempdir().unwrap();

        // Create a signature with invalid hex
        let sig_path = temp_dir.path().join("test.sig");
        fs::write(&sig_path, "not valid hex!").unwrap();

        let result = read_signature(sig_path.to_str().unwrap());
        assert!(result.is_err());
    }
}
