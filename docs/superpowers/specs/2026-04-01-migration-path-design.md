# Migration Path for Existing Installations

**Date:** 2026-04-01
**Status:** Approved

## Problem

Players often have existing Ultima Online installations — from the server's own previous distribution, or from other private servers sharing the same base UO data files. These installations are frequently in protected locations like `C:\Program Files`. The current launcher has basic detection (`detect_existing_installation()`) but no multi-path scanning, no migration workflow, and fragile elevation handling.

## Goals

- Server owners configure known installation paths in `brand.json`
- Launcher auto-scans those paths on first run and presents findings
- Users choose: copy to a safe location, use in place (with explicit elevation), or skip to fresh install
- Manual "Find existing installation" available in Settings at any time
- No silent permission escalation — every elevation is user-initiated and explained

## Non-Goals

- Wildcard/glob path matching (server owners provide exact paths)
- Smart/partial copying (full directory copy, updater fills gaps)
- Hardlink/symlink optimization
- Version extraction from existing files (updater determines what needs patching)

---

## Design

### 1. Brand Config Extension

New optional `migration` object in `brand.json`:

```json
{
  "migration": {
    "searchPaths": [
      "C:\\Program Files\\MyServer",
      "C:\\Program Files (x86)\\EA Games\\Ultima Online",
      "C:\\Games\\UO"
    ]
  }
}
```

- `searchPaths` — exact directories to check for existing UO installations
- If omitted or empty, no auto-scan happens (fresh-install-only servers)
- Parsed into `BrandConfig` at build time via `include_str!`

### 2. Detection Flow

**First run** (no existing `launcher.json`), if `migration.searchPaths` is populated:

1. Scan each path in order using the existing `detect_existing_installation()` heuristic
2. Collect all results with confidence levels (HIGH/MEDIUM/LOW)
3. Present findings in a new **MigrationWizard** screen (before InstallWizard)
4. If nothing found, fall through to the normal InstallWizard

**From Settings** (any time), via "Find existing installation" button:

- Re-scan the brand's `searchPaths`
- Browse to a directory manually
- Detection result is validated and shown (confidence, found files, missing files) but user is not blocked from proceeding

### 3. User Decision Screen

When an installation is detected (auto-scan or manual browse), the user sees:

- Where the installation was found
- Detection confidence and file summary (e.g., "Found ClassicUO.exe + 5/5 data files")
- Three options:

**Option A: "Copy to new location"** (recommended)
- Pre-filled with recommended safe path (`%LOCALAPPDATA%\{ServerName}`)
- User can change the destination
- No elevation needed
- Note: "Original files will not be modified"

**Option B: "Use in place"**
- Points the launcher at the existing directory
- If protected path: warning that admin privileges are required for all future launches
- Offers "Relaunch as Administrator" if needed

**Option C: "Skip — install fresh"**
- Falls through to the normal InstallWizard

### 4. Migration (Copy) Phase

When the user chooses "Copy to new location":

1. **Validate destination** — writable, sufficient disk space (check size of source directory), not a protected path
2. **Copy with progress** — iterate files in source, copy each to destination, report progress as "Migrating files... (142/380)"
3. **No hash verification during copy** — the updater verifies everything in the next phase
4. **On failure** — clean up partial copy, show error, let user retry or choose different option
5. **On success** — write `launcher.json` with `install_path` set to new location, `install_complete: true`, `current_version: None`

Implemented as `migrate_installation(source, destination, progress_callback)` — a standalone function separate from installer and updater.

### 5. "Use in Place" Path

1. **Non-protected path** — write `launcher.json` with `install_path` pointing at existing directory, `install_complete: true`, `current_version: None`. Fall through to update check.
2. **Protected path** — show elevation warning. If user accepts, trigger `relaunch_as_admin()`. After relaunch, proceed to update check with admin privileges.
3. **Persist `requires_elevation: true`** in `launcher.json` — on every subsequent launch, if this flag is true and process is not elevated, immediately trigger `relaunch_as_admin()` and exit the non-elevated instance.
4. **One UAC prompt per launch** — standard Windows behavior, nothing hidden.
5. **Migrating away later** (via Settings to a safe path) clears the flag.

### 6. Settings Integration

New "Installation" section in Settings:

- **Current install path** — displayed, read-only
- **"Find existing installation" button** — triggers detection flow (scan brand paths + manual browse), presents Migration Decision screen
- **"Change install path" button** — pick a new directory. Protected path triggers elevation warning and `requires_elevation` logic. Moving from protected to safe clears the flag.

Reuses the same migration/detection components — no separate code paths for first-run vs Settings.

### 7. AppPhase State Machine Changes

Two new phases:

```
Initializing -> NeedsMigration -> NeedsInstall -> Installing -> CheckingUpdates -> ...
                     |                                              ^
                     |-- (copy) --> Migrating -----------------------|
                     |-- (use in place) ----------------------------|
                     |-- (skip) --> NeedsInstall
```

- **NeedsMigration** — entered when first-run detection finds existing installations. Shows Migration Decision screen.
- **Migrating** — copy in progress with progress UI.
- If no installations found during scan, skips straight to **NeedsInstall**.
- Rest of the state machine is untouched.

### 8. Error Handling

| Scenario | Behavior |
|----------|----------|
| Copy fails mid-way (disk full, permission denied) | Clean up partial destination, return to Migration Decision with error message. User can retry, pick different destination, or skip to fresh install. |
| Source path disappears between detection and copy | Re-run detection, show updated results. |
| Elevation relaunch fails (user denies UAC) | Stay on decision screen, suggest "Copy to new location" instead. |
| Detection finds nothing | Message: "No existing installations found. Browse manually or install fresh." |
| Protected path + no admin on subsequent launch | `requires_elevation` auto-relaunch. Repeated UAC denial shows explanation and offers to migrate to safe path from Settings. |

No retry loops or silent fallbacks — every failure is surfaced with clear options.

---

## Key Files to Modify

| File | Changes |
|------|---------|
| `branding/brand.json` | Add `migration` object |
| `app/src-tauri/src/config.rs` | Parse `migration` from BrandConfig, add `requires_elevation` to LauncherConfig |
| `app/src-tauri/src/state.rs` | Add `NeedsMigration` and `Migrating` AppPhase variants |
| `app/src-tauri/src/installer.rs` | Extract detection into reusable multi-path scanner |
| `app/src-tauri/src/migration.rs` (new) | `migrate_installation()`, `scan_migration_paths()`, progress reporting |
| `app/src-tauri/src/commands/install.rs` | New commands: `scan_for_migrations`, `start_migration`, `use_in_place` |
| `app/src-tauri/src/lib.rs` | Register new commands, check `requires_elevation` on startup |
| `app/src/components/MigrationWizard.tsx` (new) | Migration decision UI |
| `app/src/hooks/useMigration.ts` (new) | Hook for migration state and commands |
| `app/src/components/Settings.tsx` | Add Installation section with find/change buttons |
