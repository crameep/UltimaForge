//! Migration command handlers for UltimaForge.
//!
//! These commands handle detection and migration of existing UO installations.

use crate::config::{default_config_path, LauncherConfig};
use crate::installer::{detect_existing_installation, detect_with_manifest, DetectionResult, Installer};
use crate::migration::{migrate_installation, scan_migration_paths};
use crate::state::{AppPhase, AppState};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{Emitter, State};
use tracing::{error, info, warn};

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
/// Fetches the manifest from the update server for accurate file-based detection.
/// Falls back to heuristic detection if the manifest can't be fetched.
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

    // Try to fetch the manifest for accurate detection
    let manifest = match Installer::new(brand_config.clone()) {
        Ok(mut installer) => match installer.fetch_manifest().await {
            Ok(m) => {
                info!("Fetched manifest for migration detection: {} files", m.files.len());
                Some(m)
            }
            Err(e) => {
                warn!("Could not fetch manifest for migration scan, using heuristic detection: {}", e);
                None
            }
        },
        Err(e) => {
            warn!("Could not create installer for manifest fetch: {}", e);
            None
        }
    };

    let paths_scanned = search_paths.len();
    let detected = scan_migration_paths(&search_paths, manifest.as_ref());

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
/// Tries manifest-based detection first (uses cached manifest if available),
/// falls back to heuristic detection.
/// Used for manual "browse to directory" from Settings and InstallWizard.
#[tauri::command]
pub async fn detect_at_path(
    path: String,
    state: State<'_, AppState>,
) -> Result<DetectionResult, String> {
    info!("Detecting installation at user-specified path: {}", path);
    let path_buf = PathBuf::from(&path);

    // Try manifest-based detection first
    let brand_config = state.brand_config();
    if let Some(bc) = brand_config {
        if let Ok(mut installer) = Installer::new(bc) {
            if let Ok(manifest) = installer.fetch_manifest().await {
                return Ok(detect_with_manifest(&path_buf, &manifest));
            }
        }
    }

    // Fall back to heuristic detection
    Ok(detect_existing_installation(&path_buf))
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

    let requires_elevation = Installer::path_requires_elevation_static(&path);

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
