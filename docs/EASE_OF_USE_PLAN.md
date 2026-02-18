# UltimaForge Ease-of-Use Improvements

This document summarizes the ease-of-use work added to make it simpler for
server owners to package a launcher and deliver updates.

## Goals

- Reduce multi-step setup to guided workflows
- Publish game + launcher updates in one command
- Host everything from a single server path
- Provide quick dev/testing flows
- Keep versioning consistent across the project

## What Was Added

### 1) Server Owner Wizard

Guided CLI wizard that generates `branding/brand.json`, optionally generates
keys, and syncs Tauri branding automatically.

Script:
- `app/scripts/server-owner-wizard.js`

Run:
```bash
node app/scripts/server-owner-wizard.js
```

### 2) Publish All (Game + Launcher)

One-command flow that:
1) Publishes game updates (manifest + signature + blobs)
2) Generates launcher self-update metadata
3) Copies launcher binary into the hosted `launcher/files` folder
4) Prints a summary and smoke-test URLs

Script:
- `app/scripts/publish-all.js`

Run:
```bash
node app/scripts/publish-all.js
```

Optional:
- `--game-public-key` to validate the game update output
- `TAURI_UPDATER_SIGNATURE` env var to avoid signature prompts

### 3) Launcher Updates Hosted on the Same Server

The built-in host server now serves launcher updates:

- `GET /launcher/{target}/{arch}/{current_version}`
- `GET /launcher/files/{filename}`

Metadata is read from:
- `updates/launcher/latest.json` (fallback)
- `updates/launcher/{target}-{arch}.json` (preferred)

### 4) Version Bump Automation

Updates workspace Cargo version, Tauri config, and npm package version in one
script.

Script:
- `app/scripts/bump-version.js`

Run:
```bash
node app/scripts/bump-version.js --version x.y.z
```

### 5) Dev All-in-One

Starts the host server and launcher in a single terminal session. Generates
test updates if missing.

Script:
- `app/scripts/dev-all-in-one.js`

Run:
```bash
npm run dev:all
```

### 6) Updated Menu Options

New options added to `ultimaforge.bat`:
- `C` Publish Launcher Update Metadata
- `D` Server Owner Wizard
- `E` Publish All (game + launcher)
- `F` Dev All-in-One

## Hosting Layout (Single Server)

```
updates/
├── manifest.json
├── manifest.sig
├── files/
└── launcher/
    ├── latest.json
    ├── windows-x86_64.json
    └── files/
        └── YourLauncher-1.2.0-x64-setup.exe
```

## Updater Endpoint

Tauri updater should point to:

```
http://your-server/launcher/{{target}}/{{arch}}/{{current_version}}
```

## Validation

The host server `/validate` endpoint now reports both game update and launcher
update folder status.

## Docs Updated

- `docs/PUBLISHING.md`
- `docs/SELF-UPDATE.md`
- `docs/SETUP.md`
- `QUICKSTART.md`
- `README.md`

## Next Steps (Optional)

- Wire automatic generation of the Tauri updater signature in CI
- Provide a "single publish" GitHub Action
- Add a guided UI wizard inside the launcher for non-technical owners
