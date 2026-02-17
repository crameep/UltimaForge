//! Tauri command handlers for UltimaForge.
//!
//! This module contains all the IPC command handlers that the frontend can invoke.
//! Commands are organized into submodules by functionality:
//!
//! - [`crypto`] - Cryptographic commands (keypair generation)
//! - [`install`] - First-run installation commands
//! - [`update`] - Update checking and application commands
//! - [`launch`] - Game launching commands
//! - [`settings`] - Configuration and settings commands
//!
//! # Usage
//!
//! Register all commands in the Tauri builder:
//!
//! ```ignore
//! tauri::Builder::default()
//!     .invoke_handler(tauri::generate_handler![
//!         commands::crypto::generate_keypair,
//!         commands::install::check_install_status,
//!         commands::install::start_install,
//!         commands::install::validate_install_path,
//!         commands::update::check_for_updates,
//!         commands::update::start_update,
//!         commands::launch::launch_game,
//!         commands::launch::validate_client,
//!         commands::settings::get_settings,
//!         commands::settings::save_settings,
//!         commands::settings::get_brand_config,
//!     ])
//! ```

pub mod crypto;
pub mod install;
pub mod launch;
pub mod settings;
pub mod update;

// Re-export all commands for convenient access
pub use crypto::*;
pub use install::*;
pub use launch::*;
pub use settings::*;
pub use update::*;
