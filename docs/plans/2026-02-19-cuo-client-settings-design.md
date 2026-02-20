# ClassicUO Client Settings Design

**Date:** 2026-02-19
**Status:** Approved

## Problem

The launcher currently has no awareness of ClassicUO's `settings.json`. The old
launcher (Unchained.exe) wrote hardcoded absolute paths for the assistant plugin,
which break for any player installed to a different location. There is also no way
to switch between Live and Test server, choose between Razor and Razor Enhanced,
or launch multiple clients for multiboxing — all common needs for UO players.

## Directory Structure

```
<install_path>/
├── ClassicUO.exe, ClassicUO.*.dll      ← update via manifest
├── settings.json                        ← launcher-managed, NOT in manifest
├── Files/                               ← mul/uop game data, update via manifest
├── Data/
│   ├── Client/
│   │   ├── JournalLogs/                ← user-generated, never touch
│   │   ├── Screenshots/                ← user-generated, never touch
│   │   └── *.xml, *.txt               ← CUO client data, update via manifest
│   └── Plugins/
│       ├── Razor/
│       │   ├── Razor.exe               ← update via manifest
│       │   ├── Profiles/               ← user data, never touch
│       │   └── items.xml, etc.         ← update via manifest
│       └── RazorEnhanced/
│           ├── RazorEnhanced.exe       ← update via manifest
│           ├── Config/                 ← update via manifest
│           ├── Profiles/               ← user data, never touch
│           ├── Scripts/                ← user scripts, never touch
│           └── Backup/                 ← user data, never touch
├── Logs/                               ← user-generated, never touch
└── Macros/                             ← user macros, never touch
```

## Configuration

### brand.json additions

Server owners add a `cuo` block. `test_server` is optional — if absent, the
server dropdown is hidden from players entirely.

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

### launcher.json additions (persisted player choices)

```json
"selectedServer": "live",
"selectedAssistant": "razor_enhanced",
"clientCount": 1
```

Defaults come from `brand.json` (`default_server`, `default_assistant`). Client
count defaults to 1, capped at 5.

## settings.json Management

The launcher **owns exactly five fields** in ClassicUO's `settings.json` and
writes them right before every launch:

| Field | Value |
|-------|-------|
| `ip` | Selected server IP from brand.json |
| `port` | Selected server port from brand.json |
| `ultimaonlinedirectory` | Always `.\\Files` |
| `clientversion` | From `brand.json cuo.client_version` |
| `plugins` | Absolute path to chosen assistant exe |

**All other fields** (`fps`, `window_size`, `username`, `saveaccount`,
`autologin`, `reconnect`, `login_music`, etc.) are left untouched.

The launcher reads the existing `settings.json`, patches only those five fields,
and writes it back. If the file doesn't exist (first run), it is created with the
managed fields and sensible defaults.

`settings.json` is **never included in the update manifest**.

### Plugin paths per assistant

- **Razor Enhanced** → `<install_path>\Data\Plugins\RazorEnhanced\RazorEnhanced.exe`
- **Razor** → `<install_path>\Data\Plugins\Razor\Razor.exe`
- **None** → `[]`

## UI — Main Screen

Controls sit between the hero/patch notes area and the launch button:

```
┌──────────────────────────────────────────────┐
│  Server      [ Live Server ▼ ]               │
│  Assistant   [ Razor Enhanced ▼ ]            │
└──────────────────────────────────────────────┘

         [ ─  Launch Game  ─ ]  [−] 1 [+]
```

- **Server dropdown** — hidden if no `test_server` in brand.json. Shows
  `live_server.label` / `test_server.label`.
- **Assistant dropdown** — shows only assistants listed in
  `available_assistants`. Rendered as a read-only label if only one is
  configured.
- **Client count spinner** `[−] N [+]` — minimum 1, maximum 5. When N > 1,
  `close_on_launch` is ignored and the launcher stays open.
- **Status bar** — shows "N clients running" when multiple instances are active.

## Backend Changes

### 1. `config.rs` — BrandConfig

Add `CuoConfig` struct:

```rust
pub struct ServerConfig {
    pub label: String,
    pub ip: String,
    pub port: u16,
}

pub struct CuoConfig {
    pub client_version: String,
    pub live_server: ServerConfig,
    pub test_server: Option<ServerConfig>,
    pub available_assistants: Vec<AssistantKind>,
    pub default_assistant: AssistantKind,
    pub default_server: ServerChoice,
}

pub enum AssistantKind { RazorEnhanced, Razor, None }
pub enum ServerChoice { Live, Test }
```

`BrandConfig` gains an optional `cuo: Option<CuoConfig>` field.

### 2. `config.rs` — LauncherConfig

Add three fields:

```rust
pub selected_server: ServerChoice,     // default: brand default_server
pub selected_assistant: AssistantKind, // default: brand default_assistant
pub client_count: u8,                  // default: 1
```

### 3. New command: `write_cuo_settings`

Called automatically inside the launch command before spawning any process.
Reads existing `settings.json` as a `serde_json::Value`, patches the five
managed fields, writes back with pretty-printing. If the file is absent, creates
it from scratch.

### 4. `launcher.rs` — multi-client spawning

The launch command spawns `client_count` processes sequentially with a 300ms
delay between each (prevents racing on settings.json). `AppState` tracks
`Vec<Child>` instead of a single process. The running count is included in
`AppStatus` returned to the frontend.

## What Is NOT Changing

- Update manifest structure — no new manifest fields needed
- The five-step atomic update engine — untouched
- Existing Settings tab — untouched (verify, repair, launcher update, etc.)
- Any user-owned files — never written by the launcher
