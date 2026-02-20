# ClassicUO Client Settings Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add server/assistant selection and multi-client (multiboxing) support to the launcher, with the launcher surgically managing five fields in ClassicUO's `settings.json` before each launch.

**Architecture:** New `CuoConfig` structs in `BrandConfig` define what the server owner configures. Three new fields in `LauncherConfig` persist the player's choices. A new `cuo_settings.rs` module owns the `settings.json` merge logic. The existing launch command is extended to call that logic and spawn N processes.

**Tech Stack:** Rust (serde_json for settings.json merge), React/TypeScript (new `CuoControls` component), Tauri IPC.

**Reference:** Design doc at `docs/plans/2026-02-19-cuo-client-settings-design.md`

---

### Task 1: Add CuoConfig structs to config.rs

**Files:**
- Modify: `app/src-tauri/src/config.rs`

**Step 1: Write failing tests** (add inside the existing `#[cfg(test)]` module at the bottom of config.rs)

```rust
#[test]
fn test_parse_brand_config_with_cuo_block() {
    let json = format!(r#"{{
        "product": {{"displayName": "Test", "serverName": "Test"}},
        "updateUrl": "https://test.com",
        "publicKey": "{key}",
        "cuo": {{
            "client_version": "7.0.10.3",
            "live_server": {{"label": "Live", "ip": "live.example.com", "port": 2593}},
            "test_server": {{"label": "TC", "ip": "tc.example.com", "port": 2594}},
            "available_assistants": ["razor_enhanced", "razor"],
            "default_assistant": "razor_enhanced",
            "default_server": "live"
        }}
    }}"#, key = TEST_PUBLIC_KEY);

    let config = BrandConfig::parse_str(&json).expect("Should parse");
    let cuo = config.cuo.expect("Should have cuo config");
    assert_eq!(cuo.client_version, "7.0.10.3");
    assert_eq!(cuo.live_server.ip, "live.example.com");
    assert_eq!(cuo.live_server.port, 2593);
    assert!(cuo.test_server.is_some());
    assert_eq!(cuo.test_server.unwrap().port, 2594);
    assert_eq!(cuo.available_assistants.len(), 2);
    assert_eq!(cuo.default_assistant, AssistantKind::RazorEnhanced);
    assert_eq!(cuo.default_server, ServerChoice::Live);
}

#[test]
fn test_parse_brand_config_without_cuo_block() {
    let json = minimal_brand_json(); // already exists in tests
    let config = BrandConfig::parse_str(&json).expect("Should parse");
    assert!(config.cuo.is_none());
}

#[test]
fn test_parse_cuo_config_no_test_server() {
    let json = format!(r#"{{
        "product": {{"displayName": "Test", "serverName": "Test"}},
        "updateUrl": "https://test.com",
        "publicKey": "{key}",
        "cuo": {{
            "client_version": "7.0.10.3",
            "live_server": {{"label": "Live", "ip": "live.example.com", "port": 2593}},
            "available_assistants": ["razor_enhanced"],
            "default_assistant": "razor_enhanced",
            "default_server": "live"
        }}
    }}"#, key = TEST_PUBLIC_KEY);

    let config = BrandConfig::parse_str(&json).expect("Should parse");
    let cuo = config.cuo.expect("Should have cuo config");
    assert!(cuo.test_server.is_none());
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p ultimaforge-lib test_parse_brand_config_with_cuo_block 2>&1 | tail -5
```

Expected: compile error — `AssistantKind`, `ServerChoice`, `CuoConfig` not defined.

**Step 3: Add the new structs and fields** (add before `BrandConfig` in config.rs)

```rust
/// Which assistant program to use with ClassicUO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AssistantKind {
    /// Razor Enhanced (Python-based macro client).
    #[default]
    RazorEnhanced,
    /// Legacy Razor macro client.
    Razor,
    /// No assistant.
    None,
}

/// Which server to connect to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServerChoice {
    /// Live / production server.
    #[default]
    Live,
    /// Test Center / staging server.
    Test,
}

/// Connection details for a single server endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    /// Display label shown in the launcher dropdown.
    pub label: String,
    /// Login server hostname or IP.
    pub ip: String,
    /// Login server port.
    pub port: u16,
}

/// ClassicUO-specific configuration embedded in brand.json.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CuoConfig {
    /// UO client version string passed to ClassicUO (e.g. "7.0.10.3").
    #[serde(rename = "client_version")]
    pub client_version: String,

    /// Live / production server connection details.
    #[serde(rename = "live_server")]
    pub live_server: ServerConfig,

    /// Optional test server. If absent, no server dropdown is shown.
    #[serde(rename = "test_server", default)]
    pub test_server: Option<ServerConfig>,

    /// Which assistants are available for players to choose from.
    #[serde(rename = "available_assistants")]
    pub available_assistants: Vec<AssistantKind>,

    /// Default assistant selected on first run.
    #[serde(rename = "default_assistant", default)]
    pub default_assistant: AssistantKind,

    /// Default server selected on first run.
    #[serde(rename = "default_server", default)]
    pub default_server: ServerChoice,
}
```

Add `cuo` field to `BrandConfig`:

```rust
/// Optional ClassicUO client configuration.
#[serde(default)]
pub cuo: Option<CuoConfig>,
```

**Step 4: Run tests to verify they pass**

```bash
cargo test -p ultimaforge-lib test_parse_brand_config 2>&1 | tail -10
cargo test -p ultimaforge-lib test_parse_cuo_config 2>&1 | tail -10
```

Expected: all three new tests PASS, all existing config tests still PASS.

**Step 5: Commit**

```bash
git add app/src-tauri/src/config.rs
git commit -m "feat: add CuoConfig structs to BrandConfig"
```

---

### Task 2: Add player-choice fields to LauncherConfig

**Files:**
- Modify: `app/src-tauri/src/config.rs`

**Step 1: Write failing tests** (add to existing `#[cfg(test)]` module)

```rust
#[test]
fn test_launcher_config_cuo_defaults() {
    let config = LauncherConfig::new();
    assert_eq!(config.selected_server, ServerChoice::Live);
    assert_eq!(config.selected_assistant, AssistantKind::RazorEnhanced);
    assert_eq!(config.client_count, 1);
}

#[test]
fn test_launcher_config_cuo_serialization() {
    let mut config = LauncherConfig::new();
    config.selected_server = ServerChoice::Test;
    config.selected_assistant = AssistantKind::Razor;
    config.client_count = 3;

    let json = config.to_json_string().unwrap();
    let loaded = LauncherConfig::parse_str(&json).unwrap();

    assert_eq!(loaded.selected_server, ServerChoice::Test);
    assert_eq!(loaded.selected_assistant, AssistantKind::Razor);
    assert_eq!(loaded.client_count, 3);
}

#[test]
fn test_launcher_config_client_count_capped() {
    let mut config = LauncherConfig::new();
    config.client_count = config.client_count.min(3);
    assert!(config.client_count <= 3);
}
```

**Step 2: Run tests to verify they fail**

```bash
cargo test -p ultimaforge-lib test_launcher_config_cuo 2>&1 | tail -5
```

Expected: compile error — fields don't exist yet.

**Step 3: Add fields to `LauncherConfig`**

In the `LauncherConfig` struct, add after `client_executable`:

```rust
/// Which server the player has selected (live or test).
#[serde(rename = "selectedServer", default)]
pub selected_server: ServerChoice,

/// Which assistant the player has selected.
#[serde(rename = "selectedAssistant", default)]
pub selected_assistant: AssistantKind,

/// Number of client instances to launch (1-3).
#[serde(rename = "clientCount", default = "default_client_count")]
pub client_count: u8,
```

Add the default function near the other defaults:

```rust
fn default_client_count() -> u8 { 1 }
```

Update `LauncherConfig::default()` to include the new fields:

```rust
selected_server: ServerChoice::Live,
selected_assistant: AssistantKind::RazorEnhanced,
client_count: 1,
```

**Step 4: Run tests**

```bash
cargo test -p ultimaforge-lib 2>&1 | tail -15
```

Expected: all tests PASS including the three new ones.

**Step 5: Commit**

```bash
git add app/src-tauri/src/config.rs
git commit -m "feat: add server/assistant/client-count fields to LauncherConfig"
```

---

### Task 3: New cuo_settings.rs module

**Files:**
- Create: `app/src-tauri/src/cuo_settings.rs`
- Modify: `app/src-tauri/src/lib.rs` (add `pub mod cuo_settings;`)

This module owns the logic for reading, patching, and writing ClassicUO's `settings.json`.

**Step 1: Create the file with tests first**

```rust
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

    // Read existing or start empty
    let mut json: Value = if settings_path.exists() {
        let text = std::fs::read_to_string(&settings_path)?;
        serde_json::from_str(&text)?
    } else {
        Value::Object(serde_json::Map::new())
    };

    // Select the server
    let server = match server_choice {
        ServerChoice::Test => cuo_config
            .test_server
            .as_ref()
            .unwrap_or(&cuo_config.live_server),
        ServerChoice::Live => &cuo_config.live_server,
    };

    // Build the plugin path for the chosen assistant
    let plugins = assistant_plugins(install_path, assistant)?;

    // Patch exactly the five managed fields
    let obj = json.as_object_mut().ok_or(CuoSettingsError::InvalidPath)?;
    obj.insert("ip".into(), Value::String(server.ip.clone()));
    obj.insert("port".into(), Value::Number(server.port.into()));
    obj.insert("ultimaonlinedirectory".into(), Value::String(".\\Files".into()));
    obj.insert("clientversion".into(), Value::String(cuo_config.client_version.clone()));
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
    let path_str = |rel: &str| -> Result<Value, CuoSettingsError> {
        let p = install_path.join(rel);
        Ok(Value::String(
            p.to_str().ok_or(CuoSettingsError::InvalidPath)?.to_string(),
        ))
    };

    match assistant {
        AssistantKind::RazorEnhanced => Ok(vec![path_str(
            "Data/Plugins/RazorEnhanced/RazorEnhanced.exe",
        )?]),
        AssistantKind::Razor => Ok(vec![path_str("Data/Plugins/Razor/Razor.exe")?]),
        AssistantKind::None => Ok(vec![]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CuoConfig, ServerConfig};
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
        assert!(json["plugins"][0].as_str().unwrap().contains("RazorEnhanced"));
    }

    #[test]
    fn test_patches_only_managed_fields() {
        let dir = TempDir::new().unwrap();

        // Write an existing settings.json with user fields
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
        ).unwrap();

        let config = test_cuo_config();
        write_cuo_settings(dir.path(), &config, &ServerChoice::Live, &AssistantKind::Razor)
            .expect("Should patch");

        let text = std::fs::read_to_string(dir.path().join("settings.json")).unwrap();
        let json: Value = serde_json::from_str(&text).unwrap();

        // Managed fields updated
        assert_eq!(json["ip"], "live.example.com");
        assert_eq!(json["port"], 2593);
        assert!(json["plugins"][0].as_str().unwrap().contains("Razor.exe"));

        // User fields preserved
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
```

**Step 2: Register the module** — in `app/src-tauri/src/lib.rs`, add:

```rust
pub mod cuo_settings;
```

**Step 3: Run tests to verify they pass**

```bash
cargo test -p ultimaforge-lib cuo_settings 2>&1 | tail -15
```

Expected: all 4 tests PASS.

**Step 4: Commit**

```bash
git add app/src-tauri/src/cuo_settings.rs app/src-tauri/src/lib.rs
git commit -m "feat: add cuo_settings module for settings.json management"
```

---

### Task 4: Update AppState to track running client count

**Files:**
- Modify: `app/src-tauri/src/state.rs`

The `AppStatus` struct needs to expose the number of running clients to the frontend.

**Step 1: Add `running_clients` to `AppStateInner`** (replace `is_game_running: bool`  with a count — keeping `is_game_running` as a computed property for backwards compat)

In `AppStateInner`, add after `is_game_running`:

```rust
/// Number of game client instances currently running.
running_clients: usize,
```

**Step 2: Add accessor and mutator methods to `AppState`**

Find the `set_game_running` and `is_game_running` methods and add alongside them:

```rust
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
```

**Step 3: Update `get_status` to include `running_clients`**

Find the `AppStatus` struct in state.rs and add:

```rust
/// Number of client instances currently running.
pub running_clients: usize,
```

In `get_status()`, populate it:

```rust
running_clients: inner.running_clients,
```

**Step 4: Run all tests**

```bash
cargo test -p ultimaforge-lib 2>&1 | tail -15
```

Expected: all tests PASS. (No new tests needed — the existing `is_game_running` tests still cover the boolean behaviour via the computed field.)

**Step 5: Commit**

```bash
git add app/src-tauri/src/state.rs
git commit -m "feat: track running client count in AppState"
```

---

### Task 5: Update launch command for multi-client + settings.json write

**Files:**
- Modify: `app/src-tauri/src/commands/launch.rs`

**Step 1: Extend `LaunchGameRequest`** to carry the player's choices:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchGameRequest {
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub close_after_launch: Option<bool>,
    /// Number of client instances to launch (1-3).
    #[serde(default = "default_client_count")]
    pub client_count: u8,
    /// Which server to connect to.
    #[serde(default)]
    pub server_choice: ServerChoice,
    /// Which assistant to use.
    #[serde(default)]
    pub assistant_choice: AssistantKind,
}

fn default_client_count() -> u8 { 1 }
```

Add the required imports at top of file:

```rust
use crate::config::{AssistantKind, ServerChoice};
use crate::cuo_settings::write_cuo_settings;
```

**Step 2: Update `LaunchResponse`** to include `running_clients`:

```rust
pub struct LaunchResponse {
    pub success: bool,
    pub pid: Option<u32>,       // pid of first instance
    pub error: Option<String>,
    pub should_close_launcher: bool,
    pub running_clients: usize,
}
```

**Step 3: Rewrite the core of `launch_game`** — replace the single `launcher.launch()` call with:

```rust
// Clamp client_count to valid range
let client_count = request.client_count.clamp(1, 3) as usize;

// Write settings.json with player's current choices before spawning
if let Some(cuo_config) = &brand_config.cuo {
    if let Err(e) = write_cuo_settings(
        &install_path,
        cuo_config,
        &request.server_choice,
        &request.assistant_choice,
    ) {
        warn!("Failed to write CUO settings: {}", e);
        // Non-fatal — proceed with launch anyway
    }
}

// Spawn N instances sequentially
let mut first_pid = None;
let mut launched = 0usize;

for i in 0..client_count {
    let launcher = ClientLauncher::with_config(&install_path, config.clone());
    match launcher.launch() {
        Ok(result) if result.success => {
            if first_pid.is_none() {
                first_pid = result.pid;
            }
            launched += 1;
            info!("Client {} launched (PID {:?})", i + 1, result.pid);
        }
        Ok(result) => {
            warn!("Client {} failed to launch: {:?}", i + 1, result.error_message);
        }
        Err(e) => {
            warn!("Client {} error: {}", i + 1, e);
        }
    }
    // Brief delay between instances to avoid settings.json read races
    if i + 1 < client_count {
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}

if launched == 0 {
    state.set_running_clients(0);
    return Ok(LaunchResponse {
        success: false,
        pid: None,
        error: Some("No client instances launched successfully".into()),
        should_close_launcher: false,
        running_clients: 0,
    });
}

state.set_running_clients(launched);

// close_on_launch only applies when launching a single instance
let should_close = client_count == 1
    && request.close_after_launch.unwrap_or(launcher_config.close_on_launch);

Ok(LaunchResponse {
    success: true,
    pid: first_pid,
    error: None,
    should_close_launcher: should_close,
    running_clients: launched,
})
```

**Step 4: Update existing tests** in launch.rs — add `client_count`, `server_choice`, `assistant_choice` to `LaunchGameRequest::default()`:

```rust
impl Default for LaunchGameRequest {
    fn default() -> Self {
        Self {
            args: Vec::new(),
            close_after_launch: None,
            client_count: 1,
            server_choice: ServerChoice::Live,
            assistant_choice: AssistantKind::RazorEnhanced,
        }
    }
}
```

**Step 5: Run tests**

```bash
cargo test -p ultimaforge-lib 2>&1 | tail -15
```

Expected: all tests PASS.

**Step 6: Commit**

```bash
git add app/src-tauri/src/commands/launch.rs
git commit -m "feat: multi-client launch with settings.json write"
```

---

### Task 6: New get_cuo_config Tauri command + wire everything in lib.rs

**Files:**
- Modify: `app/src-tauri/src/commands/settings.rs`
- Modify: `app/src-tauri/src/lib.rs`

**Step 1: Add `get_cuo_config` to `commands/settings.rs`**

```rust
/// Returns the CUO config block from brand.json so the frontend can
/// build the server and assistant dropdowns.
#[tauri::command]
pub async fn get_cuo_config(
    state: State<'_, AppState>,
) -> Result<Option<serde_json::Value>, String> {
    let brand = state.brand_config().ok_or("Brand config not available")?;
    match &brand.cuo {
        Some(cuo) => Ok(Some(serde_json::to_value(cuo).map_err(|e| e.to_string())?)),
        None => Ok(None),
    }
}
```

Add `use serde_json;` import if not present.

**Step 2: Register in lib.rs** — find `invoke_handler!` macro and add:

```rust
commands::settings::get_cuo_config,
```

**Step 3: Build to check compilation**

```bash
cd app && npm run tauri build -- --debug 2>&1 | grep -E "error|warning\[" | head -20
```

Or on Windows in cmd: use the same command via npm.

Expected: compiles cleanly.

**Step 4: Commit**

```bash
git add app/src-tauri/src/commands/settings.rs app/src-tauri/src/lib.rs
git commit -m "feat: add get_cuo_config Tauri command"
```

---

### Task 7: TypeScript types

**Files:**
- Modify: `app/src/lib/types.ts`

**Step 1: Add new types** (add after the `UserSettings` section)

```typescript
/** Which assistant is active. Mirrors Rust AssistantKind. */
export type AssistantKind = "razor_enhanced" | "razor" | "none";

/** Which server to connect to. Mirrors Rust ServerChoice. */
export type ServerChoice = "live" | "test";

/** Single server endpoint from brand.json. */
export interface ServerConfig {
  label: string;
  ip: string;
  port: number;
}

/** CUO block from brand.json. Null if server owner didn't configure it. */
export interface CuoConfig {
  client_version: string;
  live_server: ServerConfig;
  test_server: ServerConfig | null;
  available_assistants: AssistantKind[];
  default_assistant: AssistantKind;
  default_server: ServerChoice;
}
```

**Step 2: Update `AppStatus`** — add after `is_game_running`:

```typescript
/** Number of game client instances currently running. */
running_clients: number;
```

**Step 3: Update `LaunchGameRequest`** — add:

```typescript
/** Number of client instances to open (1-3). */
client_count?: number;
/** Which server to connect to. */
server_choice?: ServerChoice;
/** Which assistant to use. */
assistant_choice?: AssistantKind;
```

**Step 4: Update `LaunchResponse`** — add:

```typescript
/** Number of clients that launched successfully. */
running_clients: number;
```

**Step 5: Verify TypeScript compiles**

```bash
cd app && npm run build 2>&1 | grep -E "error TS" | head -20
```

Expected: no TypeScript errors.

**Step 6: Commit**

```bash
git add app/src/lib/types.ts
git commit -m "feat: add CUO types to TypeScript definitions"
```

---

### Task 8: api.ts — add getCuoConfig

**Files:**
- Modify: `app/src/lib/api.ts`

**Step 1: Add the function** (find the settings-related functions and add nearby)

```typescript
/** Fetches the CUO config block from brand.json. Returns null if not configured. */
export async function getCuoConfig(): Promise<CuoConfig | null> {
  return invoke<CuoConfig | null>("get_cuo_config");
}
```

Ensure `CuoConfig` is imported from `./types`.

**Step 2: Verify build**

```bash
cd app && npm run build 2>&1 | grep -E "error TS" | head -10
```

**Step 3: Commit**

```bash
git add app/src/lib/api.ts
git commit -m "feat: add getCuoConfig API call"
```

---

### Task 9: CuoControls component

**Files:**
- Create: `app/src/components/CuoControls.tsx`
- Create: `app/src/components/CuoControls.css`

This component renders the server dropdown and assistant dropdown. It receives the CuoConfig plus current selections and calls back on change.

**Step 1: Create CuoControls.tsx**

```tsx
/**
 * CuoControls Component
 *
 * Server and assistant selection dropdowns for ClassicUO.
 * Hidden entirely if no CuoConfig is available (non-CUO servers).
 */

import "./CuoControls.css";
import type { AssistantKind, CuoConfig, ServerChoice } from "../lib/types";

interface CuoControlsProps {
  config: CuoConfig;
  selectedServer: ServerChoice;
  selectedAssistant: AssistantKind;
  onServerChange: (server: ServerChoice) => void;
  onAssistantChange: (assistant: AssistantKind) => void;
  disabled?: boolean;
}

const ASSISTANT_LABELS: Record<AssistantKind, string> = {
  razor_enhanced: "Razor Enhanced",
  razor: "Razor",
  none: "None",
};

export function CuoControls({
  config,
  selectedServer,
  selectedAssistant,
  onServerChange,
  onAssistantChange,
  disabled,
}: CuoControlsProps) {
  const showServerDropdown = config.test_server !== null;
  const showAssistantDropdown = config.available_assistants.length > 1;

  return (
    <div className="cuo-controls">
      {showServerDropdown && (
        <div className="cuo-control-row">
          <label className="cuo-control-label">Server</label>
          <select
            className="cuo-control-select"
            value={selectedServer}
            disabled={disabled}
            onChange={(e) => onServerChange(e.target.value as ServerChoice)}
          >
            <option value="live">{config.live_server.label}</option>
            {config.test_server && (
              <option value="test">{config.test_server.label}</option>
            )}
          </select>
        </div>
      )}

      {showAssistantDropdown ? (
        <div className="cuo-control-row">
          <label className="cuo-control-label">Assistant</label>
          <select
            className="cuo-control-select"
            value={selectedAssistant}
            disabled={disabled}
            onChange={(e) => onAssistantChange(e.target.value as AssistantKind)}
          >
            {config.available_assistants.map((a) => (
              <option key={a} value={a}>
                {ASSISTANT_LABELS[a]}
              </option>
            ))}
          </select>
        </div>
      ) : (
        config.available_assistants.length === 1 && (
          <div className="cuo-control-row">
            <label className="cuo-control-label">Assistant</label>
            <span className="cuo-control-value">
              {ASSISTANT_LABELS[config.available_assistants[0]]}
            </span>
          </div>
        )
      )}
    </div>
  );
}
```

**Step 2: Create CuoControls.css**

```css
.cuo-controls {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 16px;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 8px;
  margin-bottom: 16px;
}

.cuo-control-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.cuo-control-label {
  font-size: 13px;
  color: rgba(255, 255, 255, 0.6);
  width: 72px;
  flex-shrink: 0;
}

.cuo-control-select {
  flex: 1;
  background: rgba(255, 255, 255, 0.06);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 6px;
  color: #eaeaea;
  font-size: 13px;
  padding: 6px 10px;
  cursor: pointer;
  outline: none;
}

.cuo-control-select:hover:not(:disabled) {
  border-color: rgba(255, 255, 255, 0.25);
}

.cuo-control-select:focus {
  border-color: var(--color-secondary, #e94560);
}

.cuo-control-select:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.cuo-control-value {
  font-size: 13px;
  color: #eaeaea;
}
```

**Step 3: Verify it renders** — will be wired in Task 11, but check for TypeScript errors now:

```bash
cd app && npm run build 2>&1 | grep -E "error TS" | head -10
```

**Step 4: Commit**

```bash
git add app/src/components/CuoControls.tsx app/src/components/CuoControls.css
git commit -m "feat: add CuoControls component for server/assistant dropdowns"
```

---

### Task 10: Client count spinner in LaunchButton

**Files:**
- Modify: `app/src/components/LaunchButton.tsx`

**Step 1: Read the current file before editing**

```bash
cat app/src/components/LaunchButton.tsx
```

**Step 2: Add a `clientCount` prop and spinner**

Add to the component's props interface:

```typescript
clientCount?: number;
onClientCountChange?: (count: number) => void;
```

Add the spinner JSX alongside the launch button. The spinner sits to the right:

```tsx
<div className="launch-row">
  <button ...existing launch button...>
    {/* existing content */}
  </button>

  {onClientCountChange && (
    <div className="client-count-spinner">
      <button
        className="client-count-btn"
        onClick={() => onClientCountChange(Math.max(1, (clientCount ?? 1) - 1))}
        disabled={disabled || (clientCount ?? 1) <= 1}
        aria-label="Decrease client count"
      >
        −
      </button>
      <span className="client-count-value">{clientCount ?? 1}</span>
      <button
        className="client-count-btn"
        onClick={() => onClientCountChange(Math.min(5, (clientCount ?? 1) + 1))}
        disabled={disabled || (clientCount ?? 1) >= 5}
        aria-label="Increase client count"
      >
        +
      </button>
    </div>
  )}
</div>
```

Add CSS to `LaunchButton.css` (or the existing stylesheet):

```css
.launch-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.client-count-spinner {
  display: flex;
  align-items: center;
  gap: 6px;
}

.client-count-btn {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.06);
  color: #eaeaea;
  font-size: 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}

.client-count-btn:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.12);
}

.client-count-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.client-count-value {
  font-size: 14px;
  color: #eaeaea;
  min-width: 16px;
  text-align: center;
}
```

**Step 3: Verify build**

```bash
cd app && npm run build 2>&1 | grep -E "error TS" | head -10
```

**Step 4: Commit**

```bash
git add app/src/components/LaunchButton.tsx
git commit -m "feat: add client count spinner to LaunchButton"
```

---

### Task 11: Wire CuoControls into the main view + update useLaunch

**Files:**
- Modify: `app/src/hooks/useLaunch.ts`
- Modify: `app/src/App.tsx` (or the main layout component that renders the launch button)

**Step 1: Update `useLaunch.ts`**

Add state and loading for CUO config:

```typescript
import { getCuoConfig } from "../lib/api";
import type { AssistantKind, CuoConfig, ServerChoice } from "../lib/types";

// Inside the hook, add state:
const [cuoConfig, setCuoConfig] = useState<CuoConfig | null>(null);
const [selectedServer, setSelectedServer] = useState<ServerChoice>("live");
const [selectedAssistant, setSelectedAssistant] = useState<AssistantKind>("razor_enhanced");
const [clientCount, setClientCount] = useState<number>(1);

// Load CUO config on mount (alongside existing useEffect):
useEffect(() => {
  getCuoConfig().then((cfg) => {
    if (cfg) {
      setCuoConfig(cfg);
      setSelectedServer(cfg.default_server);
      setSelectedAssistant(cfg.default_assistant);
    }
  });
}, []);
```

Update the launch call to pass the new fields:

```typescript
// In the launch function, update the LaunchGameRequest:
const request = {
  args: [],
  client_count: clientCount,
  server_choice: selectedServer,
  assistant_choice: selectedAssistant,
};
```

Return the new state and setters from the hook so the UI can bind to them.

**Step 2: Wire into App.tsx / main layout**

Find where `LaunchButton` is rendered. Add `CuoControls` immediately above it:

```tsx
import { CuoControls } from "./components/CuoControls";

// In JSX, above LaunchButton:
{cuoConfig && (
  <CuoControls
    config={cuoConfig}
    selectedServer={selectedServer}
    selectedAssistant={selectedAssistant}
    onServerChange={setSelectedServer}
    onAssistantChange={setSelectedAssistant}
    disabled={isLaunching || isUpdating}
  />
)}

// LaunchButton gets new props:
<LaunchButton
  ...existingProps
  clientCount={clientCount}
  onClientCountChange={setClientCount}
/>
```

**Step 3: Status bar — show running client count**

Find the `StatusBar` component. Update the game-running label:

```tsx
// Instead of "Game Running", show count when > 1:
const gameLabel = appStatus.running_clients > 1
  ? `${appStatus.running_clients} clients running`
  : "Game Running";
```

**Step 4: Manual smoke test**

```bash
cd app && npm run tauri dev
```

Verify:
- Server dropdown appears (if `test_server` is in brand.json) or is hidden
- Assistant dropdown appears with correct options
- Spinner increments/decrements 1-5
- Clicking Launch writes `settings.json` with the correct ip/port/plugins
- Two clients launch when count is 2

**Step 5: Commit**

```bash
git add app/src/hooks/useLaunch.ts app/src/App.tsx app/src/components/StatusBar.tsx
git commit -m "feat: wire CUO controls into launch flow"
```

---

### Task 12: Update brand.json for testing

**Files:**
- Modify: `branding/brand.json`

Add the `cuo` block so the dev build shows the controls:

```json
"cuo": {
  "client_version": "7.0.10.3",
  "live_server": {
    "label": "UO Unchained",
    "ip": "login.patchuo.com",
    "port": 2593
  },
  "test_server": {
    "label": "Test Center",
    "ip": "login.patchuo.com",
    "port": 2594
  },
  "available_assistants": ["razor_enhanced", "razor"],
  "default_assistant": "razor_enhanced",
  "default_server": "live"
}
```

Then sync branding (required after any brand.json change):

```bash
node app/scripts/sync-branding.js
```

**Commit**

```bash
git add branding/brand.json
git commit -m "chore: add cuo config block to brand.json for dev testing"
```

---

### Final verification

Run the full Rust test suite to confirm nothing regressed:

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests PASS, zero failures.
