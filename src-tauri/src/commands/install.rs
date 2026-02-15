//! Installation command handlers for UltimaForge.
//!
//! These commands handle the first-run installation flow:
//! - Checking installation status
//! - Validating installation paths
//! - Performing full installation

use crate::config::{BrandConfig, LauncherConfig};
use crate::installer::{InstallProgress, Installer, PathValidationResult};
use crate::state::{AppPhase, AppState, AppStatus};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;
use tracing::{error, info};

/// Response for install status check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallStatusResponse {
    /// Whether installation is required.
    pub needs_install: bool,
    /// Current install path if set.
    pub install_path: Option<PathBuf>,
    /// Current installed version if set.
    pub current_version: Option<String>,
    /// Whether installation is complete.
    pub install_complete: bool,
}

/// Request for starting installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartInstallRequest {
    /// Path to install the game to.
    pub install_path: String,
}

/// Response for installation operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
    /// Installed version if successful.
    pub version: Option<String>,
}

/// Checks the current installation status.
///
/// Returns information about whether installation is needed and the current state.
#[tauri::command]
pub async fn check_install_status(
    state: State<'_, AppState>,
) -> Result<InstallStatusResponse, String> {
    info!("Checking installation status");

    let launcher_config = state.launcher_config();

    let (install_path, current_version, install_complete) = match launcher_config {
        Some(config) => (
            config.install_path,
            config.current_version,
            config.install_complete,
        ),
        None => (None, None, false),
    };

    let needs_install = install_path.is_none() || !install_complete;

    Ok(InstallStatusResponse {
        needs_install,
        install_path,
        current_version,
        install_complete,
    })
}

/// Validates a proposed installation path.
///
/// Checks that the path is valid, writable, and has sufficient space.
#[tauri::command]
pub async fn validate_install_path(
    path: String,
    state: State<'_, AppState>,
) -> Result<PathValidationResult, String> {
    info!("Validating install path: {}", path);

    let brand_config = state.brand_config().ok_or("Brand configuration not available")?;

    let installer = Installer::new(brand_config)
        .map_err(|e| format!("Failed to create installer: {}", e))?;

    let path_buf = PathBuf::from(&path);

    // We don't know the required size yet, so validate with 0
    // The actual size check happens during installation
    let result = installer.validate_install_path(&path_buf, 0);

    Ok(result)
}

/// Starts the installation process.
///
/// Downloads all files to the specified directory and updates configuration.
#[tauri::command]
pub async fn start_install(
    request: StartInstallRequest,
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<InstallResponse, String> {
    info!("Starting installation to: {}", request.install_path);

    // Check if already installing
    if state.is_installing() {
        return Err("Installation already in progress".to_string());
    }

    // Get brand config
    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    // Mark installation as started
    state.set_installing(true);
    state.set_install_path(PathBuf::from(&request.install_path));
    state.set_current_operation("Preparing installation...");

    // Create installer
    let mut installer = Installer::new(brand_config.clone())
        .map_err(|e| {
            state.set_installing(false);
            state.set_error(format!("Failed to create installer: {}", e));
            format!("Failed to create installer: {}", e)
        })?;

    // Create a clone of app_handle for the callback
    let app_handle_clone = app_handle.clone();

    // Perform installation
    let install_path = PathBuf::from(&request.install_path);
    let result = installer
        .full_install(&install_path, move |progress| {
            // Emit progress events to frontend
            let _ = app_handle_clone.emit("install-progress", progress);
        })
        .await;

    match result {
        Ok(version) => {
            info!("Installation completed successfully: version {}", version);

            // Update state
            state.complete_installation(install_path.clone(), version.clone());

            // Update launcher config
            let mut launcher_config = state.launcher_config().unwrap_or_else(LauncherConfig::new);
            launcher_config.set_installed(install_path, &version);
            state.set_launcher_config(launcher_config);

            Ok(InstallResponse {
                success: true,
                error: None,
                version: Some(version),
            })
        }
        Err(e) => {
            error!("Installation failed: {}", e);
            state.set_installing(false);
            state.set_error(format!("Installation failed: {}", e));

            Ok(InstallResponse {
                success: false,
                error: Some(e.to_string()),
                version: None,
            })
        }
    }
}

/// Gets the current application status.
///
/// Returns a snapshot of the current application state for the frontend.
#[tauri::command]
pub async fn get_app_status(state: State<'_, AppState>) -> Result<AppStatus, String> {
    Ok(state.get_status())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_status_response_serialization() {
        let response = InstallStatusResponse {
            needs_install: true,
            install_path: Some(PathBuf::from("/game")),
            current_version: Some("1.0.0".to_string()),
            install_complete: false,
        };

        let json = serde_json::to_string(&response).expect("Should serialize");
        assert!(json.contains("needs_install"));
        assert!(json.contains("install_path"));
    }

    #[test]
    fn test_start_install_request_serialization() {
        let request = StartInstallRequest {
            install_path: "/game/uo".to_string(),
        };

        let json = serde_json::to_string(&request).expect("Should serialize");
        assert!(json.contains("/game/uo"));
    }

    #[test]
    fn test_install_response_success() {
        let response = InstallResponse {
            success: true,
            error: None,
            version: Some("1.0.0".to_string()),
        };

        assert!(response.success);
        assert!(response.error.is_none());
        assert_eq!(response.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_install_response_failure() {
        let response = InstallResponse {
            success: false,
            error: Some("Download failed".to_string()),
            version: None,
        };

        assert!(!response.success);
        assert_eq!(response.error, Some("Download failed".to_string()));
        assert!(response.version.is_none());
    }
}
