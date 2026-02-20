//! Application state management for UltimaForge.
//!
//! This module provides thread-safe application state that is shared across
//! all Tauri commands. The state tracks installation status, update progress,
//! and other runtime information.
//!
//! # Thread Safety
//!
//! All state is wrapped in `Arc<Mutex<>>` or `Arc<RwLock<>>` for safe concurrent
//! access from multiple command handlers and background tasks.
//!
//! # Usage with Tauri
//!
//! ```ignore
//! use tauri::Manager;
//! use ultimaforge_lib::state::AppState;
//!
//! fn main() {
//!     tauri::Builder::default()
//!         .setup(|app| {
//!             app.manage(AppState::new());
//!             Ok(())
//!         })
//!         .run(tauri::generate_context!())
//!         .expect("error while running application");
//! }
//! ```

use crate::config::{BrandConfig, LauncherConfig};
use crate::manifest::Manifest;
use crate::updater::UpdateProgress;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, RwLock};

/// Current phase of the application lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppPhase {
    /// Application is initializing.
    Initializing,
    /// First-run installation required.
    NeedsInstall,
    /// Installation is in progress.
    Installing,
    /// Checking for updates.
    CheckingUpdates,
    /// Update is available.
    UpdateAvailable,
    /// Update is in progress.
    Updating,
    /// Ready to launch the game.
    Ready,
    /// Game is currently running.
    GameRunning,
    /// An error occurred.
    Error,
}

impl Default for AppPhase {
    fn default() -> Self {
        Self::Initializing
    }
}

impl std::fmt::Display for AppPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Initializing => write!(f, "Initializing"),
            Self::NeedsInstall => write!(f, "Installation Required"),
            Self::Installing => write!(f, "Installing"),
            Self::CheckingUpdates => write!(f, "Checking for Updates"),
            Self::UpdateAvailable => write!(f, "Update Available"),
            Self::Updating => write!(f, "Updating"),
            Self::Ready => write!(f, "Ready"),
            Self::GameRunning => write!(f, "Game Running"),
            Self::Error => write!(f, "Error"),
        }
    }
}

/// Inner state data that requires locking for mutation.
#[derive(Debug, Default)]
struct AppStateInner {
    /// Current application phase.
    phase: AppPhase,
    /// Path to the UO client installation directory.
    install_path: Option<PathBuf>,
    /// Current installed version (matches manifest version).
    current_version: Option<String>,
    /// Whether an update is available.
    update_available: bool,
    /// Version available on the server (if update available).
    available_version: Option<String>,
    /// Number of files that need updating.
    files_to_update: usize,
    /// Total download size for the update (bytes).
    update_download_size: u64,
    /// Whether an installation is currently in progress.
    is_installing: bool,
    /// Whether an update is currently in progress.
    is_updating: bool,
    /// Whether the game client is currently running.
    is_game_running: bool,
    /// Number of game client instances currently running.
    running_clients: usize,
    /// Current error message (if any).
    error_message: Option<String>,
    /// Update progress information.
    update_progress: Option<UpdateProgress>,
    /// Installation progress percentage (0-100).
    install_progress: f64,
    /// Current operation description for UI.
    current_operation: Option<String>,
}

/// Thread-safe application state for the launcher.
///
/// This struct is managed by Tauri and shared across all command handlers.
/// It tracks the current state of installation, updates, and game launching.
///
/// # Example
///
/// ```ignore
/// use ultimaforge_lib::state::AppState;
///
/// // In a Tauri command handler:
/// #[tauri::command]
/// async fn get_status(state: tauri::State<'_, AppState>) -> Result<AppStatus, String> {
///     Ok(state.get_status())
/// }
/// ```
pub struct AppState {
    /// Inner mutable state protected by a mutex.
    inner: Mutex<AppStateInner>,
    /// Brand configuration (immutable after initialization).
    brand_config: RwLock<Option<BrandConfig>>,
    /// Launcher configuration (mutable for settings changes).
    launcher_config: RwLock<Option<LauncherConfig>>,
    /// Cached manifest from last update check.
    cached_manifest: RwLock<Option<Manifest>>,
}

impl AppState {
    /// Creates a new application state instance.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(AppStateInner::default()),
            brand_config: RwLock::new(None),
            launcher_config: RwLock::new(None),
            cached_manifest: RwLock::new(None),
        }
    }

    /// Creates a new application state with the given brand configuration.
    pub fn with_brand_config(brand_config: BrandConfig) -> Self {
        Self {
            inner: Mutex::new(AppStateInner::default()),
            brand_config: RwLock::new(Some(brand_config)),
            launcher_config: RwLock::new(None),
            cached_manifest: RwLock::new(None),
        }
    }

    /// Initializes the application state with configurations.
    ///
    /// This should be called during app setup to load the brand and launcher
    /// configurations.
    pub fn initialize(
        &self,
        brand_config: BrandConfig,
        launcher_config: LauncherConfig,
    ) {
        {
            let mut brand = self.brand_config.write().unwrap();
            *brand = Some(brand_config);
        }

        {
            let mut launcher = self.launcher_config.write().unwrap();
            let is_first_run = launcher_config.is_first_run();
            let install_path = launcher_config.install_path.clone();
            let current_version = launcher_config.current_version.clone();
            *launcher = Some(launcher_config);

            // Update inner state based on launcher config
            let mut inner = self.inner.lock().unwrap();
            inner.install_path = install_path;
            inner.current_version = current_version;
            inner.phase = if is_first_run {
                AppPhase::NeedsInstall
            } else {
                AppPhase::CheckingUpdates
            };
        }
    }

    // === Phase Management ===

    /// Returns the current application phase.
    pub fn phase(&self) -> AppPhase {
        self.inner.lock().unwrap().phase.clone()
    }

    /// Sets the application phase.
    pub fn set_phase(&self, phase: AppPhase) {
        self.inner.lock().unwrap().phase = phase;
    }

    /// Returns true if the application is in an operational state.
    pub fn is_operational(&self) -> bool {
        matches!(
            self.phase(),
            AppPhase::Ready | AppPhase::GameRunning | AppPhase::CheckingUpdates
        )
    }

    // === Installation State ===

    /// Returns the current install path, if set.
    pub fn install_path(&self) -> Option<PathBuf> {
        self.inner.lock().unwrap().install_path.clone()
    }

    /// Sets the installation path.
    pub fn set_install_path(&self, path: PathBuf) {
        self.inner.lock().unwrap().install_path = Some(path);
    }

    /// Returns the current installed version.
    pub fn current_version(&self) -> Option<String> {
        self.inner.lock().unwrap().current_version.clone()
    }

    /// Sets the current installed version.
    pub fn set_current_version(&self, version: String) {
        self.inner.lock().unwrap().current_version = Some(version);
    }

    /// Returns true if an installation is in progress.
    pub fn is_installing(&self) -> bool {
        self.inner.lock().unwrap().is_installing
    }

    /// Sets whether an installation is in progress.
    pub fn set_installing(&self, installing: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.is_installing = installing;
        inner.phase = if installing {
            AppPhase::Installing
        } else if inner.install_path.is_some() && inner.current_version.is_some() {
            AppPhase::Ready
        } else {
            AppPhase::NeedsInstall
        };
    }

    /// Gets the installation progress percentage.
    pub fn install_progress(&self) -> f64 {
        self.inner.lock().unwrap().install_progress
    }

    /// Sets the installation progress percentage.
    pub fn set_install_progress(&self, progress: f64) {
        self.inner.lock().unwrap().install_progress = progress.clamp(0.0, 100.0);
    }

    // === Update State ===

    /// Returns true if an update is available.
    pub fn update_available(&self) -> bool {
        self.inner.lock().unwrap().update_available
    }

    /// Sets whether an update is available.
    pub fn set_update_available(
        &self,
        available: bool,
        version: Option<String>,
        files: usize,
        download_size: u64,
    ) {
        let mut inner = self.inner.lock().unwrap();
        inner.update_available = available;
        inner.available_version = version;
        inner.files_to_update = files;
        inner.update_download_size = download_size;
        if available && !inner.is_updating && !inner.is_installing {
            inner.phase = AppPhase::UpdateAvailable;
        }
    }

    /// Returns the available update version.
    pub fn available_version(&self) -> Option<String> {
        self.inner.lock().unwrap().available_version.clone()
    }

    /// Returns the number of files that need updating.
    pub fn files_to_update(&self) -> usize {
        self.inner.lock().unwrap().files_to_update
    }

    /// Returns the total download size for the update.
    pub fn update_download_size(&self) -> u64 {
        self.inner.lock().unwrap().update_download_size
    }

    /// Returns true if an update is in progress.
    pub fn is_updating(&self) -> bool {
        self.inner.lock().unwrap().is_updating
    }

    /// Sets whether an update is in progress.
    pub fn set_updating(&self, updating: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.is_updating = updating;
        if updating {
            inner.phase = AppPhase::Updating;
        } else if inner.update_available {
            inner.phase = AppPhase::UpdateAvailable;
        } else {
            inner.phase = AppPhase::Ready;
        }
    }

    /// Gets the current update progress.
    pub fn update_progress(&self) -> Option<UpdateProgress> {
        self.inner.lock().unwrap().update_progress.clone()
    }

    /// Sets the update progress.
    pub fn set_update_progress(&self, progress: UpdateProgress) {
        self.inner.lock().unwrap().update_progress = Some(progress);
    }

    /// Clears the update progress.
    pub fn clear_update_progress(&self) {
        self.inner.lock().unwrap().update_progress = None;
    }

    // === Game State ===

    /// Returns true if the game is currently running.
    pub fn is_game_running(&self) -> bool {
        self.inner.lock().unwrap().is_game_running
    }

    /// Returns the number of currently running client instances.
    pub fn running_clients(&self) -> usize {
        self.inner.lock().unwrap().running_clients
    }

    /// Sets the number of running client instances.
    /// Automatically updates is_game_running and phase.
    pub fn set_running_clients(&self, count: usize) {
        let mut inner = self.inner.lock().unwrap();
        inner.running_clients = count;
        inner.is_game_running = count > 0;
        if count == 0 && inner.phase == AppPhase::GameRunning {
            inner.phase = AppPhase::Ready;
        } else if count > 0 {
            inner.phase = AppPhase::GameRunning;
        }
    }

    /// Increments the running client count.
    pub fn increment_running_clients(&self) -> usize {
        let mut inner = self.inner.lock().unwrap();
        inner.running_clients = inner.running_clients.saturating_add(1);
        inner.is_game_running = true;
        inner.phase = AppPhase::GameRunning;
        inner.running_clients
    }

    /// Decrements the running client count.
    pub fn decrement_running_clients(&self) -> usize {
        let mut inner = self.inner.lock().unwrap();
        if inner.running_clients > 0 {
            inner.running_clients -= 1;
        }
        inner.is_game_running = inner.running_clients > 0;
        if inner.running_clients == 0 && inner.phase == AppPhase::GameRunning {
            inner.phase = AppPhase::Ready;
        }
        inner.running_clients
    }

    /// Sets whether the game is currently running.
    pub fn set_game_running(&self, running: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.is_game_running = running;
        inner.running_clients = if running { 1 } else { 0 };
        if running {
            inner.phase = AppPhase::GameRunning;
        } else if !inner.is_updating && !inner.is_installing {
            inner.phase = AppPhase::Ready;
        }
    }

    // === Error State ===

    /// Returns the current error message, if any.
    pub fn error_message(&self) -> Option<String> {
        self.inner.lock().unwrap().error_message.clone()
    }

    /// Sets an error state with the given message.
    pub fn set_error(&self, message: impl Into<String>) {
        let mut inner = self.inner.lock().unwrap();
        inner.error_message = Some(message.into());
        inner.phase = AppPhase::Error;
        inner.is_installing = false;
        inner.is_updating = false;
    }

    /// Clears the error state.
    pub fn clear_error(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.error_message = None;
        // Return to appropriate phase
        if inner.install_path.is_none() {
            inner.phase = AppPhase::NeedsInstall;
        } else if inner.update_available {
            inner.phase = AppPhase::UpdateAvailable;
        } else {
            inner.phase = AppPhase::Ready;
        }
    }

    // === Current Operation ===

    /// Gets the current operation description.
    pub fn current_operation(&self) -> Option<String> {
        self.inner.lock().unwrap().current_operation.clone()
    }

    /// Sets the current operation description.
    pub fn set_current_operation(&self, operation: impl Into<String>) {
        self.inner.lock().unwrap().current_operation = Some(operation.into());
    }

    /// Clears the current operation description.
    pub fn clear_current_operation(&self) {
        self.inner.lock().unwrap().current_operation = None;
    }

    // === Configuration Access ===

    /// Returns a clone of the brand configuration, if set.
    pub fn brand_config(&self) -> Option<BrandConfig> {
        self.brand_config.read().unwrap().clone()
    }

    /// Returns a clone of the launcher configuration, if set.
    pub fn launcher_config(&self) -> Option<LauncherConfig> {
        self.launcher_config.read().unwrap().clone()
    }

    /// Updates the launcher configuration.
    pub fn set_launcher_config(&self, config: LauncherConfig) {
        let mut launcher = self.launcher_config.write().unwrap();
        *launcher = Some(config);
    }

    /// Returns a clone of the cached manifest, if available.
    pub fn cached_manifest(&self) -> Option<Manifest> {
        self.cached_manifest.read().unwrap().clone()
    }

    /// Sets the cached manifest.
    pub fn set_cached_manifest(&self, manifest: Manifest) {
        let mut cached = self.cached_manifest.write().unwrap();
        *cached = Some(manifest);
    }

    /// Clears the cached manifest.
    pub fn clear_cached_manifest(&self) {
        let mut cached = self.cached_manifest.write().unwrap();
        *cached = None;
    }

    // === Status Summary ===

    /// Returns a serializable status summary for the frontend.
    pub fn get_status(&self) -> AppStatus {
        let inner = self.inner.lock().unwrap();
        AppStatus {
            phase: inner.phase.clone(),
            install_path: inner.install_path.clone(),
            current_version: inner.current_version.clone(),
            update_available: inner.update_available,
            available_version: inner.available_version.clone(),
            files_to_update: inner.files_to_update,
            update_download_size: inner.update_download_size,
            is_installing: inner.is_installing,
            is_updating: inner.is_updating,
            is_game_running: inner.is_game_running,
            running_clients: inner.running_clients,
            error_message: inner.error_message.clone(),
            install_progress: inner.install_progress,
            current_operation: inner.current_operation.clone(),
        }
    }

    /// Marks installation as complete with the given version.
    pub fn complete_installation(&self, install_path: PathBuf, version: String) {
        let mut inner = self.inner.lock().unwrap();
        inner.install_path = Some(install_path);
        inner.current_version = Some(version);
        inner.is_installing = false;
        inner.install_progress = 100.0;
        inner.phase = AppPhase::Ready;
        inner.current_operation = None;
    }

    /// Marks update as complete with the new version.
    pub fn complete_update(&self, version: String) {
        let mut inner = self.inner.lock().unwrap();
        inner.current_version = Some(version);
        inner.is_updating = false;
        inner.update_available = false;
        inner.available_version = None;
        inner.files_to_update = 0;
        inner.update_download_size = 0;
        inner.update_progress = None;
        inner.phase = AppPhase::Ready;
        inner.current_operation = None;
    }

    /// Resets state for a fresh update check.
    pub fn begin_update_check(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.phase = AppPhase::CheckingUpdates;
        inner.current_operation = Some("Checking for updates...".to_string());
    }

    /// Completes an update check and resets phase appropriately.
    ///
    /// This should be called after an update check finishes (whether an update
    /// was found or not). It resets the phase from `CheckingUpdates` to either
    /// `UpdateAvailable` (if an update is available) or `Ready` (if up to date).
    pub fn end_update_check(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.current_operation = None;
        // Transition to appropriate phase based on update availability
        if inner.update_available {
            inner.phase = AppPhase::UpdateAvailable;
        } else {
            inner.phase = AppPhase::Ready;
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// Implement Debug manually since Mutex<T> has its own Debug
impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.lock().unwrap();
        f.debug_struct("AppState")
            .field("phase", &inner.phase)
            .field("install_path", &inner.install_path)
            .field("current_version", &inner.current_version)
            .field("update_available", &inner.update_available)
            .field("is_installing", &inner.is_installing)
            .field("is_updating", &inner.is_updating)
            .field("is_game_running", &inner.is_game_running)
            .field("running_clients", &inner.running_clients)
            .finish()
    }
}

/// Serializable status summary for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    /// Current application phase.
    pub phase: AppPhase,
    /// Path to the UO client installation directory.
    pub install_path: Option<PathBuf>,
    /// Current installed version.
    pub current_version: Option<String>,
    /// Whether an update is available.
    pub update_available: bool,
    /// Version available on the server.
    pub available_version: Option<String>,
    /// Number of files that need updating.
    pub files_to_update: usize,
    /// Total download size for the update (bytes).
    pub update_download_size: u64,
    /// Whether an installation is in progress.
    pub is_installing: bool,
    /// Whether an update is in progress.
    pub is_updating: bool,
    /// Whether the game is currently running.
    pub is_game_running: bool,
    /// Number of client instances currently running.
    pub running_clients: usize,
    /// Current error message (if any).
    pub error_message: Option<String>,
    /// Installation progress percentage (0-100).
    pub install_progress: f64,
    /// Current operation description.
    pub current_operation: Option<String>,
}

impl AppStatus {
    /// Returns a human-readable download size string.
    pub fn download_size_formatted(&self) -> String {
        format_size(self.update_download_size)
    }

    /// Returns true if the app is busy with an operation.
    pub fn is_busy(&self) -> bool {
        self.is_installing || self.is_updating
    }

    /// Returns true if the game can be launched.
    pub fn can_launch(&self) -> bool {
        matches!(self.phase, AppPhase::Ready | AppPhase::UpdateAvailable)
            && !self.is_busy()
            && self.install_path.is_some()
    }
}

/// Formats a byte size into a human-readable string.
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::BrandConfigBuilder;

    /// Valid 64-character hex public key for testing.
    const TEST_PUBLIC_KEY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    fn test_brand_config() -> BrandConfig {
        BrandConfigBuilder::new()
            .display_name("Test Server")
            .server_name("TestServer")
            .update_url("http://localhost:8080")
            .public_key(TEST_PUBLIC_KEY)
            .build()
            .unwrap()
    }

    #[test]
    fn test_app_phase_display() {
        assert_eq!(AppPhase::Initializing.to_string(), "Initializing");
        assert_eq!(AppPhase::NeedsInstall.to_string(), "Installation Required");
        assert_eq!(AppPhase::Installing.to_string(), "Installing");
        assert_eq!(AppPhase::CheckingUpdates.to_string(), "Checking for Updates");
        assert_eq!(AppPhase::UpdateAvailable.to_string(), "Update Available");
        assert_eq!(AppPhase::Updating.to_string(), "Updating");
        assert_eq!(AppPhase::Ready.to_string(), "Ready");
        assert_eq!(AppPhase::GameRunning.to_string(), "Game Running");
        assert_eq!(AppPhase::Error.to_string(), "Error");
    }

    #[test]
    fn test_app_phase_default() {
        assert_eq!(AppPhase::default(), AppPhase::Initializing);
    }

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        assert_eq!(state.phase(), AppPhase::Initializing);
        assert!(state.install_path().is_none());
        assert!(state.current_version().is_none());
        assert!(!state.update_available());
        assert!(!state.is_installing());
        assert!(!state.is_updating());
        assert!(!state.is_game_running());
        assert_eq!(state.running_clients(), 0);
        assert!(state.error_message().is_none());
    }

    #[test]
    fn test_app_state_with_brand_config() {
        let brand = test_brand_config();
        let state = AppState::with_brand_config(brand.clone());

        let stored_brand = state.brand_config();
        assert!(stored_brand.is_some());
        assert_eq!(stored_brand.unwrap().product.display_name, "Test Server");
    }

    #[test]
    fn test_app_state_initialize() {
        let state = AppState::new();
        let brand = test_brand_config();
        let launcher = LauncherConfig::new();

        state.initialize(brand, launcher);

        // Should be in NeedsInstall phase for first run
        assert_eq!(state.phase(), AppPhase::NeedsInstall);
    }

    #[test]
    fn test_app_state_initialize_existing_install() {
        let state = AppState::new();
        let brand = test_brand_config();
        let mut launcher = LauncherConfig::new();
        launcher.install_path = Some(PathBuf::from("/test/path"));
        launcher.current_version = Some("1.0.0".to_string());
        launcher.install_complete = true;

        state.initialize(brand, launcher);

        // Should be checking updates for existing install
        assert_eq!(state.phase(), AppPhase::CheckingUpdates);
        assert_eq!(state.install_path(), Some(PathBuf::from("/test/path")));
        assert_eq!(state.current_version(), Some("1.0.0".to_string()));
    }

    #[test]
    fn test_app_state_set_phase() {
        let state = AppState::new();

        state.set_phase(AppPhase::Ready);
        assert_eq!(state.phase(), AppPhase::Ready);

        state.set_phase(AppPhase::Updating);
        assert_eq!(state.phase(), AppPhase::Updating);
    }

    #[test]
    fn test_app_state_is_operational() {
        let state = AppState::new();

        state.set_phase(AppPhase::Ready);
        assert!(state.is_operational());

        state.set_phase(AppPhase::GameRunning);
        assert!(state.is_operational());

        state.set_phase(AppPhase::CheckingUpdates);
        assert!(state.is_operational());

        state.set_phase(AppPhase::Error);
        assert!(!state.is_operational());

        state.set_phase(AppPhase::Installing);
        assert!(!state.is_operational());
    }

    #[test]
    fn test_app_state_install_path() {
        let state = AppState::new();

        assert!(state.install_path().is_none());

        state.set_install_path(PathBuf::from("/game/uo"));
        assert_eq!(state.install_path(), Some(PathBuf::from("/game/uo")));
    }

    #[test]
    fn test_app_state_current_version() {
        let state = AppState::new();

        assert!(state.current_version().is_none());

        state.set_current_version("2.0.0".to_string());
        assert_eq!(state.current_version(), Some("2.0.0".to_string()));
    }

    #[test]
    fn test_app_state_installing() {
        let state = AppState::new();

        assert!(!state.is_installing());

        state.set_installing(true);
        assert!(state.is_installing());
        assert_eq!(state.phase(), AppPhase::Installing);

        state.set_installing(false);
        assert!(!state.is_installing());
        assert_eq!(state.phase(), AppPhase::NeedsInstall);
    }

    #[test]
    fn test_app_state_install_progress() {
        let state = AppState::new();

        assert!((state.install_progress() - 0.0).abs() < f64::EPSILON);

        state.set_install_progress(50.0);
        assert!((state.install_progress() - 50.0).abs() < f64::EPSILON);

        // Test clamping
        state.set_install_progress(150.0);
        assert!((state.install_progress() - 100.0).abs() < f64::EPSILON);

        state.set_install_progress(-10.0);
        assert!((state.install_progress() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_app_state_update_available() {
        let state = AppState::new();

        assert!(!state.update_available());

        state.set_update_available(true, Some("2.0.0".to_string()), 5, 1024 * 1024);

        assert!(state.update_available());
        assert_eq!(state.available_version(), Some("2.0.0".to_string()));
        assert_eq!(state.files_to_update(), 5);
        assert_eq!(state.update_download_size(), 1024 * 1024);
        assert_eq!(state.phase(), AppPhase::UpdateAvailable);
    }

    #[test]
    fn test_app_state_updating() {
        let state = AppState::new();
        state.set_phase(AppPhase::Ready);

        assert!(!state.is_updating());

        state.set_updating(true);
        assert!(state.is_updating());
        assert_eq!(state.phase(), AppPhase::Updating);

        state.set_updating(false);
        assert!(!state.is_updating());
        assert_eq!(state.phase(), AppPhase::Ready);
    }

    #[test]
    fn test_app_state_update_progress() {
        let state = AppState::new();

        assert!(state.update_progress().is_none());

        let progress = UpdateProgress::new();
        state.set_update_progress(progress.clone());

        assert!(state.update_progress().is_some());

        state.clear_update_progress();
        assert!(state.update_progress().is_none());
    }

    #[test]
    fn test_app_state_game_running() {
        let state = AppState::new();
        state.set_phase(AppPhase::Ready);

        assert!(!state.is_game_running());
        assert_eq!(state.running_clients(), 0);

        state.set_game_running(true);
        assert!(state.is_game_running());
        assert_eq!(state.running_clients(), 1);
        assert_eq!(state.phase(), AppPhase::GameRunning);

        state.set_game_running(false);
        assert!(!state.is_game_running());
        assert_eq!(state.running_clients(), 0);
        assert_eq!(state.phase(), AppPhase::Ready);
    }

    #[test]
    fn test_app_state_error() {
        let state = AppState::new();
        state.set_phase(AppPhase::Ready);

        assert!(state.error_message().is_none());

        state.set_error("Something went wrong");
        assert_eq!(state.error_message(), Some("Something went wrong".to_string()));
        assert_eq!(state.phase(), AppPhase::Error);
        assert!(!state.is_installing());
        assert!(!state.is_updating());

        state.clear_error();
        assert!(state.error_message().is_none());
        // Should return to appropriate phase - in this case Ready since no install_path
        assert_eq!(state.phase(), AppPhase::NeedsInstall);
    }

    #[test]
    fn test_app_state_error_clears_to_ready() {
        let state = AppState::new();
        state.set_install_path(PathBuf::from("/game"));
        state.set_phase(AppPhase::Ready);

        state.set_error("Error!");
        state.clear_error();

        assert_eq!(state.phase(), AppPhase::Ready);
    }

    #[test]
    fn test_app_state_current_operation() {
        let state = AppState::new();

        assert!(state.current_operation().is_none());

        state.set_current_operation("Downloading files...");
        assert_eq!(state.current_operation(), Some("Downloading files...".to_string()));

        state.clear_current_operation();
        assert!(state.current_operation().is_none());
    }

    #[test]
    fn test_app_state_launcher_config() {
        let state = AppState::new();

        assert!(state.launcher_config().is_none());

        let config = LauncherConfig::new();
        state.set_launcher_config(config);

        assert!(state.launcher_config().is_some());
    }

    #[test]
    fn test_app_state_cached_manifest() {
        let state = AppState::new();

        assert!(state.cached_manifest().is_none());

        use crate::manifest::{FileEntry, ManifestBuilder};
        let manifest = ManifestBuilder::new()
            .version("1.0.0")
            .timestamp("2026-02-15T00:00:00Z")
            .client_executable("client.exe")
            .add_file(FileEntry::new(
                "client.exe",
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                1000,
            ))
            .build()
            .unwrap();

        state.set_cached_manifest(manifest);
        assert!(state.cached_manifest().is_some());

        state.clear_cached_manifest();
        assert!(state.cached_manifest().is_none());
    }

    #[test]
    fn test_app_state_get_status() {
        let state = AppState::new();
        state.set_install_path(PathBuf::from("/game"));
        state.set_current_version("1.0.0".to_string());
        state.set_phase(AppPhase::Ready);

        let status = state.get_status();

        assert_eq!(status.phase, AppPhase::Ready);
        assert_eq!(status.install_path, Some(PathBuf::from("/game")));
        assert_eq!(status.current_version, Some("1.0.0".to_string()));
        assert!(!status.update_available);
        assert!(!status.is_installing);
        assert!(!status.is_updating);
    }

    #[test]
    fn test_app_state_complete_installation() {
        let state = AppState::new();
        state.set_installing(true);
        state.set_install_progress(50.0);

        state.complete_installation(PathBuf::from("/game/uo"), "1.0.0".to_string());

        assert_eq!(state.install_path(), Some(PathBuf::from("/game/uo")));
        assert_eq!(state.current_version(), Some("1.0.0".to_string()));
        assert!(!state.is_installing());
        assert!((state.install_progress() - 100.0).abs() < f64::EPSILON);
        assert_eq!(state.phase(), AppPhase::Ready);
    }

    #[test]
    fn test_app_state_complete_update() {
        let state = AppState::new();
        state.set_updating(true);
        state.set_update_available(true, Some("2.0.0".to_string()), 5, 1000000);

        state.complete_update("2.0.0".to_string());

        assert_eq!(state.current_version(), Some("2.0.0".to_string()));
        assert!(!state.is_updating());
        assert!(!state.update_available());
        assert!(state.available_version().is_none());
        assert_eq!(state.files_to_update(), 0);
        assert_eq!(state.update_download_size(), 0);
        assert_eq!(state.phase(), AppPhase::Ready);
    }

    #[test]
    fn test_app_state_begin_update_check() {
        let state = AppState::new();
        state.set_phase(AppPhase::Ready);

        state.begin_update_check();

        assert_eq!(state.phase(), AppPhase::CheckingUpdates);
        assert_eq!(state.current_operation(), Some("Checking for updates...".to_string()));
    }

    #[test]
    fn test_app_state_end_update_check_no_update() {
        let state = AppState::new();
        state.begin_update_check();

        state.end_update_check();

        assert_eq!(state.phase(), AppPhase::Ready);
        assert!(state.current_operation().is_none());
    }

    #[test]
    fn test_app_state_end_update_check_with_update() {
        let state = AppState::new();
        state.begin_update_check();
        state.set_update_available(true, Some("2.0.0".to_string()), 5, 1024 * 1024);

        state.end_update_check();

        assert_eq!(state.phase(), AppPhase::UpdateAvailable);
        assert!(state.current_operation().is_none());
        assert!(state.update_available());
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert_eq!(state.phase(), AppPhase::Initializing);
    }

    #[test]
    fn test_app_state_debug() {
        let state = AppState::new();
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("AppState"));
        assert!(debug_str.contains("phase"));
    }

    #[test]
    fn test_app_status_download_size_formatted() {
        let status = AppStatus {
            phase: AppPhase::UpdateAvailable,
            install_path: None,
            current_version: None,
            update_available: true,
            available_version: Some("1.0.0".to_string()),
            files_to_update: 5,
            update_download_size: 1024 * 1024 * 50, // 50 MB
            is_installing: false,
            is_updating: false,
            is_game_running: false,
            running_clients: 0,
            error_message: None,
            install_progress: 0.0,
            current_operation: None,
        };

        assert_eq!(status.download_size_formatted(), "50.00 MB");
    }

    #[test]
    fn test_app_status_is_busy() {
        let mut status = AppStatus {
            phase: AppPhase::Ready,
            install_path: None,
            current_version: None,
            update_available: false,
            available_version: None,
            files_to_update: 0,
            update_download_size: 0,
            is_installing: false,
            is_updating: false,
            is_game_running: false,
            running_clients: 0,
            error_message: None,
            install_progress: 0.0,
            current_operation: None,
        };

        assert!(!status.is_busy());

        status.is_installing = true;
        assert!(status.is_busy());

        status.is_installing = false;
        status.is_updating = true;
        assert!(status.is_busy());
    }

    #[test]
    fn test_app_status_can_launch() {
        let mut status = AppStatus {
            phase: AppPhase::Ready,
            install_path: Some(PathBuf::from("/game")),
            current_version: Some("1.0.0".to_string()),
            update_available: false,
            available_version: None,
            files_to_update: 0,
            update_download_size: 0,
            is_installing: false,
            is_updating: false,
            is_game_running: false,
            running_clients: 0,
            error_message: None,
            install_progress: 0.0,
            current_operation: None,
        };

        assert!(status.can_launch());

        // Can launch with update available
        status.phase = AppPhase::UpdateAvailable;
        status.update_available = true;
        assert!(status.can_launch());

        // Cannot launch while updating
        status.is_updating = true;
        assert!(!status.can_launch());

        // Cannot launch without install path
        status.is_updating = false;
        status.install_path = None;
        assert!(!status.can_launch());

        // Cannot launch in error state
        status.install_path = Some(PathBuf::from("/game"));
        status.phase = AppPhase::Error;
        assert!(!status.can_launch());
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(0), "0 bytes");
        assert_eq!(format_size(500), "500 bytes");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1536), "1.50 KB");
        assert_eq!(format_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_size(1024 * 1024 * 1024 * 2), "2.00 GB");
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;

        let state = Arc::new(AppState::new());
        let mut handles = vec![];

        // Spawn multiple threads that access and modify state
        for i in 0..10 {
            let state_clone = Arc::clone(&state);
            let handle = thread::spawn(move || {
                state_clone.set_install_progress(i as f64 * 10.0);
                let _ = state_clone.phase();
                let _ = state_clone.get_status();
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // State should still be valid
        let _ = state.get_status();
    }
}
