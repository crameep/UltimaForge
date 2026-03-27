//! Launch command handlers for UltimaForge.
//!
//! These commands handle game client launching:
//! - Validating the client executable
//! - Launching the game
//! - Handling launch options

use crate::config::{default_config_path, AssistantKind, ServerChoice};
use crate::cuo_settings::{resolve_uo_data_directory, write_cuo_settings};
use crate::launcher::{ClientLauncher, LaunchConfig};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::time::{sleep, Duration};
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
    /// Number of client instances to launch (1-3).
    #[serde(default = "default_client_count")]
    pub client_count: u8,
    /// Which server to connect to.
    #[serde(default)]
    pub server_choice: ServerChoice,
    /// Which assistant to use.
    #[serde(default)]
    pub assistant_choice: AssistantKind,
}

fn default_client_count() -> u8 {
    1
}

impl Default for LaunchGameRequest {
    fn default() -> Self {
        Self {
            args: Vec::new(),
            close_after_launch: None,
            client_count: 1,
            server_choice: ServerChoice::Live,
            assistant_choice: AssistantKind::RazorEnhanced,
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
    /// Number of client instances currently running.
    pub running_clients: usize,
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
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<LaunchResponse, String> {
    let request = request.unwrap_or_default();
    info!("Launching game with {} args", request.args.len());

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

    let mut launcher_config = state
        .launcher_config()
        .ok_or("Launcher configuration not available")?;

    let install_path = launcher_config
        .install_path
        .clone()
        .ok_or("Installation path not set")?;

    // Get client executable: prefer cached manifest, then persisted config, then fallback
    let manifest = state.cached_manifest();
    let executable = manifest
        .as_ref()
        .map(|m| m.client_executable.clone())
        .or_else(|| launcher_config.client_executable.clone())
        .unwrap_or_else(|| "client.exe".to_string());

    // Re-verify game running state before blocking. The in-memory flag can be
    // stale (e.g. process monitor missed an exit, or the game crashed without
    // a clean shutdown). Try to open the exe for write — Windows locks running
    // executables, so success means the game is NOT actually running.
    if state.is_game_running() {
        let exe_path = install_path.join(&executable);
        let actually_running = if exe_path.exists() {
            std::fs::OpenOptions::new()
                .write(true)
                .create(false)
                .open(&exe_path)
                .is_err() // locked = still running
        } else {
            false
        };

        if actually_running {
            return Err("Game is already running".to_string());
        }

        // Stale flag — game has closed since the flag was set. Clear it.
        info!("Clearing stale is_game_running flag before launch");
        state.set_running_clients(0);
    }

    // Clamp client count to valid range
    let client_count = request.client_count.clamp(1, 3) as usize;

    // Create launcher
    let mut launch_args = request.args.clone();
    if let Some(cuo_data_root) = launcher_config.cuo_data_path.as_ref() {
        let uo_data_path = resolve_uo_data_directory(&install_path);
        let settings_path = cuo_data_root.join("settings.json");
        let profiles_path = cuo_data_root.join("Profiles");
        launch_args.push("-settings".to_string());
        launch_args.push(settings_path.display().to_string());
        launch_args.push("-profilespath".to_string());
        launch_args.push(profiles_path.display().to_string());
        launch_args.push("-uopath".to_string());
        launch_args.push(uo_data_path.display().to_string());
    }

    let mut config = LaunchConfig::new(&executable);
    config = config.with_args(launch_args);

    let launcher = ClientLauncher::with_config(&install_path, config.clone());

    // Validate before launching
    if let Err(e) = launcher.validate() {
        error!("Client validation failed: {}", e);
        return Ok(LaunchResponse {
            success: false,
            pid: None,
            error: Some(format!("Client validation failed: {}", e)),
            should_close_launcher: false,
            running_clients: 0,
        });
    }

    state.set_current_operation("Launching game...");

    launcher_config.selected_server = request.server_choice.clone();
    launcher_config.selected_assistant = request.assistant_choice.clone();
    launcher_config.client_count = request.client_count.clamp(1, 3);
    state.set_launcher_config(launcher_config.clone());
    let config_path = default_config_path(&brand_config.product.server_name);
    if let Err(e) = launcher_config.save(&config_path) {
        warn!("Failed to save launcher config: {}", e);
    }

    if let Some(cuo_config) = &brand_config.cuo {
        if let Err(e) = write_cuo_settings(
            &install_path,
            launcher_config.cuo_data_path.as_deref(),
            cuo_config,
            &request.server_choice,
            &request.assistant_choice,
        ) {
            warn!("Failed to write CUO settings: {}", e);
        }
    }

    state.set_running_clients(0);

    let mut first_pid = None;
    let mut launched = 0usize;

    for i in 0..client_count {
        let launcher = ClientLauncher::with_config(&install_path, config.clone());
        match launcher.spawn_child() {
            Ok(mut child) => {
                let pid = child.id();
                if first_pid.is_none() {
                    first_pid = Some(pid);
                }
                launched += 1;
                state.increment_running_clients();
                info!("Client {} launched (PID {})", i + 1, pid);

                let handle = app_handle.clone();
                std::thread::spawn(move || {
                    let _ = child.wait();
                    let app_state = handle.state::<AppState>();
                    let remaining = app_state.decrement_running_clients();
                    let _ = handle.emit("client-count-changed", remaining);
                });
            }
            Err(e) => {
                warn!("Client {} error: {}", i + 1, e);
            }
        }

        if i + 1 < client_count {
            sleep(Duration::from_millis(300)).await;
        }
    }

    state.clear_current_operation();

    if launched == 0 {
        state.set_running_clients(0);
        return Ok(LaunchResponse {
            success: false,
            pid: None,
            error: Some("No client instances launched successfully".into()),
            should_close_launcher: false,
            running_clients: 0,
        });
    }

    let should_close = client_count == 1
        && request
            .close_after_launch
            .unwrap_or(launcher_config.close_on_launch);

    Ok(LaunchResponse {
        success: true,
        pid: first_pid,
        error: None,
        should_close_launcher: should_close,
        running_clients: launched,
    })
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

    // Get client executable: prefer cached manifest, then persisted config, then fallback
    let manifest = state.cached_manifest();
    let executable = manifest
        .as_ref()
        .map(|m| m.client_executable.clone())
        .or_else(|| launcher_config.client_executable.clone())
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
    state.set_running_clients(0);
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
        assert_eq!(request.client_count, 1);
        assert_eq!(request.server_choice, ServerChoice::Live);
        assert_eq!(request.assistant_choice, AssistantKind::RazorEnhanced);
    }

    #[test]
    fn test_launch_game_request_with_args() {
        let request = LaunchGameRequest {
            args: vec!["--server".to_string(), "127.0.0.1".to_string()],
            close_after_launch: Some(true),
            client_count: 2,
            server_choice: ServerChoice::Test,
            assistant_choice: AssistantKind::Razor,
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
            running_clients: 1,
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
            running_clients: 0,
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
            client_count: 1,
            server_choice: ServerChoice::Live,
            assistant_choice: AssistantKind::RazorEnhanced,
        };

        let json = serde_json::to_string(&request).expect("Should serialize");
        let parsed: LaunchGameRequest = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(request.args, parsed.args);
        assert_eq!(request.close_after_launch, parsed.close_after_launch);
    }
}
