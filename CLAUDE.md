# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

UltimaForge is a self-hosted game launcher/patcher for Ultima Online private servers. Server owners build a branded launcher executable (Tauri + React + Rust) that players download. The launcher handles first-run installation, cryptographically-signed atomic updates, and game launching.

## Commands

All commands run from `app/` (where `package.json` lives), unless noted.

```bash
# Development
npm run dev              # Vite frontend only (port 1420)
npm run dev:all          # Host server (port 8080) + launcher together
npm run tauri dev        # Tauri dev mode (starts Vite + Rust backend)

# Build
npm run tauri build      # Production build with signing
cargo build --release -p host-server -p publish-cli  # Build Rust tools (from repo root)

# Testing
cargo test               # All Rust unit tests (run from repo root)
cargo test -p ultimaforge-lib  # Launcher crate tests only
./tests/e2e/run-e2e-tests.sh   # E2E tests (Linux/macOS)
./tests/e2e/run-e2e-tests.ps1  # E2E tests (Windows)

# Rust formatting
cargo fmt                # Format all Rust code

# Branding sync (required after editing branding/*.png)
node app/scripts/sync-branding.js  # Or use ultimaforge.bat option 2

# Version bump
node app/scripts/bump-version.js --version x.y.z

# Publish updates
cargo run -p publish-cli -- publish --source ./uo-client --output ./updates --key ./keys/private.key --version 1.0.0 --executable client.exe
node app/scripts/publish-all.js  # End-to-end game + launcher publish
```

On Windows, `ultimaforge.bat` wraps all of the above with a numbered menu.

## Architecture

### Workspace Structure

```
Cargo.toml               # Workspace root: 3 members
app/
  src/                   # React/TypeScript frontend
  src-tauri/             # Tauri Rust crate (the launcher binary)
  tools/
    host-server/         # Axum HTTP server for serving updates
    publish-cli/         # CLI to create manifests, sign, and generate blobs
  scripts/               # Node.js helper scripts
branding/                # Server-owner editable (brand.json, images)
updates/                 # Update artifacts served by host-server
keys/                    # Ed25519 keypair + Tauri updater keys
```

### How Branding Works

`branding/brand.json` is compiled into the launcher binary at build time via `include_str!` in `app/src-tauri/src/lib.rs`. **Any change to brand.json requires a full rebuild.** Images in `branding/` must be synced to `app/public/branding/` before a dev server or build will pick them up.

### Rust Backend (`app/src-tauri/src/`)

| Module | Role |
|--------|------|
| `lib.rs` | Entry point: loads brand config, initializes `AppState`, registers Tauri commands |
| `state.rs` | Thread-safe `AppState` (Mutex + RwLock); drives `AppPhase` state machine |
| `config.rs` | `BrandConfig` (compile-time) and `LauncherConfig` (runtime, persisted to disk) |
| `updater.rs` | Atomic update engine: download → stage → verify → backup → apply → rollback |
| `installer.rs` | First-run installation logic |
| `downloader.rs` | HTTP downloads with resume support and progress reporting |
| `manifest.rs` | Manifest JSON parsing and path traversal validation |
| `signature.rs` | Ed25519 signature verification |
| `hash.rs` | SHA-256 file hashing |
| `launcher.rs` | Game process spawning |
| `commands/` | Tauri IPC command handlers (crypto, install, update, launch, settings) |
| `error.rs` | Unified error types |

### Frontend (`app/src/`)

React hooks drive all async operations and call Tauri commands via `invoke()`:
- `useUpdate.ts` — update checking and application
- `useInstall.ts` — first-run installation wizard
- `useLaunch.ts` — game launching
- `useSettings.ts` — settings persistence
- `useBrand.ts` — brand config access

Components map 1:1 with app phases: `SetupWizard`, `InstallWizard`, `UpdateProgress`, `LaunchButton`, `Settings`, `PatchNotes`, `Layout`, `Sidebar`, `StatusBar`.

### AppPhase State Machine

```
Initializing → NeedsInstall → Installing → CheckingUpdates → [UpdateAvailable → Updating] → Ready → GameRunning
```

`AppState` (managed by Tauri) is the single source of truth. State transitions happen in the Rust command handlers and are reflected back to the frontend via `get_app_status`.

### Update System

1. `publish-cli` generates `manifest.json` (file list + SHA-256 hashes + version), signs it with Ed25519, and stores files as content-addressed blobs in `updates/files/` by their SHA-256 hash.
2. `host-server` (Axum) serves the updates directory. Endpoints: `/manifest.json`, `/manifest.sig`, `/files/{hash}`, `/launcher/{target}/{arch}/{version}`.
3. The launcher fetches the manifest, verifies the Ed25519 signature against the public key **embedded at build time**, diffs against local files, and performs an atomic staged update.

### Launcher Self-Updates

The launcher uses Tauri's built-in updater plugin. Keys are stored in `keys/tauri-updater/`. The updater endpoint is configured in `app/src-tauri/tauri.conf.json`. On Windows, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` must be non-empty — Windows silently drops empty env vars when spawning processes.

## Coding Conventions

- TypeScript/React: 2-space indentation, double quotes
- Rust: rustfmt defaults (`cargo fmt`)
- New Tauri commands must be registered in `lib.rs`'s `invoke_handler!` macro
- Rust crate-level re-exports live in `lib.rs`; `main.rs` only calls `run()`
