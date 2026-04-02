# Migration Path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the launcher detect existing UO installations at server-owner-configured paths and migrate them (copy or adopt in-place) so players don't re-download 5-10 GB of data files.

**Architecture:** New `migration` field in `brand.json` holds exact search paths. On first run, the launcher scans those paths, presents a decision screen (copy / use-in-place / skip), then either copies files with progress or adopts the directory. Two new `AppPhase` variants (`NeedsMigration`, `Migrating`) slot between `Initializing` and `NeedsInstall`. A `requires_elevation` flag in `launcher.json` triggers auto-elevation on subsequent launches when the user chose a protected path.

**Tech Stack:** Rust (Tauri backend), React/TypeScript (frontend), serde for config, Tauri events for progress, Windows UAC via PowerShell `Start-Process -Verb RunAs`.

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `branding/brand.json` | Modify | Add `migration.searchPaths` |
| `app/src-tauri/src/config.rs` | Modify | `MigrationConfig` struct, add to `BrandConfig`; add `requires_elevation` to `LauncherConfig` |
| `app/src-tauri/src/state.rs` | Modify | Add `NeedsMigration` and `Migrating` to `AppPhase`; add migration state fields to `AppStateInner` |
| `app/src-tauri/src/migration.rs` | Create | `scan_migration_paths()`, `migrate_installation()`, `MigrationProgress` |
| `app/src-tauri/src/commands/migration.rs` | Create | Tauri commands: `scan_for_migrations`, `start_migration`, `use_in_place` |
| `app/src-tauri/src/commands/mod.rs` | Modify | Add `pub mod migration;` |
| `app/src-tauri/src/lib.rs` | Modify | Register migration commands; add `requires_elevation` auto-relaunch check at startup |
| `app/src/lib/types.ts` | Modify | Add migration-related TypeScript types |
| `app/src/lib/api.ts` | Modify | Add migration API wrappers |
| `app/src/hooks/useMigration.ts` | Create | React hook for migration state |
| `app/src/components/MigrationWizard.tsx` | Create | Migration decision UI component |
| `app/src/components/MigrationWizard.css` | Create | Styles for MigrationWizard |
| `app/src/App.tsx` | Modify | Insert `NeedsMigration`/`Migrating` phase handling |

---

### Task 1: Add `MigrationConfig` to BrandConfig

**Files:**
- Modify: `app/src-tauri/src/config.rs:300-324` (BrandConfig struct)
- Modify: `branding/brand.json`

- [ ] **Step 1: Write the failing test**

Add to the test module in `app/src-tauri/src/config.rs`:

```rust
#[test]
fn test_migration_config_parsing() {
    let json = r#"{
        "product": { "displayName": "Test", "serverName": "Test" },
        "updateUrl": "http://example.com",
        "publicKey": "2a26d57c2e53b821c554c28ea6bc3802b18a18f26eaf39e86ce3aaa9b25dc449",
        "migration": {
            "searchPaths": ["C:\\Program Files\\MyServer", "C:\\Games\\UO"]
        }
    }"#;

    let config: BrandConfig = serde_json::from_str(json).expect("Should parse");
    let migration = config.migration.expect("Should have migration config");
    assert_eq!(migration.search_paths.len(), 2);
    assert_eq!(migration.search_paths[0], "C:\\Program Files\\MyServer");
}

#[test]
fn test_migration_config_optional() {
    let json = r#"{
        "product": { "displayName": "Test", "serverName": "Test" },
        "updateUrl": "http://example.com",
        "publicKey": "2a26d57c2e53b821c554c28ea6bc3802b18a18f26eaf39e86ce3aaa9b25dc449"
    }"#;

    let config: BrandConfig = serde_json::from_str(json).expect("Should parse");
    assert!(config.migration.is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_migration_config -- --nocapture` from repo root.
Expected: FAIL — `BrandConfig` has no `migration` field.

- [ ] **Step 3: Add MigrationConfig struct and field to BrandConfig**

In `app/src-tauri/src/config.rs`, add the struct before `BrandConfig`:

```rust
/// Migration configuration for detecting existing installations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationConfig {
    /// Exact directory paths to scan for existing UO installations.
    #[serde(rename = "searchPaths", default)]
    pub search_paths: Vec<String>,
}
```

Add the field to `BrandConfig` (after `brand_version`):

```rust
    /// Optional migration configuration for detecting existing installations.
    #[serde(default)]
    pub migration: Option<MigrationConfig>,
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_migration_config -- --nocapture`
Expected: PASS

- [ ] **Step 5: Add searchPaths to brand.json**

Add to `branding/brand.json` before the closing `}`:

```json
  "migration": {
    "searchPaths": [
      "C:\\Program Files\\EA Games\\Ultima Online",
      "C:\\Program Files (x86)\\EA Games\\Ultima Online"
    ]
  }
```

- [ ] **Step 6: Run full test suite to check nothing broke**

Run: `cargo test`
Expected: All tests pass, including the existing `test_config_loading` which parses the embedded `brand.json`.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/src/config.rs branding/brand.json
git commit -m "feat(migration): add MigrationConfig to BrandConfig"
```

---

### Task 2: Add `requires_elevation` to LauncherConfig

**Files:**
- Modify: `app/src-tauri/src/config.rs:450-496` (LauncherConfig struct)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_requires_elevation_default_false() {
    let config = LauncherConfig::new();
    assert!(!config.requires_elevation);
}

#[test]
fn test_requires_elevation_roundtrip() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.json");

    let mut config = LauncherConfig::new();
    config.requires_elevation = true;
    config.save(&config_path).unwrap();

    let loaded = LauncherConfig::load(&config_path).unwrap();
    assert!(loaded.requires_elevation);
}

#[test]
fn test_requires_elevation_missing_from_json() {
    // Old configs without the field should default to false
    let json = r#"{"installComplete": true}"#;
    let config: LauncherConfig = serde_json::from_str(json).unwrap();
    assert!(!config.requires_elevation);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_requires_elevation -- --nocapture`
Expected: FAIL — no `requires_elevation` field.

- [ ] **Step 3: Add the field to LauncherConfig**

In `app/src-tauri/src/config.rs`, add to `LauncherConfig` struct (after `config_version`):

```rust
    /// Whether the install path requires admin elevation.
    /// When true, the launcher auto-relaunches as admin on startup.
    #[serde(rename = "requiresElevation", default)]
    pub requires_elevation: bool,
```

Add `requires_elevation: false,` to the `Default` impl (after `client_count: 1,`).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_requires_elevation -- --nocapture`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add app/src-tauri/src/config.rs
git commit -m "feat(migration): add requires_elevation to LauncherConfig"
```

---

### Task 3: Add `NeedsMigration` and `Migrating` to AppPhase

**Files:**
- Modify: `app/src-tauri/src/state.rs:37-79` (AppPhase enum and Display impl)

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn test_migration_phases_exist() {
    let phase = AppPhase::NeedsMigration;
    assert_eq!(format!("{}", phase), "Migration Available");

    let phase2 = AppPhase::Migrating;
    assert_eq!(format!("{}", phase2), "Migrating");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_migration_phases_exist -- --nocapture`
Expected: FAIL — variants don't exist.

- [ ] **Step 3: Add the new variants**

In `app/src-tauri/src/state.rs`, add to the `AppPhase` enum after `Initializing`:

```rust
    /// Existing installation detected, user needs to choose migration option.
    NeedsMigration,
    /// Migration (file copy) is in progress.
    Migrating,
```

Add to the `Display` impl:

```rust
            Self::NeedsMigration => write!(f, "Migration Available"),
            Self::Migrating => write!(f, "Migrating"),
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_migration_phases_exist -- --nocapture`
Expected: PASS

- [ ] **Step 5: Add migration state fields to AppStateInner**

In `app/src-tauri/src/state.rs`, add to `AppStateInner` (after `current_operation`):

```rust
    /// Migration progress: number of files copied so far.
    migration_files_copied: usize,
    /// Migration progress: total number of files to copy.
    migration_files_total: usize,
```

Add getter/setter methods to `AppState` impl:

```rust
    // === Migration State ===

    /// Gets migration progress as (copied, total).
    pub fn migration_progress(&self) -> (usize, usize) {
        let inner = self.inner.lock().unwrap();
        (inner.migration_files_copied, inner.migration_files_total)
    }

    /// Sets migration progress.
    pub fn set_migration_progress(&self, copied: usize, total: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.migration_files_copied = copied;
        inner.migration_files_total = total;
    }
```

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All pass. The new variants may cause non-exhaustive match warnings — fix any that appear in `is_operational()` or other match blocks by adding the new variants to the appropriate arms.

- [ ] **Step 7: Commit**

```bash
git add app/src-tauri/src/state.rs
git commit -m "feat(migration): add NeedsMigration and Migrating AppPhase variants"
```

---

### Task 4: Create `migration.rs` — scan and copy logic

**Files:**
- Create: `app/src-tauri/src/migration.rs`
- Modify: `app/src-tauri/src/lib.rs:7` (add `pub mod migration;`)

- [ ] **Step 1: Write the failing test for scan_migration_paths**

Create `app/src-tauri/src/migration.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

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
        let results = scan_migration_paths(&[
            "C:\\NonExistent\\Path\\12345".to_string(),
        ]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_scan_migration_paths_empty_list() {
        let results = scan_migration_paths(&[]);
        assert!(results.is_empty());
    }
}
```

- [ ] **Step 2: Add module declaration to lib.rs**

In `app/src-tauri/src/lib.rs`, add after `pub mod manifest;`:

```rust
pub mod migration;
```

- [ ] **Step 3: Run test to verify it fails**

Run: `cargo test test_scan_migration_paths -- --nocapture`
Expected: FAIL — `scan_migration_paths` not defined.

- [ ] **Step 4: Implement scan_migration_paths**

Add to top of `app/src-tauri/src/migration.rs` (above the `#[cfg(test)]` block):

```rust
use crate::installer::{detect_existing_installation, DetectionResult};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::info;

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
```

- [ ] **Step 5: Run test to verify it passes**

Run: `cargo test test_scan_migration_paths -- --nocapture`
Expected: PASS

- [ ] **Step 6: Write the failing test for migrate_installation**

Add to the tests module:

```rust
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
        let result = migrate_installation(
            src_dir.path(),
            &dst_path,
            |progress| { copied_count = progress.files_copied; },
        );

        assert!(result.is_ok());
        assert!(dst_path.join("client.exe").exists());
        assert!(dst_path.join("art.mul").exists());
        assert!(dst_path.join("sub/nested.dat").exists());
        assert_eq!(fs::read_to_string(dst_path.join("client.exe")).unwrap(), "exe_content");
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
```

- [ ] **Step 7: Run test to verify it fails**

Run: `cargo test test_migrate_installation -- --nocapture`
Expected: FAIL — `migrate_installation` not defined.

- [ ] **Step 8: Implement migrate_installation**

Add to `app/src-tauri/src/migration.rs` (after `scan_migration_paths`):

```rust
use std::fs;
use std::path::PathBuf;
use tracing::{error, warn};

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
        return Err(format!("Source directory does not exist: {}", source.display()));
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
    fs::create_dir_all(destination)
        .map_err(|e| format!("Failed to create destination {}: {}", destination.display(), e))?;

    let mut files_copied = 0;
    let result = copy_recursive(source, destination, &mut files_copied, files_total, &mut progress_cb);

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
```

- [ ] **Step 9: Run test to verify it passes**

Run: `cargo test test_migrate_installation -- --nocapture`
Expected: PASS

- [ ] **Step 10: Commit**

```bash
git add app/src-tauri/src/migration.rs app/src-tauri/src/lib.rs
git commit -m "feat(migration): add scan_migration_paths and migrate_installation"
```

---

### Task 5: Create migration Tauri commands

**Files:**
- Create: `app/src-tauri/src/commands/migration.rs`
- Modify: `app/src-tauri/src/commands/mod.rs`
- Modify: `app/src-tauri/src/lib.rs:162-197` (invoke_handler)

- [ ] **Step 1: Add module declaration**

In `app/src-tauri/src/commands/mod.rs`, add:

```rust
pub mod migration;
```

- [ ] **Step 2: Create the commands file**

Create `app/src-tauri/src/commands/migration.rs`:

```rust
//! Migration command handlers for UltimaForge.
//!
//! These commands handle detection and migration of existing UO installations.

use crate::config::{default_config_path, LauncherConfig};
use crate::installer::{detect_existing_installation, DetectionResult};
use crate::migration::{migrate_installation, scan_migration_paths, MigrationProgress};
use crate::state::{AppPhase, AppState};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Emitter, State};
use tracing::{error, info};

/// Response from scanning for migratable installations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanMigrationResponse {
    /// List of detected installations (medium+ confidence only).
    pub detected: Vec<DetectionResult>,
    /// Total number of paths scanned.
    pub paths_scanned: usize,
}

/// Request to start a file-copy migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartMigrationRequest {
    /// Source directory to copy from.
    pub source_path: String,
    /// Destination directory to copy to.
    pub destination_path: String,
}

/// Request to adopt an existing installation in-place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseInPlaceRequest {
    /// Path to the existing installation.
    pub install_path: String,
}

/// Scans brand-configured migration paths for existing installations.
///
/// Returns all detected installations with medium or high confidence.
#[tauri::command]
pub async fn scan_for_migrations(
    state: State<'_, AppState>,
) -> Result<ScanMigrationResponse, String> {
    info!("Scanning for migratable installations");

    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    let search_paths = brand_config
        .migration
        .as_ref()
        .map(|m| m.search_paths.clone())
        .unwrap_or_default();

    let paths_scanned = search_paths.len();
    let detected = scan_migration_paths(&search_paths);

    if !detected.is_empty() {
        state.set_phase(AppPhase::NeedsMigration);
    }

    Ok(ScanMigrationResponse {
        detected,
        paths_scanned,
    })
}

/// Detects an existing installation at a user-specified path.
///
/// Used for manual "browse to directory" migration from Settings.
#[tauri::command]
pub async fn detect_at_path(
    path: String,
) -> Result<DetectionResult, String> {
    info!("Detecting installation at user-specified path: {}", path);
    Ok(detect_existing_installation(&PathBuf::from(&path)))
}

/// Starts a file-copy migration from source to destination.
///
/// Copies all files, reports progress via events, and configures the launcher
/// to use the destination path.
#[tauri::command]
pub async fn start_migration(
    request: StartMigrationRequest,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    info!(
        "Starting migration: {} -> {}",
        request.source_path, request.destination_path
    );

    state.set_phase(AppPhase::Migrating);

    let source = PathBuf::from(&request.source_path);
    let destination = PathBuf::from(&request.destination_path);

    let app_handle_clone = app_handle.clone();

    // Run the copy on a blocking thread to avoid blocking the async runtime
    let dest_clone = destination.clone();
    let result = tokio::task::spawn_blocking(move || {
        migrate_installation(&source, &dest_clone, |progress| {
            let _ = app_handle_clone.emit("migration-progress", &progress);
        })
    })
    .await
    .map_err(|e| format!("Migration task panicked: {}", e))?;

    match result {
        Ok(()) => {
            // Configure launcher to use the new path
            let mut config = state.launcher_config().unwrap_or_else(LauncherConfig::new);
            config.install_path = Some(destination.clone());
            config.install_complete = true;
            // current_version stays None — updater will determine what to patch
            config.requires_elevation = false; // copied to safe location
            state.set_launcher_config(config.clone());
            state.set_install_path(destination);
            state.set_phase(AppPhase::CheckingUpdates);

            // Save config to disk
            let brand_config = state.brand_config();
            let config_path = brand_config
                .as_ref()
                .map(|b| default_config_path(&b.product.server_name))
                .unwrap_or_else(|| default_config_path("UltimaForge"));
            if let Err(e) = config.save(&config_path) {
                error!("Failed to save config after migration: {}", e);
            }

            Ok(())
        }
        Err(e) => {
            state.set_phase(AppPhase::NeedsMigration);
            Err(e)
        }
    }
}

/// Adopts an existing installation directory in-place.
///
/// If the path requires elevation, sets the `requires_elevation` flag so the
/// launcher auto-relaunches as admin on future startups.
#[tauri::command]
pub async fn use_in_place(
    request: UseInPlaceRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let path = PathBuf::from(&request.install_path);
    info!("Adopting installation in-place at: {}", path.display());

    let requires_elevation = crate::installer::Installer::path_requires_elevation_static(&path);

    let mut config = state.launcher_config().unwrap_or_else(LauncherConfig::new);
    config.install_path = Some(path.clone());
    config.install_complete = true;
    config.requires_elevation = requires_elevation;
    state.set_launcher_config(config.clone());
    state.set_install_path(path);
    state.set_phase(AppPhase::CheckingUpdates);

    // Save config to disk
    let brand_config = state.brand_config();
    let config_path = brand_config
        .as_ref()
        .map(|b| default_config_path(&b.product.server_name))
        .unwrap_or_else(|| default_config_path("UltimaForge"));
    if let Err(e) = config.save(&config_path) {
        error!("Failed to save config after use-in-place: {}", e);
    }

    Ok(())
}
```

- [ ] **Step 3: Expose `path_requires_elevation` as a static method**

The command above calls `Installer::path_requires_elevation_static`. The existing `path_requires_elevation` is a private associated function. In `app/src-tauri/src/installer.rs`, add a public static wrapper near the existing function (around line 402):

```rust
    /// Public wrapper for checking if a path requires elevation.
    /// Used by migration commands to determine if `requires_elevation` should be set.
    pub fn path_requires_elevation_static(path: &Path) -> bool {
        Self::path_requires_elevation(path)
    }
```

- [ ] **Step 4: Register commands in lib.rs**

In `app/src-tauri/src/lib.rs`, add to the `invoke_handler!` macro (after the settings commands):

```rust
            // Migration commands
            commands::migration::scan_for_migrations,
            commands::migration::detect_at_path,
            commands::migration::start_migration,
            commands::migration::use_in_place,
```

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All pass. (The commands themselves need a Tauri runtime to test end-to-end, but the underlying functions were tested in Task 4.)

- [ ] **Step 6: Commit**

```bash
git add app/src-tauri/src/commands/migration.rs app/src-tauri/src/commands/mod.rs app/src-tauri/src/lib.rs app/src-tauri/src/installer.rs
git commit -m "feat(migration): add Tauri migration commands"
```

---

### Task 6: Add auto-elevation on startup

**Files:**
- Modify: `app/src-tauri/src/lib.rs:56-100` (setup closure)

- [ ] **Step 1: Add elevation check to setup**

In `app/src-tauri/src/lib.rs`, inside the `.setup(|app| { ... })` closure, right after the `launcher_config` is loaded (after the `LauncherConfig::load` block, around line 85), add:

```rust
                    // Auto-elevate if the install path requires admin privileges
                    #[cfg(target_os = "windows")]
                    if launcher_config.requires_elevation {
                        if !installer::Installer::is_running_elevated_static() {
                            info!("Install path requires elevation, relaunching as admin");
                            use std::process::Command;
                            let exe_path = std::env::current_exe()
                                .expect("Failed to get executable path");
                            let _ = Command::new("powershell")
                                .args([
                                    "-Command",
                                    &format!(
                                        "Start-Process -FilePath '{}' -Verb RunAs",
                                        exe_path.display()
                                    ),
                                ])
                                .spawn();
                            // Exit this non-elevated instance
                            std::process::exit(0);
                        }
                    }
```

- [ ] **Step 2: Expose is_running_elevated as a static method**

In `app/src-tauri/src/installer.rs`, add near the existing `is_running_elevated` function:

```rust
    /// Public wrapper for checking if the process is running elevated.
    pub fn is_running_elevated_static() -> bool {
        Self::is_running_elevated()
    }
```

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: All pass. The elevation code is behind `#[cfg(target_os = "windows")]` so it compiles on all platforms but only runs on Windows.

- [ ] **Step 4: Commit**

```bash
git add app/src-tauri/src/lib.rs app/src-tauri/src/installer.rs
git commit -m "feat(migration): auto-elevate on startup when requires_elevation is set"
```

---

### Task 7: Add frontend TypeScript types and API wrappers

**Files:**
- Modify: `app/src/lib/types.ts`
- Modify: `app/src/lib/api.ts`

- [ ] **Step 1: Add types to types.ts**

In `app/src/lib/types.ts`, add after the `InstallStatusResponse` interface (around line 117):

```typescript
// ============================================================================
// Migration Types
// ============================================================================

/**
 * Result of detecting an existing installation at a path.
 */
export interface DetectionResult {
  /** Whether an installation was detected */
  detected: boolean;
  /** Path where installation was detected */
  install_path: string | null;
  /** Confidence level: High, Medium, Low, None */
  confidence: "High" | "Medium" | "Low" | "None";
  /** Detected version if determinable */
  detected_version: string | null;
  /** List of found executable files */
  found_executables: string[];
  /** List of found data files */
  found_data_files: string[];
  /** List of missing expected files */
  missing_files: string[];
}

/**
 * Response from scanning for migratable installations.
 */
export interface ScanMigrationResponse {
  /** Detected installations with medium+ confidence */
  detected: DetectionResult[];
  /** Number of paths that were scanned */
  paths_scanned: number;
}

/**
 * Progress information during file-copy migration.
 */
export interface MigrationProgress {
  /** Number of files copied so far */
  files_copied: number;
  /** Total number of files to copy */
  files_total: number;
  /** Current file being copied */
  current_file: string | null;
}
```

Add `NeedsMigration` and `Migrating` to the `AppPhase` type:

```typescript
export type AppPhase =
  | "Initializing"
  | "NeedsMigration"
  | "Migrating"
  | "NeedsInstall"
  // ... rest unchanged
```

Add to `TauriEvents`:

```typescript
  /** Migration progress event */
  MIGRATION_PROGRESS: "migration-progress",
```

- [ ] **Step 2: Add API wrappers to api.ts**

In `app/src/lib/api.ts`, add the imports for the new types and add this section after the Install Commands section:

```typescript
// ============================================================================
// Migration Commands
// ============================================================================

/**
 * Scans brand-configured paths for existing installations.
 */
export async function scanForMigrations(): Promise<ScanMigrationResponse> {
  return invoke<ScanMigrationResponse>("scan_for_migrations");
}

/**
 * Detects an existing installation at a user-specified path.
 */
export async function detectAtPath(path: string): Promise<DetectionResult> {
  return invoke<DetectionResult>("detect_at_path", { path });
}

/**
 * Starts a file-copy migration from source to destination.
 */
export async function startMigration(
  sourcePath: string,
  destinationPath: string
): Promise<void> {
  return invoke<void>("start_migration", {
    request: { source_path: sourcePath, destination_path: destinationPath },
  });
}

/**
 * Adopts an existing installation directory in-place.
 */
export async function useInPlace(installPath: string): Promise<void> {
  return invoke<void>("use_in_place", {
    request: { install_path: installPath },
  });
}

/**
 * Listens for migration progress events.
 */
export async function onMigrationProgress(
  callback: (progress: MigrationProgress) => void
): Promise<UnlistenFn> {
  return listen<MigrationProgress>(TauriEvents.MIGRATION_PROGRESS, (event) => {
    callback(event.payload);
  });
}
```

Add the new types to the import block at the top of `api.ts`:

```typescript
import type {
  // ... existing imports ...
  DetectionResult,
  ScanMigrationResponse,
  MigrationProgress,
} from "./types";
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd app && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add app/src/lib/types.ts app/src/lib/api.ts
git commit -m "feat(migration): add frontend types and API wrappers"
```

---

### Task 8: Create `useMigration` React hook

**Files:**
- Create: `app/src/hooks/useMigration.ts`

- [ ] **Step 1: Create the hook**

Create `app/src/hooks/useMigration.ts`:

```typescript
/**
 * Custom hook for managing the migration flow.
 *
 * Handles scanning for existing installations, presenting choices,
 * and performing file-copy migration with progress tracking.
 */

import { useState, useCallback, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import {
  scanForMigrations,
  detectAtPath,
  startMigration,
  useInPlace,
  onMigrationProgress,
  getRecommendedInstallPath,
  isRunningAsAdmin,
  relaunchAsAdmin,
  validateInstallPath,
} from "../lib/api";

import type {
  DetectionResult,
  MigrationProgress,
  PathValidationResult,
} from "../lib/types";

export type MigrationStep =
  | "scanning"
  | "decision"
  | "choose_destination"
  | "migrating"
  | "complete"
  | "not_found"
  | "error";

export interface UseMigrationState {
  /** Current step in the migration flow */
  step: MigrationStep;
  /** Detected installations from auto-scan */
  detected: DetectionResult[];
  /** The installation the user selected to migrate from */
  selectedSource: DetectionResult | null;
  /** Destination path for file copy */
  destinationPath: string;
  /** Validation result for the destination path */
  destinationValidation: PathValidationResult | null;
  /** Migration progress */
  progress: MigrationProgress | null;
  /** Error message */
  error: string | null;
  /** Whether the app is running as admin */
  isAdmin: boolean;
}

export interface UseMigrationActions {
  /** Start scanning brand-configured paths */
  scan: () => Promise<void>;
  /** Browse for an installation manually */
  browseForInstallation: () => Promise<void>;
  /** Select a detected installation as the migration source */
  selectSource: (result: DetectionResult) => void;
  /** Set the destination path for file copy */
  setDestinationPath: (path: string) => void;
  /** Navigate to a specific migration step */
  setStep: (step: MigrationStep) => void;
  /** Choose "Copy to new location" */
  copyToNewLocation: () => Promise<void>;
  /** Choose "Use in place" */
  adoptInPlace: () => Promise<void>;
  /** Choose "Skip — install fresh" */
  skip: () => void;
  /** Relaunch as admin for elevation */
  relaunchAsAdmin: () => Promise<void>;
  /** Reset to initial state */
  reset: () => void;
}

export function useMigration(
  onComplete: () => void,
  onSkip: () => void
): [UseMigrationState, UseMigrationActions] {
  const [step, setStep] = useState<MigrationStep>("scanning");
  const [detected, setDetected] = useState<DetectionResult[]>([]);
  const [selectedSource, setSelectedSource] = useState<DetectionResult | null>(null);
  const [destinationPath, setDestinationPathState] = useState<string>("");
  const [destinationValidation, setDestinationValidation] = useState<PathValidationResult | null>(null);
  const [progress, setProgress] = useState<MigrationProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isAdmin, setIsAdmin] = useState(false);

  // Check admin status on mount
  useEffect(() => {
    isRunningAsAdmin().then(setIsAdmin).catch(() => setIsAdmin(false));
  }, []);

  // Set default destination path
  useEffect(() => {
    getRecommendedInstallPath()
      .then(setDestinationPathState)
      .catch(() => setDestinationPathState("C:\\Games\\Game"));
  }, []);

  // Listen for migration progress events
  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const subscribe = async () => {
      unlisten = await onMigrationProgress((p) => {
        setProgress(p);
        if (p.files_copied === p.files_total && p.files_total > 0) {
          setStep("complete");
        }
      });
    };

    subscribe();
    return () => { if (unlisten) unlisten(); };
  }, []);

  // Validate destination when it changes
  useEffect(() => {
    if (!destinationPath) {
      setDestinationValidation(null);
      return;
    }
    validateInstallPath(destinationPath)
      .then(setDestinationValidation)
      .catch(() => setDestinationValidation(null));
  }, [destinationPath]);

  const scan = useCallback(async () => {
    setStep("scanning");
    setError(null);
    try {
      const response = await scanForMigrations();
      if (response.detected.length > 0) {
        setDetected(response.detected);
        // Auto-select the first high/medium confidence result
        setSelectedSource(response.detected[0]);
        setStep("decision");
      } else {
        setStep("not_found");
      }
    } catch (e) {
      setStep("not_found");
    }
  }, []);

  const browseForInstallation = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Existing Installation Directory",
      });

      if (selected && typeof selected === "string") {
        const result = await detectAtPath(selected);
        if (result.detected) {
          setDetected([result]);
          setSelectedSource(result);
          setStep("decision");
        } else {
          setError("No recognizable UO installation found at that location.");
          setStep("not_found");
        }
      }
    } catch (e) {
      // User cancelled
    }
  }, []);

  const selectSource = useCallback((result: DetectionResult) => {
    setSelectedSource(result);
  }, []);

  const setDestinationPath = useCallback((path: string) => {
    setDestinationPathState(path);
  }, []);

  const copyToNewLocation = useCallback(async () => {
    if (!selectedSource?.install_path || !destinationPath) return;

    setStep("migrating");
    setError(null);
    setProgress(null);

    try {
      await startMigration(selectedSource.install_path, destinationPath);
      setStep("complete");
      onComplete();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setStep("error");
    }
  }, [selectedSource, destinationPath, onComplete]);

  const adoptInPlace = useCallback(async () => {
    if (!selectedSource?.install_path) return;

    setError(null);
    try {
      await useInPlace(selectedSource.install_path);
      onComplete();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setStep("error");
    }
  }, [selectedSource, onComplete]);

  const skip = useCallback(() => {
    onSkip();
  }, [onSkip]);

  const handleRelaunchAsAdmin = useCallback(async () => {
    try {
      await relaunchAsAdmin();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to relaunch as admin");
    }
  }, []);

  const reset = useCallback(() => {
    setStep("scanning");
    setDetected([]);
    setSelectedSource(null);
    setProgress(null);
    setError(null);
  }, []);

  const state: UseMigrationState = {
    step,
    detected,
    selectedSource,
    destinationPath,
    destinationValidation,
    progress,
    error,
    isAdmin,
  };

  const actions: UseMigrationActions = {
    scan,
    browseForInstallation,
    selectSource,
    setDestinationPath,
    setStep,
    copyToNewLocation,
    adoptInPlace,
    skip,
    relaunchAsAdmin: handleRelaunchAsAdmin,
    reset,
  };

  return [state, actions];
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd app && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 3: Commit**

```bash
git add app/src/hooks/useMigration.ts
git commit -m "feat(migration): add useMigration React hook"
```

---

### Task 9: Create MigrationWizard component

**Files:**
- Create: `app/src/components/MigrationWizard.tsx`
- Create: `app/src/components/MigrationWizard.css`

- [ ] **Step 1: Create the component**

Create `app/src/components/MigrationWizard.tsx`:

```tsx
/**
 * Migration wizard for detecting and migrating existing UO installations.
 *
 * Shows detected installations and lets the user choose to copy, adopt in-place,
 * or skip to a fresh install.
 */

import { useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useMigration } from "../hooks/useMigration";
import { formatBytes } from "../lib/types";
import "./MigrationWizard.css";

interface MigrationWizardProps {
  /** Called when migration completes (copy or adopt). */
  onComplete: () => void;
  /** Called when user skips migration (proceed to fresh install). */
  onSkip: () => void;
  /** Server display name for UI text. */
  serverName: string;
}

export function MigrationWizard({ onComplete, onSkip, serverName }: MigrationWizardProps) {
  const [state, actions] = useMigration(onComplete, onSkip);

  // Auto-scan on mount
  useEffect(() => {
    actions.scan();
  }, []);

  // Scanning state
  if (state.step === "scanning") {
    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Checking for Existing Installations</h2>
          <p className="migration-subtitle">
            Looking for existing Ultima Online files on your system...
          </p>
          <div className="migration-spinner" />
        </div>
      </div>
    );
  }

  // Nothing found
  if (state.step === "not_found") {
    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>No Existing Installations Found</h2>
          <p className="migration-subtitle">
            No existing Ultima Online installations were found at the configured locations.
          </p>
          <div className="migration-actions">
            <button
              className="migration-btn migration-btn-secondary"
              onClick={actions.browseForInstallation}
            >
              Browse Manually
            </button>
            <button
              className="migration-btn migration-btn-primary"
              onClick={actions.skip}
            >
              Install Fresh
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Decision screen — user picks what to do
  if (state.step === "decision" && state.selectedSource) {
    const source = state.selectedSource;
    const isProtected = source.install_path?.toLowerCase().includes("program files") ?? false;

    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Existing Installation Found</h2>
          <div className="migration-detection-info">
            <p className="migration-path">{source.install_path}</p>
            <p className="migration-confidence">
              Confidence: <span className={`confidence-${source.confidence.toLowerCase()}`}>
                {source.confidence}
              </span>
            </p>
            <p className="migration-files">
              Found: {source.found_executables.join(", ")}
              {source.found_data_files.length > 0 &&
                ` + ${source.found_data_files.length} data files`}
            </p>
            {source.missing_files.length > 0 && (
              <p className="migration-missing">
                Missing: {source.missing_files.join(", ")}
                <span className="migration-missing-note">
                  {" "}(will be downloaded during update)
                </span>
              </p>
            )}
          </div>

          <div className="migration-options">
            <button
              className="migration-option migration-option-recommended"
              onClick={() => actions.setStep("choose_destination")}
            >
              <div className="option-header">
                <span className="option-title">Copy to New Location</span>
                <span className="option-badge">Recommended</span>
              </div>
              <p className="option-description">
                Copy files to a safe location. No admin required. Original files untouched.
              </p>
            </button>

            <button
              className="migration-option"
              onClick={actions.adoptInPlace}
            >
              <div className="option-header">
                <span className="option-title">Use in Place</span>
              </div>
              <p className="option-description">
                Use the existing directory for updates.
                {isProtected && (
                  <span className="option-warning">
                    {" "}This location requires administrator privileges for every launch.
                  </span>
                )}
              </p>
              {isProtected && !state.isAdmin && (
                <button
                  className="migration-btn migration-btn-small"
                  onClick={(e) => {
                    e.stopPropagation();
                    actions.relaunchAsAdmin();
                  }}
                >
                  Relaunch as Administrator
                </button>
              )}
            </button>

            <button
              className="migration-option migration-option-skip"
              onClick={actions.skip}
            >
              <div className="option-header">
                <span className="option-title">Skip — Install Fresh</span>
              </div>
              <p className="option-description">
                Ignore existing files and download everything new.
              </p>
            </button>
          </div>

          {state.detected.length > 1 && (
            <div className="migration-other-results">
              <p>Other installations found:</p>
              {state.detected
                .filter((d) => d.install_path !== source.install_path)
                .map((d) => (
                  <button
                    key={d.install_path}
                    className="migration-alt-source"
                    onClick={() => actions.selectSource(d)}
                  >
                    {d.install_path} ({d.confidence})
                  </button>
                ))}
            </div>
          )}
        </div>
      </div>
    );
  }

  // Choose destination for file copy
  if (state.step === "choose_destination" && state.selectedSource) {
    const destValid = state.destinationValidation;

    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Choose Destination</h2>
          <p className="migration-subtitle">
            Files will be copied from{" "}
            <strong>{state.selectedSource.install_path}</strong> to:
          </p>
          <div className="migration-dest-input">
            <input
              type="text"
              value={state.destinationPath}
              onChange={(e) => actions.setDestinationPath(e.target.value)}
              className="migration-path-input"
            />
            <button
              className="migration-btn migration-btn-secondary"
              onClick={async () => {
                const selected = await open({
                  directory: true,
                  multiple: false,
                  title: "Select Destination Directory",
                });
                if (selected && typeof selected === "string") {
                  actions.setDestinationPath(selected);
                }
              }}
            >
              Browse
            </button>
          </div>
          {destValid && !destValid.is_valid && (
            <p className="migration-error">{destValid.reason}</p>
          )}
          {destValid && destValid.requires_elevation && (
            <p className="option-warning">
              This path requires administrator privileges. Consider choosing a
              different location.
            </p>
          )}
          <p className="migration-note">
            Original files will not be modified.
          </p>
          <div className="migration-actions">
            <button
              className="migration-btn migration-btn-secondary"
              onClick={() => actions.setStep("decision")}
            >
              Back
            </button>
            <button
              className="migration-btn migration-btn-primary"
              disabled={!destValid?.is_valid}
              onClick={actions.copyToNewLocation}
            >
              Start Copy
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Migrating — file copy in progress
  if (state.step === "migrating" && state.progress) {
    const pct =
      state.progress.files_total > 0
        ? Math.round(
            (state.progress.files_copied / state.progress.files_total) * 100
          )
        : 0;

    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Migrating Files</h2>
          <p className="migration-subtitle">
            Copying files to {state.destinationPath}...
          </p>
          <div className="migration-progress-bar">
            <div
              className="migration-progress-fill"
              style={{ width: `${pct}%` }}
            />
          </div>
          <p className="migration-progress-text">
            {state.progress.files_copied} / {state.progress.files_total} files ({pct}%)
          </p>
          {state.progress.current_file && (
            <p className="migration-current-file">
              {state.progress.current_file}
            </p>
          )}
        </div>
      </div>
    );
  }

  // Complete
  if (state.step === "complete") {
    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Migration Complete</h2>
          <p className="migration-subtitle">
            Files have been copied successfully. The launcher will now check for updates.
          </p>
        </div>
      </div>
    );
  }

  // Error
  if (state.step === "error") {
    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Migration Failed</h2>
          <p className="migration-error">{state.error}</p>
          <div className="migration-actions">
            <button
              className="migration-btn migration-btn-secondary"
              onClick={actions.reset}
            >
              Try Again
            </button>
            <button
              className="migration-btn migration-btn-primary"
              onClick={actions.skip}
            >
              Install Fresh Instead
            </button>
          </div>
        </div>
      </div>
    );
  }

  return null;
}
```

- [ ] **Step 2: Create the CSS file**

Create `app/src/components/MigrationWizard.css`:

```css
.migration-wizard {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 100%;
  padding: 2rem;
}

.migration-card {
  background: var(--color-surface);
  border-radius: 12px;
  padding: 2rem;
  max-width: 600px;
  width: 100%;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.3);
}

.migration-card h2 {
  margin: 0 0 0.5rem 0;
  color: var(--color-text);
  font-size: 1.5rem;
}

.migration-subtitle {
  color: var(--color-text);
  opacity: 0.7;
  margin-bottom: 1.5rem;
}

.migration-spinner {
  width: 40px;
  height: 40px;
  border: 3px solid rgba(255, 255, 255, 0.1);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: migration-spin 0.8s linear infinite;
  margin: 2rem auto;
}

@keyframes migration-spin {
  to { transform: rotate(360deg); }
}

.migration-detection-info {
  background: rgba(0, 0, 0, 0.2);
  border-radius: 8px;
  padding: 1rem;
  margin-bottom: 1.5rem;
}

.migration-path {
  font-family: monospace;
  font-size: 0.9rem;
  color: var(--color-primary);
  margin-bottom: 0.5rem;
  word-break: break-all;
}

.migration-confidence {
  font-size: 0.85rem;
  color: var(--color-text);
  opacity: 0.8;
}

.confidence-high { color: #4caf50; }
.confidence-medium { color: #ff9800; }
.confidence-low { color: #f44336; }

.migration-files,
.migration-missing {
  font-size: 0.85rem;
  color: var(--color-text);
  opacity: 0.7;
  margin-top: 0.25rem;
}

.migration-missing-note {
  font-style: italic;
  opacity: 0.6;
}

.migration-options {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.migration-option {
  background: rgba(0, 0, 0, 0.15);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 8px;
  padding: 1rem;
  cursor: pointer;
  text-align: left;
  color: var(--color-text);
  transition: border-color 0.2s, background 0.2s;
}

.migration-option:hover {
  border-color: var(--color-primary);
  background: rgba(0, 0, 0, 0.25);
}

.migration-option-recommended {
  border-color: var(--color-primary);
}

.migration-option-skip {
  opacity: 0.7;
}

.option-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 0.25rem;
}

.option-title {
  font-weight: 600;
  font-size: 1rem;
}

.option-badge {
  background: var(--color-primary);
  color: white;
  font-size: 0.7rem;
  padding: 2px 8px;
  border-radius: 4px;
  text-transform: uppercase;
}

.option-description {
  font-size: 0.85rem;
  opacity: 0.7;
  margin: 0;
}

.option-warning {
  color: #ff9800;
  font-weight: 500;
}

.migration-actions {
  display: flex;
  gap: 0.75rem;
  margin-top: 1.5rem;
}

.migration-btn {
  padding: 0.6rem 1.2rem;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-size: 0.9rem;
  font-weight: 500;
  transition: opacity 0.2s;
}

.migration-btn:hover {
  opacity: 0.9;
}

.migration-btn-primary {
  background: var(--color-primary);
  color: white;
}

.migration-btn-secondary {
  background: rgba(255, 255, 255, 0.1);
  color: var(--color-text);
}

.migration-btn-small {
  padding: 0.3rem 0.8rem;
  font-size: 0.8rem;
  margin-top: 0.5rem;
  background: rgba(255, 152, 0, 0.2);
  color: #ff9800;
  border: 1px solid rgba(255, 152, 0, 0.3);
  border-radius: 4px;
  cursor: pointer;
}

.migration-dest-input {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 0.75rem;
}

.migration-path-input {
  flex: 1;
  padding: 0.5rem 0.75rem;
  background: rgba(0, 0, 0, 0.3);
  border: 1px solid rgba(255, 255, 255, 0.15);
  border-radius: 6px;
  color: var(--color-text);
  font-family: monospace;
  font-size: 0.85rem;
}

.migration-path-input:focus {
  outline: none;
  border-color: var(--color-primary);
}

.migration-note {
  font-size: 0.8rem;
  color: var(--color-text);
  opacity: 0.5;
  font-style: italic;
  margin-bottom: 1rem;
}

.migration-progress-bar {
  background: rgba(0, 0, 0, 0.3);
  border-radius: 6px;
  height: 8px;
  overflow: hidden;
  margin-bottom: 0.75rem;
}

.migration-progress-fill {
  background: var(--color-primary);
  height: 100%;
  border-radius: 6px;
  transition: width 0.3s ease;
}

.migration-progress-text {
  text-align: center;
  font-size: 0.9rem;
  color: var(--color-text);
}

.migration-current-file {
  text-align: center;
  font-size: 0.8rem;
  color: var(--color-text);
  opacity: 0.5;
  font-family: monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.migration-error {
  color: #f44336;
  background: rgba(244, 67, 54, 0.1);
  padding: 1rem;
  border-radius: 8px;
  margin-bottom: 1rem;
}

.migration-other-results {
  margin-top: 1.5rem;
  padding-top: 1rem;
  border-top: 1px solid rgba(255, 255, 255, 0.1);
}

.migration-other-results p {
  font-size: 0.85rem;
  opacity: 0.6;
  margin-bottom: 0.5rem;
}

.migration-alt-source {
  display: block;
  width: 100%;
  text-align: left;
  background: none;
  border: 1px solid rgba(255, 255, 255, 0.05);
  color: var(--color-text);
  opacity: 0.6;
  padding: 0.5rem;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.85rem;
  margin-bottom: 0.25rem;
}

.migration-alt-source:hover {
  opacity: 1;
  border-color: var(--color-primary);
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd app && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add app/src/components/MigrationWizard.tsx app/src/components/MigrationWizard.css
git commit -m "feat(migration): add MigrationWizard component and styles"
```

---

### Task 10: Wire MigrationWizard into App.tsx

**Files:**
- Modify: `app/src/App.tsx`

- [ ] **Step 1: Add imports**

At the top of `app/src/App.tsx`, add:

```typescript
import { MigrationWizard } from "./components/MigrationWizard";
import { scanForMigrations } from "./lib/api";
```

- [ ] **Step 2: Update the AppPhase type**

Update the local `AppPhase` type in `App.tsx` (line 19-28) to include the new phases:

```typescript
type AppPhase =
  | "Initializing"
  | "NeedsMigration"
  | "Migrating"
  | "NeedsInstall"
  | "Installing"
  | "CheckingUpdates"
  | "UpdateAvailable"
  | "Updating"
  | "Ready"
  | "GameRunning"
  | "Error";
```

- [ ] **Step 3: Add migration check to initialization**

In the `checkInstallation` async function (around line 64), modify the block that checks install status. After `const status = await checkNeedsInstall();` and before the `if (status.needs_install ...)` block, add migration scanning:

```typescript
        if (status.needs_install || !status.install_complete) {
          // Before showing install wizard, check for migratable installations
          try {
            const migrationScan = await scanForMigrations();
            if (migrationScan.detected.length > 0) {
              setPhase("NeedsMigration");
              setStatusMessage("Existing installation found");
              return;
            }
          } catch {
            // Migration scan failed — fall through to install wizard
          }

          setPhase("NeedsInstall");
          setStatusMessage("Installation required");
        } else {
```

- [ ] **Step 4: Add migration completion handlers**

Add these handlers after `handleInstallComplete`:

```typescript
  const handleMigrationComplete = async () => {
    // After migration, check for updates (updater fills in any gaps)
    setPhase("CheckingUpdates");
    setStatusMessage("Checking for updates...");
    await updateActions.checkForUpdates();
    setPhase("Ready");
    setStatusMessage("Migration complete!");
  };

  const handleMigrationSkip = () => {
    setPhase("NeedsInstall");
    setStatusMessage("Installation required");
  };
```

- [ ] **Step 5: Add MigrationWizard render block**

Add this block before the existing `NeedsInstall` check (before line 282):

```tsx
  // Show migration wizard when existing installations are found
  if (phase === "NeedsMigration" || phase === "Migrating") {
    return (
      <Layout
        phase={phase}
        statusMessage={statusMessage}
        version={appVersion}
        clientVersion={clientVersion}
        runningClients={launchState.runningClients}
      >
        <MigrationWizard
          serverName={brandInfo?.display_name || "UltimaForge"}
          onComplete={handleMigrationComplete}
          onSkip={handleMigrationSkip}
        />
      </Layout>
    );
  }
```

- [ ] **Step 6: Verify TypeScript compiles**

Run: `cd app && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 7: Commit**

```bash
git add app/src/App.tsx
git commit -m "feat(migration): wire MigrationWizard into app initialization flow"
```

---

### Task 11: Add "Find existing installation" to Settings

**Files:**
- Modify: `app/src/components/Settings.tsx`

- [ ] **Step 1: Read Settings.tsx**

Read the current Settings component to understand its structure.

- [ ] **Step 2: Add migration trigger to Settings**

Add an "Installation" section to Settings with a "Find Existing Installation" button. Import `scanForMigrations`, `detectAtPath`, and `useInPlace` from `../lib/api`. Import `open` from `@tauri-apps/plugin-dialog`.

Add state:

```typescript
const [showMigrationSearch, setShowMigrationSearch] = useState(false);
const [migrationResults, setMigrationResults] = useState<DetectionResult[]>([]);
const [migrationSearching, setMigrationSearching] = useState(false);
```

Add handlers:

```typescript
const handleFindInstallation = async () => {
  setMigrationSearching(true);
  try {
    const response = await scanForMigrations();
    setMigrationResults(response.detected);
    setShowMigrationSearch(true);
  } catch (e) {
    setMigrationResults([]);
    setShowMigrationSearch(true);
  } finally {
    setMigrationSearching(false);
  }
};

const handleBrowseInstallation = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select Existing Installation Directory",
  });
  if (selected && typeof selected === "string") {
    const result = await detectAtPath(selected);
    if (result.detected) {
      setMigrationResults([result]);
      setShowMigrationSearch(true);
    }
  }
};
```

Add to the settings UI an "Installation" section with:
- Current install path (read-only display)
- "Find Existing Installation" button that calls `handleFindInstallation`
- "Browse..." button that calls `handleBrowseInstallation`

The exact JSX depends on the existing Settings component structure — follow its patterns.

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd app && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 4: Commit**

```bash
git add app/src/components/Settings.tsx
git commit -m "feat(migration): add find existing installation to Settings"
```

---

### Task 12: Run full test suite and verify build

**Files:** None (verification only)

- [ ] **Step 1: Run Rust tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 2: Run TypeScript type check**

Run: `cd app && npx tsc --noEmit`
Expected: No errors.

- [ ] **Step 3: Run Rust format check**

Run: `cargo fmt -- --check`
Expected: No formatting issues. If any, run `cargo fmt` and commit.

- [ ] **Step 4: Verify dev build starts**

Run: `cd app && npm run dev`
Expected: Vite starts successfully on port 1420.

- [ ] **Step 5: Final commit if any formatting fixes**

```bash
git add -A
git commit -m "chore: format and cleanup migration feature"
```
