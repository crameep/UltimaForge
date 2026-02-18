//! Validation module for UltimaForge Host Server
//!
//! Provides validation endpoint to check update folder structure.

use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use serde::Serialize;
use std::{path::Path, sync::Arc};

use crate::routes::AppState;

/// Validation result response
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    /// Whether the update folder structure is valid
    pub valid: bool,
    /// Whether manifest.json exists
    pub manifest_exists: bool,
    /// Whether manifest.sig exists
    pub signature_exists: bool,
    /// Whether files/ directory exists
    pub files_dir_exists: bool,
    /// Human-readable status message
    pub message: String,
    /// Whether launcher update folder structure is valid
    pub launcher_valid: bool,
    /// Whether launcher/ directory exists
    pub launcher_dir_exists: bool,
    /// Whether launcher/latest.json exists
    pub launcher_latest_exists: bool,
    /// Whether launcher/files/ directory exists
    pub launcher_files_dir_exists: bool,
    /// Human-readable launcher status message
    pub launcher_message: String,
}

impl ValidationResult {
    /// Validate an update folder structure
    pub fn validate(serve_dir: &Path) -> Self {
        let manifest_exists = serve_dir.join("manifest.json").exists();
        let signature_exists = serve_dir.join("manifest.sig").exists();
        let files_dir_exists = serve_dir.join("files").is_dir();

        let valid = manifest_exists && signature_exists && files_dir_exists;
        let message = if valid {
            "Update folder structure is valid".to_string()
        } else {
            let mut missing = Vec::new();
            if !manifest_exists {
                missing.push("manifest.json");
            }
            if !signature_exists {
                missing.push("manifest.sig");
            }
            if !files_dir_exists {
                missing.push("files/");
            }
            format!("Missing required files: {}", missing.join(", "))
        };

        let launcher_dir = serve_dir.join("launcher");
        let launcher_dir_exists = launcher_dir.is_dir();
        let launcher_latest_exists = launcher_dir.join("latest.json").exists();
        let launcher_files_dir_exists = launcher_dir.join("files").is_dir();

        let launcher_valid =
            launcher_dir_exists && launcher_latest_exists && launcher_files_dir_exists;
        let launcher_message = if launcher_valid {
            "Launcher update folder structure is valid".to_string()
        } else if !launcher_dir_exists {
            "Launcher updates not configured (missing launcher/ folder)".to_string()
        } else {
            let mut missing = Vec::new();
            if !launcher_latest_exists {
                missing.push("launcher/latest.json");
            }
            if !launcher_files_dir_exists {
                missing.push("launcher/files/");
            }
            format!("Missing launcher update files: {}", missing.join(", "))
        };

        Self {
            valid,
            manifest_exists,
            signature_exists,
            files_dir_exists,
            message,
            launcher_valid,
            launcher_dir_exists,
            launcher_latest_exists,
            launcher_files_dir_exists,
            launcher_message,
        }
    }

    /// Create a valid result (for testing)
    #[allow(dead_code)]
    pub fn valid() -> Self {
        Self {
            valid: true,
            manifest_exists: true,
            signature_exists: true,
            files_dir_exists: true,
            message: "Update folder structure is valid".to_string(),
            launcher_valid: true,
            launcher_dir_exists: true,
            launcher_latest_exists: true,
            launcher_files_dir_exists: true,
            launcher_message: "Launcher update folder structure is valid".to_string(),
        }
    }

    /// Create an invalid result with missing files (for testing)
    #[allow(dead_code)]
    pub fn invalid(missing: &[&str]) -> Self {
        Self {
            valid: false,
            manifest_exists: !missing.contains(&"manifest.json"),
            signature_exists: !missing.contains(&"manifest.sig"),
            files_dir_exists: !missing.contains(&"files/"),
            message: format!("Missing required files: {}", missing.join(", ")),
            launcher_valid: false,
            launcher_dir_exists: !missing.contains(&"launcher/"),
            launcher_latest_exists: !missing.contains(&"launcher/latest.json"),
            launcher_files_dir_exists: !missing.contains(&"launcher/files/"),
            launcher_message: "Launcher update folder structure is invalid".to_string(),
        }
    }
}

/// Validate update folder structure endpoint handler
///
/// Checks that all required files exist:
/// - manifest.json
/// - manifest.sig
/// - files/ directory
///
/// Returns JSON response:
/// ```json
/// {
///   "valid": true,
///   "manifest_exists": true,
///   "signature_exists": true,
///   "files_dir_exists": true,
///   "message": "Update folder structure is valid"
/// }
/// ```
pub async fn validate_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(ValidationResult::validate(&state.serve_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_validation_result_valid() {
        let result = ValidationResult::valid();
        assert!(result.valid);
        assert!(result.manifest_exists);
        assert!(result.signature_exists);
        assert!(result.files_dir_exists);
        assert_eq!(result.message, "Update folder structure is valid");
        assert!(result.launcher_valid);
        assert!(result.launcher_dir_exists);
        assert!(result.launcher_latest_exists);
        assert!(result.launcher_files_dir_exists);
        assert_eq!(
            result.launcher_message,
            "Launcher update folder structure is valid"
        );
    }

    #[test]
    fn test_validation_result_invalid() {
        let result = ValidationResult::invalid(&["manifest.json", "files/"]);
        assert!(!result.valid);
        assert!(!result.manifest_exists);
        assert!(result.signature_exists);
        assert!(!result.files_dir_exists);
        assert!(result.message.contains("manifest.json"));
        assert!(result.message.contains("files/"));
        assert!(!result.launcher_valid);
    }

    #[test]
    fn test_validation_result_serialization() {
        let result = ValidationResult::valid();
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"valid\":true"));
        assert!(json.contains("\"manifest_exists\":true"));
        assert!(json.contains("\"signature_exists\":true"));
        assert!(json.contains("\"files_dir_exists\":true"));
        assert!(json.contains("\"launcher_valid\":true"));
    }

    #[test]
    fn test_validate_empty_directory() {
        let dir = tempdir().unwrap();
        let result = ValidationResult::validate(dir.path());

        assert!(!result.valid);
        assert!(!result.manifest_exists);
        assert!(!result.signature_exists);
        assert!(!result.files_dir_exists);
        assert!(result.message.contains("manifest.json"));
        assert!(result.message.contains("manifest.sig"));
        assert!(result.message.contains("files/"));
        assert!(!result.launcher_valid);
        assert!(!result.launcher_dir_exists);
        assert!(!result.launcher_latest_exists);
        assert!(!result.launcher_files_dir_exists);
    }

    #[test]
    fn test_validate_partial_directory() {
        let dir = tempdir().unwrap();

        // Create manifest.json only
        fs::write(dir.path().join("manifest.json"), "{}").unwrap();

        let result = ValidationResult::validate(dir.path());

        assert!(!result.valid);
        assert!(result.manifest_exists);
        assert!(!result.signature_exists);
        assert!(!result.files_dir_exists);
        assert!(!result.message.contains("manifest.json"));
        assert!(result.message.contains("manifest.sig"));
        assert!(result.message.contains("files/"));
        assert!(!result.launcher_valid);
    }

    #[test]
    fn test_validate_complete_directory() {
        let dir = tempdir().unwrap();

        // Create all required files
        fs::write(dir.path().join("manifest.json"), "{}").unwrap();
        fs::write(dir.path().join("manifest.sig"), "sig").unwrap();
        fs::create_dir(dir.path().join("files")).unwrap();
        fs::create_dir(dir.path().join("launcher")).unwrap();
        fs::create_dir(dir.path().join("launcher/files")).unwrap();
        fs::write(dir.path().join("launcher/latest.json"), "{}").unwrap();

        let result = ValidationResult::validate(dir.path());

        assert!(result.valid);
        assert!(result.manifest_exists);
        assert!(result.signature_exists);
        assert!(result.files_dir_exists);
        assert_eq!(result.message, "Update folder structure is valid");
        assert!(result.launcher_valid);
    }
}
