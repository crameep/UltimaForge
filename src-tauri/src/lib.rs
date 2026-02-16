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
use tracing::{error, info, warn};
use tracing_subscriber;

/// Finds and loads the brand configuration from branding/brand.json.
///
/// Searches in the following order:
/// 1. Relative to the executable (production)
/// 2. Relative to the current working directory (development)
/// 3. One level up from CWD (src-tauri dev mode)
/// 4. Two levels up from executable (Tauri dev bundle structure)
fn load_brand_config() -> Option<BrandConfig> {
    let brand_file = "branding/brand.json";

    let search_paths = vec![
        // Try relative to the executable first (production)
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join(brand_file))),
        // Relative to CWD (development)
        Some(PathBuf::from(brand_file)),
        // One level up from CWD (src-tauri dev mode)
        Some(PathBuf::from("../branding/brand.json")),
        // Two levels up from executable (Tauri dev bundle)
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().and_then(|p| p.parent().map(|p| p.join(brand_file)))),
    ];

    for path_opt in search_paths {
        if let Some(path) = path_opt {
            if path.exists() {
                info!("Found brand config at: {}", path.display());
                match BrandConfig::load(&path) {
                    Ok(config) => {
                        info!("Successfully loaded brand: {}", config.product.display_name);
                        return Some(config);
                    }
                    Err(e) => {
                        error!("Failed to load brand config from {}: {}", path.display(), e);
                    }
                }
            } else {
                info!("Brand config not found at: {}", path.display());
            }
        }
    }

    warn!("Brand configuration not found in any search location");
    warn!("CWD: {:?}", std::env::current_dir());
    None
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing for structured logging
    tracing_subscriber::fmt::init();

    // Load brand configuration at startup
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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .manage(app_state)
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
            commands::settings::save_brand_config,
            commands::settings::get_theme_colors,
            commands::settings::verify_installation,
            commands::settings::clear_cache,
            commands::settings::get_repair_list,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
