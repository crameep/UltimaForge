//! Ed25519 keypair generation for UltimaForge publishing.
//!
//! This module provides cryptographic key generation for signing manifests.
//! The private key is used by the server owner to sign updates, while the
//! public key is embedded in the launcher for verification.
//!
//! # File Formats
//!
//! - `private.key` - Raw 32-byte Ed25519 seed (hex-encoded, 64 chars)
//! - `public.key` - Raw 32-byte Ed25519 public key (hex-encoded, 64 chars)
//!
//! # Security
//!
//! - Private key must be kept secure and never distributed
//! - Public key is safe to embed in launchers
//! - Uses OsRng for cryptographically secure random number generation

use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{info, warn};

/// Errors that can occur during key generation.
#[derive(Debug, Error)]
pub enum KeygenError {
    /// Failed to create output directory.
    #[error("Failed to create output directory: {0}")]
    CreateDirFailed(#[source] std::io::Error),

    /// Failed to read private key file.
    #[error("Failed to read private key: {0}")]
    ReadPrivateKeyFailed(#[source] std::io::Error),

    /// Failed to read public key file.
    #[error("Failed to read public key: {0}")]
    ReadPublicKeyFailed(#[source] std::io::Error),

    /// Failed to write private key file.
    #[error("Failed to write private key: {0}")]
    WritePrivateKeyFailed(#[source] std::io::Error),

    /// Failed to write public key file.
    #[error("Failed to write public key: {0}")]
    WritePublicKeyFailed(#[source] std::io::Error),

    /// Output directory already contains key files.
    #[error("Key files already exist in output directory. Use --force to overwrite.")]
    KeysAlreadyExist,
}

/// Result of key generation.
pub struct KeygenResult {
    /// Path to the generated private key file.
    pub private_key_path: String,
    /// Path to the generated public key file.
    pub public_key_path: String,
    /// Hex-encoded public key (for display/embedding).
    pub public_key_hex: String,
}

/// Generates a new Ed25519 keypair and writes it to the specified directory.
///
/// # Arguments
///
/// * `output_dir` - Directory where key files will be written
/// * `force` - If true, overwrite existing key files
///
/// # Returns
///
/// Returns `KeygenResult` with paths and public key on success.
///
/// # Example
///
/// ```ignore
/// use publish_cli::keygen::generate_keypair;
///
/// let result = generate_keypair("./keys", false)?;
/// println!("Public key: {}", result.public_key_hex);
/// ```
pub fn generate_keypair(output_dir: &str, force: bool) -> Result<KeygenResult, KeygenError> {
    let output_path = Path::new(output_dir);
    let private_key_path = output_path.join("private.key");
    let public_key_path = output_path.join("public.key");

    // Check for existing keys
    if !force && (private_key_path.exists() || public_key_path.exists()) {
        return Err(KeygenError::KeysAlreadyExist);
    }

    // Create output directory if it doesn't exist
    if !output_path.exists() {
        fs::create_dir_all(output_path).map_err(KeygenError::CreateDirFailed)?;
        info!("Created output directory: {}", output_dir);
    }

    // Generate keypair using cryptographically secure RNG
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    // Encode keys as hex strings
    let private_key_hex = hex::encode(signing_key.to_bytes());
    let public_key_hex = hex::encode(verifying_key.to_bytes());

    // Write private key
    fs::write(&private_key_path, &private_key_hex)
        .map_err(KeygenError::WritePrivateKeyFailed)?;
    info!("Wrote private key to: {}", private_key_path.display());

    // Write public key
    fs::write(&public_key_path, &public_key_hex)
        .map_err(KeygenError::WritePublicKeyFailed)?;
    info!("Wrote public key to: {}", public_key_path.display());

    // Security reminder
    warn!("SECURITY: Keep private.key secure and never distribute it!");

    Ok(KeygenResult {
        private_key_path: private_key_path.to_string_lossy().to_string(),
        public_key_path: public_key_path.to_string_lossy().to_string(),
        public_key_hex,
    })
}

/// Reads a private key from a file and returns the SigningKey.
///
/// # Arguments
///
/// * `path` - Path to the private key file (hex-encoded)
///
/// # Returns
///
/// Returns the SigningKey on success.
pub fn read_private_key(path: &str) -> Result<SigningKey, KeygenError> {
    let hex_content = fs::read_to_string(path)
        .map_err(KeygenError::ReadPrivateKeyFailed)?;

    let bytes = hex::decode(hex_content.trim())
        .map_err(|_| KeygenError::ReadPrivateKeyFailed(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid hex encoding in private key file"
        )))?;

    if bytes.len() != 32 {
        return Err(KeygenError::ReadPrivateKeyFailed(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid private key length: expected 32 bytes, got {}", bytes.len())
        )));
    }

    let key_bytes: [u8; 32] = bytes.try_into()
        .expect("already validated length");

    Ok(SigningKey::from_bytes(&key_bytes))
}

/// Reads a public key from a file and returns the bytes.
///
/// # Arguments
///
/// * `path` - Path to the public key file (hex-encoded)
///
/// # Returns
///
/// Returns the public key bytes on success.
pub fn read_public_key(path: &str) -> Result<[u8; 32], KeygenError> {
    let hex_content = fs::read_to_string(path)
        .map_err(KeygenError::ReadPublicKeyFailed)?;

    let bytes = hex::decode(hex_content.trim())
        .map_err(|_| KeygenError::ReadPublicKeyFailed(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Invalid hex encoding in public key file"
        )))?;

    if bytes.len() != 32 {
        return Err(KeygenError::ReadPublicKeyFailed(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Invalid public key length: expected 32 bytes, got {}", bytes.len())
        )));
    }

    let key_bytes: [u8; 32] = bytes.try_into()
        .expect("already validated length");

    Ok(key_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, Verifier};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_generate_keypair() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path().to_str().unwrap();

        let result = generate_keypair(output_dir, false);
        assert!(result.is_ok(), "Keygen should succeed");

        let keygen_result = result.unwrap();

        // Verify files were created
        assert!(Path::new(&keygen_result.private_key_path).exists());
        assert!(Path::new(&keygen_result.public_key_path).exists());

        // Verify public key hex is 64 chars (32 bytes)
        assert_eq!(keygen_result.public_key_hex.len(), 64);
    }

    #[test]
    fn test_generated_keys_work() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path().to_str().unwrap();

        let keygen_result = generate_keypair(output_dir, false).unwrap();

        // Read keys back
        let signing_key = read_private_key(&keygen_result.private_key_path).unwrap();
        let public_key_bytes = read_public_key(&keygen_result.public_key_path).unwrap();

        // Verify the keys match
        assert_eq!(
            signing_key.verifying_key().to_bytes(),
            public_key_bytes
        );

        // Test signing and verification
        let message = b"test message";
        let signature = signing_key.sign(message);

        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(&public_key_bytes).unwrap();
        assert!(verifying_key.verify(message, &signature).is_ok());
    }

    #[test]
    fn test_no_overwrite_without_force() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path().to_str().unwrap();

        // Generate first time
        generate_keypair(output_dir, false).unwrap();

        // Try again without force - should fail
        let result = generate_keypair(output_dir, false);
        assert!(matches!(result, Err(KeygenError::KeysAlreadyExist)));
    }

    #[test]
    fn test_overwrite_with_force() {
        let temp_dir = tempdir().unwrap();
        let output_dir = temp_dir.path().to_str().unwrap();

        // Generate first time
        let first_result = generate_keypair(output_dir, false).unwrap();
        let first_public_key = first_result.public_key_hex.clone();

        // Generate again with force - should succeed with new keys
        let second_result = generate_keypair(output_dir, true).unwrap();

        // Keys should be different (overwhelmingly likely)
        assert_ne!(first_public_key, second_result.public_key_hex);
    }

    #[test]
    fn test_creates_output_directory() {
        let temp_dir = tempdir().unwrap();
        let nested_dir = temp_dir.path().join("nested").join("keys");
        let output_dir = nested_dir.to_str().unwrap();

        assert!(!nested_dir.exists());

        let result = generate_keypair(output_dir, false);
        assert!(result.is_ok());
        assert!(nested_dir.exists());
    }
}
