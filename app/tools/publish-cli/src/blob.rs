//! Content-addressed blob storage for UltimaForge publishing.
//!
//! This module provides functionality to copy files to content-addressed
//! storage, where each file is stored using its SHA-256 hash as the filename.
//! This enables deduplication and ensures file integrity.
//!
//! # Directory Structure
//!
//! Given a source directory like:
//! ```text
//! source/
//!   client.exe
//!   data/
//!     map0.mul
//! ```
//!
//! The output will be:
//! ```text
//! files/
//!   {sha256_of_client.exe}
//!   {sha256_of_map0.mul}
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use publish_cli::blob::create_blobs;
//!
//! let result = create_blobs("./source", "./files")?;
//! println!("Created {} blobs", result.blob_count);
//! ```

use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};
use walkdir::WalkDir;

/// Errors that can occur during blob creation.
#[derive(Debug, Error)]
pub enum BlobError {
    /// Failed to access the source directory.
    #[error("Failed to access source directory: {0}")]
    SourceDirAccessFailed(#[source] std::io::Error),

    /// Source directory does not exist.
    #[error("Source directory does not exist: {0}")]
    SourceDirNotFound(String),

    /// Failed to read a file for hashing/copying.
    #[error("Failed to read file '{path}': {source}")]
    ReadFileFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Failed to write a blob file.
    #[error("Failed to write blob '{path}': {source}")]
    WriteBlobFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// Failed to create output directory.
    #[error("Failed to create output directory: {0}")]
    CreateDirFailed(#[source] std::io::Error),

    /// Failed to walk the directory tree.
    #[error("Failed to walk directory: {0}")]
    #[allow(dead_code)]
    WalkDirFailed(#[source] walkdir::Error),
}

/// Information about a single blob created.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BlobInfo {
    /// Original relative path of the file.
    pub original_path: String,
    /// SHA-256 hash (also the blob filename).
    pub sha256: String,
    /// Size of the file in bytes.
    pub size: u64,
}

/// Result of blob creation.
pub struct BlobResult {
    /// Path to the output directory.
    pub output_dir: String,
    /// Number of blobs created.
    pub blob_count: usize,
    /// Number of files that were deduplicated (same content).
    pub deduplicated_count: usize,
    /// Total size of all unique blobs in bytes.
    pub total_size: u64,
    /// List of all blobs created.
    pub blobs: Vec<BlobInfo>,
}

/// Computes the SHA-256 hash of a file while reading it into a buffer.
///
/// Returns both the hash and the file contents for efficient single-pass processing.
fn compute_hash_and_read(file_path: &Path) -> Result<(String, Vec<u8>), BlobError> {
    let mut file = File::open(file_path).map_err(|e| BlobError::ReadFileFailed {
        path: file_path.display().to_string(),
        source: e,
    })?;

    let mut hasher = Sha256::new();
    let mut contents = Vec::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .map_err(|e| BlobError::ReadFileFailed {
                path: file_path.display().to_string(),
                source: e,
            })?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
        contents.extend_from_slice(&buffer[..bytes_read]);
    }

    Ok((hex::encode(hasher.finalize()), contents))
}

/// Creates content-addressed blobs from a source directory.
///
/// This function walks the source directory, computes SHA-256 hashes for
/// each file, and copies them to the output directory using the hash as
/// the filename. Duplicate files (same content) are automatically
/// deduplicated.
///
/// # Arguments
///
/// * `source_dir` - Path to the source directory containing files
/// * `output_dir` - Path where blob files will be written
///
/// # Returns
///
/// Returns `BlobResult` with statistics about the blobs created.
///
/// # Example
///
/// ```ignore
/// use publish_cli::blob::create_blobs;
///
/// let result = create_blobs("./uo-client", "./files")?;
/// println!("Created {} blobs ({} bytes)", result.blob_count, result.total_size);
/// ```
pub fn create_blobs(source_dir: &str, output_dir: &str) -> Result<BlobResult, BlobError> {
    let source_path = Path::new(source_dir);
    let output_path = Path::new(output_dir);

    // Validate source directory exists
    if !source_path.exists() {
        return Err(BlobError::SourceDirNotFound(source_dir.to_string()));
    }

    if !source_path.is_dir() {
        return Err(BlobError::SourceDirAccessFailed(io::Error::new(
            io::ErrorKind::NotADirectory,
            "Source path is not a directory",
        )));
    }

    info!("Scanning source directory: {}", source_dir);

    // Create output directory if needed
    if !output_path.exists() {
        fs::create_dir_all(output_path).map_err(BlobError::CreateDirFailed)?;
        info!("Created output directory: {}", output_dir);
    }

    // Track unique hashes for deduplication
    let mut seen_hashes: HashSet<String> = HashSet::new();
    let mut blobs: Vec<BlobInfo> = Vec::new();
    let mut total_size: u64 = 0;
    let mut deduplicated_count: usize = 0;

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
            BlobError::SourceDirAccessFailed(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Failed to compute relative path",
            ))
        })?;

        // Normalize path separators to forward slashes
        let relative_path_str = relative_path.to_string_lossy().replace('\\', "/");

        debug!("Processing file: {}", relative_path_str);

        // Compute hash and read file contents
        let (sha256, contents) = compute_hash_and_read(path)?;
        let size = contents.len() as u64;

        // Check if we've already written this blob
        if seen_hashes.contains(&sha256) {
            debug!("  Deduplicated (hash already exists): {}", sha256);
            deduplicated_count += 1;
        } else {
            // Write blob to output directory
            let blob_path = output_path.join(&sha256);
            let mut blob_file =
                File::create(&blob_path).map_err(|e| BlobError::WriteBlobFailed {
                    path: blob_path.display().to_string(),
                    source: e,
                })?;

            blob_file
                .write_all(&contents)
                .map_err(|e| BlobError::WriteBlobFailed {
                    path: blob_path.display().to_string(),
                    source: e,
                })?;

            debug!("  Created blob: {} ({} bytes)", sha256, size);
            seen_hashes.insert(sha256.clone());
            total_size += size;
        }

        blobs.push(BlobInfo {
            original_path: relative_path_str,
            sha256,
            size,
        });
    }

    // Sort blobs by original path for consistent output
    blobs.sort_by(|a, b| a.original_path.cmp(&b.original_path));

    info!("Created blobs in: {}", output_dir);
    info!("  Unique blobs: {}", seen_hashes.len());
    info!("  Total files:  {}", blobs.len());
    info!("  Deduplicated: {}", deduplicated_count);
    info!("  Total size:   {} bytes", total_size);

    Ok(BlobResult {
        output_dir: output_dir.to_string(),
        blob_count: seen_hashes.len(),
        deduplicated_count,
        total_size,
        blobs,
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
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_create_blobs_basic() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let output_dir = temp_dir.path().join("files");
        fs::create_dir(&source_dir).unwrap();

        // Create test files
        fs::write(source_dir.join("file1.txt"), b"hello world").unwrap();
        fs::write(source_dir.join("file2.txt"), b"goodbye world").unwrap();

        let result =
            create_blobs(source_dir.to_str().unwrap(), output_dir.to_str().unwrap()).unwrap();

        assert_eq!(result.blob_count, 2);
        assert_eq!(result.blobs.len(), 2);
        assert!(output_dir.exists());

        // Verify blobs were created with correct names (SHA-256 hashes)
        for blob in &result.blobs {
            let blob_path = output_dir.join(&blob.sha256);
            assert!(
                blob_path.exists(),
                "Blob file should exist: {}",
                blob.sha256
            );
        }
    }

    #[test]
    fn test_create_blobs_with_subdirectories() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let data_dir = source_dir.join("data");
        let output_dir = temp_dir.path().join("files");
        fs::create_dir_all(&data_dir).unwrap();

        // Create test files in subdirectory
        fs::write(source_dir.join("root.txt"), b"root content").unwrap();
        fs::write(data_dir.join("nested.txt"), b"nested content").unwrap();

        let result =
            create_blobs(source_dir.to_str().unwrap(), output_dir.to_str().unwrap()).unwrap();

        assert_eq!(result.blob_count, 2);

        // Verify paths are normalized
        let paths: Vec<_> = result
            .blobs
            .iter()
            .map(|b| b.original_path.as_str())
            .collect();
        assert!(paths.contains(&"root.txt"));
        assert!(paths.contains(&"data/nested.txt"));
    }

    #[test]
    fn test_create_blobs_deduplication() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let output_dir = temp_dir.path().join("files");
        fs::create_dir(&source_dir).unwrap();

        // Create files with same content (should be deduplicated)
        let same_content = b"identical content";
        fs::write(source_dir.join("file1.txt"), same_content).unwrap();
        fs::write(source_dir.join("file2.txt"), same_content).unwrap();
        fs::write(source_dir.join("file3.txt"), b"different content").unwrap();

        let result =
            create_blobs(source_dir.to_str().unwrap(), output_dir.to_str().unwrap()).unwrap();

        // 3 files total, but only 2 unique blobs
        assert_eq!(result.blobs.len(), 3);
        assert_eq!(result.blob_count, 2);
        assert_eq!(result.deduplicated_count, 1);

        // Verify only 2 blob files exist
        let blob_files: Vec<_> = fs::read_dir(&output_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(blob_files.len(), 2);
    }

    #[test]
    fn test_create_blobs_correct_hash() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let output_dir = temp_dir.path().join("files");
        fs::create_dir(&source_dir).unwrap();

        // Create file with known content
        fs::write(source_dir.join("test.txt"), b"hello world").unwrap();

        let result =
            create_blobs(source_dir.to_str().unwrap(), output_dir.to_str().unwrap()).unwrap();

        // SHA-256 of "hello world"
        let expected_hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert_eq!(result.blobs[0].sha256, expected_hash);

        // Verify blob file exists with correct name
        assert!(output_dir.join(expected_hash).exists());
    }

    #[test]
    fn test_create_blobs_preserves_content() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let output_dir = temp_dir.path().join("files");
        fs::create_dir(&source_dir).unwrap();

        let content = b"test content for verification";
        fs::write(source_dir.join("test.txt"), content).unwrap();

        let result =
            create_blobs(source_dir.to_str().unwrap(), output_dir.to_str().unwrap()).unwrap();

        // Read blob and verify content matches
        let blob_path = output_dir.join(&result.blobs[0].sha256);
        let blob_content = fs::read(&blob_path).unwrap();
        assert_eq!(blob_content, content);
    }

    #[test]
    fn test_create_blobs_source_not_found() {
        let result = create_blobs("/nonexistent/path", "./files");
        assert!(matches!(result, Err(BlobError::SourceDirNotFound(_))));
    }

    #[test]
    fn test_create_blobs_creates_output_directory() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let output_dir = temp_dir.path().join("nested/output/files");
        fs::create_dir(&source_dir).unwrap();

        // Create a test file
        fs::write(source_dir.join("test.txt"), b"content").unwrap();

        assert!(!output_dir.exists());

        let result = create_blobs(source_dir.to_str().unwrap(), output_dir.to_str().unwrap());

        assert!(result.is_ok());
        assert!(output_dir.exists());
    }

    #[test]
    fn test_create_blobs_empty_directory() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let output_dir = temp_dir.path().join("files");
        fs::create_dir(&source_dir).unwrap();

        let result =
            create_blobs(source_dir.to_str().unwrap(), output_dir.to_str().unwrap()).unwrap();

        assert_eq!(result.blob_count, 0);
        assert_eq!(result.blobs.len(), 0);
        assert_eq!(result.total_size, 0);
    }

    #[test]
    fn test_create_blobs_total_size() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let output_dir = temp_dir.path().join("files");
        fs::create_dir(&source_dir).unwrap();

        // Create files with known sizes
        let content1 = b"1234567890"; // 10 bytes
        let content2 = b"abcdefghij"; // 10 bytes
        fs::write(source_dir.join("file1.txt"), content1).unwrap();
        fs::write(source_dir.join("file2.txt"), content2).unwrap();

        let result =
            create_blobs(source_dir.to_str().unwrap(), output_dir.to_str().unwrap()).unwrap();

        assert_eq!(result.total_size, 20);
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
    fn test_blob_info_size() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let output_dir = temp_dir.path().join("files");
        fs::create_dir(&source_dir).unwrap();

        let content = b"exact size content"; // 18 bytes
        fs::write(source_dir.join("sized.txt"), content).unwrap();

        let result =
            create_blobs(source_dir.to_str().unwrap(), output_dir.to_str().unwrap()).unwrap();

        assert_eq!(result.blobs[0].size, 18);
    }

    #[test]
    fn test_blobs_sorted_by_path() {
        let temp_dir = tempdir().unwrap();
        let source_dir = temp_dir.path().join("source");
        let output_dir = temp_dir.path().join("files");
        fs::create_dir(&source_dir).unwrap();

        // Create files in non-alphabetical order
        fs::write(source_dir.join("zzz.txt"), b"z").unwrap();
        fs::write(source_dir.join("aaa.txt"), b"a").unwrap();
        fs::write(source_dir.join("mmm.txt"), b"m").unwrap();

        let result =
            create_blobs(source_dir.to_str().unwrap(), output_dir.to_str().unwrap()).unwrap();

        let paths: Vec<_> = result
            .blobs
            .iter()
            .map(|b| b.original_path.as_str())
            .collect();
        assert_eq!(paths, vec!["aaa.txt", "mmm.txt", "zzz.txt"]);
    }
}
