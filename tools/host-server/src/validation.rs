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

        Self {
            valid,
            manifest_exists,
            signature_exists,
            files_dir_exists,
            message,
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
    }

    #[test]
    fn test_validation_result_serialization() {
        let result = ValidationResult::valid();
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"valid\":true"));
        assert!(json.contains("\"manifest_exists\":true"));
        assert!(json.contains("\"signature_exists\":true"));
        assert!(json.contains("\"files_dir_exists\":true"));
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
    }

    #[test]
    fn test_validate_complete_directory() {
        let dir = tempdir().unwrap();

        // Create all required files
        fs::write(dir.path().join("manifest.json"), "{}").unwrap();
        fs::write(dir.path().join("manifest.sig"), "sig").unwrap();
        fs::create_dir(dir.path().join("files")).unwrap();

        let result = ValidationResult::validate(dir.path());

        assert!(result.valid);
        assert!(result.manifest_exists);
        assert!(result.signature_exists);
        assert!(result.files_dir_exists);
        assert_eq!(result.message, "Update folder structure is valid");
    }
}
