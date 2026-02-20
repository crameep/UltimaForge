//! Update command handlers for UltimaForge.
//!
//! These commands handle checking for and applying updates:
//! - Checking for available updates
//! - Starting update process
//! - Getting update progress

use crate::config::{default_config_path, LauncherConfig};
use crate::state::AppState;
use crate::updater::{UpdateProgress, Updater};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};
use tracing::{error, info, warn};

/// Response for update check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResponse {
    /// Whether an update is available.
    pub update_available: bool,
    /// Current installed version.
    pub current_version: Option<String>,
    /// Server version available.
    pub server_version: Option<String>,
    /// Number of files that need updating.
    pub files_to_update: usize,
    /// Total download size in bytes.
    pub download_size: u64,
    /// Human-readable download size.
    pub download_size_formatted: String,
    /// URL to patch notes if available.
    pub patch_notes_url: Option<String>,
    /// Error message if check failed.
    pub error: Option<String>,
}

/// Response for update operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// New version if update was successful.
    pub new_version: Option<String>,
    /// Whether a rollback occurred.
    pub rolled_back: bool,
}

/// Checks for available updates.
///
/// Downloads and verifies the manifest from the update server, then
/// compares it against the current installation.
#[tauri::command]
pub async fn check_for_updates(state: State<'_, AppState>) -> Result<UpdateCheckResponse, String> {
    info!("Checking for updates");

    // Get required configuration
    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    let launcher_config = state.launcher_config();
    let install_path = launcher_config
        .as_ref()
        .and_then(|c| c.install_path.clone())
        .ok_or("Installation path not set")?;

    let current_version = launcher_config
        .as_ref()
        .and_then(|c| c.current_version.clone());

    // Update state
    state.begin_update_check();

    // Create updater and check for updates
    let updater = Updater::new(install_path, brand_config).map_err(|e| {
        state.set_error(format!("Failed to create updater: {}", e));
        format!("Failed to create updater: {}", e)
    })?;

    match updater
        .check_for_updates(current_version.as_deref())
        .await
    {
        Ok(result) => {
            info!(
                "Update check complete: {} files to update ({} bytes)",
                result.files_to_update, result.download_size
            );

            // Clone values before any partial moves
            let server_version = result.server_version.clone();
            let download_size_formatted = result.download_size_formatted();

            // Update state with result
            state.set_update_available(
                result.update_available,
                Some(server_version.clone()),
                result.files_to_update,
                result.download_size,
            );

            // Reset phase appropriately (Ready if no update, UpdateAvailable if update found)
            state.end_update_check();

            // Persist client_executable so the launch command works after a restart
            // even when the update server is unreachable (cached manifest is lost).
            {
                let mut lconfig = state.launcher_config().unwrap_or_else(LauncherConfig::new);
                lconfig.client_executable = Some(result.client_executable.clone());
                state.set_launcher_config(lconfig.clone());
                let config_path = state.brand_config()
                    .map(|b| default_config_path(&b.product.server_name))
                    .unwrap_or_else(|| default_config_path("UltimaForge"));
                if let Err(e) = lconfig.save(&config_path) {
                    warn!("Failed to persist client_executable to config: {}", e);
                }
            }

            Ok(UpdateCheckResponse {
                update_available: result.update_available,
                current_version: result.current_version,
                server_version: Some(server_version),
                files_to_update: result.files_to_update,
                download_size: result.download_size,
                download_size_formatted,
                patch_notes_url: result.patch_notes_url,
                error: None,
            })
        }
        Err(e) => {
            warn!("Update check failed: {}", e);

            // Reset phase appropriately (clears current operation and sets phase to Ready)
            state.end_update_check();

            // Check if this is an "already up to date" situation
            if e.to_string().contains("already up to date") {
                Ok(UpdateCheckResponse {
                    update_available: false,
                    current_version,
                    server_version: None,
                    files_to_update: 0,
                    download_size: 0,
                    download_size_formatted: "0 bytes".to_string(),
                    patch_notes_url: None,
                    error: None,
                })
            } else {
                Ok(UpdateCheckResponse {
                    update_available: false,
                    current_version,
                    server_version: None,
                    files_to_update: 0,
                    download_size: 0,
                    download_size_formatted: "0 bytes".to_string(),
                    patch_notes_url: None,
                    error: Some(e.to_string()),
                })
            }
        }
    }
}

/// Starts the update process.
///
/// Downloads and applies all pending updates with atomic application
/// and rollback support.
#[tauri::command]
pub async fn start_update(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<UpdateResponse, String> {
    info!("Starting update process");

    // Check if already updating
    if state.is_updating() {
        return Err("Update already in progress".to_string());
    }

    // Check if installing
    if state.is_installing() {
        return Err("Cannot update while installation is in progress".to_string());
    }

    // Get required configuration
    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    let launcher_config = state.launcher_config();
    let install_path = launcher_config
        .as_ref()
        .and_then(|c| c.install_path.clone())
        .ok_or("Installation path not set")?;

    // Mark update as started
    state.set_updating(true);
    state.set_current_operation("Preparing update...");

    // Create updater
    let mut updater = Updater::new(install_path.clone(), brand_config).map_err(|e| {
        state.set_updating(false);
        state.set_error(format!("Failed to create updater: {}", e));
        format!("Failed to create updater: {}", e)
    })?;

    // Clone app_handle for progress callback
    let app_handle_clone = app_handle.clone();

    // Perform update
    let result = updater
        .perform_update(move |progress| {
            // Update state for polling
            let state = app_handle_clone.state::<AppState>();
            state.set_update_progress(progress.clone());

            // Emit progress events to frontend
            let _ = app_handle_clone.emit("update-progress", progress);
        })
        .await;

    match result {
        Ok(new_version) => {
            info!("Update completed successfully: version {}", new_version);

            // Update state
            state.complete_update(new_version.clone());

            // Update launcher config
            if let Some(mut config) = state.launcher_config() {
                config.set_version(&new_version);
                state.set_launcher_config(config.clone());

                // Save config to disk
                let brand_config = state.brand_config();
                let config_path = brand_config
                    .as_ref()
                    .map(|b| default_config_path(&b.product.server_name))
                    .unwrap_or_else(|| default_config_path("UltimaForge"));

                match config.save(&config_path) {
                    Ok(()) => {
                        info!("Saved updated version to config: {}", config_path.display());
                    }
                    Err(e) => {
                        warn!("Failed to save updated version to config: {}", e);
                    }
                }
            }

            Ok(UpdateResponse {
                success: true,
                error: None,
                new_version: Some(new_version),
                rolled_back: false,
            })
        }
        Err(e) => {
            error!("Update failed: {}", e);

            // Check if rollback occurred
            let rolled_back = e.to_string().contains("rolled back")
                || e.to_string().contains("Rolled back");

            state.set_updating(false);

            if rolled_back {
                // Rollback was successful, installation is still valid
                warn!("Update failed but rollback succeeded");
                state.clear_error();
            } else {
                state.set_error(format!("Update failed: {}", e));
            }

            Ok(UpdateResponse {
                success: false,
                error: Some(e.to_string()),
                new_version: None,
                rolled_back,
            })
        }
    }
}

/// Gets the current update progress.
///
/// Returns the cached update progress from the state.
#[tauri::command]
pub async fn get_update_progress(
    state: State<'_, AppState>,
) -> Result<Option<UpdateProgress>, String> {
    Ok(state.update_progress())
}

/// Dismisses the update notification without applying.
///
/// User can still apply the update later.
#[tauri::command]
pub async fn dismiss_update(state: State<'_, AppState>) -> Result<(), String> {
    info!("Update dismissed by user");
    // Don't clear update_available - just return to ready state
    state.set_phase(crate::state::AppPhase::Ready);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_check_response_serialization() {
        let response = UpdateCheckResponse {
            update_available: true,
            current_version: Some("1.0.0".to_string()),
            server_version: Some("1.1.0".to_string()),
            files_to_update: 5,
            download_size: 1024 * 1024,
            download_size_formatted: "1.00 MB".to_string(),
            patch_notes_url: Some("https://example.com/notes".to_string()),
            error: None,
        };

        let json = serde_json::to_string(&response).expect("Should serialize");
        assert!(json.contains("update_available"));
        assert!(json.contains("1.1.0"));
        assert!(json.contains("1.00 MB"));
    }

    #[test]
    fn test_update_response_success() {
        let response = UpdateResponse {
            success: true,
            error: None,
            new_version: Some("2.0.0".to_string()),
            rolled_back: false,
        };

        assert!(response.success);
        assert!(!response.rolled_back);
        assert_eq!(response.new_version, Some("2.0.0".to_string()));
    }

    #[test]
    fn test_update_response_failure_with_rollback() {
        let response = UpdateResponse {
            success: false,
            error: Some("Failed to apply files".to_string()),
            new_version: None,
            rolled_back: true,
        };

        assert!(!response.success);
        assert!(response.rolled_back);
        assert!(response.new_version.is_none());
    }

    #[test]
    fn test_update_check_response_no_update() {
        let response = UpdateCheckResponse {
            update_available: false,
            current_version: Some("1.0.0".to_string()),
            server_version: Some("1.0.0".to_string()),
            files_to_update: 0,
            download_size: 0,
            download_size_formatted: "0 bytes".to_string(),
            patch_notes_url: None,
            error: None,
        };

        assert!(!response.update_available);
        assert_eq!(response.files_to_update, 0);
    }
}
