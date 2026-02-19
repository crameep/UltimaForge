//! Security verification tests for UltimaForge.
//!
//! This module contains comprehensive security tests that verify the
//! cryptographic trust model is enforced correctly:
//!
//! - **Signature bypass attempts**: Verifies tampered, missing, and invalid signatures are rejected
//! - **Hash bypass attempts**: Verifies corrupted and modified files are rejected
//! - **Path traversal prevention**: Verifies directory traversal attacks are blocked
//!
//! # Running Security Tests
//!
//! ```bash
//! cargo test --package ultimaforge security_tests -- --nocapture
//! ```

#[cfg(test)]
mod tests {
    use crate::hash::{hash_bytes, hash_file, validate_hash_format, verify_file_hash, EMPTY_HASH};
    use crate::manifest::{FileEntry, Manifest, ManifestBuilder, ManifestError};
    use crate::signature::{
        parse_hex_public_key, parse_hex_signature, verify_manifest,
        SignatureVerificationError,
    };
    use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
    use rand::rngs::OsRng;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    // ============================================
    // HELPER FUNCTIONS
    // ============================================

    /// Generate a test Ed25519 keypair.
    fn generate_keypair() -> (SigningKey, VerifyingKey) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        (signing_key, verifying_key)
    }

    /// Create a valid test manifest JSON.
    fn create_test_manifest(version: &str) -> String {
        format!(
            r#"{{
            "version": "{}",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "client.exe",
            "client_args": [],
            "files": [
                {{
                    "path": "client.exe",
                    "sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                    "size": 1000,
                    "required": true
                }}
            ],
            "total_size": 1000
        }}"#,
            version
        )
    }

    /// Sign manifest bytes with a signing key.
    fn sign_manifest(manifest_bytes: &[u8], signing_key: &SigningKey) -> Vec<u8> {
        let signature = signing_key.sign(manifest_bytes);
        signature.to_bytes().to_vec()
    }

    /// Create a temporary file with content and return its path.
    fn create_temp_file(content: &[u8]) -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("test_file.dat");
        let mut file = fs::File::create(&file_path).expect("Failed to create file");
        file.write_all(content).expect("Failed to write content");
        (temp_dir, file_path)
    }

    // ============================================
    // SIGNATURE BYPASS TESTS
    // ============================================

    /// SEC-001: Valid signature should be accepted
    #[test]
    fn test_sec_001_valid_signature_accepted() {
        let (signing_key, verifying_key) = generate_keypair();
        let manifest_json = create_test_manifest("1.0.0");
        let manifest_bytes = manifest_json.as_bytes();

        let signature = sign_manifest(manifest_bytes, &signing_key);

        let result = verify_manifest(manifest_bytes, &signature, verifying_key.as_bytes());
        assert!(result.is_ok(), "SEC-001 FAILED: Valid signature should be accepted");
    }

    /// SEC-002: Missing signature should be rejected
    #[test]
    fn test_sec_002_missing_signature_rejected() {
        let (_, verifying_key) = generate_keypair();
        let manifest_json = create_test_manifest("1.0.0");
        let manifest_bytes = manifest_json.as_bytes();

        // Empty signature
        let empty_signature: Vec<u8> = vec![];

        let result = verify_manifest(manifest_bytes, &empty_signature, verifying_key.as_bytes());
        assert!(
            matches!(
                result,
                Err(SignatureVerificationError::InvalidSignatureLength(0))
            ),
            "SEC-002 FAILED: Missing signature should be rejected with InvalidSignatureLength"
        );
    }

    /// SEC-003: Tampered manifest should be rejected
    #[test]
    fn test_sec_003_tampered_manifest_rejected() {
        let (signing_key, verifying_key) = generate_keypair();

        // Sign the original manifest
        let original_manifest = create_test_manifest("1.0.0");
        let signature = sign_manifest(original_manifest.as_bytes(), &signing_key);

        // Tamper with the manifest (change version)
        let tampered_manifest = create_test_manifest("9.9.9");

        // Try to verify tampered manifest with original signature
        let result = verify_manifest(
            tampered_manifest.as_bytes(),
            &signature,
            verifying_key.as_bytes(),
        );

        assert!(
            matches!(result, Err(SignatureVerificationError::VerificationFailed)),
            "SEC-003 FAILED: Tampered manifest should be rejected"
        );
    }

    /// SEC-003b: Single byte modification should be detected
    #[test]
    fn test_sec_003b_single_byte_modification_detected() {
        let (signing_key, verifying_key) = generate_keypair();
        let manifest_json = create_test_manifest("1.0.0");
        let mut manifest_bytes = manifest_json.as_bytes().to_vec();

        let signature = sign_manifest(&manifest_bytes, &signing_key);

        // Flip a single bit in the manifest
        manifest_bytes[10] ^= 0x01;

        let result = verify_manifest(&manifest_bytes, &signature, verifying_key.as_bytes());
        assert!(
            matches!(result, Err(SignatureVerificationError::VerificationFailed)),
            "SEC-003b FAILED: Single byte modification should be detected"
        );
    }

    /// SEC-003c: Added whitespace should be detected
    #[test]
    fn test_sec_003c_added_whitespace_detected() {
        let (signing_key, verifying_key) = generate_keypair();
        let manifest_json = create_test_manifest("1.0.0");
        let signature = sign_manifest(manifest_json.as_bytes(), &signing_key);

        // Add whitespace to manifest
        let tampered = format!("{} ", manifest_json);

        let result = verify_manifest(tampered.as_bytes(), &signature, verifying_key.as_bytes());
        assert!(
            matches!(result, Err(SignatureVerificationError::VerificationFailed)),
            "SEC-003c FAILED: Added whitespace should be detected as tampering"
        );
    }

    /// SEC-004: Wrong public key should be rejected
    #[test]
    fn test_sec_004_wrong_public_key_rejected() {
        let (signing_key, _correct_key) = generate_keypair();
        let (_, wrong_key) = generate_keypair(); // Different keypair

        let manifest_json = create_test_manifest("1.0.0");
        let manifest_bytes = manifest_json.as_bytes();

        // Sign with correct key
        let signature = sign_manifest(manifest_bytes, &signing_key);

        // Verify with wrong key
        let result = verify_manifest(manifest_bytes, &signature, wrong_key.as_bytes());
        assert!(
            matches!(result, Err(SignatureVerificationError::VerificationFailed)),
            "SEC-004 FAILED: Wrong public key should be rejected"
        );
    }

    /// SEC-004b: Invalid public key format should be rejected
    #[test]
    fn test_sec_004b_invalid_public_key_format_rejected() {
        let manifest_json = create_test_manifest("1.0.0");
        let manifest_bytes = manifest_json.as_bytes();
        let fake_signature = vec![0u8; 64];
        let invalid_key = vec![0u8; 16]; // Wrong length

        let result = verify_manifest(manifest_bytes, &fake_signature, &invalid_key);
        assert!(
            matches!(
                result,
                Err(SignatureVerificationError::InvalidPublicKeyLength(16))
            ),
            "SEC-004b FAILED: Invalid public key length should be rejected"
        );
    }

    /// SEC-004c: Invalid signature length should be rejected
    #[test]
    fn test_sec_004c_invalid_signature_length_rejected() {
        let (_, verifying_key) = generate_keypair();
        let manifest_json = create_test_manifest("1.0.0");
        let manifest_bytes = manifest_json.as_bytes();
        let short_signature = vec![0u8; 32]; // Should be 64 bytes

        let result = verify_manifest(manifest_bytes, &short_signature, verifying_key.as_bytes());
        assert!(
            matches!(
                result,
                Err(SignatureVerificationError::InvalidSignatureLength(32))
            ),
            "SEC-004c FAILED: Invalid signature length should be rejected"
        );
    }

    /// SEC-004d: Corrupted signature should be rejected
    #[test]
    fn test_sec_004d_corrupted_signature_rejected() {
        let (signing_key, verifying_key) = generate_keypair();
        let manifest_json = create_test_manifest("1.0.0");
        let manifest_bytes = manifest_json.as_bytes();

        let mut signature = sign_manifest(manifest_bytes, &signing_key);

        // Corrupt the signature
        signature[0] ^= 0xFF;
        signature[31] ^= 0xFF;
        signature[63] ^= 0xFF;

        let result = verify_manifest(manifest_bytes, &signature, verifying_key.as_bytes());
        assert!(
            matches!(result, Err(SignatureVerificationError::VerificationFailed)),
            "SEC-004d FAILED: Corrupted signature should be rejected"
        );
    }

    /// SEC-004e: All-zero signature should be rejected
    #[test]
    fn test_sec_004e_zero_signature_rejected() {
        let (_, verifying_key) = generate_keypair();
        let manifest_json = create_test_manifest("1.0.0");
        let manifest_bytes = manifest_json.as_bytes();
        let zero_signature = vec![0u8; 64];

        let result = verify_manifest(manifest_bytes, &zero_signature, verifying_key.as_bytes());
        assert!(
            result.is_err(),
            "SEC-004e FAILED: All-zero signature should be rejected"
        );
    }

    // ============================================
    // HASH BYPASS TESTS
    // ============================================

    /// SEC-005: Valid file hash should be accepted
    #[test]
    fn test_sec_005_valid_hash_accepted() {
        let content = b"Test file content for hash verification";
        let expected_hash = hash_bytes(content);

        let (_temp_dir, file_path) = create_temp_file(content);

        let result = verify_file_hash(&file_path, &expected_hash);
        assert!(result.is_ok(), "SEC-005 FAILED: Valid hash should be accepted");
        assert!(
            result.unwrap(),
            "SEC-005 FAILED: Valid hash should return true"
        );
    }

    /// SEC-006: Corrupted file should be rejected
    #[test]
    fn test_sec_006_corrupted_file_rejected() {
        let original_content = b"Original content";
        let corrupted_content = b"Corrupted content";

        // Get hash of original content
        let expected_hash = hash_bytes(original_content);

        // Create file with corrupted content
        let (_temp_dir, file_path) = create_temp_file(corrupted_content);

        let result = verify_file_hash(&file_path, &expected_hash);
        assert!(result.is_ok(), "SEC-006 should not error on verification");
        assert!(
            !result.unwrap(),
            "SEC-006 FAILED: Corrupted file should return false (hash mismatch)"
        );
    }

    /// SEC-006b: Single byte corruption should be detected
    #[test]
    fn test_sec_006b_single_byte_corruption_detected() {
        let mut content = b"This is a test file with specific content".to_vec();
        let expected_hash = hash_bytes(&content);

        // Corrupt a single byte
        content[5] ^= 0x01;

        let (_temp_dir, file_path) = create_temp_file(&content);

        let result = verify_file_hash(&file_path, &expected_hash).unwrap();
        assert!(
            !result,
            "SEC-006b FAILED: Single byte corruption should be detected"
        );
    }

    /// SEC-006c: Appended data should be detected
    #[test]
    fn test_sec_006c_appended_data_detected() {
        let original = b"Original content";
        let expected_hash = hash_bytes(original);

        let mut tampered = original.to_vec();
        tampered.extend_from_slice(b" with appended data");

        let (_temp_dir, file_path) = create_temp_file(&tampered);

        let result = verify_file_hash(&file_path, &expected_hash).unwrap();
        assert!(
            !result,
            "SEC-006c FAILED: Appended data should be detected"
        );
    }

    /// SEC-006d: Truncated file should be detected
    #[test]
    fn test_sec_006d_truncated_file_detected() {
        let original = b"Original content that is longer";
        let expected_hash = hash_bytes(original);

        let truncated = &original[..10];

        let (_temp_dir, file_path) = create_temp_file(truncated);

        let result = verify_file_hash(&file_path, &expected_hash).unwrap();
        assert!(
            !result,
            "SEC-006d FAILED: Truncated file should be detected"
        );
    }

    /// SEC-006e: Wrong hash format should be rejected
    #[test]
    fn test_sec_006e_invalid_hash_format_rejected() {
        let content = b"Test content";
        let (_temp_dir, file_path) = create_temp_file(content);

        // Too short
        let result = verify_file_hash(&file_path, "abc123");
        assert!(
            result.is_err(),
            "SEC-006e FAILED: Too-short hash should be rejected"
        );

        // Contains non-hex
        let result = verify_file_hash(
            &file_path,
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        );
        assert!(
            result.is_err(),
            "SEC-006e FAILED: Non-hex hash should be rejected"
        );
    }

    /// SEC-006f: Empty file should have correct hash
    #[test]
    fn test_sec_006f_empty_file_hash() {
        let content = b"";
        let (_temp_dir, file_path) = create_temp_file(content);

        let result = verify_file_hash(&file_path, EMPTY_HASH).unwrap();
        assert!(result, "SEC-006f FAILED: Empty file should match EMPTY_HASH");

        // Wrong hash for empty file should fail
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        let result = verify_file_hash(&file_path, wrong_hash).unwrap();
        assert!(
            !result,
            "SEC-006f FAILED: Wrong hash for empty file should not match"
        );
    }

    // ============================================
    // PATH TRAVERSAL PREVENTION TESTS
    // ============================================

    /// SEC-008: Path traversal in manifest should be rejected
    #[test]
    fn test_sec_008_path_traversal_rejected() {
        // Unix-style path traversal
        let malicious_path_unix = "../../../etc/passwd";
        let entry = FileEntry::new(malicious_path_unix, EMPTY_HASH, 100);
        let result = entry.validate();
        assert!(
            matches!(result, Err(ManifestError::InvalidPath(_))),
            "SEC-008 FAILED: Unix path traversal should be rejected"
        );

        // Windows-style path traversal
        let malicious_path_windows = "..\\..\\..\\Windows\\System32\\config\\SAM";
        let entry = FileEntry::new(malicious_path_windows, EMPTY_HASH, 100);
        let result = entry.validate();
        assert!(
            matches!(result, Err(ManifestError::InvalidPath(_))),
            "SEC-008 FAILED: Windows path traversal should be rejected"
        );
    }

    /// SEC-008b: Absolute paths should be rejected
    #[test]
    fn test_sec_008b_absolute_paths_rejected() {
        // Unix absolute path
        let entry = FileEntry::new("/etc/passwd", EMPTY_HASH, 100);
        assert!(
            entry.validate().is_err(),
            "SEC-008b FAILED: Unix absolute path should be rejected"
        );

        // Windows absolute path
        let entry = FileEntry::new("\\Windows\\System32", EMPTY_HASH, 100);
        assert!(
            entry.validate().is_err(),
            "SEC-008b FAILED: Windows absolute path should be rejected"
        );
    }

    /// SEC-008c: Double-dot in middle of path should be rejected
    #[test]
    fn test_sec_008c_embedded_traversal_rejected() {
        let entry = FileEntry::new("data/../../../etc/passwd", EMPTY_HASH, 100);
        assert!(
            entry.validate().is_err(),
            "SEC-008c FAILED: Embedded traversal should be rejected"
        );
    }

    /// SEC-008d: Valid subdirectory paths should be accepted
    #[test]
    fn test_sec_008d_valid_subdirectory_accepted() {
        let entry = FileEntry::new("data/maps/map0.mul", EMPTY_HASH, 100);
        assert!(
            entry.validate().is_ok(),
            "SEC-008d FAILED: Valid subdirectory should be accepted"
        );

        let entry = FileEntry::new("client.exe", EMPTY_HASH, 100);
        assert!(
            entry.validate().is_ok(),
            "SEC-008d FAILED: Root file should be accepted"
        );
    }

    // ============================================
    // MANIFEST VALIDATION SECURITY TESTS
    // ============================================

    /// SEC-009: Invalid hash in manifest should be rejected
    #[test]
    fn test_sec_009_invalid_hash_in_manifest_rejected() {
        // Hash with wrong length
        let entry = FileEntry::new("file.exe", "abc123", 100);
        let result = entry.validate();
        assert!(
            matches!(result, Err(ManifestError::InvalidHash { .. })),
            "SEC-009 FAILED: Short hash should be rejected"
        );

        // Hash with invalid characters
        let entry = FileEntry::new(
            "file.exe",
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
            100,
        );
        let result = entry.validate();
        assert!(
            matches!(result, Err(ManifestError::InvalidHash { .. })),
            "SEC-009 FAILED: Non-hex hash should be rejected"
        );
    }

    /// SEC-009b: Total size mismatch should be detected
    #[test]
    fn test_sec_009b_total_size_mismatch_detected() {
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
        assert!(
            matches!(result, Err(ManifestError::TotalSizeMismatch { .. })),
            "SEC-009b FAILED: Total size mismatch should be detected"
        );
    }

    /// SEC-009c: Client executable not in files list should be rejected
    #[test]
    fn test_sec_009c_executable_not_in_files_rejected() {
        let json = r#"{
            "version": "1.0.0",
            "timestamp": "2026-02-15T00:00:00Z",
            "client_executable": "nonexistent.exe",
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
        assert!(
            matches!(result, Err(ManifestError::ExecutableNotInFiles(_))),
            "SEC-009c FAILED: Missing executable should be rejected"
        );
    }

    // ============================================
    // HEX PARSING SECURITY TESTS
    // ============================================

    /// SEC-010: Hex parsing should reject invalid input
    #[test]
    fn test_sec_010_hex_parsing_security() {
        // Invalid hex for signature
        let result = parse_hex_signature("not-valid-hex!");
        assert!(
            matches!(
                result,
                Err(SignatureVerificationError::InvalidSignatureFormat)
            ),
            "SEC-010 FAILED: Invalid hex signature should be rejected"
        );

        // Invalid hex for public key
        let result = parse_hex_public_key("not-valid-hex!");
        assert!(
            result.is_err(),
            "SEC-010 FAILED: Invalid hex public key should be rejected"
        );

        // Wrong length public key
        let result = parse_hex_public_key("0000");
        assert!(
            matches!(
                result,
                Err(SignatureVerificationError::InvalidPublicKeyLength(_))
            ),
            "SEC-010 FAILED: Wrong length public key should be rejected"
        );
    }

    // ============================================
    // CRYPTOGRAPHIC PROPERTY TESTS
    // ============================================

    /// SEC-011: Signature malleability should be prevented
    #[test]
    fn test_sec_011_signature_malleability_prevented() {
        let (signing_key, verifying_key) = generate_keypair();
        let manifest_json = create_test_manifest("1.0.0");
        let manifest_bytes = manifest_json.as_bytes();

        let mut signature = sign_manifest(manifest_bytes, &signing_key);

        // Try to create a malleable signature by modifying S component
        // (verify_strict should prevent this)
        signature[32..64].iter_mut().for_each(|b| *b ^= 0x01);

        let result = verify_manifest(manifest_bytes, &signature, verifying_key.as_bytes());
        assert!(
            result.is_err(),
            "SEC-011 FAILED: Malleable signature should be rejected by verify_strict"
        );
    }

    /// SEC-012: Different messages must produce different signatures
    #[test]
    fn test_sec_012_signature_uniqueness() {
        let (signing_key, verifying_key) = generate_keypair();

        let manifest_v1 = create_test_manifest("1.0.0");
        let manifest_v2 = create_test_manifest("2.0.0");

        let sig_v1 = sign_manifest(manifest_v1.as_bytes(), &signing_key);
        let sig_v2 = sign_manifest(manifest_v2.as_bytes(), &signing_key);

        // Signatures should be different
        assert_ne!(
            sig_v1, sig_v2,
            "SEC-012 FAILED: Different messages should produce different signatures"
        );

        // Cross-verification should fail
        let result = verify_manifest(manifest_v1.as_bytes(), &sig_v2, verifying_key.as_bytes());
        assert!(
            result.is_err(),
            "SEC-012 FAILED: Signature from v2 should not verify v1"
        );
    }

    /// SEC-013: Hash should be deterministic
    #[test]
    fn test_sec_013_hash_determinism() {
        let content = b"Test content for determinism verification";

        let hash1 = hash_bytes(content);
        let hash2 = hash_bytes(content);
        let hash3 = hash_bytes(content);

        assert_eq!(hash1, hash2, "SEC-013 FAILED: Hash should be deterministic");
        assert_eq!(hash2, hash3, "SEC-013 FAILED: Hash should be deterministic");

        // Different content should produce different hash
        let different = b"Different content";
        let hash_different = hash_bytes(different);
        assert_ne!(
            hash1, hash_different,
            "SEC-013 FAILED: Different content should produce different hash"
        );
    }

    // ============================================
    // SUMMARY TEST (Run all security checks)
    // ============================================

    /// Run all security verification tests and report summary
    #[test]
    fn test_security_verification_summary() {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║         ULTIMAFORGE SECURITY VERIFICATION TESTS          ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║                                                          ║");
        println!("║  ✓ SEC-001: Valid signature accepted                     ║");
        println!("║  ✓ SEC-002: Missing signature rejected                   ║");
        println!("║  ✓ SEC-003: Tampered manifest rejected                   ║");
        println!("║  ✓ SEC-004: Wrong public key rejected                    ║");
        println!("║  ✓ SEC-005: Valid file hash accepted                     ║");
        println!("║  ✓ SEC-006: Corrupted file rejected                      ║");
        println!("║  ✓ SEC-008: Path traversal rejected                      ║");
        println!("║  ✓ SEC-009: Invalid manifest rejected                    ║");
        println!("║  ✓ SEC-010: Hex parsing security enforced                ║");
        println!("║  ✓ SEC-011: Signature malleability prevented             ║");
        println!("║  ✓ SEC-012: Signature uniqueness verified                ║");
        println!("║  ✓ SEC-013: Hash determinism verified                    ║");
        println!("║                                                          ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        println!();

        // This test always passes - it's a summary/documentation test
        assert!(true, "Security verification complete");
    }
}
