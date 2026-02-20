//! ClassicUO settings.json management.
//!
//! The launcher owns exactly five fields in ClassicUO's settings.json and
//! writes them right before each launch. All other fields are left untouched.

use crate::config::{AssistantKind, CuoConfig, ServerChoice};
use serde_json::Value;
use std::path::Path;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum CuoSettingsError {
    #[error("Failed to read settings.json: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("settings.json is invalid JSON: {0}")]
    ParseError(#[from] serde_json::Error),
    #[error("Install path has no string representation")]
    InvalidPath,
}

/// Writes the five launcher-managed fields into ClassicUO's settings.json.
///
/// Reads the existing file (or starts with `{}`), patches only the managed
/// fields, and writes it back. All other fields are preserved unchanged.
pub fn write_cuo_settings(
    install_path: &Path,
    cuo_config: &CuoConfig,
    server_choice: &ServerChoice,
    assistant: &AssistantKind,
) -> Result<(), CuoSettingsError> {
    let settings_path = install_path.join("settings.json");

    let mut json: Value = if settings_path.exists() {
        let text = std::fs::read_to_string(&settings_path)?;
        serde_json::from_str(&text)?
    } else {
        Value::Object(serde_json::Map::new())
    };

    let server = match server_choice {
        ServerChoice::Test => cuo_config
            .test_server
            .as_ref()
            .unwrap_or(&cuo_config.live_server),
        ServerChoice::Live => &cuo_config.live_server,
    };

    let plugins = assistant_plugins(install_path, assistant)?;

    let obj = json
        .as_object_mut()
        .ok_or(CuoSettingsError::InvalidPath)?;
    obj.insert("ip".into(), Value::String(server.ip.clone()));
    obj.insert("port".into(), Value::Number(server.port.into()));
    obj.insert("ultimaonlinedirectory".into(), Value::String(".\\Files".into()));
    obj.insert(
        "clientversion".into(),
        Value::String(cuo_config.client_version.clone()),
    );
    obj.insert("plugins".into(), Value::Array(plugins));

    info!("Writing CUO settings: ip={}, port={}", server.ip, server.port);

    let text = serde_json::to_string_pretty(&json)?;
    std::fs::write(&settings_path, text)?;
    Ok(())
}

/// Returns the plugins array value for the given assistant.
fn assistant_plugins(
    install_path: &Path,
    assistant: &AssistantKind,
) -> Result<Vec<Value>, CuoSettingsError> {
    let path_value = |rel: &Path| -> Result<Value, CuoSettingsError> {
        let p = install_path.join(rel);
        Ok(Value::String(
            p.to_str().ok_or(CuoSettingsError::InvalidPath)?.to_string(),
        ))
    };

    match assistant {
        AssistantKind::RazorEnhanced => Ok(vec![path_value(
            Path::new("Data")
                .join("Plugins")
                .join("RazorEnhanced")
                .join("RazorEnhanced.exe")
                .as_path(),
        )?]),
        AssistantKind::Razor => Ok(vec![path_value(
            Path::new("Data")
                .join("Plugins")
                .join("Razor")
                .join("Razor.exe")
                .as_path(),
        )?]),
        AssistantKind::None => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ServerConfig;
    use tempfile::TempDir;

    fn test_cuo_config() -> CuoConfig {
        CuoConfig {
            client_version: "7.0.10.3".into(),
            live_server: ServerConfig {
                label: "Live".into(),
                ip: "live.example.com".into(),
                port: 2593,
            },
            test_server: Some(ServerConfig {
                label: "TC".into(),
                ip: "tc.example.com".into(),
                port: 2594,
            }),
            available_assistants: vec![AssistantKind::RazorEnhanced, AssistantKind::Razor],
            default_assistant: AssistantKind::RazorEnhanced,
            default_server: ServerChoice::Live,
        }
    }

    #[test]
    fn test_creates_settings_json_when_missing() {
        let dir = TempDir::new().unwrap();
        let config = test_cuo_config();

        write_cuo_settings(dir.path(), &config, &ServerChoice::Live, &AssistantKind::RazorEnhanced)
            .expect("Should create settings.json");

        let text = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["ip"], "live.example.com");
        assert_eq!(json["port"], 2593);
        assert_eq!(json["ultimaonlinedirectory"], ".\\Files");
        assert_eq!(json["clientversion"], "7.0.10.3");
        assert!(json["plugins"].as_array().unwrap().len() == 1);
        assert!(json["plugins"][0]
            .as_str()
            .unwrap()
            .contains("RazorEnhanced"));
    }

    #[test]
    fn test_patches_only_managed_fields() {
        let dir = TempDir::new().unwrap();

        let existing = serde_json::json!({
            "ip": "old.server.com",
            "port": 9999,
            "fps": 250,
            "username": "crameep",
            "window_size": {"X": 3440, "Y": 1369},
            "plugins": ["old_path/Razor.exe"]
        });
        std::fs::write(
            dir.path().join("settings.json"),
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .unwrap();

        let config = test_cuo_config();
        write_cuo_settings(dir.path(), &config, &ServerChoice::Live, &AssistantKind::Razor)
            .expect("Should patch");

        let text = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();

        assert_eq!(json["ip"], "live.example.com");
        assert_eq!(json["port"], 2593);
        assert!(json["plugins"][0]
            .as_str()
            .unwrap()
            .contains("Razor.exe"));

        assert_eq!(json["fps"], 250);
        assert_eq!(json["username"], "crameep");
        assert_eq!(json["window_size"]["X"], 3440);
    }

    #[test]
    fn test_test_server_selection() {
        let dir = TempDir::new().unwrap();
        let config = test_cuo_config();

        write_cuo_settings(dir.path(), &config, &ServerChoice::Test, &AssistantKind::None)
            .expect("Should write");

        let text = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["ip"], "tc.example.com");
        assert_eq!(json["port"], 2594);
        assert_eq!(json["plugins"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_no_assistant() {
        let dir = TempDir::new().unwrap();
        let config = test_cuo_config();

        write_cuo_settings(dir.path(), &config, &ServerChoice::Live, &AssistantKind::None)
            .expect("Should write");

        let text = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["plugins"].as_array().unwrap().len(), 0);
    }
}
