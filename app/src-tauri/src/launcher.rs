//! UO client process spawning for UltimaForge.
//!
//! This module handles launching the Ultima Online client executable
//! after installation and updates are complete.
//!
//! # Features
//!
//! - Launch client executable with configurable arguments
//! - Set working directory to install path
//! - Validate executable exists and is runnable
//! - Handle launch errors gracefully
//!
//! # Security
//!
//! - Only launches executables from the verified installation directory
//! - Validates paths to prevent directory traversal
//! - Uses shell plugin for process spawning (platform-native)
//!
//! # Example
//!
//! ```ignore
//! use ultimaforge_lib::launcher::{ClientLauncher, LaunchConfig};
//! use std::path::Path;
//!
//! let launcher = ClientLauncher::new(Path::new("/game/uo"), "client.exe");
//! launcher.launch_with_args(&["--server", "127.0.0.1"]).await?;
//! ```

use crate::error::LaunchError;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use tracing::{debug, error, info, warn};

/// Configuration for launching the UO client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchConfig {
    /// Path to the client executable relative to install directory.
    pub executable: String,

    /// Additional command-line arguments to pass to the client.
    #[serde(default)]
    pub args: Vec<String>,

    /// Whether to wait for the client to exit.
    #[serde(default)]
    pub wait_for_exit: bool,

    /// Environment variables to set for the client process.
    #[serde(default)]
    pub env_vars: Vec<(String, String)>,
}

impl LaunchConfig {
    /// Creates a new launch configuration with the given executable.
    pub fn new(executable: impl Into<String>) -> Self {
        Self {
            executable: executable.into(),
            args: Vec::new(),
            wait_for_exit: false,
            env_vars: Vec::new(),
        }
    }

    /// Adds command-line arguments.
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Sets whether to wait for the client to exit.
    pub fn wait_for_exit(mut self, wait: bool) -> Self {
        self.wait_for_exit = wait;
        self
    }

    /// Adds an environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    /// Validates the configuration.
    pub fn validate(&self) -> Result<(), LaunchError> {
        // Validate executable path
        if self.executable.is_empty() {
            return Err(LaunchError::ExecutableNotFound {
                path: PathBuf::from(""),
            });
        }

        // Check for path traversal attempts
        if self.executable.contains("..") {
            return Err(LaunchError::NotExecutable {
                path: PathBuf::from(&self.executable),
            });
        }

        Ok(())
    }
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self::new("client.exe")
    }
}

/// Result of a client launch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchResult {
    /// Whether the launch was successful.
    pub success: bool,

    /// Process ID of the launched client (if available).
    pub pid: Option<u32>,

    /// Exit code if wait_for_exit was true and process completed.
    pub exit_code: Option<i32>,

    /// Error message if launch failed.
    pub error_message: Option<String>,
}

impl LaunchResult {
    /// Creates a successful launch result.
    pub fn success(pid: u32) -> Self {
        Self {
            success: true,
            pid: Some(pid),
            exit_code: None,
            error_message: None,
        }
    }

    /// Creates a successful launch result with exit code.
    pub fn success_with_exit(pid: u32, exit_code: i32) -> Self {
        Self {
            success: true,
            pid: Some(pid),
            exit_code: Some(exit_code),
            error_message: None,
        }
    }

    /// Creates a failed launch result.
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            success: false,
            pid: None,
            exit_code: None,
            error_message: Some(error.into()),
        }
    }
}

/// Client launcher for spawning the UO client process.
pub struct ClientLauncher {
    /// Installation directory containing the client.
    install_path: PathBuf,

    /// Default launch configuration.
    config: LaunchConfig,
}

impl ClientLauncher {
    /// Creates a new client launcher.
    ///
    /// # Arguments
    ///
    /// * `install_path` - Path to the installation directory
    /// * `executable` - Name of the client executable (relative to install_path)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let launcher = ClientLauncher::new(Path::new("/game/uo"), "client.exe");
    /// ```
    pub fn new(install_path: impl AsRef<Path>, executable: impl Into<String>) -> Self {
        Self {
            install_path: install_path.as_ref().to_path_buf(),
            config: LaunchConfig::new(executable),
        }
    }

    /// Creates a new client launcher with a full configuration.
    pub fn with_config(install_path: impl AsRef<Path>, config: LaunchConfig) -> Self {
        Self {
            install_path: install_path.as_ref().to_path_buf(),
            config,
        }
    }

    /// Returns the full path to the client executable.
    pub fn executable_path(&self) -> PathBuf {
        self.install_path.join(&self.config.executable)
    }

    /// Returns the installation path.
    pub fn install_path(&self) -> &Path {
        &self.install_path
    }

    /// Returns the current launch configuration.
    pub fn config(&self) -> &LaunchConfig {
        &self.config
    }

    /// Validates that the client can be launched.
    ///
    /// This checks:
    /// - Installation directory exists
    /// - Executable file exists
    /// - Executable appears to be valid
    pub fn validate(&self) -> Result<(), LaunchError> {
        // Validate configuration
        self.config.validate()?;

        // Check installation directory exists
        if !self.install_path.exists() {
            return Err(LaunchError::NoInstallPath);
        }

        if !self.install_path.is_dir() {
            return Err(LaunchError::NoInstallPath);
        }

        // Check executable exists
        let exe_path = self.executable_path();
        if !exe_path.exists() {
            return Err(LaunchError::ExecutableNotFound { path: exe_path });
        }

        // Check it's a file, not a directory
        if !exe_path.is_file() {
            return Err(LaunchError::NotExecutable { path: exe_path });
        }

        // Platform-specific executable validation
        #[cfg(target_os = "windows")]
        {
            // On Windows, check for .exe extension
            if let Some(ext) = exe_path.extension() {
                let ext_lower = ext.to_string_lossy().to_lowercase();
                if ext_lower != "exe" && ext_lower != "bat" && ext_lower != "cmd" {
                    warn!(
                        "Executable '{}' has unusual extension: {}",
                        exe_path.display(),
                        ext_lower
                    );
                }
            }
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            // On Unix, check execute permission
            if let Ok(metadata) = std::fs::metadata(&exe_path) {
                let permissions = metadata.permissions();
                if permissions.mode() & 0o111 == 0 {
                    return Err(LaunchError::NotExecutable { path: exe_path });
                }
            }
        }

        Ok(())
    }

    /// Launches the client executable.
    ///
    /// # Returns
    ///
    /// A `LaunchResult` containing the process ID or error information.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let launcher = ClientLauncher::new("/game/uo", "client.exe");
    /// let result = launcher.launch()?;
    /// println!("Launched with PID: {:?}", result.pid);
    /// ```
    pub fn launch(&self) -> Result<LaunchResult, LaunchError> {
        self.launch_with_args(&self.config.args)
    }

    /// Spawns the client process without waiting and returns the child handle.
    pub fn spawn_child(&self) -> Result<Child, LaunchError> {
        self.spawn_child_with_args(&self.config.args)
    }

    /// Launches the client executable with custom arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments to pass to the client
    ///
    /// # Returns
    ///
    /// A `LaunchResult` containing the process ID or error information.
    pub fn launch_with_args(&self, args: &[String]) -> Result<LaunchResult, LaunchError> {
        let child = self.spawn_child_with_args(args)?;
        let pid = child.id();
        info!("Client launched successfully with PID: {}", pid);

        // If configured to wait, do so
        if self.config.wait_for_exit {
            self.wait_for_child(child, pid)
        } else {
            Ok(LaunchResult::success(pid))
        }
    }

    /// Spawns the client executable with custom arguments and returns the child handle.
    pub fn spawn_child_with_args(&self, args: &[String]) -> Result<Child, LaunchError> {
        // Validate before launching
        self.validate()?;

        let exe_path = self.executable_path();

        info!(
            "Launching client: {} with {} args",
            exe_path.display(),
            args.len()
        );
        debug!("Working directory: {}", self.install_path.display());
        debug!("Arguments: {:?}", args);

        // Build the command
        let mut command = Command::new(&exe_path);

        // Set working directory to install path
        command.current_dir(&self.install_path);

        // Add arguments
        command.args(args);

        // Add environment variables
        for (key, value) in &self.config.env_vars {
            command.env(key, value);
            debug!("Setting env: {}={}", key, value);
        }

        // Configure stdio
        command.stdin(Stdio::null());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());

        // Spawn the process
        command.spawn().map_err(|e| {
            error!("Failed to spawn client process: {}", e);

            // Check for specific error types
            if e.kind() == std::io::ErrorKind::NotFound {
                LaunchError::ExecutableNotFound { path: exe_path.clone() }
            } else if e.kind() == std::io::ErrorKind::PermissionDenied {
                LaunchError::NotExecutable { path: exe_path.clone() }
            } else {
                LaunchError::ProcessSpawnFailed { source: e }
            }
        })
    }

    /// Waits for a child process to complete.
    fn wait_for_child(&self, mut child: Child, pid: u32) -> Result<LaunchResult, LaunchError> {
        info!("Waiting for client (PID: {}) to exit...", pid);

        match child.wait() {
            Ok(status) => {
                let exit_code = status.code().unwrap_or(-1);
                info!("Client exited with code: {}", exit_code);

                // Check for crash (non-zero exit immediately might indicate a problem)
                // We don't treat non-zero as an error here since games often exit with
                // non-zero codes for legitimate reasons
                Ok(LaunchResult::success_with_exit(pid, exit_code))
            }
            Err(e) => {
                error!("Error waiting for client: {}", e);
                Err(LaunchError::ProcessSpawnFailed { source: e })
            }
        }
    }

    /// Launches the client and returns immediately without waiting.
    ///
    /// This is equivalent to calling `launch()` with `wait_for_exit = false`.
    pub fn launch_detached(&self) -> Result<LaunchResult, LaunchError> {
        let mut config = self.config.clone();
        config.wait_for_exit = false;

        let launcher = ClientLauncher::with_config(&self.install_path, config);
        launcher.launch()
    }
}

/// Convenience function to launch a client.
///
/// # Arguments
///
/// * `install_path` - Path to the installation directory
/// * `executable` - Name of the client executable
/// * `args` - Command-line arguments
///
/// # Returns
///
/// A `LaunchResult` with the launch outcome.
///
/// # Example
///
/// ```ignore
/// use ultimaforge_lib::launcher::launch_client;
/// use std::path::Path;
///
/// let result = launch_client(
///     Path::new("/game/uo"),
///     "client.exe",
///     &["--server".to_string(), "127.0.0.1".to_string()],
/// )?;
/// ```
pub fn launch_client(
    install_path: &Path,
    executable: &str,
    args: &[String],
) -> Result<LaunchResult, LaunchError> {
    let launcher = ClientLauncher::new(install_path, executable);
    launcher.launch_with_args(args)
}

/// Validates that a client can be launched without actually launching it.
///
/// # Arguments
///
/// * `install_path` - Path to the installation directory
/// * `executable` - Name of the client executable
///
/// # Returns
///
/// `Ok(())` if the client can be launched, `Err(LaunchError)` otherwise.
pub fn validate_client(install_path: &Path, executable: &str) -> Result<(), LaunchError> {
    let launcher = ClientLauncher::new(install_path, executable);
    launcher.validate()
}

/// Result type alias for launch operations.
pub type LaunchResultType<T> = Result<T, LaunchError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    /// Creates a test installation directory with a mock executable.
    fn create_test_install() -> TempDir {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create a mock executable file
        #[cfg(target_os = "windows")]
        {
            let exe_path = temp_dir.path().join("client.exe");
            File::create(&exe_path).expect("Failed to create mock executable");
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let exe_path = temp_dir.path().join("client.exe");
            File::create(&exe_path).expect("Failed to create mock executable");

            // Make it executable
            let mut perms = fs::metadata(&exe_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&exe_path, perms).expect("Failed to set permissions");
        }

        temp_dir
    }

    #[test]
    fn test_launch_config_new() {
        let config = LaunchConfig::new("client.exe");
        assert_eq!(config.executable, "client.exe");
        assert!(config.args.is_empty());
        assert!(!config.wait_for_exit);
        assert!(config.env_vars.is_empty());
    }

    #[test]
    fn test_launch_config_with_args() {
        let config = LaunchConfig::new("client.exe")
            .with_args(vec!["--server".to_string(), "127.0.0.1".to_string()]);

        assert_eq!(config.args.len(), 2);
        assert_eq!(config.args[0], "--server");
        assert_eq!(config.args[1], "127.0.0.1");
    }

    #[test]
    fn test_launch_config_wait_for_exit() {
        let config = LaunchConfig::new("client.exe").wait_for_exit(true);
        assert!(config.wait_for_exit);

        let config = LaunchConfig::new("client.exe").wait_for_exit(false);
        assert!(!config.wait_for_exit);
    }

    #[test]
    fn test_launch_config_with_env() {
        let config = LaunchConfig::new("client.exe")
            .with_env("UO_SERVER", "127.0.0.1")
            .with_env("UO_PORT", "2593");

        assert_eq!(config.env_vars.len(), 2);
        assert_eq!(config.env_vars[0], ("UO_SERVER".to_string(), "127.0.0.1".to_string()));
        assert_eq!(config.env_vars[1], ("UO_PORT".to_string(), "2593".to_string()));
    }

    #[test]
    fn test_launch_config_validate_empty_executable() {
        let config = LaunchConfig::new("");
        let result = config.validate();
        assert!(matches!(result, Err(LaunchError::ExecutableNotFound { .. })));
    }

    #[test]
    fn test_launch_config_validate_path_traversal() {
        let config = LaunchConfig::new("../../../etc/passwd");
        let result = config.validate();
        assert!(matches!(result, Err(LaunchError::NotExecutable { .. })));

        let config = LaunchConfig::new("..\\..\\windows\\system32\\cmd.exe");
        let result = config.validate();
        assert!(matches!(result, Err(LaunchError::NotExecutable { .. })));
    }

    #[test]
    fn test_launch_config_validate_valid() {
        let config = LaunchConfig::new("client.exe");
        assert!(config.validate().is_ok());

        let config = LaunchConfig::new("subdir/client.exe");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_launch_config_default() {
        let config = LaunchConfig::default();
        assert_eq!(config.executable, "client.exe");
    }

    #[test]
    fn test_launch_result_success() {
        let result = LaunchResult::success(1234);
        assert!(result.success);
        assert_eq!(result.pid, Some(1234));
        assert!(result.exit_code.is_none());
        assert!(result.error_message.is_none());
    }

    #[test]
    fn test_launch_result_success_with_exit() {
        let result = LaunchResult::success_with_exit(1234, 0);
        assert!(result.success);
        assert_eq!(result.pid, Some(1234));
        assert_eq!(result.exit_code, Some(0));
    }

    #[test]
    fn test_launch_result_failed() {
        let result = LaunchResult::failed("Test error");
        assert!(!result.success);
        assert!(result.pid.is_none());
        assert_eq!(result.error_message, Some("Test error".to_string()));
    }

    #[test]
    fn test_client_launcher_new() {
        let launcher = ClientLauncher::new("/game/uo", "client.exe");
        assert_eq!(launcher.install_path(), Path::new("/game/uo"));
        assert_eq!(launcher.config().executable, "client.exe");
    }

    #[test]
    fn test_client_launcher_with_config() {
        let config = LaunchConfig::new("custom.exe")
            .with_args(vec!["--test".to_string()])
            .wait_for_exit(true);

        let launcher = ClientLauncher::with_config("/game/uo", config);
        assert_eq!(launcher.config().executable, "custom.exe");
        assert!(launcher.config().wait_for_exit);
    }

    #[test]
    fn test_client_launcher_executable_path() {
        let launcher = ClientLauncher::new("/game/uo", "client.exe");
        assert_eq!(launcher.executable_path(), PathBuf::from("/game/uo/client.exe"));

        let launcher = ClientLauncher::new("/game/uo", "subfolder/client.exe");
        assert_eq!(
            launcher.executable_path(),
            PathBuf::from("/game/uo/subfolder/client.exe")
        );
    }

    #[test]
    fn test_client_launcher_validate_no_install_path() {
        let launcher = ClientLauncher::new("/nonexistent/path/12345", "client.exe");
        let result = launcher.validate();
        assert!(matches!(result, Err(LaunchError::NoInstallPath)));
    }

    #[test]
    fn test_client_launcher_validate_no_executable() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let launcher = ClientLauncher::new(temp_dir.path(), "missing.exe");
        let result = launcher.validate();
        assert!(matches!(result, Err(LaunchError::ExecutableNotFound { .. })));
    }

    #[test]
    fn test_client_launcher_validate_valid() {
        let temp_dir = create_test_install();
        let launcher = ClientLauncher::new(temp_dir.path(), "client.exe");
        assert!(launcher.validate().is_ok());
    }

    #[test]
    fn test_client_launcher_validate_directory_not_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let exe_dir = temp_dir.path().join("client.exe");
        fs::create_dir(&exe_dir).expect("Failed to create directory");

        let launcher = ClientLauncher::new(temp_dir.path(), "client.exe");
        let result = launcher.validate();
        assert!(matches!(result, Err(LaunchError::NotExecutable { .. })));
    }

    #[test]
    fn test_client_launcher_validate_install_path_is_file() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let file_path = temp_dir.path().join("notadir");
        File::create(&file_path).expect("Failed to create file");

        let launcher = ClientLauncher::new(&file_path, "client.exe");
        let result = launcher.validate();
        assert!(matches!(result, Err(LaunchError::NoInstallPath)));
    }

    #[test]
    fn test_validate_client_function() {
        let temp_dir = create_test_install();
        assert!(validate_client(temp_dir.path(), "client.exe").is_ok());
        assert!(validate_client(temp_dir.path(), "missing.exe").is_err());
    }

    #[test]
    fn test_launch_config_serialization() {
        let config = LaunchConfig::new("client.exe")
            .with_args(vec!["--server".to_string(), "127.0.0.1".to_string()])
            .with_env("TEST", "value");

        let json = serde_json::to_string(&config).expect("Should serialize");
        let parsed: LaunchConfig = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(config, parsed);
    }

    #[test]
    fn test_launch_result_serialization() {
        let result = LaunchResult::success(1234);
        let json = serde_json::to_string(&result).expect("Should serialize");
        let parsed: LaunchResult = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(result.success, parsed.success);
        assert_eq!(result.pid, parsed.pid);
    }

    #[test]
    fn test_launch_config_chaining() {
        let config = LaunchConfig::new("client.exe")
            .with_args(vec!["arg1".to_string()])
            .wait_for_exit(true)
            .with_env("KEY1", "VALUE1")
            .with_env("KEY2", "VALUE2");

        assert_eq!(config.executable, "client.exe");
        assert_eq!(config.args.len(), 1);
        assert!(config.wait_for_exit);
        assert_eq!(config.env_vars.len(), 2);
    }

    #[test]
    fn test_launch_error_from_executable_not_found() {
        let launcher = ClientLauncher::new("/nonexistent", "test.exe");
        let result = launcher.launch();

        assert!(result.is_err());
        match result {
            Err(LaunchError::NoInstallPath) => {}
            Err(other) => panic!("Unexpected error type: {:?}", other),
            Ok(_) => panic!("Expected error"),
        }
    }

    #[test]
    fn test_client_launcher_accessors() {
        let temp_dir = create_test_install();
        let config = LaunchConfig::new("client.exe").wait_for_exit(true);
        let launcher = ClientLauncher::with_config(temp_dir.path(), config);

        assert_eq!(launcher.install_path(), temp_dir.path());
        assert_eq!(launcher.config().executable, "client.exe");
        assert!(launcher.config().wait_for_exit);
    }

    #[cfg(unix)]
    #[test]
    fn test_unix_executable_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let exe_path = temp_dir.path().join("client.exe");

        // Create file without execute permission
        File::create(&exe_path).expect("Failed to create file");
        let mut perms = fs::metadata(&exe_path).unwrap().permissions();
        perms.set_mode(0o644); // Read/write, no execute
        fs::set_permissions(&exe_path, perms).expect("Failed to set permissions");

        let launcher = ClientLauncher::new(temp_dir.path(), "client.exe");
        let result = launcher.validate();

        assert!(matches!(result, Err(LaunchError::NotExecutable { .. })));
    }

    #[test]
    fn test_subdirectory_executable() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let subdir = temp_dir.path().join("bin");
        fs::create_dir(&subdir).expect("Failed to create subdirectory");

        // Create executable in subdirectory
        #[cfg(target_os = "windows")]
        {
            let exe_path = subdir.join("client.exe");
            File::create(&exe_path).expect("Failed to create mock executable");
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let exe_path = subdir.join("client.exe");
            File::create(&exe_path).expect("Failed to create mock executable");

            let mut perms = fs::metadata(&exe_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&exe_path, perms).expect("Failed to set permissions");
        }

        let launcher = ClientLauncher::new(temp_dir.path(), "bin/client.exe");
        assert!(launcher.validate().is_ok());
        assert_eq!(
            launcher.executable_path(),
            temp_dir.path().join("bin/client.exe")
        );
    }

    #[test]
    fn test_launch_client_function() {
        // This test just validates the function signature and basic validation
        let result = launch_client(
            Path::new("/nonexistent"),
            "client.exe",
            &[],
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_launch_config_deserialization_with_defaults() {
        let json = r#"{"executable": "test.exe"}"#;
        let config: LaunchConfig = serde_json::from_str(json).expect("Should deserialize");

        assert_eq!(config.executable, "test.exe");
        assert!(config.args.is_empty());
        assert!(!config.wait_for_exit);
        assert!(config.env_vars.is_empty());
    }

    #[test]
    fn test_multiple_env_vars() {
        let config = LaunchConfig::new("client.exe")
            .with_env("VAR1", "value1")
            .with_env("VAR2", "value2")
            .with_env("VAR3", "value3");

        assert_eq!(config.env_vars.len(), 3);

        // Verify order is preserved
        assert_eq!(config.env_vars[0].0, "VAR1");
        assert_eq!(config.env_vars[1].0, "VAR2");
        assert_eq!(config.env_vars[2].0, "VAR3");
    }

    #[test]
    fn test_launch_result_types() {
        // Test that LaunchResult can be serialized/deserialized
        let success = LaunchResult::success(100);
        let success_exit = LaunchResult::success_with_exit(200, 0);
        let failed = LaunchResult::failed("error message");

        // Verify all fields
        assert!(success.success);
        assert_eq!(success.pid, Some(100));
        assert!(success.exit_code.is_none());

        assert!(success_exit.success);
        assert_eq!(success_exit.pid, Some(200));
        assert_eq!(success_exit.exit_code, Some(0));

        assert!(!failed.success);
        assert!(failed.pid.is_none());
        assert_eq!(failed.error_message, Some("error message".to_string()));
    }

    #[test]
    fn test_path_with_spaces() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let space_dir = temp_dir.path().join("path with spaces");
        fs::create_dir(&space_dir).expect("Failed to create directory");

        #[cfg(target_os = "windows")]
        {
            let exe_path = space_dir.join("client game.exe");
            File::create(&exe_path).expect("Failed to create mock executable");
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let exe_path = space_dir.join("client game.exe");
            File::create(&exe_path).expect("Failed to create mock executable");

            let mut perms = fs::metadata(&exe_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&exe_path, perms).expect("Failed to set permissions");
        }

        let launcher = ClientLauncher::new(&space_dir, "client game.exe");
        assert!(launcher.validate().is_ok());
    }
}
