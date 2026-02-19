//! Ed25519 signature verification module for UltimaForge.
//!
//! This module provides cryptographic signature verification for manifests,
//! ensuring that update manifests have been signed by the server owner's
//! private key before they are trusted.
//!
//! # Security
//!
//! - Uses `verify_strict()` for stronger security guarantees
//! - ALWAYS verify signature BEFORE parsing manifest contents
//! - Public key is embedded at build time, never downloaded

use ed25519_dalek::{Signature, VerifyingKey, SignatureError};

/// Errors that can occur during signature verification.
#[derive(Debug, thiserror::Error)]
pub enum SignatureVerificationError {
    /// The public key bytes are invalid or malformed.
    #[error("Invalid public key: {0}")]
    InvalidPublicKey(#[from] SignatureError),

    /// The signature bytes are invalid or malformed.
    #[error("Invalid signature format")]
    InvalidSignatureFormat,

    /// The signature is invalid length (expected 64 bytes).
    #[error("Invalid signature length: expected 64 bytes, got {0}")]
    InvalidSignatureLength(usize),

    /// The public key is invalid length (expected 32 bytes).
    #[error("Invalid public key length: expected 32 bytes, got {0}")]
    InvalidPublicKeyLength(usize),

    /// The signature verification failed - the data was not signed by this key.
    #[error("Signature verification failed")]
    VerificationFailed,
}

/// Verifies that the given data was signed with the corresponding private key.
///
/// # Arguments
///
/// * `data` - The data bytes that were signed (e.g., manifest.json contents)
/// * `signature_bytes` - The Ed25519 signature (64 bytes)
/// * `public_key_bytes` - The Ed25519 public key (32 bytes)
///
/// # Returns
///
/// Returns `Ok(())` if the signature is valid, or an error describing why
/// verification failed.
///
/// # Example
///
/// ```ignore
/// use ultimaforge_lib::signature::verify_signature;
///
/// let manifest_bytes = b"...manifest json...";
/// let signature = &[/* 64 bytes */];
/// let public_key = &[/* 32 bytes */];
///
/// // ALWAYS verify before parsing!
/// verify_signature(manifest_bytes, signature, public_key)?;
/// let manifest: Manifest = serde_json::from_slice(manifest_bytes)?;
/// ```
pub fn verify_signature(
    data: &[u8],
    signature_bytes: &[u8],
    public_key_bytes: &[u8],
) -> Result<(), SignatureVerificationError> {
    // Validate public key length
    if public_key_bytes.len() != 32 {
        return Err(SignatureVerificationError::InvalidPublicKeyLength(
            public_key_bytes.len(),
        ));
    }

    // Validate signature length
    if signature_bytes.len() != 64 {
        return Err(SignatureVerificationError::InvalidSignatureLength(
            signature_bytes.len(),
        ));
    }

    // Convert to fixed-size arrays
    let public_key_array: [u8; 32] = public_key_bytes
        .try_into()
        .expect("already validated length");
    let signature_array: [u8; 64] = signature_bytes
        .try_into()
        .expect("already validated length");

    // Create verifying key from bytes
    let verifying_key = VerifyingKey::from_bytes(&public_key_array)?;

    // Create signature from bytes
    let signature = Signature::from_bytes(&signature_array);

    // Use verify_strict for stronger security guarantees
    // This prevents certain malleability attacks
    verifying_key
        .verify_strict(data, &signature)
        .map_err(|_| SignatureVerificationError::VerificationFailed)
}

/// Verifies a manifest's signature using the provided public key.
///
/// This is a convenience wrapper around `verify_signature` specifically
/// for manifest verification workflows.
///
/// # Arguments
///
/// * `manifest_bytes` - The raw bytes of manifest.json
/// * `signature_bytes` - The raw bytes of manifest.sig (64 bytes)
/// * `public_key_bytes` - The embedded public key (32 bytes)
///
/// # Security Note
///
/// CRITICAL: Always call this function BEFORE parsing the manifest JSON.
/// Never trust manifest contents until the signature has been verified.
pub fn verify_manifest(
    manifest_bytes: &[u8],
    signature_bytes: &[u8],
    public_key_bytes: &[u8],
) -> Result<(), SignatureVerificationError> {
    verify_signature(manifest_bytes, signature_bytes, public_key_bytes)
}

/// Parses a hex-encoded signature into raw bytes.
///
/// # Arguments
///
/// * `hex_signature` - The signature as a hex string (128 characters for 64 bytes)
///
/// # Returns
///
/// Returns the decoded signature bytes, or an error if the hex is invalid.
pub fn parse_hex_signature(hex_signature: &str) -> Result<Vec<u8>, SignatureVerificationError> {
    hex::decode(hex_signature).map_err(|_| SignatureVerificationError::InvalidSignatureFormat)
}

/// Parses a hex-encoded public key into raw bytes.
///
/// # Arguments
///
/// * `hex_key` - The public key as a hex string (64 characters for 32 bytes)
///
/// # Returns
///
/// Returns the decoded public key bytes, or an error if the hex is invalid.
pub fn parse_hex_public_key(hex_key: &str) -> Result<Vec<u8>, SignatureVerificationError> {
    let bytes = hex::decode(hex_key).map_err(|_| {
        SignatureVerificationError::InvalidPublicKeyLength(hex_key.len() / 2)
    })?;

    if bytes.len() != 32 {
        return Err(SignatureVerificationError::InvalidPublicKeyLength(bytes.len()));
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{SigningKey, Signer};
    use rand::rngs::OsRng;

    /// Generate a test keypair for use in tests.
    fn generate_test_keypair() -> (SigningKey, VerifyingKey) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        (signing_key, verifying_key)
    }

    #[test]
    fn test_verify_valid_signature() {
        let (signing_key, verifying_key) = generate_test_keypair();
        let message = b"test manifest content";

        // Sign the message
        let signature = signing_key.sign(message);

        // Verify should succeed
        let result = verify_signature(
            message,
            &signature.to_bytes(),
            verifying_key.as_bytes(),
        );

        assert!(result.is_ok(), "Valid signature should verify successfully");
    }

    #[test]
    fn test_verify_invalid_signature() {
        let (signing_key, verifying_key) = generate_test_keypair();
        let message = b"test manifest content";
        let wrong_message = b"different content";

        // Sign a different message
        let signature = signing_key.sign(wrong_message);

        // Verify should fail
        let result = verify_signature(
            message,
            &signature.to_bytes(),
            verifying_key.as_bytes(),
        );

        assert!(
            matches!(result, Err(SignatureVerificationError::VerificationFailed)),
            "Invalid signature should fail verification"
        );
    }

    #[test]
    fn test_verify_tampered_data() {
        let (signing_key, verifying_key) = generate_test_keypair();
        let original_message = b"original manifest content";

        // Sign the original message
        let signature = signing_key.sign(original_message);

        // Try to verify with tampered data
        let tampered_message = b"tampered manifest content";
        let result = verify_signature(
            tampered_message,
            &signature.to_bytes(),
            verifying_key.as_bytes(),
        );

        assert!(
            matches!(result, Err(SignatureVerificationError::VerificationFailed)),
            "Tampered data should fail verification"
        );
    }

    #[test]
    fn test_verify_wrong_public_key() {
        let (signing_key, _) = generate_test_keypair();
        let (_, wrong_verifying_key) = generate_test_keypair();
        let message = b"test manifest content";

        // Sign with the correct key
        let signature = signing_key.sign(message);

        // Verify with wrong public key should fail
        let result = verify_signature(
            message,
            &signature.to_bytes(),
            wrong_verifying_key.as_bytes(),
        );

        assert!(
            matches!(result, Err(SignatureVerificationError::VerificationFailed)),
            "Wrong public key should fail verification"
        );
    }

    #[test]
    fn test_invalid_public_key_length() {
        let message = b"test";
        let signature = [0u8; 64];
        let short_key = [0u8; 16]; // Wrong length

        let result = verify_signature(message, &signature, &short_key);

        assert!(
            matches!(
                result,
                Err(SignatureVerificationError::InvalidPublicKeyLength(16))
            ),
            "Short public key should return length error"
        );
    }

    #[test]
    fn test_invalid_signature_length() {
        let message = b"test";
        let short_signature = [0u8; 32]; // Wrong length
        let public_key = [0u8; 32];

        let result = verify_signature(message, &short_signature, &public_key);

        assert!(
            matches!(
                result,
                Err(SignatureVerificationError::InvalidSignatureLength(32))
            ),
            "Short signature should return length error"
        );
    }

    #[test]
    fn test_verify_manifest_wrapper() {
        let (signing_key, verifying_key) = generate_test_keypair();
        let manifest_json = br#"{"version":"1.0.0","files":[]}"#;

        let signature = signing_key.sign(manifest_json);

        let result = verify_manifest(
            manifest_json,
            &signature.to_bytes(),
            verifying_key.as_bytes(),
        );

        assert!(result.is_ok(), "verify_manifest should work like verify_signature");
    }

    #[test]
    fn test_parse_hex_signature() {
        let valid_hex = "0".repeat(128); // 64 bytes = 128 hex chars
        let result = parse_hex_signature(&valid_hex);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 64);
    }

    #[test]
    fn test_parse_hex_signature_invalid() {
        let invalid_hex = "not_valid_hex!";
        let result = parse_hex_signature(invalid_hex);
        assert!(matches!(
            result,
            Err(SignatureVerificationError::InvalidSignatureFormat)
        ));
    }

    #[test]
    fn test_parse_hex_public_key() {
        let valid_hex = "0".repeat(64); // 32 bytes = 64 hex chars
        let result = parse_hex_public_key(&valid_hex);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 32);
    }

    #[test]
    fn test_parse_hex_public_key_wrong_length() {
        let wrong_length_hex = "0".repeat(48); // 24 bytes
        let result = parse_hex_public_key(&wrong_length_hex);
        assert!(matches!(
            result,
            Err(SignatureVerificationError::InvalidPublicKeyLength(24))
        ));
    }

    #[test]
    fn test_empty_message_verification() {
        let (signing_key, verifying_key) = generate_test_keypair();
        let empty_message = b"";

        let signature = signing_key.sign(empty_message);

        let result = verify_signature(
            empty_message,
            &signature.to_bytes(),
            verifying_key.as_bytes(),
        );

        assert!(result.is_ok(), "Empty message should be signable and verifiable");
    }

    #[test]
    fn test_large_message_verification() {
        let (signing_key, verifying_key) = generate_test_keypair();
        let large_message = vec![0xABu8; 1_000_000]; // 1MB message

        let signature = signing_key.sign(&large_message);

        let result = verify_signature(
            &large_message,
            &signature.to_bytes(),
            verifying_key.as_bytes(),
        );

        assert!(result.is_ok(), "Large message should verify correctly");
    }

    #[test]
    fn test_signature_malleability_protection() {
        // verify_strict() should prevent signature malleability
        // This test ensures we're using strict verification
        let (signing_key, verifying_key) = generate_test_keypair();
        let message = b"test message for malleability";

        let signature = signing_key.sign(message);
        let mut modified_sig = signature.to_bytes();

        // Flip a bit in the signature - should fail strict verification
        modified_sig[0] ^= 0x01;

        let result = verify_signature(
            message,
            &modified_sig,
            verifying_key.as_bytes(),
        );

        assert!(
            result.is_err(),
            "Modified signature should fail strict verification"
        );
    }
}
