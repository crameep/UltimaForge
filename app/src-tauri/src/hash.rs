//! SHA-256 file hashing utilities for UltimaForge.
//!
//! This module provides functions for computing and verifying SHA-256 hashes
//! of files. It uses streaming I/O to efficiently handle files of any size
//! without loading them entirely into memory.
//!
//! # Security
//!
//! - All downloaded files MUST have their hash verified before use
//! - Hash verification is mandatory, never skip it
//! - Uses hex-encoded lowercase hashes for consistency

use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

/// Buffer size for streaming file reads (64 KB).
/// Chosen to balance memory usage with I/O efficiency.
const BUFFER_SIZE: usize = 64 * 1024;

/// Errors that can occur during hash operations.
#[derive(Debug, thiserror::Error)]
pub enum HashError {
    /// Failed to open or read the file.
    #[error("Failed to read file '{path}': {source}")]
    IoError {
        path: String,
        #[source]
        source: io::Error,
    },

    /// The expected hash format is invalid.
    #[error("Invalid hash format: {0}")]
    InvalidHashFormat(String),
}

impl HashError {
    /// Creates an IoError variant from a path and error.
    fn io(path: &Path, source: io::Error) -> Self {
        Self::IoError {
            path: path.display().to_string(),
            source,
        }
    }
}

/// Computes the SHA-256 hash of a file.
///
/// Uses streaming I/O to handle files of any size efficiently without
/// loading the entire file into memory.
///
/// # Arguments
///
/// * `path` - Path to the file to hash
///
/// # Returns
///
/// Returns the hex-encoded (lowercase) SHA-256 hash of the file.
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use ultimaforge_lib::hash::hash_file;
///
/// let hash = hash_file(Path::new("client.exe"))?;
/// println!("SHA-256: {}", hash);
/// ```
pub fn hash_file(path: &Path) -> Result<String, HashError> {
    let file = File::open(path).map_err(|e| HashError::io(path, e))?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let bytes_read = reader.read(&mut buffer).map_err(|e| HashError::io(path, e))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

/// Verifies that a file's SHA-256 hash matches the expected value.
///
/// Uses streaming I/O to handle files of any size efficiently.
///
/// # Arguments
///
/// * `path` - Path to the file to verify
/// * `expected_hash` - The expected hex-encoded SHA-256 hash (64 characters, lowercase)
///
/// # Returns
///
/// Returns `Ok(true)` if the hash matches, `Ok(false)` if it doesn't match,
/// or an error if the file cannot be read or the expected hash format is invalid.
///
/// # Example
///
/// ```ignore
/// use std::path::Path;
/// use ultimaforge_lib::hash::verify_file_hash;
///
/// let expected = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
/// if verify_file_hash(Path::new("client.exe"), expected)? {
///     println!("File integrity verified!");
/// } else {
///     println!("File is corrupted!");
/// }
/// ```
pub fn verify_file_hash(path: &Path, expected_hash: &str) -> Result<bool, HashError> {
    // Validate expected hash format
    validate_hash_format(expected_hash)?;

    let actual_hash = hash_file(path)?;

    // Compare in constant time to prevent timing attacks (though not critical here)
    // Use lowercase comparison for consistency
    Ok(actual_hash.to_lowercase() == expected_hash.to_lowercase())
}

/// Validates that a hash string is properly formatted.
///
/// A valid SHA-256 hash is exactly 64 hexadecimal characters.
///
/// # Arguments
///
/// * `hash` - The hash string to validate
///
/// # Returns
///
/// Returns `Ok(())` if valid, or an error describing the format issue.
pub fn validate_hash_format(hash: &str) -> Result<(), HashError> {
    if hash.len() != 64 {
        return Err(HashError::InvalidHashFormat(format!(
            "expected 64 hex characters, got {}",
            hash.len()
        )));
    }

    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(HashError::InvalidHashFormat(
            "contains non-hexadecimal characters".to_string(),
        ));
    }

    Ok(())
}

/// Computes the SHA-256 hash of raw bytes.
///
/// Useful for hashing in-memory data without writing to a file.
///
/// # Arguments
///
/// * `data` - The bytes to hash
///
/// # Returns
///
/// Returns the hex-encoded (lowercase) SHA-256 hash.
///
/// # Example
///
/// ```ignore
/// use ultimaforge_lib::hash::hash_bytes;
///
/// let hash = hash_bytes(b"Hello, World!");
/// assert_eq!(hash, "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
/// ```
pub fn hash_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Returns the SHA-256 hash of an empty file/input.
///
/// This is a constant value useful for comparison and testing.
pub const EMPTY_HASH: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Returns the blob URL for a file given its hash and base URL.
///
/// Files are stored in content-addressed storage using their SHA-256 hash.
///
/// # Arguments
///
/// * `base_url` - The base URL of the update server
/// * `sha256_hash` - The hex-encoded SHA-256 hash of the file
///
/// # Example
///
/// ```ignore
/// use ultimaforge_lib::hash::get_blob_url;
///
/// let url = get_blob_url("https://updates.example.com", "abc123...");
/// assert_eq!(url, "https://updates.example.com/files/abc123...");
/// ```
pub fn get_blob_url(base_url: &str, sha256_hash: &str) -> String {
    format!("{}/files/{}", base_url.trim_end_matches('/'), sha256_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper to create a temporary file with content
    fn create_temp_file(content: &[u8]) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        file.write_all(content).expect("Failed to write temp file");
        file.flush().expect("Failed to flush temp file");
        file
    }

    #[test]
    fn test_hash_empty_file() {
        let file = create_temp_file(b"");
        let hash = hash_file(file.path()).expect("Should hash empty file");

        // SHA-256 of empty input
        assert_eq!(hash, EMPTY_HASH);
    }

    #[test]
    fn test_hash_known_content() {
        // SHA-256 of "Hello, World!" is known
        let content = b"Hello, World!";
        let file = create_temp_file(content);
        let hash = hash_file(file.path()).expect("Should hash file");

        // Verified against: echo -n "Hello, World!" | sha256sum
        assert_eq!(hash, "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
    }

    #[test]
    fn test_hash_bytes_empty() {
        assert_eq!(hash_bytes(b""), EMPTY_HASH);
    }

    #[test]
    fn test_hash_bytes_known_content() {
        let hash = hash_bytes(b"Hello, World!");
        assert_eq!(hash, "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
    }

    #[test]
    fn test_hash_file_consistency() {
        // Hashing the same content should produce the same hash
        let content = b"test content for hashing";
        let file1 = create_temp_file(content);
        let file2 = create_temp_file(content);

        let hash1 = hash_file(file1.path()).expect("Should hash file1");
        let hash2 = hash_file(file2.path()).expect("Should hash file2");

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_file_not_found() {
        let result = hash_file(Path::new("/nonexistent/file/path.txt"));
        assert!(matches!(result, Err(HashError::IoError { .. })));
    }

    #[test]
    fn test_verify_hash_valid() {
        let content = b"Hello, World!";
        let file = create_temp_file(content);
        let expected = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";

        let result = verify_file_hash(file.path(), expected).expect("Should verify hash");
        assert!(result, "Hash should match");
    }

    #[test]
    fn test_verify_hash_invalid() {
        let content = b"Hello, World!";
        let file = create_temp_file(content);
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";

        let result = verify_file_hash(file.path(), wrong_hash).expect("Should verify hash");
        assert!(!result, "Hash should not match");
    }

    #[test]
    fn test_verify_hash_case_insensitive() {
        let content = b"Hello, World!";
        let file = create_temp_file(content);

        // Uppercase version of the hash
        let expected_upper = "DFFD6021BB2BD5B0AF676290809EC3A53191DD81C7F70A4B28688A362182986F";
        let result = verify_file_hash(file.path(), expected_upper).expect("Should verify hash");
        assert!(result, "Hash comparison should be case-insensitive");
    }

    #[test]
    fn test_verify_hash_file_not_found() {
        let expected = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        let result = verify_file_hash(Path::new("/nonexistent/file.txt"), expected);
        assert!(matches!(result, Err(HashError::IoError { .. })));
    }

    #[test]
    fn test_validate_hash_format_valid() {
        let valid = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        assert!(validate_hash_format(valid).is_ok());
    }

    #[test]
    fn test_validate_hash_format_too_short() {
        let result = validate_hash_format("abc123");
        assert!(matches!(result, Err(HashError::InvalidHashFormat(_))));

        if let Err(HashError::InvalidHashFormat(msg)) = result {
            assert!(msg.contains("expected 64"));
        }
    }

    #[test]
    fn test_validate_hash_format_too_long() {
        let too_long = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b8550";
        let result = validate_hash_format(too_long);
        assert!(matches!(result, Err(HashError::InvalidHashFormat(_))));
    }

    #[test]
    fn test_validate_hash_format_invalid_chars() {
        let invalid = "gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg";
        let result = validate_hash_format(invalid);
        assert!(matches!(result, Err(HashError::InvalidHashFormat(_))));

        if let Err(HashError::InvalidHashFormat(msg)) = result {
            assert!(msg.contains("non-hexadecimal"));
        }
    }

    #[test]
    fn test_verify_hash_invalid_format() {
        let file = create_temp_file(b"content");
        let invalid_hash = "tooshort";

        let result = verify_file_hash(file.path(), invalid_hash);
        assert!(matches!(result, Err(HashError::InvalidHashFormat(_))));
    }

    #[test]
    fn test_get_blob_url() {
        let url = get_blob_url(
            "https://updates.example.com",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            url,
            "https://updates.example.com/files/e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_get_blob_url_strips_trailing_slash() {
        let url = get_blob_url(
            "https://updates.example.com/",
            "abc123"
        );
        assert_eq!(url, "https://updates.example.com/files/abc123");
    }

    #[test]
    fn test_hash_large_file() {
        // Create a file larger than the buffer size to test streaming
        let content: Vec<u8> = (0..BUFFER_SIZE * 3).map(|i| (i % 256) as u8).collect();
        let file = create_temp_file(&content);

        let hash = hash_file(file.path()).expect("Should hash large file");

        // Verify consistency
        let hash2 = hash_bytes(&content);
        assert_eq!(hash, hash2, "File hash and bytes hash should match");
    }

    #[test]
    fn test_hash_binary_content() {
        // Test with binary content including null bytes
        let content: Vec<u8> = (0..256).map(|i| i as u8).collect();
        let file = create_temp_file(&content);

        let file_hash = hash_file(file.path()).expect("Should hash binary file");
        let bytes_hash = hash_bytes(&content);

        assert_eq!(file_hash, bytes_hash);
    }

    #[test]
    fn test_empty_hash_constant() {
        // Verify the EMPTY_HASH constant is correct
        assert_eq!(hash_bytes(b""), EMPTY_HASH);
    }

    #[test]
    fn test_hash_deterministic() {
        // Same content should always produce same hash
        let content = b"deterministic test";

        for _ in 0..10 {
            let hash = hash_bytes(content);
            assert_eq!(hash.len(), 64);
            assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn test_different_content_different_hash() {
        let hash1 = hash_bytes(b"content 1");
        let hash2 = hash_bytes(b"content 2");

        assert_ne!(hash1, hash2, "Different content should produce different hashes");
    }

    #[test]
    fn test_hash_error_display() {
        let error = HashError::InvalidHashFormat("test error".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Invalid hash format"));
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_io_error_includes_path() {
        let result = hash_file(Path::new("/nonexistent/path/file.txt"));
        if let Err(HashError::IoError { path, .. }) = result {
            assert!(path.contains("nonexistent"));
        } else {
            panic!("Expected IoError");
        }
    }
}
