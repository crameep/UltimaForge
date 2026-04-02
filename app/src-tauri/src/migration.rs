use crate::installer::{detect_existing_installation, DetectionResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{error, info};

/// Progress information emitted during migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationProgress {
    /// Number of files copied so far.
    pub files_copied: usize,
    /// Total number of files to copy.
    pub files_total: usize,
    /// Current file being copied (relative path).
    pub current_file: Option<String>,
}

/// Scans a list of exact paths for existing UO installations.
///
/// Returns only results where a valid installation was detected
/// (medium or high confidence). Paths that don't exist or contain
/// no recognizable files are silently skipped.
pub fn scan_migration_paths(paths: &[String]) -> Vec<DetectionResult> {
    let mut results = Vec::new();

    for path_str in paths {
        let path = Path::new(path_str);
        info!("Scanning migration path: {}", path.display());

        let result = detect_existing_installation(path);
        if result.detected {
            info!(
                "Found installation at {} with {} confidence",
                path.display(),
                result.confidence
            );
            results.push(result);
        }
    }

    results
}

/// Counts all files in a directory recursively.
fn count_files(dir: &Path) -> usize {
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                count += count_files(&path);
            } else {
                count += 1;
            }
        }
    }
    count
}

/// Recursively copies files from source to destination, calling the progress
/// callback after each file.
fn copy_recursive(
    src: &Path,
    dst: &Path,
    files_copied: &mut usize,
    files_total: usize,
    progress_cb: &mut dyn FnMut(MigrationProgress),
) -> Result<(), String> {
    let entries = fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory {}: {}", src.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(&file_name);

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)
                .map_err(|e| format!("Failed to create directory {}: {}", dst_path.display(), e))?;
            copy_recursive(&src_path, &dst_path, files_copied, files_total, progress_cb)?;
        } else {
            let relative = src_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string());

            fs::copy(&src_path, &dst_path)
                .map_err(|e| format!("Failed to copy {}: {}", src_path.display(), e))?;

            *files_copied += 1;
            progress_cb(MigrationProgress {
                files_copied: *files_copied,
                files_total,
                current_file: relative,
            });
        }
    }

    Ok(())
}

/// Copies an entire directory from source to destination with progress reporting.
///
/// Creates the destination directory if it doesn't exist. On failure, attempts
/// to clean up the partial copy.
pub fn migrate_installation(
    source: &Path,
    destination: &Path,
    mut progress_cb: impl FnMut(MigrationProgress),
) -> Result<(), String> {
    if !source.exists() || !source.is_dir() {
        return Err(format!(
            "Source directory does not exist: {}",
            source.display()
        ));
    }

    let files_total = count_files(source);
    if files_total == 0 {
        return Err("Source directory is empty".to_string());
    }

    info!(
        "Starting migration: {} -> {} ({} files)",
        source.display(),
        destination.display(),
        files_total
    );

    // Create destination
    fs::create_dir_all(destination).map_err(|e| {
        format!(
            "Failed to create destination {}: {}",
            destination.display(),
            e
        )
    })?;

    let mut files_copied = 0;
    let result = copy_recursive(
        source,
        destination,
        &mut files_copied,
        files_total,
        &mut progress_cb,
    );

    if let Err(ref err) = result {
        error!("Migration failed: {}. Cleaning up partial copy.", err);
        // Best-effort cleanup
        if destination.exists() {
            let _ = fs::remove_dir_all(destination);
        }
    } else {
        info!("Migration complete: {} files copied", files_copied);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_migration_paths_finds_installation() {
        let temp_dir = TempDir::new().unwrap();
        let uo_dir = temp_dir.path().join("UO");
        fs::create_dir_all(&uo_dir).unwrap();

        // Create enough files for medium confidence
        fs::write(uo_dir.join("ClassicUO.exe"), b"fake").unwrap();
        fs::write(uo_dir.join("art.mul"), b"fake").unwrap();
        fs::write(uo_dir.join("artidx.mul"), b"fake").unwrap();
        fs::write(uo_dir.join("map0.mul"), b"fake").unwrap();

        let results = scan_migration_paths(&[uo_dir.to_string_lossy().to_string()]);
        assert_eq!(results.len(), 1);
        assert!(results[0].is_valid_installation());
    }

    #[test]
    fn test_scan_migration_paths_skips_missing() {
        let results = scan_migration_paths(&["C:\\NonExistent\\Path\\12345".to_string()]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_migration_paths_empty_list() {
        let results = scan_migration_paths(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_migrate_installation_copies_files() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();
        let dst_path = dst_dir.path().join("game");

        // Create source files
        fs::write(src_dir.path().join("client.exe"), b"exe_content").unwrap();
        fs::write(src_dir.path().join("art.mul"), b"art_content").unwrap();
        fs::create_dir_all(src_dir.path().join("sub")).unwrap();
        fs::write(src_dir.path().join("sub/nested.dat"), b"nested").unwrap();

        let mut copied_count = 0;
        let result = migrate_installation(src_dir.path(), &dst_path, |progress| {
            copied_count = progress.files_copied;
        });

        assert!(result.is_ok());
        assert!(dst_path.join("client.exe").exists());
        assert!(dst_path.join("art.mul").exists());
        assert!(dst_path.join("sub/nested.dat").exists());
        assert_eq!(
            fs::read_to_string(dst_path.join("client.exe")).unwrap(),
            "exe_content"
        );
        assert!(copied_count > 0);
    }

    #[test]
    fn test_migrate_installation_source_missing() {
        let dst_dir = TempDir::new().unwrap();
        let result = migrate_installation(
            Path::new("/nonexistent/source"),
            &dst_dir.path().join("game"),
            |_| {},
        );
        assert!(result.is_err());
    }
}
