//! Legacy installation migration helpers.
//!
//! This module imports existing ClassicUO installs into launcher-managed state
//! without requiring users to reinstall game files.

use crate::config::{server_data_dir, BrandConfig, LauncherConfig, MigrationConfig};
use crate::installer::detect_existing_installation;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Outcome returned after a successful migration.
#[derive(Debug, Clone)]
pub struct MigrationOutcome {
    /// Imported installation path.
    pub source_path: PathBuf,
    /// Optional per-user CUO data directory.
    pub cuo_data_path: Option<PathBuf>,
    /// Files copied during migration.
    pub copied_entries: Vec<String>,
}

/// Dry-run preview of what a migration would do.
#[derive(Debug, Clone)]
pub struct MigrationPreview {
    /// Source installation path.
    pub source_path: PathBuf,
    /// Whether a valid install was detected.
    pub valid_installation: bool,
    /// Detection confidence as text.
    pub confidence: String,
    /// Detected executable candidates.
    pub found_executables: Vec<String>,
    /// Detected UO data files.
    pub found_data_files: Vec<String>,
    /// Expected files that were missing during detection.
    pub missing_files: Vec<String>,
    /// Optional per-user CUO data target directory.
    pub cuo_data_target: Option<PathBuf>,
    /// Destination entries that would be copied.
    pub entries_to_copy: Vec<String>,
}

/// Resolves an auto-detect path template from branding settings.
pub fn resolve_auto_detect_path(config: &MigrationConfig, brand_config: &BrandConfig) -> Option<PathBuf> {
    let template = config.auto_detect_path.as_ref()?.trim();
    if template.is_empty() {
        return None;
    }

    let with_server = template
        .replace("{serverName}", &brand_config.product.server_name)
        .replace("{server_name}", &brand_config.product.server_name);
    let expanded = expand_env_vars(&with_server);

    Some(PathBuf::from(expanded))
}

/// Attempts automatic migration using the brand-defined auto-detect path.
pub fn try_auto_migrate(
    brand_config: &BrandConfig,
    launcher_config: &mut LauncherConfig,
) -> Result<Option<MigrationOutcome>, String> {
    let Some(migration_config) = brand_config.migration.as_ref() else {
        return Ok(None);
    };

    if !migration_config.auto_migrate_on_first_launch {
        return Ok(None);
    }

    if launcher_config.install_complete || launcher_config.migration_completed {
        return Ok(None);
    }

    let Some(path) = resolve_auto_detect_path(migration_config, brand_config) else {
        return Ok(None);
    };

    if !path.exists() {
        return Ok(None);
    }

    let detection = detect_existing_installation(&path);
    if !detection.is_valid_installation() {
        return Ok(None);
    }

    let outcome = migrate_from_install_path(brand_config, launcher_config, &path)?;
    Ok(Some(outcome))
}

/// Performs migration from a selected legacy install path.
pub fn migrate_from_install_path(
    brand_config: &BrandConfig,
    launcher_config: &mut LauncherConfig,
    source_path: &Path,
) -> Result<MigrationOutcome, String> {
    if !source_path.is_absolute() {
        return Err("Migration path must be absolute".to_string());
    }
    if !source_path.exists() || !source_path.is_dir() {
        return Err(format!(
            "Migration path does not exist or is not a directory: {}",
            source_path.display()
        ));
    }

    let detection = detect_existing_installation(source_path);
    if !detection.is_valid_installation() {
        return Err(format!(
            "No valid installation detected at {}",
            source_path.display()
        ));
    }

    let mut copied_entries = Vec::new();
    let mut cuo_data_path = None;

    if brand_config.cuo.is_some() {
        let target_root = server_data_dir(&brand_config.product.server_name).join("cuo");
        fs::create_dir_all(&target_root).map_err(|e| {
            format!(
                "Failed to create migration directory '{}': {}",
                target_root.display(),
                e
            )
        })?;

        copy_file_if_missing(
            &source_path.join("settings.json"),
            &target_root.join("settings.json"),
            &mut copied_entries,
        )
        .map_err(|e| format!("Failed to migrate settings.json: {}", e))?;

        copy_first_existing_dir(
            &[source_path.join("Profiles"), source_path.join("Data").join("Profiles")],
            &target_root.join("Profiles"),
            &mut copied_entries,
        )
        .map_err(|e| format!("Failed to migrate profiles: {}", e))?;

        copy_first_existing_dir(
            &[source_path.join("Plugins"), source_path.join("Data").join("Plugins")],
            &target_root.join("Plugins"),
            &mut copied_entries,
        )
        .map_err(|e| format!("Failed to migrate plugins: {}", e))?;

        launcher_config.cuo_data_path = Some(target_root.clone());
        cuo_data_path = Some(target_root);
    }

    launcher_config.set_from_detection(source_path.to_path_buf());
    launcher_config.migration_completed = true;
    launcher_config.migrated_from = Some(source_path.to_path_buf());

    Ok(MigrationOutcome {
        source_path: source_path.to_path_buf(),
        cuo_data_path,
        copied_entries,
    })
}

/// Returns a dry-run preview for a selected migration source directory.
pub fn preview_migration_from_install_path(
    brand_config: &BrandConfig,
    source_path: &Path,
) -> Result<MigrationPreview, String> {
    if !source_path.is_absolute() {
        return Err("Migration path must be absolute".to_string());
    }
    if !source_path.exists() || !source_path.is_dir() {
        return Err(format!(
            "Migration path does not exist or is not a directory: {}",
            source_path.display()
        ));
    }

    let detection = detect_existing_installation(source_path);

    let mut entries_to_copy = Vec::new();
    let mut cuo_data_target = None;

    if brand_config.cuo.is_some() {
        let target_root = server_data_dir(&brand_config.product.server_name).join("cuo");
        cuo_data_target = Some(target_root.clone());

        collect_file_copy_if_missing(
            &source_path.join("settings.json"),
            &target_root.join("settings.json"),
            &mut entries_to_copy,
        );

        collect_first_existing_dir_copy(
            &[source_path.join("Profiles"), source_path.join("Data").join("Profiles")],
            &target_root.join("Profiles"),
            &mut entries_to_copy,
        )
        .map_err(|e| format!("Failed to preview profiles copy: {}", e))?;

        collect_first_existing_dir_copy(
            &[source_path.join("Plugins"), source_path.join("Data").join("Plugins")],
            &target_root.join("Plugins"),
            &mut entries_to_copy,
        )
        .map_err(|e| format!("Failed to preview plugins copy: {}", e))?;
    }

    Ok(MigrationPreview {
        source_path: source_path.to_path_buf(),
        valid_installation: detection.is_valid_installation(),
        confidence: detection.confidence.to_string(),
        found_executables: detection.found_executables,
        found_data_files: detection.found_data_files,
        missing_files: detection.missing_files,
        cuo_data_target,
        entries_to_copy,
    })
}

fn copy_file_if_missing(src: &Path, dst: &Path, copied_entries: &mut Vec<String>) -> io::Result<()> {
    if !src.exists() || !src.is_file() || dst.exists() {
        return Ok(());
    }

    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(src, dst)?;
    copied_entries.push(dst.display().to_string());
    Ok(())
}

fn collect_file_copy_if_missing(src: &Path, dst: &Path, entries_to_copy: &mut Vec<String>) {
    if src.exists() && src.is_file() && !dst.exists() {
        entries_to_copy.push(dst.display().to_string());
    }
}

fn copy_first_existing_dir(
    candidates: &[PathBuf],
    target: &Path,
    copied_entries: &mut Vec<String>,
) -> io::Result<()> {
    for candidate in candidates {
        if candidate.exists() && candidate.is_dir() {
            if !dir_contains_files_recursive(candidate)? {
                continue;
            }

            return copy_dir_contents_if_missing(candidate, target, copied_entries);
        }
    }
    Ok(())
}

fn dir_contains_files_recursive(path: &Path) -> io::Result<bool> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;

        if file_type.is_file() {
            return Ok(true);
        }

        if file_type.is_dir() && dir_contains_files_recursive(&entry.path())? {
            return Ok(true);
        }
    }

    Ok(false)
}

fn collect_first_existing_dir_copy(
    candidates: &[PathBuf],
    target: &Path,
    entries_to_copy: &mut Vec<String>,
) -> io::Result<()> {
    for candidate in candidates {
        if candidate.exists() && candidate.is_dir() {
            return collect_dir_copy_if_missing(candidate, target, entries_to_copy);
        }
    }
    Ok(())
}

fn copy_dir_contents_if_missing(src: &Path, dst: &Path, copied_entries: &mut Vec<String>) -> io::Result<()> {
    if !src.exists() || !src.is_dir() {
        return Ok(());
    }

    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            copy_dir_contents_if_missing(&src_path, &dst_path, copied_entries)?;
        } else if file_type.is_file() && !dst_path.exists() {
            if let Some(parent) = dst_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src_path, &dst_path)?;
            copied_entries.push(dst_path.display().to_string());
        }
    }

    Ok(())
}

fn collect_dir_copy_if_missing(
    src: &Path,
    dst: &Path,
    entries_to_copy: &mut Vec<String>,
) -> io::Result<()> {
    if !src.exists() || !src.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_dir_copy_if_missing(&src_path, &dst_path, entries_to_copy)?;
        } else if file_type.is_file() && !dst_path.exists() {
            entries_to_copy.push(dst_path.display().to_string());
        }
    }

    Ok(())
}

fn expand_env_vars(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '%' {
            let mut var_name = String::new();
            while let Some(next) = chars.peek() {
                if *next == '%' {
                    chars.next();
                    break;
                }
                var_name.push(*next);
                chars.next();
            }

            if var_name.is_empty() {
                output.push('%');
            } else if let Ok(value) = std::env::var(&var_name) {
                output.push_str(&value);
            } else {
                output.push('%');
                output.push_str(&var_name);
                output.push('%');
            }
        } else {
            output.push(ch);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BrandConfigBuilder;
    use tempfile::TempDir;

    const TEST_PUBLIC_KEY: &str =
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn test_brand_config() -> BrandConfig {
        let mut config = BrandConfigBuilder::new()
            .display_name("Test Server")
            .server_name("TestServer")
            .update_url("http://localhost:8080")
            .public_key(TEST_PUBLIC_KEY)
            .build()
            .unwrap();
        config.cuo = Some(crate::config::CuoConfig {
            client_version: "7.0.10.3".to_string(),
            live_server: crate::config::ServerConfig {
                label: "Live".to_string(),
                ip: "127.0.0.1".to_string(),
                port: 2593,
            },
            test_server: None,
            available_assistants: vec![crate::config::AssistantKind::Razor],
            default_assistant: crate::config::AssistantKind::Razor,
            default_server: crate::config::ServerChoice::Live,
        });
        config
    }

    #[test]
    fn test_resolve_auto_detect_path_expands_placeholders() {
        std::env::set_var("UF_MIG_TEST", "C:\\Legacy");
        let brand = test_brand_config();
        let migration = MigrationConfig {
            auto_detect_path: Some("%UF_MIG_TEST%\\{serverName}".to_string()),
            auto_migrate_on_first_launch: true,
        };

        let resolved = resolve_auto_detect_path(&migration, &brand)
            .unwrap()
            .display()
            .to_string();
        assert!(resolved.contains("Legacy"));
        assert!(resolved.contains("TestServer"));
    }

    #[test]
    fn test_migrate_from_install_path_copies_data() {
        let source = TempDir::new().unwrap();
        let brand = test_brand_config();
        let mut launcher = LauncherConfig::new();

        // Minimal detection footprint: executable + 3 data files
        std::fs::write(source.path().join("ClassicUO.exe"), b"exe").unwrap();
        std::fs::write(source.path().join("art.mul"), b"1").unwrap();
        std::fs::write(source.path().join("artidx.mul"), b"1").unwrap();
        std::fs::write(source.path().join("map0.mul"), b"1").unwrap();

        std::fs::write(source.path().join("settings.json"), b"{}\n").unwrap();
        std::fs::create_dir_all(source.path().join("Data").join("Profiles")).unwrap();
        std::fs::write(
            source
                .path()
                .join("Data")
                .join("Profiles")
                .join("profile.json"),
            b"{}",
        )
        .unwrap();

        let outcome = migrate_from_install_path(&brand, &mut launcher, source.path()).unwrap();

        assert!(launcher.migration_completed);
        assert_eq!(launcher.install_path.as_deref(), Some(source.path()));
        assert!(outcome.cuo_data_path.is_some());
        assert!(!outcome.copied_entries.is_empty());
    }

    #[test]
    fn test_preview_migration_reports_copy_targets() {
        let source = TempDir::new().unwrap();
        let brand = test_brand_config();

        std::fs::write(source.path().join("ClassicUO.exe"), b"exe").unwrap();
        std::fs::write(source.path().join("art.mul"), b"1").unwrap();
        std::fs::write(source.path().join("artidx.mul"), b"1").unwrap();
        std::fs::write(source.path().join("map0.mul"), b"1").unwrap();
        std::fs::write(source.path().join("settings.json"), b"{}\n").unwrap();

        let preview = preview_migration_from_install_path(&brand, source.path()).unwrap();
        assert!(preview.valid_installation);
        assert!(!preview.entries_to_copy.is_empty());
        assert!(preview
            .entries_to_copy
            .iter()
            .any(|p| p.to_lowercase().ends_with("settings.json")));
    }

    #[test]
    fn test_copy_first_existing_dir_skips_empty_candidate() {
        let root = TempDir::new().unwrap();
        let target = TempDir::new().unwrap();
        let mut copied = Vec::new();

        let empty = root.path().join("Profiles");
        std::fs::create_dir_all(&empty).unwrap();

        let fallback = root.path().join("Data").join("Profiles");
        std::fs::create_dir_all(&fallback).unwrap();
        std::fs::write(fallback.join("profile.json"), b"{}\n").unwrap();

        copy_first_existing_dir(&[empty, fallback], target.path(), &mut copied).unwrap();

        assert!(target.path().join("profile.json").exists());
        assert_eq!(copied.len(), 1);
    }

    #[test]
    fn test_migrate_from_install_path_uses_data_fallback_when_root_dirs_empty() {
        let source = TempDir::new().unwrap();
        let brand = test_brand_config();
        let mut launcher = LauncherConfig::new();

        // Minimal detection footprint: executable + 3 data files
        std::fs::write(source.path().join("ClassicUO.exe"), b"exe").unwrap();
        std::fs::write(source.path().join("art.mul"), b"1").unwrap();
        std::fs::write(source.path().join("artidx.mul"), b"1").unwrap();
        std::fs::write(source.path().join("map0.mul"), b"1").unwrap();

        std::fs::write(source.path().join("settings.json"), b"{}\n").unwrap();
        std::fs::create_dir_all(source.path().join("Profiles")).unwrap();
        std::fs::create_dir_all(source.path().join("Plugins")).unwrap();

        let data_profiles = source.path().join("Data").join("Profiles");
        std::fs::create_dir_all(&data_profiles).unwrap();
        std::fs::write(data_profiles.join("profile.json"), b"{}\n").unwrap();

        let data_plugins = source.path().join("Data").join("Plugins").join("Razor");
        std::fs::create_dir_all(&data_plugins).unwrap();
        std::fs::write(data_plugins.join("Razor.exe"), b"plugin").unwrap();

        let outcome = migrate_from_install_path(&brand, &mut launcher, source.path()).unwrap();
        let target_root = outcome.cuo_data_path.expect("Expected CUO data path");

        assert!(target_root.join("Profiles").join("profile.json").exists());
        assert!(
            target_root
                .join("Plugins")
                .join("Razor")
                .join("Razor.exe")
                .exists()
        );
    }
}
