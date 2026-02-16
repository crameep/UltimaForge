//! Application configuration and embedded branding for UltimaForge.
//!
//! This module handles two types of configuration:
//!
//! 1. **Brand Configuration** - Embedded at build time from `branding/brand.json`.
//!    Contains server-specific branding, update URL, and public key.
//!
//! 2. **Launcher Configuration** - User-specific settings persisted locally.
//!    Contains install path, preferences, and current state.
//!
//! # Security
//!
//! - The public key for signature verification is embedded at build time
//! - Never download or accept public keys at runtime
//! - Update URL comes from embedded branding, not user input

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Errors that can occur during configuration operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Failed to read or write configuration file.
    #[error("Configuration I/O error for '{path}': {source}")]
    IoError {
        path: String,
        #[source]
        source: io::Error,
    },

    /// Configuration JSON is malformed.
    #[error("Invalid configuration JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    /// A required field is missing from the configuration.
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// The configuration value is invalid.
    #[error("Invalid configuration value for '{field}': {reason}")]
    InvalidValue { field: String, reason: String },

    /// The branding configuration is not available.
    #[error("Branding configuration not available: {0}")]
    BrandingUnavailable(String),
}

impl ConfigError {
    /// Creates an IoError variant from a path and error.
    fn io(path: &Path, source: io::Error) -> Self {
        Self::IoError {
            path: path.display().to_string(),
            source,
        }
    }
}

/// Product information from the brand configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProductInfo {
    /// Display name shown in the launcher UI (e.g., "My UO Server").
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// Server name for branding (e.g., "MyServer").
    #[serde(rename = "serverName")]
    pub server_name: String,

    /// Optional server description or tagline.
    #[serde(default)]
    pub description: Option<String>,

    /// Support email address for the server.
    #[serde(default)]
    pub support_email: Option<String>,

    /// Server website URL.
    #[serde(default)]
    pub website: Option<String>,

    /// Discord invite link.
    #[serde(default)]
    pub discord: Option<String>,
}

/// Theme colors from the brand configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeColors {
    /// Primary brand color (hex, e.g., "#1a1a2e").
    #[serde(default = "default_primary")]
    pub primary: String,

    /// Secondary/accent color (hex, e.g., "#e94560").
    #[serde(default = "default_secondary")]
    pub secondary: String,

    /// Background color (hex, e.g., "#16213e").
    #[serde(default = "default_background")]
    pub background: String,

    /// Text color (hex, e.g., "#ffffff").
    #[serde(default = "default_text")]
    pub text: String,
}

fn default_primary() -> String {
    "#1a1a2e".to_string()
}

fn default_secondary() -> String {
    "#e94560".to_string()
}

fn default_background() -> String {
    "#16213e".to_string()
}

fn default_text() -> String {
    "#ffffff".to_string()
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            primary: default_primary(),
            secondary: default_secondary(),
            background: default_background(),
            text: default_text(),
        }
    }
}

/// UI configuration from the brand configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiConfig {
    /// Theme colors for the launcher.
    #[serde(default)]
    pub colors: ThemeColors,

    /// Background image filename (relative to branding folder).
    #[serde(rename = "backgroundImage")]
    pub background_image: Option<String>,

    /// Logo image filename (relative to branding folder).
    #[serde(rename = "logoUrl")]
    pub logo_url: Option<String>,

    /// Whether to show patch notes in the launcher.
    #[serde(rename = "showPatchNotes", default = "default_show_patch_notes")]
    pub show_patch_notes: bool,

    /// Window title override (defaults to display_name).
    #[serde(rename = "windowTitle")]
    pub window_title: Option<String>,
}

fn default_show_patch_notes() -> bool {
    true
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            colors: ThemeColors::default(),
            background_image: None,
            logo_url: None,
            show_patch_notes: true,
            window_title: None,
        }
    }
}

/// Brand configuration embedded at build time.
///
/// This structure mirrors the `branding/brand.json` file that server owners
/// customize before building their launcher.
///
/// # Example brand.json
///
/// ```json
/// {
///   "product": {
///     "displayName": "My UO Server",
///     "serverName": "MyServer"
///   },
///   "updateUrl": "https://updates.myserver.com",
///   "publicKey": "abc123...(64 hex chars)...",
///   "ui": {
///     "colors": {
///       "primary": "#1a1a2e"
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BrandConfig {
    /// Product information.
    pub product: ProductInfo,

    /// Base URL for the update server.
    #[serde(rename = "updateUrl")]
    pub update_url: String,

    /// Ed25519 public key for manifest signature verification (hex-encoded, 64 chars).
    #[serde(rename = "publicKey")]
    pub public_key: String,

    /// UI customization settings.
    #[serde(default)]
    pub ui: UiConfig,

    /// Version of this branding configuration format.
    #[serde(rename = "brandVersion", default = "default_brand_version")]
    pub brand_version: String,
}

fn default_brand_version() -> String {
    "1.0".to_string()
}

impl BrandConfig {
    /// Parses a brand configuration from JSON bytes.
    pub fn parse(json_bytes: &[u8]) -> Result<Self, ConfigError> {
        let config: Self = serde_json::from_slice(json_bytes)?;
        config.validate()?;
        Ok(config)
    }

    /// Parses a brand configuration from a JSON string.
    pub fn parse_str(json_str: &str) -> Result<Self, ConfigError> {
        let config: Self = serde_json::from_str(json_str)?;
        config.validate()?;
        Ok(config)
    }

    /// Loads a brand configuration from a file.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let bytes = fs::read(path).map_err(|e| ConfigError::io(path, e))?;
        Self::parse(&bytes)
    }

    /// Validates the brand configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Validate product info
        if self.product.display_name.is_empty() {
            return Err(ConfigError::MissingField("product.displayName".to_string()));
        }

        if self.product.server_name.is_empty() {
            return Err(ConfigError::MissingField("product.serverName".to_string()));
        }

        // Validate update URL
        if self.update_url.is_empty() {
            return Err(ConfigError::MissingField("updateUrl".to_string()));
        }

        if !self.update_url.starts_with("http://") && !self.update_url.starts_with("https://") {
            return Err(ConfigError::InvalidValue {
                field: "updateUrl".to_string(),
                reason: "must start with http:// or https://".to_string(),
            });
        }

        // Validate public key format (64 hex characters for 32 bytes)
        if self.public_key.is_empty() {
            return Err(ConfigError::MissingField("publicKey".to_string()));
        }

        if self.public_key.len() != 64 {
            return Err(ConfigError::InvalidValue {
                field: "publicKey".to_string(),
                reason: format!("expected 64 hex characters, got {}", self.public_key.len()),
            });
        }

        if !self.public_key.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ConfigError::InvalidValue {
                field: "publicKey".to_string(),
                reason: "must be hexadecimal characters only".to_string(),
            });
        }

        // Validate color formats if provided
        self.validate_color(&self.ui.colors.primary, "ui.colors.primary")?;
        self.validate_color(&self.ui.colors.secondary, "ui.colors.secondary")?;
        self.validate_color(&self.ui.colors.background, "ui.colors.background")?;
        self.validate_color(&self.ui.colors.text, "ui.colors.text")?;

        Ok(())
    }

    /// Validates a hex color value.
    fn validate_color(&self, color: &str, field: &str) -> Result<(), ConfigError> {
        if !color.starts_with('#') {
            return Err(ConfigError::InvalidValue {
                field: field.to_string(),
                reason: "color must start with #".to_string(),
            });
        }

        let hex_part = &color[1..];
        if hex_part.len() != 6 && hex_part.len() != 3 {
            return Err(ConfigError::InvalidValue {
                field: field.to_string(),
                reason: "color must be #RGB or #RRGGBB format".to_string(),
            });
        }

        if !hex_part.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(ConfigError::InvalidValue {
                field: field.to_string(),
                reason: "color must contain only hexadecimal characters".to_string(),
            });
        }

        Ok(())
    }

    /// Returns the public key as raw bytes.
    pub fn public_key_bytes(&self) -> Result<Vec<u8>, ConfigError> {
        hex::decode(&self.public_key).map_err(|_| ConfigError::InvalidValue {
            field: "publicKey".to_string(),
            reason: "failed to decode hex".to_string(),
        })
    }

    /// Returns the window title, using display_name as fallback.
    pub fn window_title(&self) -> &str {
        self.ui
            .window_title
            .as_deref()
            .unwrap_or(&self.product.display_name)
    }
}

/// Launcher configuration persisted locally by the user.
///
/// This contains user-specific settings that persist between launcher runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LauncherConfig {
    /// Path to the UO client installation directory.
    #[serde(rename = "installPath")]
    pub install_path: Option<PathBuf>,

    /// Current installed version (matches manifest version).
    #[serde(rename = "currentVersion")]
    pub current_version: Option<String>,

    /// Whether the first-run installation is complete.
    #[serde(rename = "installComplete", default)]
    pub install_complete: bool,

    /// Auto-launch client after successful update.
    #[serde(rename = "autoLaunch", default)]
    pub auto_launch: bool,

    /// Close launcher after launching game.
    #[serde(rename = "closeOnLaunch", default = "default_close_on_launch")]
    pub close_on_launch: bool,

    /// Check for updates on startup.
    #[serde(rename = "checkUpdatesOnStartup", default = "default_check_updates")]
    pub check_updates_on_startup: bool,

    /// Version of this configuration format.
    #[serde(rename = "configVersion", default = "default_config_version")]
    pub config_version: u32,
}

fn default_close_on_launch() -> bool {
    true
}

fn default_check_updates() -> bool {
    true
}

fn default_config_version() -> u32 {
    1
}

impl Default for LauncherConfig {
    fn default() -> Self {
        Self {
            install_path: None,
            current_version: None,
            install_complete: false,
            auto_launch: false,
            close_on_launch: true,
            check_updates_on_startup: true,
            config_version: 1,
        }
    }
}

impl LauncherConfig {
    /// Creates a new default launcher configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads the launcher configuration from a file.
    ///
    /// If the file doesn't exist, returns a default configuration.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let bytes = fs::read(path).map_err(|e| ConfigError::io(path, e))?;
        let config: Self = serde_json::from_slice(&bytes)?;
        Ok(config)
    }

    /// Loads configuration from a JSON string.
    pub fn parse_str(json_str: &str) -> Result<Self, ConfigError> {
        let config: Self = serde_json::from_str(json_str)?;
        Ok(config)
    }

    /// Saves the launcher configuration to a file.
    ///
    /// Creates parent directories if they don't exist.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| ConfigError::io(parent, e))?;
            }
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json).map_err(|e| ConfigError::io(path, e))?;
        Ok(())
    }

    /// Serializes the configuration to a JSON string.
    pub fn to_json_string(&self) -> Result<String, ConfigError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Returns whether the launcher is in "first-run" state.
    pub fn is_first_run(&self) -> bool {
        !self.install_complete || self.install_path.is_none()
    }

    /// Returns whether an update is needed based on version comparison.
    pub fn needs_update(&self, manifest_version: &str) -> bool {
        match &self.current_version {
            Some(current) => current != manifest_version,
            None => true,
        }
    }

    /// Sets the install path and marks installation as complete.
    pub fn set_installed(&mut self, path: PathBuf, version: &str) {
        self.install_path = Some(path);
        self.current_version = Some(version.to_string());
        self.install_complete = true;
    }

    /// Updates the current version after a successful update.
    pub fn set_version(&mut self, version: &str) {
        self.current_version = Some(version.to_string());
    }

    /// Returns the install path, if set.
    pub fn install_path(&self) -> Option<&Path> {
        self.install_path.as_deref()
    }
}

/// Default configuration file name for the launcher.
pub const LAUNCHER_CONFIG_FILE: &str = "launcher.json";

/// Default branding file name.
pub const BRAND_CONFIG_FILE: &str = "brand.json";

/// Returns the default path for the launcher configuration file.
///
/// On Windows: `%APPDATA%\UltimaForge\{server_name}\launcher.json`
/// On Linux: `~/.config/ultimaforge/{server_name}/launcher.json`
/// On macOS: `~/Library/Application Support/UltimaForge/{server_name}/launcher.json`
pub fn default_config_path(server_name: &str) -> PathBuf {
    let base = dirs_config_path();
    base.join(server_name).join(LAUNCHER_CONFIG_FILE)
}

/// Returns the platform-specific config directory base path.
fn dirs_config_path() -> PathBuf {
    // Use std::env for cross-platform config directory
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("UltimaForge")
    }

    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("Library")
            .join("Application Support")
            .join("UltimaForge")
    }

    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".config"))
                    .unwrap_or_else(|_| PathBuf::from(".config"))
            })
            .join("ultimaforge")
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        PathBuf::from(".").join("ultimaforge")
    }
}

/// Builder for creating BrandConfig for testing or programmatic use.
#[derive(Debug, Default)]
pub struct BrandConfigBuilder {
    display_name: Option<String>,
    server_name: Option<String>,
    update_url: Option<String>,
    public_key: Option<String>,
    description: Option<String>,
    support_email: Option<String>,
    website: Option<String>,
    discord: Option<String>,
    colors: Option<ThemeColors>,
    show_patch_notes: Option<bool>,
    window_title: Option<String>,
}

impl BrandConfigBuilder {
    /// Creates a new brand config builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the display name.
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }

    /// Sets the server name.
    pub fn server_name(mut self, name: impl Into<String>) -> Self {
        self.server_name = Some(name.into());
        self
    }

    /// Sets the update URL.
    pub fn update_url(mut self, url: impl Into<String>) -> Self {
        self.update_url = Some(url.into());
        self
    }

    /// Sets the public key.
    pub fn public_key(mut self, key: impl Into<String>) -> Self {
        self.public_key = Some(key.into());
        self
    }

    /// Sets the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Sets the support email.
    pub fn support_email(mut self, email: impl Into<String>) -> Self {
        self.support_email = Some(email.into());
        self
    }

    /// Sets the website URL.
    pub fn website(mut self, url: impl Into<String>) -> Self {
        self.website = Some(url.into());
        self
    }

    /// Sets the Discord invite link.
    pub fn discord(mut self, link: impl Into<String>) -> Self {
        self.discord = Some(link.into());
        self
    }

    /// Sets the theme colors.
    pub fn colors(mut self, colors: ThemeColors) -> Self {
        self.colors = Some(colors);
        self
    }

    /// Sets whether to show patch notes.
    pub fn show_patch_notes(mut self, show: bool) -> Self {
        self.show_patch_notes = Some(show);
        self
    }

    /// Sets the window title.
    pub fn window_title(mut self, title: impl Into<String>) -> Self {
        self.window_title = Some(title.into());
        self
    }

    /// Builds the brand configuration.
    pub fn build(self) -> Result<BrandConfig, ConfigError> {
        let display_name = self
            .display_name
            .ok_or_else(|| ConfigError::MissingField("displayName".to_string()))?;
        let server_name = self
            .server_name
            .ok_or_else(|| ConfigError::MissingField("serverName".to_string()))?;
        let update_url = self
            .update_url
            .ok_or_else(|| ConfigError::MissingField("updateUrl".to_string()))?;
        let public_key = self
            .public_key
            .ok_or_else(|| ConfigError::MissingField("publicKey".to_string()))?;

        let config = BrandConfig {
            product: ProductInfo {
                display_name,
                server_name,
                description: self.description,
                support_email: self.support_email,
                website: self.website,
                discord: self.discord,
            },
            update_url,
            public_key,
            ui: UiConfig {
                colors: self.colors.unwrap_or_default(),
                background_image: None,
                logo_url: None,
                show_patch_notes: self.show_patch_notes.unwrap_or(true),
                window_title: self.window_title,
            },
            brand_version: "1.0".to_string(),
        };

        config.validate()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Valid 64-character hex public key for testing.
    const TEST_PUBLIC_KEY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    /// Creates a valid brand config JSON string.
    fn valid_brand_json() -> String {
        format!(
            r##"{{
            "product": {{
                "displayName": "Test Server",
                "serverName": "TestServer",
                "description": "A test UO server",
                "supportEmail": "support@test.com",
                "website": "https://test.com",
                "discord": "https://discord.gg/test"
            }},
            "updateUrl": "https://updates.test.com",
            "publicKey": "{TEST_PUBLIC_KEY}",
            "ui": {{
                "colors": {{
                    "primary": "#1a1a2e",
                    "secondary": "#e94560",
                    "background": "#16213e",
                    "text": "#ffffff"
                }},
                "showPatchNotes": true,
                "windowTitle": "Test UO Launcher"
            }},
            "brandVersion": "1.0"
        }}"##
        )
    }

    /// Creates a minimal brand config JSON string.
    fn minimal_brand_json() -> String {
        format!(
            r##"{{
            "product": {{
                "displayName": "Minimal Server",
                "serverName": "MinServer"
            }},
            "updateUrl": "https://updates.min.com",
            "publicKey": "{TEST_PUBLIC_KEY}"
        }}"##
        )
    }

    #[test]
    fn test_parse_valid_brand_config() {
        let json = valid_brand_json();
        let config = BrandConfig::parse_str(&json).expect("Should parse valid config");

        assert_eq!(config.product.display_name, "Test Server");
        assert_eq!(config.product.server_name, "TestServer");
        assert_eq!(
            config.product.description,
            Some("A test UO server".to_string())
        );
        assert_eq!(config.update_url, "https://updates.test.com");
        assert_eq!(config.public_key, TEST_PUBLIC_KEY);
        assert_eq!(config.ui.colors.primary, "#1a1a2e");
        assert!(config.ui.show_patch_notes);
        assert_eq!(
            config.ui.window_title,
            Some("Test UO Launcher".to_string())
        );
    }

    #[test]
    fn test_parse_minimal_brand_config() {
        let json = minimal_brand_json();
        let config = BrandConfig::parse_str(&json).expect("Should parse minimal config");

        assert_eq!(config.product.display_name, "Minimal Server");
        assert_eq!(config.product.server_name, "MinServer");
        assert!(config.product.description.is_none());
        assert_eq!(config.update_url, "https://updates.min.com");

        // Check defaults
        assert_eq!(config.ui.colors.primary, "#1a1a2e");
        assert!(config.ui.show_patch_notes);
        assert!(config.ui.window_title.is_none());
    }

    #[test]
    fn test_parse_missing_display_name() {
        let json = format!(
            r#"{{
            "product": {{
                "displayName": "",
                "serverName": "TestServer"
            }},
            "updateUrl": "https://test.com",
            "publicKey": "{TEST_PUBLIC_KEY}"
        }}"#
        );

        let result = BrandConfig::parse_str(&json);
        assert!(matches!(result, Err(ConfigError::MissingField(f)) if f == "product.displayName"));
    }

    #[test]
    fn test_parse_missing_server_name() {
        let json = format!(
            r#"{{
            "product": {{
                "displayName": "Test",
                "serverName": ""
            }},
            "updateUrl": "https://test.com",
            "publicKey": "{TEST_PUBLIC_KEY}"
        }}"#
        );

        let result = BrandConfig::parse_str(&json);
        assert!(matches!(result, Err(ConfigError::MissingField(f)) if f == "product.serverName"));
    }

    #[test]
    fn test_parse_invalid_update_url() {
        let json = format!(
            r#"{{
            "product": {{
                "displayName": "Test",
                "serverName": "Test"
            }},
            "updateUrl": "not-a-url",
            "publicKey": "{TEST_PUBLIC_KEY}"
        }}"#
        );

        let result = BrandConfig::parse_str(&json);
        assert!(matches!(result, Err(ConfigError::InvalidValue { field, .. }) if field == "updateUrl"));
    }

    #[test]
    fn test_parse_invalid_public_key_length() {
        let json = r#"{
            "product": {
                "displayName": "Test",
                "serverName": "Test"
            },
            "updateUrl": "https://test.com",
            "publicKey": "tooshort"
        }"#;

        let result = BrandConfig::parse_str(json);
        assert!(matches!(result, Err(ConfigError::InvalidValue { field, .. }) if field == "publicKey"));
    }

    #[test]
    fn test_parse_invalid_public_key_chars() {
        let json = r#"{
            "product": {
                "displayName": "Test",
                "serverName": "Test"
            },
            "updateUrl": "https://test.com",
            "publicKey": "gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg"
        }"#;

        let result = BrandConfig::parse_str(json);
        assert!(matches!(result, Err(ConfigError::InvalidValue { field, .. }) if field == "publicKey"));
    }

    #[test]
    fn test_parse_invalid_color_format() {
        let json = format!(
            r#"{{
            "product": {{
                "displayName": "Test",
                "serverName": "Test"
            }},
            "updateUrl": "https://test.com",
            "publicKey": "{TEST_PUBLIC_KEY}",
            "ui": {{
                "colors": {{
                    "primary": "not-a-color"
                }}
            }}
        }}"#
        );

        let result = BrandConfig::parse_str(&json);
        assert!(
            matches!(result, Err(ConfigError::InvalidValue { field, .. }) if field == "ui.colors.primary")
        );
    }

    #[test]
    fn test_public_key_bytes() {
        let json = minimal_brand_json();
        let config = BrandConfig::parse_str(&json).unwrap();

        let bytes = config.public_key_bytes().expect("Should decode hex");
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn test_window_title_fallback() {
        let json = minimal_brand_json();
        let config = BrandConfig::parse_str(&json).unwrap();

        // Should fall back to display_name
        assert_eq!(config.window_title(), "Minimal Server");
    }

    #[test]
    fn test_window_title_custom() {
        let json = valid_brand_json();
        let config = BrandConfig::parse_str(&json).unwrap();

        // Should use custom window title
        assert_eq!(config.window_title(), "Test UO Launcher");
    }

    #[test]
    fn test_launcher_config_default() {
        let config = LauncherConfig::new();

        assert!(config.install_path.is_none());
        assert!(config.current_version.is_none());
        assert!(!config.install_complete);
        assert!(!config.auto_launch);
        assert!(config.close_on_launch);
        assert!(config.check_updates_on_startup);
        assert_eq!(config.config_version, 1);
    }

    #[test]
    fn test_launcher_config_is_first_run() {
        let mut config = LauncherConfig::new();
        assert!(config.is_first_run());

        config.install_complete = true;
        assert!(config.is_first_run()); // Still first run - no install path

        config.install_path = Some(PathBuf::from("/test"));
        assert!(!config.is_first_run());
    }

    #[test]
    fn test_launcher_config_needs_update() {
        let mut config = LauncherConfig::new();

        // No version set - needs update
        assert!(config.needs_update("1.0.0"));

        config.current_version = Some("1.0.0".to_string());

        // Same version - no update needed
        assert!(!config.needs_update("1.0.0"));

        // Different version - needs update
        assert!(config.needs_update("1.1.0"));
    }

    #[test]
    fn test_launcher_config_set_installed() {
        let mut config = LauncherConfig::new();
        config.set_installed(PathBuf::from("/game/uo"), "1.0.0");

        assert_eq!(config.install_path, Some(PathBuf::from("/game/uo")));
        assert_eq!(config.current_version, Some("1.0.0".to_string()));
        assert!(config.install_complete);
    }

    #[test]
    fn test_launcher_config_save_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("test_config.json");

        // Create and save config
        let mut config = LauncherConfig::new();
        config.install_path = Some(PathBuf::from("/test/path"));
        config.current_version = Some("2.0.0".to_string());
        config.install_complete = true;
        config.auto_launch = true;

        config.save(&config_path).expect("Should save config");

        // Load and verify
        let loaded = LauncherConfig::load(&config_path).expect("Should load config");

        assert_eq!(loaded.install_path, Some(PathBuf::from("/test/path")));
        assert_eq!(loaded.current_version, Some("2.0.0".to_string()));
        assert!(loaded.install_complete);
        assert!(loaded.auto_launch);
    }

    #[test]
    fn test_launcher_config_load_nonexistent() {
        let config = LauncherConfig::load(Path::new("/nonexistent/path.json"))
            .expect("Should return default for nonexistent file");

        assert!(config.is_first_run());
        assert!(config.install_path.is_none());
    }

    #[test]
    fn test_launcher_config_creates_parent_dirs() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let nested_path = temp_dir.path().join("nested").join("deep").join("config.json");

        let config = LauncherConfig::new();
        config.save(&nested_path).expect("Should create parent dirs and save");

        assert!(nested_path.exists());
    }

    #[test]
    fn test_launcher_config_to_json_string() {
        let config = LauncherConfig::new();
        let json = config.to_json_string().expect("Should serialize");

        assert!(json.contains("installPath"));
        assert!(json.contains("configVersion"));
    }

    #[test]
    fn test_launcher_config_parse_str() {
        let json = r#"{
            "installPath": "/test/path",
            "currentVersion": "1.5.0",
            "installComplete": true,
            "autoLaunch": false,
            "closeOnLaunch": false,
            "checkUpdatesOnStartup": true,
            "configVersion": 1
        }"#;

        let config = LauncherConfig::parse_str(json).expect("Should parse");

        assert_eq!(config.install_path, Some(PathBuf::from("/test/path")));
        assert_eq!(config.current_version, Some("1.5.0".to_string()));
        assert!(config.install_complete);
        assert!(!config.auto_launch);
        assert!(!config.close_on_launch);
    }

    #[test]
    fn test_brand_config_builder() {
        let config = BrandConfigBuilder::new()
            .display_name("My Server")
            .server_name("MyServer")
            .update_url("https://updates.myserver.com")
            .public_key(TEST_PUBLIC_KEY)
            .description("Best server ever")
            .support_email("help@myserver.com")
            .show_patch_notes(false)
            .build()
            .expect("Should build config");

        assert_eq!(config.product.display_name, "My Server");
        assert_eq!(config.product.server_name, "MyServer");
        assert_eq!(config.update_url, "https://updates.myserver.com");
        assert_eq!(
            config.product.description,
            Some("Best server ever".to_string())
        );
        assert!(!config.ui.show_patch_notes);
    }

    #[test]
    fn test_brand_config_builder_missing_required() {
        let result = BrandConfigBuilder::new()
            .display_name("Test")
            .server_name("Test")
            // Missing update_url
            .public_key(TEST_PUBLIC_KEY)
            .build();

        assert!(matches!(result, Err(ConfigError::MissingField(_))));
    }

    #[test]
    fn test_brand_config_load_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config_path = temp_dir.path().join("brand.json");

        let json = minimal_brand_json();
        std::fs::write(&config_path, json).expect("Should write file");

        let config = BrandConfig::load(&config_path).expect("Should load config");
        assert_eq!(config.product.display_name, "Minimal Server");
    }

    #[test]
    fn test_theme_colors_default() {
        let colors = ThemeColors::default();

        assert_eq!(colors.primary, "#1a1a2e");
        assert_eq!(colors.secondary, "#e94560");
        assert_eq!(colors.background, "#16213e");
        assert_eq!(colors.text, "#ffffff");
    }

    #[test]
    fn test_validate_color_shorthand() {
        let json = format!(
            r##"{{
            "product": {{
                "displayName": "Test",
                "serverName": "Test"
            }},
            "updateUrl": "https://test.com",
            "publicKey": "{TEST_PUBLIC_KEY}",
            "ui": {{
                "colors": {{
                    "primary": "#abc"
                }}
            }}
        }}"##
        );

        // #RGB shorthand should be valid
        let config = BrandConfig::parse_str(&json).expect("Should accept #RGB format");
        assert_eq!(config.ui.colors.primary, "#abc");
    }

    #[test]
    fn test_default_config_path() {
        let path = default_config_path("TestServer");

        // Should contain the server name
        assert!(path.to_string_lossy().contains("TestServer"));
        // Should end with launcher.json
        assert!(path.to_string_lossy().ends_with("launcher.json"));
    }

    #[test]
    fn test_config_error_display() {
        let error = ConfigError::MissingField("test".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Missing required field"));
        assert!(display.contains("test"));
    }

    #[test]
    fn test_config_serialization_roundtrip() {
        let original = BrandConfigBuilder::new()
            .display_name("Roundtrip Server")
            .server_name("RTServer")
            .update_url("https://rt.test.com")
            .public_key(TEST_PUBLIC_KEY)
            .build()
            .expect("Should build");

        let json = serde_json::to_string_pretty(&original).expect("Should serialize");
        let parsed = BrandConfig::parse_str(&json).expect("Should parse");

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_launcher_config_serialization_roundtrip() {
        let mut original = LauncherConfig::new();
        original.install_path = Some(PathBuf::from("/game"));
        original.current_version = Some("3.0.0".to_string());
        original.install_complete = true;

        let json = original.to_json_string().expect("Should serialize");
        let parsed = LauncherConfig::parse_str(&json).expect("Should parse");

        assert_eq!(original, parsed);
    }

    #[test]
    fn test_install_path_accessor() {
        let mut config = LauncherConfig::new();
        assert!(config.install_path().is_none());

        config.install_path = Some(PathBuf::from("/test"));
        assert_eq!(config.install_path(), Some(Path::new("/test")));
    }

    #[test]
    fn test_set_version() {
        let mut config = LauncherConfig::new();
        config.set_version("2.5.0");
        assert_eq!(config.current_version, Some("2.5.0".to_string()));
    }

    #[test]
    fn test_http_update_url_accepted() {
        let json = format!(
            r#"{{
            "product": {{
                "displayName": "Test",
                "serverName": "Test"
            }},
            "updateUrl": "http://localhost:8080",
            "publicKey": "{TEST_PUBLIC_KEY}"
        }}"#
        );

        // HTTP should be allowed (for local testing)
        let config = BrandConfig::parse_str(&json).expect("Should accept http://");
        assert_eq!(config.update_url, "http://localhost:8080");
    }
}
