//! Settings command handlers for UltimaForge.
//!
//! These commands handle configuration and settings:
//! - Getting and saving launcher settings
//! - Getting brand configuration
//! - Managing user preferences
//! - Saving brand configuration for setup wizard

use crate::config::{default_config_path, BrandConfig, LauncherConfig, ThemeColors, UiConfig};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::State;
use tracing::{error, info, warn};

/// User-editable settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSettings {
    /// Auto-launch client after successful update.
    pub auto_launch: bool,
    /// Close launcher after launching game.
    pub close_on_launch: bool,
    /// Check for updates on startup.
    pub check_updates_on_startup: bool,
}

impl From<&LauncherConfig> for UserSettings {
    fn from(config: &LauncherConfig) -> Self {
        Self {
            auto_launch: config.auto_launch,
            close_on_launch: config.close_on_launch,
            check_updates_on_startup: config.check_updates_on_startup,
        }
    }
}

/// Response for getting settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetSettingsResponse {
    /// Current user settings.
    pub settings: UserSettings,
    /// Installation path (read-only for display).
    pub install_path: Option<String>,
    /// Current installed version (read-only for display).
    pub current_version: Option<String>,
    /// Whether installation is complete.
    pub install_complete: bool,
}

/// Request for saving settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveSettingsRequest {
    /// Updated user settings.
    pub settings: UserSettings,
}

/// Response for save operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveResponse {
    /// Whether the save was successful.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

// ============================================================================
// Brand Configuration Input Types (for SetupWizard)
// ============================================================================

/// Product input from frontend (camelCase).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductInput {
    /// Display name shown in the launcher UI.
    pub display_name: String,
    /// Server name for branding.
    pub server_name: String,
    /// Optional server description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Support email address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub support_email: Option<String>,
    /// Server website URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    /// Discord invite link.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord: Option<String>,
}

/// Colors input from frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorsInput {
    /// Primary brand color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary: Option<String>,
    /// Secondary/accent color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secondary: Option<String>,
    /// Background color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    /// Text color.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// UI configuration input from frontend (camelCase).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiConfigInput {
    /// Theme colors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<ColorsInput>,
    /// Whether to show patch notes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_patch_notes: Option<bool>,
    /// Window title override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_title: Option<String>,
}

/// Brand configuration input from frontend (camelCase).
///
/// This matches the TypeScript BrandConfig interface in SetupWizard.tsx.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandConfigInput {
    /// Product information.
    pub product: ProductInput,
    /// Update server URL.
    pub update_url: String,
    /// Public key for signature verification (hex encoded).
    pub public_key: String,
    /// UI configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui: Option<UiConfigInput>,
    /// Brand configuration version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brand_version: Option<String>,
}

/// Brand information for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandInfo {
    /// Display name of the server.
    pub display_name: String,
    /// Server name identifier.
    pub server_name: String,
    /// Server description.
    pub description: Option<String>,
    /// Support email address.
    pub support_email: Option<String>,
    /// Server website URL.
    pub website: Option<String>,
    /// Discord invite link.
    pub discord: Option<String>,
    /// Theme colors.
    pub colors: ThemeColors,
    /// Whether to show patch notes.
    pub show_patch_notes: bool,
    /// Window title.
    pub window_title: String,
}

impl From<&BrandConfig> for BrandInfo {
    fn from(config: &BrandConfig) -> Self {
        Self {
            display_name: config.product.display_name.clone(),
            server_name: config.product.server_name.clone(),
            description: config.product.description.clone(),
            support_email: config.product.support_email.clone(),
            website: config.product.website.clone(),
            discord: config.product.discord.clone(),
            colors: config.ui.colors.clone(),
            show_patch_notes: config.ui.show_patch_notes,
            window_title: config.window_title().to_string(),
        }
    }
}

/// Gets the current user settings.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<GetSettingsResponse, String> {
    info!("Getting user settings");

    let launcher_config = state
        .launcher_config()
        .unwrap_or_else(LauncherConfig::new);

    let settings = UserSettings::from(&launcher_config);

    Ok(GetSettingsResponse {
        settings,
        install_path: launcher_config.install_path.map(|p| p.display().to_string()),
        current_version: launcher_config.current_version,
        install_complete: launcher_config.install_complete,
    })
}

/// Saves user settings.
#[tauri::command]
pub async fn save_settings(
    request: SaveSettingsRequest,
    state: State<'_, AppState>,
) -> Result<SaveResponse, String> {
    info!("Saving user settings");

    // Get current config or create new one
    let mut config = state
        .launcher_config()
        .unwrap_or_else(LauncherConfig::new);

    // Update settings
    config.auto_launch = request.settings.auto_launch;
    config.close_on_launch = request.settings.close_on_launch;
    config.check_updates_on_startup = request.settings.check_updates_on_startup;

    // Update state
    state.set_launcher_config(config.clone());

    // Save to disk
    let brand_config = state.brand_config();
    let config_path = brand_config
        .as_ref()
        .map(|b| default_config_path(&b.product.server_name))
        .unwrap_or_else(|| default_config_path("UltimaForge"));

    match config.save(&config_path) {
        Ok(()) => {
            info!("Settings saved to {}", config_path.display());
            Ok(SaveResponse {
                success: true,
                error: None,
            })
        }
        Err(e) => {
            error!("Failed to save settings: {}", e);
            Ok(SaveResponse {
                success: false,
                error: Some(e.to_string()),
            })
        }
    }
}

/// Gets the brand configuration for display.
#[tauri::command]
pub async fn get_brand_config(state: State<'_, AppState>) -> Result<BrandInfo, String> {
    info!("Getting brand configuration");

    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    Ok(BrandInfo::from(&brand_config))
}

/// Saves brand configuration to branding/brand.json.
///
/// This command is used by the SetupWizard to save server branding configuration.
/// It writes the configuration to the `branding/brand.json` file which is read
/// at build time to customize the launcher.
#[tauri::command]
pub async fn save_brand_config(config: BrandConfigInput) -> Result<SaveResponse, String> {
    info!("Saving brand configuration for server: {}", config.product.server_name);

    let brand_path = Path::new("branding/brand.json");

    // Create branding directory if it doesn't exist
    if let Some(parent) = brand_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create branding directory: {}", e))?;
    }

    // Serialize to pretty JSON
    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize configuration: {}", e))?;

    // Write to file
    fs::write(brand_path, &json)
        .map_err(|e| format!("Failed to write configuration file: {}", e))?;

    info!("Brand configuration saved to {}", brand_path.display());

    Ok(SaveResponse {
        success: true,
        error: None,
    })
}

/// Gets the full theme colors for styling.
#[tauri::command]
pub async fn get_theme_colors(state: State<'_, AppState>) -> Result<ThemeColors, String> {
    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    Ok(brand_config.ui.colors)
}

/// Verifies the installation integrity.
///
/// Checks all installed files against their expected hashes.
#[tauri::command]
pub async fn verify_installation(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<VerifyResponse, String> {
    info!("Verifying installation integrity");

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

    // Create installer for verification
    let mut installer = crate::installer::Installer::new(brand_config)
        .map_err(|e| format!("Failed to create installer: {}", e))?;

    state.set_current_operation("Verifying files...");

    // Clone app_handle for callback
    let app_handle_clone = app_handle.clone();

    match installer
        .verify_installation(&install_path, move |progress| {
            let _ = app_handle_clone.emit("verify-progress", progress);
        })
        .await
    {
        Ok(results) => {
            state.clear_current_operation();

            let total_files = results.len();
            let valid_files = results.values().filter(|v| **v).count();
            let invalid_files: Vec<String> = results
                .iter()
                .filter(|(_, valid)| !**valid)
                .map(|(path, _)| path.clone())
                .collect();

            info!(
                "Verification complete: {}/{} files valid",
                valid_files, total_files
            );

            Ok(VerifyResponse {
                success: invalid_files.is_empty(),
                total_files,
                valid_files,
                invalid_files,
                error: None,
            })
        }
        Err(e) => {
            error!("Verification failed: {}", e);
            state.clear_current_operation();

            Ok(VerifyResponse {
                success: false,
                total_files: 0,
                valid_files: 0,
                invalid_files: Vec::new(),
                error: Some(e.to_string()),
            })
        }
    }
}

/// Response for verification operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    /// Whether all files are valid.
    pub success: bool,
    /// Total number of files checked.
    pub total_files: usize,
    /// Number of valid files.
    pub valid_files: usize,
    /// List of invalid file paths.
    pub invalid_files: Vec<String>,
    /// Error message if verification failed.
    pub error: Option<String>,
}

/// Clears cached data (manifests, etc.).
#[tauri::command]
pub async fn clear_cache(state: State<'_, AppState>) -> Result<SaveResponse, String> {
    info!("Clearing cache");

    state.clear_cached_manifest();
    state.clear_update_progress();

    Ok(SaveResponse {
        success: true,
        error: None,
    })
}

/// Gets the repair list for damaged installation.
#[tauri::command]
pub async fn get_repair_list(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    info!("Getting repair list");

    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    let launcher_config = state
        .launcher_config()
        .ok_or("Launcher configuration not available")?;

    let install_path = launcher_config
        .install_path
        .ok_or("Installation path not set")?;

    let mut installer = crate::installer::Installer::new(brand_config)
        .map_err(|e| format!("Failed to create installer: {}", e))?;

    installer
        .get_repair_list(&install_path)
        .await
        .map_err(|e| format!("Failed to get repair list: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_settings_serialization() {
        let settings = UserSettings {
            auto_launch: true,
            close_on_launch: false,
            check_updates_on_startup: true,
        };

        let json = serde_json::to_string(&settings).expect("Should serialize");
        assert!(json.contains("auto_launch"));
        assert!(json.contains("close_on_launch"));
    }

    #[test]
    fn test_user_settings_from_launcher_config() {
        let mut config = LauncherConfig::new();
        config.auto_launch = true;
        config.close_on_launch = false;
        config.check_updates_on_startup = true;

        let settings = UserSettings::from(&config);

        assert!(settings.auto_launch);
        assert!(!settings.close_on_launch);
        assert!(settings.check_updates_on_startup);
    }

    #[test]
    fn test_get_settings_response() {
        let response = GetSettingsResponse {
            settings: UserSettings {
                auto_launch: true,
                close_on_launch: true,
                check_updates_on_startup: true,
            },
            install_path: Some("/game/uo".to_string()),
            current_version: Some("1.0.0".to_string()),
            install_complete: true,
        };

        let json = serde_json::to_string(&response).expect("Should serialize");
        assert!(json.contains("/game/uo"));
        assert!(json.contains("1.0.0"));
    }

    #[test]
    fn test_save_response_success() {
        let response = SaveResponse {
            success: true,
            error: None,
        };

        assert!(response.success);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_save_response_failure() {
        let response = SaveResponse {
            success: false,
            error: Some("Permission denied".to_string()),
        };

        assert!(!response.success);
        assert_eq!(response.error, Some("Permission denied".to_string()));
    }

    #[test]
    fn test_verify_response_success() {
        let response = VerifyResponse {
            success: true,
            total_files: 100,
            valid_files: 100,
            invalid_files: Vec::new(),
            error: None,
        };

        assert!(response.success);
        assert_eq!(response.total_files, 100);
        assert_eq!(response.valid_files, 100);
        assert!(response.invalid_files.is_empty());
    }

    #[test]
    fn test_verify_response_with_invalid_files() {
        let response = VerifyResponse {
            success: false,
            total_files: 100,
            valid_files: 98,
            invalid_files: vec!["client.exe".to_string(), "data.mul".to_string()],
            error: None,
        };

        assert!(!response.success);
        assert_eq!(response.invalid_files.len(), 2);
    }

    #[test]
    fn test_brand_info_serialization() {
        let info = BrandInfo {
            display_name: "Test Server".to_string(),
            server_name: "TestServer".to_string(),
            description: Some("A test server".to_string()),
            support_email: Some("test@test.com".to_string()),
            website: Some("https://test.com".to_string()),
            discord: Some("https://discord.gg/test".to_string()),
            colors: ThemeColors::default(),
            show_patch_notes: true,
            window_title: "Test Launcher".to_string(),
        };

        let json = serde_json::to_string(&info).expect("Should serialize");
        assert!(json.contains("Test Server"));
        assert!(json.contains("TestServer"));
        assert!(json.contains("primary"));
    }
}
