// UltimaForge - Self-Hosted UO Installer/Patcher/Launcher
//
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Core modules
pub mod commands;
pub mod config;
pub mod cuo_settings;
pub mod downloader;
pub mod error;
pub mod hash;
pub mod installer;
pub mod launcher;
pub mod manifest;
pub mod migration;
pub mod signature;
pub mod state;
pub mod updater;

// Security tests module (test-only)
#[cfg(test)]
mod security_tests;

use config::{default_config_path, BrandConfig, LauncherConfig};
use state::AppState;
use tauri::Manager;
use tracing::{error, info, warn};
use tracing_subscriber;

/// Loads the brand configuration embedded in the binary at compile time.
///
/// The brand.json file is embedded directly into the executable using include_str!,
/// making the launcher completely self-contained with no external dependencies.
fn load_brand_config() -> Option<BrandConfig> {
    // Embed the brand.json file at compile time
    const BRAND_JSON: &str = include_str!("../../../branding/brand.json");

    match serde_json::from_str::<BrandConfig>(BRAND_JSON) {
        Ok(config) => {
            info!(
                "Successfully loaded embedded brand config: {}",
                config.product.display_name
            );
            Some(config)
        }
        Err(e) => {
            error!("Failed to parse embedded brand config: {}", e);
            error!("This should not happen - brand.json may be malformed");
            None
        }
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing for structured logging
    tracing_subscriber::fmt::init();

    // Create app with managed state
    let app_result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // Load embedded brand configuration
            let app_state = match load_brand_config() {
                Some(brand_config) => {
                    info!("Brand config loaded: {}", brand_config.product.display_name);

                    // Load launcher configuration from disk
                    let config_path = default_config_path(&brand_config.product.server_name);
                    let launcher_config = match LauncherConfig::load(&config_path) {
                        Ok(config) => {
                            info!(
                                "Loaded launcher config from {:?}, install_complete: {}",
                                config_path, config.install_complete
                            );
                            config
                        }
                        Err(e) => {
                            warn!("Failed to load launcher config: {}, using defaults", e);
                            LauncherConfig::new()
                        }
                    };

                    // Auto-elevate if the install path requires admin privileges
                    #[cfg(target_os = "windows")]
                    if launcher_config.requires_elevation {
                        use crate::installer::Installer;
                        if !Installer::is_running_elevated_static() {
                            info!("Install path requires elevation, relaunching as admin");
                            let exe_path =
                                std::env::current_exe().expect("Failed to get executable path");
                            let exe_str: Vec<u16> = exe_path
                                .to_string_lossy()
                                .encode_utf16()
                                .chain(std::iter::once(0))
                                .collect();
                            let verb: Vec<u16> = "runas\0".encode_utf16().collect();
                            unsafe {
                                windows::Win32::UI::Shell::ShellExecuteW(
                                    windows::Win32::Foundation::HWND::default(),
                                    windows::core::PCWSTR(verb.as_ptr()),
                                    windows::core::PCWSTR(exe_str.as_ptr()),
                                    windows::core::PCWSTR::null(),
                                    windows::core::PCWSTR::null(),
                                    windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
                                );
                            }
                            // Exit this non-elevated instance
                            std::process::exit(0);
                        }
                    }

                    // Create state and initialize with both configs
                    let state = AppState::new();
                    state.initialize(brand_config, launcher_config);
                    state
                }
                None => {
                    warn!("Starting without brand configuration - some features may not work");
                    AppState::new()
                }
            };

            // Scan running processes to detect if the game was left open when
            // the launcher was closed. sysinfo lets us set is_game_running
            // correctly before the UI loads, so the update button is disabled
            // even when the launcher restarts mid-session.
            let detected_exe: Option<String> = {
                use sysinfo::{ProcessesToUpdate, System};
                app_state
                    .launcher_config()
                    .as_ref()
                    .and_then(|c| c.client_executable.clone())
                    .and_then(|exe_name| {
                        let exe_lower = exe_name.to_lowercase();
                        let mut sys = System::new();
                        sys.refresh_processes(ProcessesToUpdate::All, false);
                        let running = sys
                            .processes()
                            .values()
                            .any(|p| p.name().to_string_lossy().to_lowercase() == exe_lower);
                        if running {
                            info!(
                                "Detected game process '{}' already running on startup",
                                exe_name
                            );
                            app_state.set_game_running(true);
                            Some(exe_lower)
                        } else {
                            None
                        }
                    })
            };

            // Store state in app using state management
            app.manage(app_state);

            // If the game was already running when the launcher started, spawn
            // a background thread to poll for process exit so the launcher
            // transitions back to Ready when the player eventually closes the
            // game — without this the GameRunning state would stick forever.
            if let Some(exe_lower) = detected_exe {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    use sysinfo::{ProcessesToUpdate, System};
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        let mut sys = System::new();
                        sys.refresh_processes(ProcessesToUpdate::All, false);
                        let still_running = sys
                            .processes()
                            .values()
                            .any(|p| p.name().to_string_lossy().to_lowercase() == exe_lower);
                        if !still_running {
                            info!(
                                "Game process '{}' exited, clearing GameRunning state",
                                exe_lower
                            );
                            handle.state::<AppState>().set_running_clients(0);
                            break;
                        }
                    }
                });
            }

            // After a self-update the NSIS installer can relaunch the app
            // minimized or hidden. Show, restore, and focus the window so
            // players don't have to find it in the taskbar.
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Legacy command
            greet,
            // Crypto commands
            commands::crypto::generate_keypair,
            // Install commands
            commands::install::check_install_status,
            commands::install::validate_install_path,
            commands::install::start_install,
            commands::install::get_app_status,
            // Update commands
            commands::update::check_for_updates,
            commands::update::start_update,
            commands::update::get_update_progress,
            commands::update::dismiss_update,
            // Launch commands
            commands::launch::launch_game,
            commands::launch::validate_client,
            commands::launch::game_closed,
            commands::launch::get_launch_options,
            commands::launch::save_launch_options,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::get_brand_config,
            commands::settings::get_cuo_config,
            commands::settings::get_launcher_dir,
            commands::settings::save_brand_config,
            commands::settings::get_theme_colors,
            commands::settings::verify_installation,
            commands::settings::clear_cache,
            commands::settings::get_repair_list,
            commands::settings::get_recommended_install_path,
            commands::settings::is_running_as_admin,
            commands::settings::relaunch_as_admin,
            commands::settings::open_install_folder,
            commands::settings::remove_game_files,
            // Migration commands
            commands::migration::scan_for_migrations,
            commands::migration::detect_at_path,
            commands::migration::start_migration,
            commands::migration::use_in_place,
            commands::migration::remove_old_installation,
        ]);

    app_result
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ============================================
// UNIT TESTS FOR CONFIG LOADING AT STARTUP
// ============================================

#[cfg(test)]
mod tests {
    use super::*;
    use config::{BrandConfig, LauncherConfig};
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Test that the embedded brand.json can be parsed at startup.
    ///
    /// This verifies the config loading that occurs in lib.rs::load_brand_config().
    /// The brand.json is embedded at compile time, so we test parsing it directly.
    #[test]
    fn test_config_loading() {
        // Test 1: Embedded brand.json should parse successfully
        const BRAND_JSON: &str = include_str!("../../../branding/brand.json");

        let brand_config = serde_json::from_str::<BrandConfig>(BRAND_JSON)
            .expect("Embedded brand.json should be valid JSON");

        // Verify required fields are present and valid
        assert!(
            !brand_config.product.display_name.is_empty(),
            "Brand config should have a display name"
        );
        assert!(
            !brand_config.product.server_name.is_empty(),
            "Brand config should have a server name"
        );
        assert!(
            !brand_config.update_url.is_empty(),
            "Brand config should have an update URL"
        );
        assert!(
            !brand_config.public_key.is_empty(),
            "Brand config should have a public key"
        );

        // Verify the config passes validation
        brand_config
            .validate()
            .expect("Brand config should pass validation");

        // Test 2: LauncherConfig defaults should be valid at startup
        let launcher_config = LauncherConfig::new();
        assert!(
            launcher_config.is_first_run(),
            "New launcher config should indicate first run"
        );
        assert!(
            launcher_config.install_path.is_none(),
            "New launcher config should have no install path"
        );
        assert!(
            !launcher_config.install_complete,
            "New launcher config should not be marked as complete"
        );

        // Test 3: LauncherConfig should load defaults for non-existent file
        let nonexistent_path = PathBuf::from("/nonexistent/path/launcher.json");
        let loaded_config = LauncherConfig::load(&nonexistent_path)
            .expect("Loading from nonexistent path should return defaults");
        assert!(
            loaded_config.is_first_run(),
            "Loaded config from nonexistent file should be first run"
        );

        // Test 4: Config path generation should work
        let config_path = default_config_path(&brand_config.product.server_name);
        assert!(
            config_path
                .to_string_lossy()
                .contains(&brand_config.product.server_name),
            "Config path should contain server name"
        );
        assert!(
            config_path.to_string_lossy().ends_with("launcher.json"),
            "Config path should end with launcher.json"
        );
    }

    /// Test that LauncherConfig can be saved and loaded correctly.
    #[test]
    fn test_config_loading_save_load_roundtrip() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("test_launcher.json");

        // Create a config with some values set
        let mut original_config = LauncherConfig::new();
        original_config.install_path = Some(PathBuf::from("/test/install/path"));
        original_config.current_version = Some("1.0.0".to_string());
        original_config.install_complete = true;

        // Save the config
        original_config
            .save(&config_path)
            .expect("Should save config successfully");

        // Load the config
        let loaded_config =
            LauncherConfig::load(&config_path).expect("Should load config successfully");

        // Verify roundtrip
        assert_eq!(
            loaded_config.install_path,
            Some(PathBuf::from("/test/install/path")),
            "Install path should match after roundtrip"
        );
        assert_eq!(
            loaded_config.current_version,
            Some("1.0.0".to_string()),
            "Current version should match after roundtrip"
        );
        assert!(
            loaded_config.install_complete,
            "Install complete should be true after roundtrip"
        );
        assert!(
            !loaded_config.is_first_run(),
            "Should not be first run after setting install_complete"
        );
    }

    /// Test that the load_brand_config function works correctly.
    #[test]
    fn test_config_loading_brand_function() {
        // Call the actual load_brand_config function
        let brand_config =
            load_brand_config().expect("load_brand_config should return a valid config");

        // Verify the loaded config has expected structure
        assert!(
            !brand_config.product.display_name.is_empty(),
            "Loaded brand config should have display name"
        );
        assert!(
            brand_config.update_url.starts_with("http"),
            "Update URL should be a valid HTTP(S) URL"
        );
        assert_eq!(
            brand_config.public_key.len(),
            64,
            "Public key should be 64 hex characters"
        );
    }
}
