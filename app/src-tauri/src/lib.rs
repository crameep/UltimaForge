// UltimaForge - Self-Hosted UO Installer/Patcher/Launcher
//
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

// Core modules
pub mod commands;
pub mod config;
pub mod downloader;
pub mod error;
pub mod hash;
pub mod installer;
pub mod launcher;
pub mod manifest;
pub mod signature;
pub mod state;
pub mod updater;

// Security tests module (test-only)
#[cfg(test)]
mod security_tests;

use config::BrandConfig;
use state::AppState;
use std::path::PathBuf;
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
            info!("Successfully loaded embedded brand config: {}", config.product.display_name);
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
        .setup(|app| {
            // Load embedded brand configuration
            let app_state = match load_brand_config() {
                Some(brand_config) => {
                    info!("Brand config loaded: {}", brand_config.product.display_name);
                    AppState::with_brand_config(brand_config)
                }
                None => {
                    warn!("Starting without brand configuration - some features may not work");
                    AppState::new()
                }
            };

            // Store state in app using state management
            app.manage(app_state);
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
            // Settings commands
            commands::settings::get_settings,
            commands::settings::save_settings,
            commands::settings::get_brand_config,
            commands::settings::get_launcher_dir,
            commands::settings::save_brand_config,
            commands::settings::get_theme_colors,
            commands::settings::verify_installation,
            commands::settings::clear_cache,
            commands::settings::get_repair_list,
            commands::settings::get_recommended_install_path,
            commands::settings::is_running_as_admin,
            commands::settings::relaunch_as_admin,
        ]);

    app_result
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
