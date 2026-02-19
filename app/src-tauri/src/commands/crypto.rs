//! Cryptographic command handlers for UltimaForge.
//!
//! These commands handle cryptographic operations:
//! - Keypair generation for manifest signing
//!
//! # Security
//!
//! - Private keys are generated using a cryptographically secure RNG
//! - Keys are returned in hex format for easy storage
//! - Server owners should store private keys securely and never commit them

use ed25519_dalek::SigningKey;
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use tracing::info;

/// Response containing a generated Ed25519 keypair.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeypairResponse {
    /// The public key in hex format (64 characters, 32 bytes).
    /// This key should be embedded in brand.json.
    pub public_key: String,
    /// The private key in hex format (128 characters, 64 bytes).
    /// This key should be kept secret and used with publish-cli.
    pub private_key: String,
}

/// Generates a new Ed25519 keypair for manifest signing.
///
/// This command creates a cryptographically secure keypair that can be used
/// for signing update manifests. The keypair is returned in hex format:
///
/// - `publicKey`: 64 hex characters (32 bytes) - embed in brand.json
/// - `privateKey`: 128 hex characters (64 bytes) - keep secret, use with publish-cli
///
/// # Returns
///
/// Returns a [`KeypairResponse`] containing both keys in hex format.
///
/// # Example Usage (Frontend)
///
/// ```typescript
/// const keys = await invoke<{ publicKey: string; privateKey: string }>('generate_keypair');
/// // Store publicKey in brand.json
/// // Save privateKey securely for use with publish-cli
/// ```
#[tauri::command]
pub async fn generate_keypair() -> Result<KeypairResponse, String> {
    info!("Generating new Ed25519 keypair for manifest signing");

    // Generate a new signing key using OS random number generator
    // Create random seed bytes and construct the signing key
    let mut seed = [0u8; 32];
    OsRng.fill_bytes(&mut seed);
    let signing_key = SigningKey::from_bytes(&seed);

    // Get the verifying (public) key
    let verifying_key = signing_key.verifying_key();

    // Convert to hex strings
    // Private key is the full 64-byte secret key (seed + public key)
    let private_key_hex = hex::encode(signing_key.to_bytes());
    // Public key is the 32-byte verifying key
    let public_key_hex = hex::encode(verifying_key.to_bytes());

    info!("Keypair generated successfully");

    Ok(KeypairResponse {
        public_key: public_key_hex,
        private_key: private_key_hex,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, VerifyingKey};

    #[tokio::test]
    async fn test_generate_keypair_returns_valid_keys() {
        let result = generate_keypair().await;
        assert!(result.is_ok(), "generate_keypair should succeed");

        let keypair = result.unwrap();

        // Check public key format (64 hex chars = 32 bytes)
        assert_eq!(
            keypair.public_key.len(),
            64,
            "Public key should be 64 hex characters"
        );
        assert!(
            keypair.public_key.chars().all(|c| c.is_ascii_hexdigit()),
            "Public key should be valid hex"
        );

        // Check private key format (64 hex chars = 32 bytes for seed)
        assert_eq!(
            keypair.private_key.len(),
            64,
            "Private key should be 64 hex characters"
        );
        assert!(
            keypair.private_key.chars().all(|c| c.is_ascii_hexdigit()),
            "Private key should be valid hex"
        );
    }

    #[tokio::test]
    async fn test_generated_keypair_can_sign_and_verify() {
        let keypair = generate_keypair().await.unwrap();

        // Decode the keys
        let private_bytes: [u8; 32] = hex::decode(&keypair.private_key)
            .unwrap()
            .try_into()
            .unwrap();
        let public_bytes: [u8; 32] = hex::decode(&keypair.public_key)
            .unwrap()
            .try_into()
            .unwrap();

        // Recreate the signing key from seed
        let signing_key = SigningKey::from_bytes(&private_bytes);
        let verifying_key = VerifyingKey::from_bytes(&public_bytes).unwrap();

        // Sign a test message
        let message = b"test manifest content";
        let signature = signing_key.sign(message);

        // Verify the signature
        use ed25519_dalek::Verifier;
        let verify_result = verifying_key.verify(message, &signature);
        assert!(
            verify_result.is_ok(),
            "Generated keypair should produce valid signatures"
        );
    }

    #[tokio::test]
    async fn test_each_generation_produces_unique_keys() {
        let keypair1 = generate_keypair().await.unwrap();
        let keypair2 = generate_keypair().await.unwrap();

        assert_ne!(
            keypair1.public_key, keypair2.public_key,
            "Each generation should produce unique public keys"
        );
        assert_ne!(
            keypair1.private_key, keypair2.private_key,
            "Each generation should produce unique private keys"
        );
    }

    #[test]
    fn test_keypair_response_serialization() {
        let response = KeypairResponse {
            public_key: "a".repeat(64),
            private_key: "b".repeat(64),
        };

        let json = serde_json::to_string(&response).expect("Should serialize");
        assert!(json.contains("publicKey"), "Should use camelCase");
        assert!(json.contains("privateKey"), "Should use camelCase");
    }
}
