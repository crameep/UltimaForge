# Repository Guidelines

## Project Structure & Module Organization
- `app/` holds the launcher application.
  - `app/src/` is the React + TypeScript UI.
  - `app/src-tauri/` is the Rust backend (Tauri) plus installer/update logic.
  - `app/tools/host-server/` and `app/tools/publish-cli/` are Rust utilities.
  - `app/public/` and `app/src/styles/` contain static assets and global styles.
- `branding/` is the server-owner editable branding source.
- `tests/e2e/` contains end-to-end test plans and scripts.
- `docs/` houses setup, publishing, and operational docs.

## Build, Test, and Development Commands
- `ultimaforge.bat` (Windows) is the primary dev entry point; option `1` runs the full dev flow.
- `npm run dev` runs the Vite UI in dev mode.
- `npm run dev:all` runs the combined host server + launcher dev flow.
- `npm run tauri build` builds the desktop app.
- `cargo build --release -p host-server -p publish-cli` builds the Rust tools.
- `npm test` currently prints “No frontend tests configured”.

## Coding Style & Naming Conventions
- Follow existing file styles; do not reformat unrelated sections.
- TypeScript/React uses 2-space indentation and double quotes (see `app/src/*.tsx`).
- Rust should be formatted with `cargo fmt` (rustfmt defaults).
- Prefer descriptive, PascalCase React components and camelCase hooks (e.g., `useUpdate`).

## Testing Guidelines
- Rust unit tests: run `cargo test` from the repo root (workspace).
- E2E tests: use `./tests/e2e/run-e2e-tests.sh` or `./tests/e2e/run-e2e-tests.ps1`.
- Test assets live under `app/test-data/` and `app/test-updates/`.

## Commit & Pull Request Guidelines
- No enforced conventional-commit format. Recent history often uses `auto-claude: subtask-<id> - <summary>`; otherwise short, sentence-style summaries.
- PRs should include a concise description, testing notes (commands run), and screenshots for UI changes.
- Link related issues or docs when behavior or workflow changes.

## Configuration & Security Notes
- Branding edits are expected only in `branding/` and are synced to `app/public/branding/` via scripts.
- Update security relies on signed manifests; keep test keys in `app/test-keys/` for local testing only.

## Recovering the UltimaForge Deployment
- Start with the [Vultr recovery notes](saved-assets-from-server/2026-11-20/README.md) before changing the launcher version, rebuilding update manifests, or migrating the old host.
- The server held a packaged Windows launcher (0.1.34) and client data, not a source checkout. The source on `main` was 0.1.29 at recovery time. Do not bump the version just to match the binary; first locate the exact source or build provenance.
- The owner has a private copy of the payload, original manifests and signature, and the complete checksum list. Ask for access to that copy if needed. Verify all checksums before restoring. Do not commit raw payloads, client/game files, private data, credentials, or signing keys to this public repository.
- Before a cutover, stage the updater on an HTTPS hostname and verify both the launcher signature and game manifest. A checksum-verified copy does not by itself prove a restore or update works. Do not boot, retire, or delete infrastructure from these notes alone.
