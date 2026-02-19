# Manual Test Guide (Server Owner)

This is a step-by-step manual test for the new “ease-of-use” workflows.

## Prerequisites

- Rust + Node installed (see `docs/SETUP.md`)
- Access to build the launcher (Tauri)

## 1) Run Server Owner Wizard

```bash
node app/scripts/server-owner-wizard.js
```

Expected:
- `branding/brand.json` is created/updated
- A new keypair is generated in `keys/` (if you chose keygen)

## 2) Sync Branding (Optional sanity check)

```bash
node app/sync-branding-config.js
```

Expected:
- `app/src-tauri/tauri.conf.json` updates product name/title

## 3) Build the Launcher

```bash
cd app
npm run tauri build
```

Expected:
- Build artifacts in `app/src-tauri/target/release/bundle/`

## 4) Publish Game Updates

```bash
node app/scripts/publish-all.js
```

When prompted:
- Use your game client folder as the source
- Use `keys/private.key` for game updates
- Provide the launcher binary/installer from the build output
- Provide a Tauri updater signature (or set `TAURI_UPDATER_SIGNATURE`)

Expected:
- `updates/manifest.json` and `updates/manifest.sig`
- `updates/files/` contains blobs
- `updates/launcher/latest.json`
- `updates/launcher/files/` contains launcher binary

## 5) Start the Host Server

```bash
cargo run -p host-server -- --dir ./updates --port 8080
```

Expected:
- Server starts on `http://localhost:8080`

## 6) Validate the Hosted Structure

```bash
curl http://localhost:8080/validate
```

Expected JSON:
- `valid: true`
- `launcher_valid: true` (if launcher updates are present)

## 7) Test Launcher Self-Update

1. Set updater endpoint in `app/src-tauri/tauri.conf.json`:
   ```
   http://localhost:8080/launcher/{{target}}/{{arch}}/{{current_version}}
   ```
2. Run the launcher (dev or build).
3. Open Settings → “Check for launcher updates”.

Expected:
- Update prompt appears if `latest.json` version is higher than current

## 8) Test Game Update Flow

1. Launch the app
2. Confirm it checks `manifest.json`
3. Apply update if prompted

Expected:
- Game files download and apply without errors

## 9) Smoke Test URLs

```bash
curl http://localhost:8080/manifest.json
curl http://localhost:8080/manifest.sig
curl http://localhost:8080/launcher/windows/x86_64/0.0.1
```

Expected:
- All endpoints return 200 with valid content

## Troubleshooting

- If `/launcher/...` returns 404, confirm:
  - `updates/launcher/latest.json` exists
  - `updates/launcher/files/` exists
- If validation fails, re-run publish steps and restart host server.
