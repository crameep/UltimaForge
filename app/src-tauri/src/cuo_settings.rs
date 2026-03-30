//! ClassicUO settings.json management.
//!
//! The launcher owns exactly five fields in ClassicUO's settings.json and
//! writes them right before each launch. All other fields are left untouched.

use crate::config::{AssistantKind, CuoConfig, ServerChoice};
use serde_json::Value;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::info;

const UO_DATA_FILE_HINTS: &[&str] = &["art.mul", "artidx.mul", "map0.mul", "staidx0.mul", "statics0.mul"];

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
    cuo_data_root: Option<&Path>,
    cuo_config: &CuoConfig,
    server_choice: &ServerChoice,
    assistant: &AssistantKind,
) -> Result<(), CuoSettingsError> {
    let settings_path = cuo_data_root
        .map(|root| root.join("settings.json"))
        .unwrap_or_else(|| install_path.join("settings.json"));

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

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

    let plugin_root = cuo_data_root.unwrap_or(install_path);
    let plugins = assistant_plugins(plugin_root, assistant, cuo_data_root.is_some())?;

    let obj = json
        .as_object_mut()
        .ok_or(CuoSettingsError::InvalidPath)?;
    obj.insert("ip".into(), Value::String(server.ip.clone()));
    obj.insert("port".into(), Value::Number(server.port.into()));
    let uo_path_value = if cuo_data_root.is_some() {
        resolve_uo_data_directory(install_path).display().to_string()
    } else {
        ".\\Files".to_string()
    };
    obj.insert("ultimaonlinedirectory".into(), Value::String(uo_path_value));
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

pub fn resolve_uo_data_directory(install_path: &Path) -> PathBuf {
    let files_dir = install_path.join("Files");
    let root_hits = count_uo_data_hints(install_path);
    let files_hits = if files_dir.is_dir() {
        count_uo_data_hints(&files_dir)
    } else {
        0
    };

    if files_hits > root_hits {
        return files_dir;
    }

    if root_hits > 0 {
        return install_path.to_path_buf();
    }

    if files_dir.is_dir() {
        return files_dir;
    }

    install_path.to_path_buf()
}

fn count_uo_data_hints(path: &Path) -> usize {
    UO_DATA_FILE_HINTS
        .iter()
        .filter(|name| path.join(name).is_file())
        .count()
}

/// Returns the plugins array value for the given assistant.
fn assistant_plugins(
    install_path: &Path,
    assistant: &AssistantKind,
    prefer_root_plugins: bool,
) -> Result<Vec<Value>, CuoSettingsError> {
    let path_value = |rel: &Path| -> Result<Value, CuoSettingsError> {
        let p = install_path.join(rel);
        Ok(Value::String(
            p.to_str().ok_or(CuoSettingsError::InvalidPath)?.to_string(),
        ))
    };

    let plugin_path_value = |assistant_dir: &str, executable: &str| -> Result<Value, CuoSettingsError> {
        let preferred = if prefer_root_plugins {
            [
                Path::new("Plugins").join(assistant_dir).join(executable),
                Path::new("Data")
                    .join("Plugins")
                    .join(assistant_dir)
                    .join(executable),
            ]
        } else {
            [
                Path::new("Data")
                    .join("Plugins")
                    .join(assistant_dir)
                    .join(executable),
                Path::new("Plugins").join(assistant_dir).join(executable),
            ]
        };

        for rel in &preferred {
            if install_path.join(rel).exists() {
                return path_value(rel.as_path());
            }
        }

        path_value(preferred[0].as_path())
    };

    match assistant {
        AssistantKind::RazorEnhanced => Ok(vec![plugin_path_value("RazorEnhanced", "RazorEnhanced.exe")?]),
        AssistantKind::Razor => Ok(vec![plugin_path_value("Razor", "Razor.exe")?]),
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

        write_cuo_settings(
            dir.path(),
            None,
            &config,
            &ServerChoice::Live,
            &AssistantKind::RazorEnhanced,
        )
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
        write_cuo_settings(
            dir.path(),
            None,
            &config,
            &ServerChoice::Live,
            &AssistantKind::Razor,
        )
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

        write_cuo_settings(
            dir.path(),
            None,
            &config,
            &ServerChoice::Test,
            &AssistantKind::None,
        )
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

        write_cuo_settings(
            dir.path(),
            None,
            &config,
            &ServerChoice::Live,
            &AssistantKind::None,
        )
            .expect("Should write");

        let text = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["plugins"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_writes_to_custom_data_root() {
        let install_dir = TempDir::new().unwrap();
        let data_dir = TempDir::new().unwrap();
        let config = test_cuo_config();

        std::fs::write(install_dir.path().join("art.mul"), b"1").unwrap();

        write_cuo_settings(
            install_dir.path(),
            Some(data_dir.path()),
            &config,
            &ServerChoice::Live,
            &AssistantKind::Razor,
        )
        .expect("Should write settings to custom root");

        let text = std::fs::read_to_string(data_dir.path().join("settings.json")).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();

        assert_eq!(
            json["ultimaonlinedirectory"],
            install_dir.path().display().to_string()
        );

        let expected_plugin = data_dir
            .path()
            .join("Plugins")
            .join("Razor")
            .join("Razor.exe")
            .display()
            .to_string();
        assert_eq!(json["plugins"][0], expected_plugin);
    }

    #[test]
    fn test_install_path_defaults_to_data_plugins_layout() {
        let install_dir = TempDir::new().unwrap();
        let config = test_cuo_config();

        write_cuo_settings(
            install_dir.path(),
            None,
            &config,
            &ServerChoice::Live,
            &AssistantKind::Razor,
        )
        .expect("Should write settings to install root");

        let text = std::fs::read_to_string(install_dir.path().join("settings.json")).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();

        let expected_plugin = install_dir
            .path()
            .join("Data")
            .join("Plugins")
            .join("Razor")
            .join("Razor.exe")
            .display()
            .to_string();
        assert_eq!(json["plugins"][0], expected_plugin);
    }

    #[test]
    fn test_install_path_uses_existing_flat_plugins_layout() {
        let install_dir = TempDir::new().unwrap();
        let config = test_cuo_config();

        let flat_plugin = install_dir
            .path()
            .join("Plugins")
            .join("Razor")
            .join("Razor.exe");
        std::fs::create_dir_all(flat_plugin.parent().unwrap()).unwrap();
        std::fs::write(&flat_plugin, b"plugin").unwrap();

        write_cuo_settings(
            install_dir.path(),
            None,
            &config,
            &ServerChoice::Live,
            &AssistantKind::Razor,
        )
        .expect("Should use existing plugin path");

        let text = std::fs::read_to_string(install_dir.path().join("settings.json")).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();

        assert_eq!(json["plugins"][0], flat_plugin.display().to_string());
    }

    #[test]
    fn test_resolve_uo_data_directory_prefers_root_when_files_empty() {
        let install_dir = TempDir::new().unwrap();
        std::fs::create_dir_all(install_dir.path().join("Files")).unwrap();
        std::fs::write(install_dir.path().join("art.mul"), b"1").unwrap();

        assert_eq!(
            resolve_uo_data_directory(install_dir.path()),
            install_dir.path().to_path_buf()
        );
    }

    #[test]
    fn test_resolve_uo_data_directory_prefers_files_when_populated() {
        let install_dir = TempDir::new().unwrap();
        let files_dir = install_dir.path().join("Files");
        std::fs::create_dir_all(&files_dir).unwrap();
        std::fs::write(files_dir.join("art.mul"), b"1").unwrap();

        assert_eq!(resolve_uo_data_directory(install_dir.path()), files_dir);
    }

    #[test]
    fn test_custom_data_root_uses_files_uo_directory_when_populated() {
        let install_dir = TempDir::new().unwrap();
        let data_dir = TempDir::new().unwrap();
        let config = test_cuo_config();

        let files_dir = install_dir.path().join("Files");
        std::fs::create_dir_all(&files_dir).unwrap();
        std::fs::write(files_dir.join("art.mul"), b"1").unwrap();

        write_cuo_settings(
            install_dir.path(),
            Some(data_dir.path()),
            &config,
            &ServerChoice::Live,
            &AssistantKind::Razor,
        )
        .expect("Should write settings for migrated install with Files data root");

        let text = std::fs::read_to_string(data_dir.path().join("settings.json")).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();

        assert_eq!(json["ultimaonlinedirectory"], files_dir.display().to_string());
    }
}
