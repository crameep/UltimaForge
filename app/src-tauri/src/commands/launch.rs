//! Launch command handlers for UltimaForge.
//!
//! These commands handle game client launching:
//! - Validating the client executable
//! - Launching the game
//! - Handling launch options

use crate::launcher::{ClientLauncher, LaunchConfig, LaunchResult};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;
use tracing::{error, info, warn};

/// Request for launching the game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchGameRequest {
    /// Additional command-line arguments (optional).
    #[serde(default)]
    pub args: Vec<String>,
    /// Whether to close the launcher after launching.
    #[serde(default)]
    pub close_after_launch: Option<bool>,
}

impl Default for LaunchGameRequest {
    fn default() -> Self {
        Self {
            args: Vec::new(),
            close_after_launch: None,
        }
    }
}

/// Response for launch operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchResponse {
    /// Whether the launch was successful.
    pub success: bool,
    /// Process ID of the launched client.
    pub pid: Option<u32>,
    /// Error message if launch failed.
    pub error: Option<String>,
    /// Whether the launcher should close.
    pub should_close_launcher: bool,
}

/// Response for client validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateClientResponse {
    /// Whether the client is valid and launchable.
    pub is_valid: bool,
    /// Path to the executable.
    pub executable_path: Option<String>,
    /// Error message if invalid.
    pub error: Option<String>,
}

/// Launches the game client.
///
/// Validates the installation and launches the configured executable.
#[tauri::command]
pub async fn launch_game(
    request: Option<LaunchGameRequest>,
    state: State<'_, AppState>,
) -> Result<LaunchResponse, String> {
    let request = request.unwrap_or_default();
    info!("Launching game with {} args", request.args.len());

    // Check if game is already running
    if state.is_game_running() {
        return Err("Game is already running".to_string());
    }

    // Check if we're busy with installation or update
    if state.is_installing() {
        return Err("Cannot launch while installation is in progress".to_string());
    }

    if state.is_updating() {
        return Err("Cannot launch while update is in progress".to_string());
    }

    // Get required configuration
    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    let launcher_config = state
        .launcher_config()
        .ok_or("Launcher configuration not available")?;

    let install_path = launcher_config
        .install_path
        .clone()
        .ok_or("Installation path not set")?;

    // Get client executable from manifest or default
    let manifest = state.cached_manifest();
    let executable = manifest
        .as_ref()
        .map(|m| m.client_executable.clone())
        .unwrap_or_else(|| "client.exe".to_string());

    // Determine if we should close after launch
    let should_close = request
        .close_after_launch
        .unwrap_or(launcher_config.close_on_launch);

    // Create launcher
    let mut config = LaunchConfig::new(&executable);
    config = config.with_args(request.args);

    let launcher = ClientLauncher::with_config(&install_path, config);

    // Validate before launching
    if let Err(e) = launcher.validate() {
        error!("Client validation failed: {}", e);
        return Ok(LaunchResponse {
            success: false,
            pid: None,
            error: Some(format!("Client validation failed: {}", e)),
            should_close_launcher: false,
        });
    }

    // Mark game as running
    state.set_game_running(true);
    state.set_current_operation("Launching game...");

    // Launch the game
    match launcher.launch() {
        Ok(result) => {
            info!("Game launched successfully with PID: {:?}", result.pid);
            state.clear_current_operation();

            // If not waiting for exit, we don't know when the game closes
            // In a real implementation, we might watch the process

            Ok(LaunchResponse {
                success: result.success,
                pid: result.pid,
                error: result.error_message,
                should_close_launcher: should_close,
            })
        }
        Err(e) => {
            error!("Game launch failed: {}", e);
            state.set_game_running(false);
            state.clear_current_operation();

            Ok(LaunchResponse {
                success: false,
                pid: None,
                error: Some(e.to_string()),
                should_close_launcher: false,
            })
        }
    }
}

/// Validates that the game client can be launched.
///
/// Checks that the executable exists and appears to be valid.
#[tauri::command]
pub async fn validate_client(state: State<'_, AppState>) -> Result<ValidateClientResponse, String> {
    info!("Validating game client");

    // Get required configuration
    let launcher_config = state
        .launcher_config()
        .ok_or("Launcher configuration not available")?;

    let install_path = launcher_config
        .install_path
        .clone()
        .ok_or("Installation path not set")?;

    // Get client executable from manifest or default
    let manifest = state.cached_manifest();
    let executable = manifest
        .as_ref()
        .map(|m| m.client_executable.clone())
        .unwrap_or_else(|| "client.exe".to_string());

    let launcher = ClientLauncher::new(&install_path, &executable);
    let exe_path = launcher.executable_path();

    match launcher.validate() {
        Ok(()) => Ok(ValidateClientResponse {
            is_valid: true,
            executable_path: Some(exe_path.display().to_string()),
            error: None,
        }),
        Err(e) => {
            warn!("Client validation failed: {}", e);
            Ok(ValidateClientResponse {
                is_valid: false,
                executable_path: Some(exe_path.display().to_string()),
                error: Some(e.to_string()),
            })
        }
    }
}

/// Marks the game as no longer running.
///
/// Should be called when the game process exits or the user indicates
/// the game has closed.
#[tauri::command]
pub async fn game_closed(state: State<'_, AppState>) -> Result<(), String> {
    info!("Game marked as closed");
    state.set_game_running(false);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_game_request_default() {
        let request = LaunchGameRequest::default();
        assert!(request.args.is_empty());
        assert!(request.close_after_launch.is_none());
    }

    #[test]
    fn test_launch_game_request_with_args() {
        let request = LaunchGameRequest {
            args: vec!["--server".to_string(), "127.0.0.1".to_string()],
            close_after_launch: Some(true),
        };

        assert_eq!(request.args.len(), 2);
        assert_eq!(request.close_after_launch, Some(true));
    }

    #[test]
    fn test_launch_response_success() {
        let response = LaunchResponse {
            success: true,
            pid: Some(12345),
            error: None,
            should_close_launcher: true,
        };

        assert!(response.success);
        assert_eq!(response.pid, Some(12345));
        assert!(response.should_close_launcher);
    }

    #[test]
    fn test_launch_response_failure() {
        let response = LaunchResponse {
            success: false,
            pid: None,
            error: Some("Executable not found".to_string()),
            should_close_launcher: false,
        };

        assert!(!response.success);
        assert!(response.pid.is_none());
        assert_eq!(response.error, Some("Executable not found".to_string()));
    }

    #[test]
    fn test_validate_client_response_valid() {
        let response = ValidateClientResponse {
            is_valid: true,
            executable_path: Some("/game/client.exe".to_string()),
            error: None,
        };

        assert!(response.is_valid);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_validate_client_response_invalid() {
        let response = ValidateClientResponse {
            is_valid: false,
            executable_path: Some("/game/client.exe".to_string()),
            error: Some("Not executable".to_string()),
        };

        assert!(!response.is_valid);
        assert_eq!(response.error, Some("Not executable".to_string()));
    }

    #[test]
    fn test_launch_game_request_serialization() {
        let request = LaunchGameRequest {
            args: vec!["--test".to_string()],
            close_after_launch: Some(false),
        };

        let json = serde_json::to_string(&request).expect("Should serialize");
        let parsed: LaunchGameRequest = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(request.args, parsed.args);
        assert_eq!(request.close_after_launch, parsed.close_after_launch);
    }
}
