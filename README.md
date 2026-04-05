# UltimaForge

**A self-hosted, secure, and brandable game launcher/patcher for Ultima Online private servers.**

UltimaForge gives server owners a professional, turnkey launcher that handles client installation, cryptographically-signed updates, and game launching. Built with Rust + React (Tauri), it compiles into a single branded executable that players download and run.

---

## Features

- **Secure Updates** - Ed25519 signed manifests + SHA-256 per-file verification
- **Atomic Updates** - Staged apply with automatic rollback on failure
- **Full Branding** - Colors, logos, window title, hero text, sidebar links, installer graphics
- **Self-Updating Launcher** - The launcher itself updates via Tauri's built-in updater
- **Migration Detection** - Auto-detects existing UO installations and offers copy/adopt/skip
- **ClassicUO Integration** - Auto-configures CUO settings, server selection, and assistant choice
- **Multi-Client** - Launch up to 3 game clients simultaneously
- **Auto-Elevation** - Detects Program Files installs and prompts to relaunch as admin
- **Resume Support** - Interrupted downloads resume where they left off
- **First-Run Wizard** - Guided installation for players
- **VPS Deployment** - Built-in rsync-based deploy to your hosting server
- **Patch Notes** - Displays server patch notes from a configurable URL

---

## Quick Start (Server Owners)

Everything runs through `ultimaforge.bat` on Windows. Double-click it and follow the numbered steps:

```
  FIRST TIME SETUP

  [1] Install Prerequisites          [...]
  [2] Configure Branding & Keys      [...]
  [3] Generate App Icons             [...]
  [4] Build Launcher                 [...]
  [5] Setup VPS (optional)           [...]

  ONGOING

  [6] Publish Game Update
  [7] Deploy to VPS
  [8] Update Launcher Source
```

Each step shows `[DONE]` or `[...]` so you know where you left off.

### Step-by-step

1. **Clone and run the batch file:**
   ```bash
   git clone https://github.com/crameep/UltimaForge.git
   cd UltimaForge
   ultimaforge.bat
   ```

2. **[1] Install Prerequisites** - Installs Rust, Node.js, and VS Build Tools if missing.

3. **[2] Configure Branding & Keys** - Interactive wizard that generates `branding/brand.json` and Ed25519 keypairs. Or edit `branding/brand.json` manually using `branding/brand.example.json` as reference.

4. **[3] Generate App Icons** - Converts your `branding/sidebar-logo.png` into all required icon sizes.

5. **[4] Build Launcher** - Compiles the branded NSIS installer. Output lands in `app/src-tauri/target/release/bundle/nsis/`.

6. **Distribute** the installer to your players. Done.

### Publishing Updates

When you update your game files:

- **[6] Publish Game Update** - Hashes your client files, generates a signed manifest, and uploads to your update server.
- **[7] Deploy to VPS** - Rsyncs the update artifacts to your hosting server.
- **[8] Update Launcher Source** - Pulls the latest UltimaForge source code.

---

## For Players

Download and run the installer from your server. The launcher will:

1. Guide you through picking an install folder (or detect an existing one)
2. Download and verify all game files
3. Keep everything up-to-date automatically
4. Launch the game with one click

---

## Branding

All customization lives in `branding/`:

| File | Purpose |
|------|---------|
| `brand.json` | Server name, colors, URLs, CUO config, update endpoint |
| `sidebar-logo.png` | Your server logo (shown in sidebar and used for icons) |
| `hero-bg.png` | Background image for the main content area |
| `sidebar-texture.png` | Optional sidebar background texture |

See `branding/brand.example.json` for the full schema with all available options including CUO settings, migration search paths, and sidebar links.

`brand.json` is compiled into the binary via `include_str!` — changes require a rebuild.

---

## Architecture

```
ultimaforge/
├── ultimaforge.bat              # Server owner tool (run this)
├── branding/                    # Your branding (edit this)
│   ├── brand.json               # Server config, colors, URLs
│   ├── brand.example.json       # Full schema reference
│   └── *.png                    # Logo, background, sidebar texture
├── keys/                        # Generated keys (do not share private keys)
│   ├── private.key / public.key # Ed25519 game update signing keys
│   └── tauri-updater/           # Tauri launcher self-update keys
├── updates/                     # Published update artifacts
├── app/
│   ├── src/                     # React/TypeScript frontend
│   │   ├── components/          # UI components (LaunchButton, Settings, etc.)
│   │   ├── hooks/               # React hooks (useLaunch, useUpdate, useInstall)
│   │   └── lib/                 # API layer, types, launcher updater
│   ├── src-tauri/src/           # Rust backend
│   │   ├── commands/            # Tauri IPC command handlers
│   │   ├── updater.rs           # Atomic update engine
│   │   ├── installer.rs         # First-run installation + path validation
│   │   ├── migration.rs         # Existing install detection + migration
│   │   ├── launcher.rs          # Game process spawning (multi-client)
│   │   ├── signature.rs         # Ed25519 verification
│   │   ├── downloader.rs        # HTTP downloads with resume
│   │   ├── cuo_settings.rs      # ClassicUO settings writer
│   │   └── config.rs            # Brand + launcher config types
│   ├── tools/
│   │   ├── host-server/         # Axum HTTP server for hosting updates
│   │   └── publish-cli/         # CLI to hash, sign, and package updates
│   └── scripts/                 # Node.js build/setup helpers
└── docs/                        # Additional documentation
```

### Update Flow

1. **Publish**: `publish-cli` hashes your game files, generates `manifest.json`, signs it with Ed25519, and stores files as content-addressed blobs by their SHA-256 hash.
2. **Host**: `host-server` (or any static file server) serves the `updates/` directory.
3. **Patch**: The launcher fetches the manifest, verifies the signature against the embedded public key, diffs against local files, and applies changes atomically.

### State Machine

```
Initializing → NeedsInstall → Installing → CheckingUpdates → [UpdateAvailable → Updating] → Ready → GameRunning
                    ↑                                                                          ↑
              NeedsMigration ─── (copy/adopt) ─────────────────────────────────────────────────┘
```

---

## Development

### Prerequisites

- Rust (stable, MSVC toolchain on Windows)
- Node.js 18+ and npm
- Visual Studio Build Tools (Windows) or build-essential (Linux)

### Dev Commands

```bash
# From the app/ directory:
npm run dev              # Vite frontend only (port 1420)
npm run dev:all          # Host server (port 8080) + launcher together
npm run tauri dev        # Full Tauri dev mode

# From the repo root:
cargo test               # All Rust tests
cargo fmt                # Format Rust code

# Or use ultimaforge.bat → [D] Developer Tools for a guided menu
```

### Developer Tools Menu

Press `D` from the main menu to access:

- Quick start (sync + server + launcher)
- Sync branding assets
- Generate test manifests
- Start test server
- Run all tests
- Build production (manual)

---

## Security

UltimaForge employs multiple layers:

1. **Ed25519 Signatures** - Manifests are signed with a private key you control; the public key is embedded at compile time
2. **SHA-256 Hashing** - Every file is verified by hash after download
3. **Path Traversal Protection** - Manifest paths are validated against directory escapes, UNC paths, and Windows device names
4. **Atomic Updates** - Files are staged, verified, then moved into place; failure at any step triggers rollback
5. **Auto-Elevation** - Detects when admin rights are needed and offers UAC relaunch

---

## System Requirements

### Building

- Windows 10/11 (primary), macOS 10.15+, or Linux
- 4 GB RAM, 2 GB disk space

### Players

- Windows 7+
- ~100 MB for the launcher
- 2-4 GB for game files (varies by server)

---

## Support

- **Docs**: [`docs/`](docs/) directory
- **Issues**: [GitHub Issues](https://github.com/crameep/UltimaForge/issues)

---

Built with [Tauri](https://tauri.app/), [React](https://react.dev/), [Rust](https://www.rust-lang.org/), and [ed25519-dalek](https://github.com/dalek-cryptography/curve25519-dalek).
