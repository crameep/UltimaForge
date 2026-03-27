//! Settings command handlers for UltimaForge.
//!
//! These commands handle configuration and settings:
//! - Getting and saving launcher settings
//! - Getting brand configuration
//! - Managing user preferences
//! - Saving brand configuration for setup wizard

use crate::config::{
    default_config_path, game_path_sidecar, BrandConfig, LauncherConfig, ThemeColors,
};
use crate::migration::{
    migrate_from_install_path, preview_migration_from_install_path, resolve_auto_detect_path,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Emitter;
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

/// Request payload for manual legacy migration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrateLegacyRequest {
    /// Source installation directory selected by the user.
    pub source_path: String,
}

/// Response for migration status and operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResponse {
    /// Whether the operation succeeded.
    pub success: bool,
    /// Optional error message.
    pub error: Option<String>,
    /// Whether migration previously completed.
    pub migration_completed: bool,
    /// Migrated source path, if any.
    pub migrated_from: Option<String>,
    /// Current install path after migration.
    pub install_path: Option<String>,
    /// Optional per-user CUO data path.
    pub cuo_data_path: Option<String>,
    /// Auto-detect migration path resolved from branding.
    pub auto_detect_path: Option<String>,
    /// Whether auto-migrate is enabled in branding.
    pub auto_migrate_on_first_launch: bool,
    /// Files copied while migrating mutable data.
    pub copied_entries: Vec<String>,
}

/// Response for migration dry-run previews.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPreviewResponse {
    /// Whether preview generation succeeded.
    pub success: bool,
    /// Optional error message.
    pub error: Option<String>,
    /// Source path being previewed.
    pub source_path: Option<String>,
    /// Whether the source looks like a valid installation.
    pub valid_installation: bool,
    /// Detection confidence label.
    pub confidence: Option<String>,
    /// Detected executables.
    pub found_executables: Vec<String>,
    /// Detected data files.
    pub found_data_files: Vec<String>,
    /// Missing expected files.
    pub missing_files: Vec<String>,
    /// Destination CUO data path, if applicable.
    pub cuo_data_target: Option<String>,
    /// Destination file paths that would be copied.
    pub entries_to_copy: Vec<String>,
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

/// Migration configuration input from frontend (camelCase).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationConfigInput {
    /// Optional path template to scan on first launch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_detect_path: Option<String>,
    /// Whether to auto-migrate once on first launch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_migrate_on_first_launch: Option<bool>,
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
    /// Optional migration configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migration: Option<MigrationConfigInput>,
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
    /// Background image URL/path.
    pub background_image: Option<String>,
    /// Logo image URL/path.
    pub logo_url: Option<String>,
    /// Sidebar background texture URL/path.
    pub sidebar_background: Option<String>,
    /// Whether to show patch notes.
    pub show_patch_notes: bool,
    /// Window title.
    pub window_title: String,
    /// Main hero title text.
    pub hero_title: Option<String>,
    /// Hero subtitle text.
    pub hero_subtitle: Option<String>,
    /// Sidebar subtitle text.
    pub sidebar_subtitle: Option<String>,
    /// Custom sidebar navigation links.
    pub sidebar_links: Option<Vec<crate::config::SidebarLink>>,
}

impl From<&BrandConfig> for BrandInfo {
    fn from(config: &BrandConfig) -> Self {
        let background_image = config
            .ui
            .background_image
            .clone()
            .or_else(|| Some("/branding/hero-bg.png".to_string()));
        let logo_url = config
            .ui
            .logo_url
            .clone()
            .or_else(|| Some("/branding/sidebar-logo.png".to_string()));
        let sidebar_background = config
            .ui
            .sidebar_background
            .clone()
            .or_else(|| Some("/branding/sidebar-texture.png".to_string()));

        Self {
            display_name: config.product.display_name.clone(),
            server_name: config.product.server_name.clone(),
            description: config.product.description.clone(),
            support_email: config.product.support_email.clone(),
            website: config.product.website.clone(),
            discord: config.product.discord.clone(),
            colors: config.ui.colors.clone(),
            background_image,
            logo_url,
            sidebar_background,
            show_patch_notes: config.ui.show_patch_notes,
            window_title: config.window_title().to_string(),
            hero_title: config.ui.hero_title.clone(),
            hero_subtitle: config.ui.hero_subtitle.clone(),
            sidebar_subtitle: config.ui.sidebar_subtitle.clone(),
            sidebar_links: config.ui.sidebar_links.clone(),
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

/// Returns current migration status and branding auto-migrate settings.
#[tauri::command]
pub async fn get_migration_status(state: State<'_, AppState>) -> Result<MigrationResponse, String> {
    let launcher_config = state.launcher_config().unwrap_or_else(LauncherConfig::new);
    let brand_config = state.brand_config();

    let (auto_detect_path, auto_migrate_on_first_launch) = if let Some(brand) = brand_config.as_ref()
    {
        if let Some(migration) = brand.migration.as_ref() {
            (
                resolve_auto_detect_path(migration, brand).map(|p| p.display().to_string()),
                migration.auto_migrate_on_first_launch,
            )
        } else {
            (None, false)
        }
    } else {
        (None, false)
    };

    Ok(MigrationResponse {
        success: true,
        error: None,
        migration_completed: launcher_config.migration_completed,
        migrated_from: launcher_config
            .migrated_from
            .as_ref()
            .map(|p| p.display().to_string()),
        install_path: launcher_config
            .install_path
            .as_ref()
            .map(|p| p.display().to_string()),
        cuo_data_path: launcher_config
            .cuo_data_path
            .as_ref()
            .map(|p| p.display().to_string()),
        auto_detect_path,
        auto_migrate_on_first_launch,
        copied_entries: Vec::new(),
    })
}

/// Manually migrates a legacy installation selected by the user.
#[tauri::command]
pub async fn migrate_legacy_install(
    request: MigrateLegacyRequest,
    state: State<'_, AppState>,
) -> Result<MigrationResponse, String> {
    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    let source_path = PathBuf::from(request.source_path.trim());
    let mut launcher_config = state.launcher_config().unwrap_or_else(LauncherConfig::new);

    let outcome = match migrate_from_install_path(&brand_config, &mut launcher_config, &source_path) {
        Ok(outcome) => outcome,
        Err(e) => {
            return Ok(MigrationResponse {
                success: false,
                error: Some(e),
                migration_completed: launcher_config.migration_completed,
                migrated_from: launcher_config
                    .migrated_from
                    .as_ref()
                    .map(|p| p.display().to_string()),
                install_path: launcher_config
                    .install_path
                    .as_ref()
                    .map(|p| p.display().to_string()),
                cuo_data_path: launcher_config
                    .cuo_data_path
                    .as_ref()
                    .map(|p| p.display().to_string()),
                auto_detect_path: None,
                auto_migrate_on_first_launch: false,
                copied_entries: Vec::new(),
            });
        }
    };

    let config_path = default_config_path(&brand_config.product.server_name);
    if let Err(e) = launcher_config.save(&config_path) {
        warn!("Failed to save launcher config after migration: {}", e);
        return Ok(MigrationResponse {
            success: false,
            error: Some(format!("Migration completed but saving config failed: {}", e)),
            migration_completed: launcher_config.migration_completed,
            migrated_from: launcher_config
                .migrated_from
                .as_ref()
                .map(|p| p.display().to_string()),
            install_path: launcher_config
                .install_path
                .as_ref()
                .map(|p| p.display().to_string()),
            cuo_data_path: launcher_config
                .cuo_data_path
                .as_ref()
                .map(|p| p.display().to_string()),
            auto_detect_path: None,
            auto_migrate_on_first_launch: false,
            copied_entries: outcome.copied_entries,
        });
    }

    let sidecar = game_path_sidecar(&brand_config.product.server_name);
    if let Err(e) = fs::write(&sidecar, outcome.source_path.to_string_lossy().as_bytes()) {
        warn!("Failed to write game_path.txt sidecar after migration: {}", e);
    }

    state.set_launcher_config(launcher_config.clone());
    state.set_install_path(outcome.source_path.clone());
    state.set_phase(crate::state::AppPhase::CheckingUpdates);

    Ok(MigrationResponse {
        success: true,
        error: None,
        migration_completed: launcher_config.migration_completed,
        migrated_from: launcher_config
            .migrated_from
            .as_ref()
            .map(|p| p.display().to_string()),
        install_path: launcher_config
            .install_path
            .as_ref()
            .map(|p| p.display().to_string()),
        cuo_data_path: outcome.cuo_data_path.map(|p| p.display().to_string()),
        auto_detect_path: None,
        auto_migrate_on_first_launch: false,
        copied_entries: outcome.copied_entries,
    })
}

/// Generates a dry-run preview for a legacy migration source directory.
#[tauri::command]
pub async fn preview_legacy_migration(
    request: MigrateLegacyRequest,
    state: State<'_, AppState>,
) -> Result<MigrationPreviewResponse, String> {
    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    let source_path = PathBuf::from(request.source_path.trim());

    let preview = match preview_migration_from_install_path(&brand_config, &source_path) {
        Ok(preview) => preview,
        Err(e) => {
            return Ok(MigrationPreviewResponse {
                success: false,
                error: Some(e),
                source_path: Some(source_path.display().to_string()),
                valid_installation: false,
                confidence: None,
                found_executables: Vec::new(),
                found_data_files: Vec::new(),
                missing_files: Vec::new(),
                cuo_data_target: None,
                entries_to_copy: Vec::new(),
            });
        }
    };

    Ok(MigrationPreviewResponse {
        success: true,
        error: None,
        source_path: Some(preview.source_path.display().to_string()),
        valid_installation: preview.valid_installation,
        confidence: Some(preview.confidence),
        found_executables: preview.found_executables,
        found_data_files: preview.found_data_files,
        missing_files: preview.missing_files,
        cuo_data_target: preview.cuo_data_target.map(|p| p.display().to_string()),
        entries_to_copy: preview.entries_to_copy,
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
///
/// Images are served from the embedded dist/branding/ folder via the webview,
/// so paths like "/branding/image.png" work directly in both dev and production.
#[tauri::command]
pub async fn get_brand_config(state: State<'_, AppState>) -> Result<BrandInfo, String> {
    info!("Getting brand configuration");

    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    // Images are already embedded in dist/branding/ and served by the webview
    // No path conversion needed - just return the config as-is
    Ok(BrandInfo::from(&brand_config))
}

/// Returns the CUO config block from brand.json so the frontend can
/// build the server and assistant dropdowns.
#[tauri::command]
pub async fn get_cuo_config(
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    let brand = state.brand_config().ok_or("Brand config not available")?;
    match &brand.cuo {
        Some(cuo) => Ok(Some(serde_json::to_value(cuo).map_err(|e| e.to_string())?)),
        None => Ok(None),
    }
}

/// Gets the launcher's installation directory.
///
/// Returns the directory where the launcher executable is installed.
/// Used for defaulting the game install path to {launcher_dir}\{server_name}
#[tauri::command]
pub async fn get_launcher_dir() -> Result<String, String> {
    match std::env::current_exe() {
        Ok(exe_path) => {
            if let Some(dir) = exe_path.parent() {
                Ok(dir.display().to_string())
            } else {
                Err("Could not get launcher directory".to_string())
            }
        }
        Err(e) => Err(format!("Failed to get executable path: {}", e)),
    }
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

/// Checks if the application is currently running with administrator privileges.
///
/// Returns true if running as admin, false otherwise.
#[tauri::command]
pub fn is_running_as_admin() -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    {
        use std::mem;
        use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

        unsafe {
            let mut token = std::mem::zeroed();

            // Open process token
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
                return Ok(false);
            }

            let mut elevation: TOKEN_ELEVATION = mem::zeroed();
            let mut size = 0u32;

            // Get token elevation information
            if GetTokenInformation(
                token,
                TokenElevation,
                Some(&mut elevation as *mut _ as *mut _),
                mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            ).is_err() {
                return Ok(false);
            }

            Ok(elevation.TokenIsElevated != 0)
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(false)
    }
}

/// Gets a recommended installation path in the user's AppData directory.
///
/// Returns a path like: C:\Users\{User}\AppData\Local\{ServerName}
#[tauri::command]
pub async fn get_recommended_install_path(state: State<'_, AppState>) -> Result<String, String> {
    let brand_config = state
        .brand_config()
        .ok_or("Brand configuration not available")?;

    let server_name = &brand_config.product.server_name;

    // Get AppData\Local directory
    let local_app_data = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("APPDATA"))
        .map_err(|_| "Could not determine AppData directory".to_string())?;

    let recommended_path = format!("{}\\{}", local_app_data, server_name);

    Ok(recommended_path)
}

/// Relaunches the application with administrator privileges.
///
/// On Windows, this uses the "runas" verb to request UAC elevation.
/// The current instance will exit after launching the elevated instance.
///
/// Note: This only works in production builds. In dev mode, it will return an error
/// instructing the user to manually restart in admin mode.
#[tauri::command]
pub async fn relaunch_as_admin() -> Result<(), String> {
    info!("Relaunching application with administrator privileges");

    #[cfg(target_os = "windows")]
    {
        use std::process::Command;

        // Get the current executable path
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get executable path: {}", e))?;

        // Check if we're running in dev mode (executable is in target/debug or target/release without being installed)
        let exe_path_str = exe_path.to_string_lossy();
        let is_dev_mode = exe_path_str.contains("target\\debug") || exe_path_str.contains("target\\release");

        if is_dev_mode {
            return Err("Elevation requires a production build. Please close this window, right-click the launcher shortcut or executable, select 'Run as administrator', and try again.".to_string());
        }

        // Use Windows shell to launch with elevation
        let _status = Command::new("powershell")
            .args([
                "-Command",
                &format!(
                    "Start-Process -FilePath '{}' -Verb RunAs",
                    exe_path.display()
                ),
            ])
            .spawn()
            .map_err(|e| format!("Failed to relaunch with elevation: {}", e))?;

        info!("Elevated instance launched, exiting current instance");

        // Exit the current instance
        std::process::exit(0);
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Elevation is only supported on Windows".to_string())
    }
}

/// Opens the game installation folder in the system file manager.
#[tauri::command]
pub async fn open_install_folder(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.launcher_config().unwrap_or_else(LauncherConfig::new);
    let path = config.install_path.ok_or("No install path configured")?;

    if !path.exists() {
        return Err(format!("Install folder does not exist: {}", path.display()));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }

    Ok(())
}

/// Removes all game files from the installation directory and resets installation state.
///
/// Refuses to operate on dangerous paths (drive roots, Windows system dirs).
/// Clears install_path, install_complete, and current_version from the persisted config.
#[tauri::command]
pub async fn remove_game_files(state: State<'_, AppState>) -> Result<SaveResponse, String> {
    let config = state.launcher_config().unwrap_or_else(LauncherConfig::new);
    let path = config.install_path.clone().ok_or("No install path configured")?;

    // Safety: refuse dangerous paths
    let path_lower = path.to_string_lossy().to_lowercase();
    let dangerous_patterns = [
        "c:\\windows", "c:/windows",
        "c:\\program files", "c:/program files",
        "c:\\program files (x86)", "c:/program files (x86)",
        "c:\\system", "c:/system",
    ];
    let is_drive_root = path_lower.len() <= 3
        && path_lower.chars().nth(1) == Some(':');
    if path_lower == "/" || is_drive_root {
        return Err(format!("Refusing to remove root/drive path: {}", path.display()));
    }
    for pattern in &dangerous_patterns {
        if path_lower.starts_with(pattern) {
            return Err(format!("Refusing to remove system path: {}", path.display()));
        }
    }

    if path.exists() {
        info!("Removing game files at {}", path.display());
        fs::remove_dir_all(&path)
            .map_err(|e| format!("Failed to remove game files: {}", e))?;
        info!("Game files removed successfully");
    } else {
        info!("Install path {} does not exist, clearing config only", path.display());
    }

    // Clear install fields from persisted config
    let mut updated_config = config;
    updated_config.install_path = None;
    updated_config.install_complete = false;
    updated_config.current_version = None;
    state.set_launcher_config(updated_config.clone());

    // Reset in-memory phase to NeedsInstall
    state.clear_installation();

    // Save updated config to disk
    let brand_config = state.brand_config();
    let config_path = brand_config
        .as_ref()
        .map(|b| default_config_path(&b.product.server_name))
        .unwrap_or_else(|| default_config_path("UltimaForge"));

    // Remove game_path.txt sidecar (used by NSIS uninstaller)
    let server_name = brand_config
        .as_ref()
        .map(|b| b.product.server_name.as_str())
        .unwrap_or("UltimaForge");
    let sidecar = game_path_sidecar(server_name);
    if sidecar.exists() {
        let _ = fs::remove_file(&sidecar);
    }

    match updated_config.save(&config_path) {
        Ok(()) => {
            info!("Config saved after game file removal");
            Ok(SaveResponse { success: true, error: None })
        }
        Err(e) => {
            error!("Failed to save config after game file removal: {}", e);
            Ok(SaveResponse { success: false, error: Some(e.to_string()) })
        }
    }
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
            background_image: None,
            logo_url: None,
            sidebar_background: None,
            show_patch_notes: true,
            window_title: "Test Launcher".to_string(),
            hero_title: None,
            hero_subtitle: None,
            sidebar_subtitle: None,
            sidebar_links: None,
        };

        let json = serde_json::to_string(&info).expect("Should serialize");
        assert!(json.contains("Test Server"));
        assert!(json.contains("TestServer"));
        assert!(json.contains("primary"));
    }
}
