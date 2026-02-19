# UltimaForge — Pre-Ship Status Report

**Date:** 2026-02-19
**Scope:** Deep dive code review of all core systems
**Reviewed by:** Claude Code (automated multi-agent analysis)

---

## System Grades

| System | Grade | Notes |
|---|---|---|
| Cryptographic Verification | A | No meaningful weaknesses found |
| Manifest Parsing | B+ | Solid path safety, minor gaps |
| Rust Backend (overall) | B+ | Strong error handling, some atomicity gaps |
| Update Engine | B | Good architecture, crash recovery missing |
| Installation | B | No cleanup on failure, disk check edge cases |
| Launch System | C+ | No process monitoring, race condition on state |
| Frontend | C+ | Three genuine bugs found, version chaos |
| State/Config | C | No persistence, TOCTOU on concurrency flags, unvalidated brand write |

---

## Findings by Severity

---

### CRITICAL — Ship Blockers

**1. `save_brand_config()` writes unvalidated user input to disk**
`app/src-tauri/src/commands/settings.rs:327–353`

The frontend can submit a `BrandConfigInput` that is serialized and written to `branding/brand.json` without ever calling `validate()`. This bypasses all BrandConfig validation (public key format, URL protocol check, etc.). A corrupted or maliciously crafted brand config written here would be picked up in the next build. Since the public key is embedded at compile time from this file, this could result in a launcher that verifies nothing.

Fix: Convert `BrandConfigInput` → `BrandConfig`, call `brand.validate()` before writing.

---

**2. `handleRepairInstallation()` calls `startInstall()` — full re-download**
`app/src/hooks/useSettings.ts:279`

The repair button calls `startInstall(installPath)`, which is the initial installation command. This triggers a complete re-download of all game files rather than any repair operation. A user clicking "Repair Installation" will silently initiate a multi-gigabyte download they didn't expect.

Fix: Either implement a dedicated repair/verify command on the backend, or clearly relabel this in the UI as "Reinstall."

---

### HIGH — Should Fix Before Release

**3. No crash recovery for in-progress operations**
`app/src-tauri/src/state.rs:143–150` / `app/src-tauri/src/updater.rs:939–1179`

All operation state (`is_installing`, `is_updating`, progress) is in-memory only. If the process is killed during `apply_staged_files()`, some files will be new and some old — a mixed state. On next launch, the app resets cleanly as if nothing happened; there's no detection of or recovery from the corrupted installation. The backup exists in `.update-backup/` but there's no code to restore it on restart.

Fix: Write a sentinel file (e.g. `.update-in-progress`) before apply begins, remove it on success. On startup, if the sentinel exists, either auto-restore from backup or warn the user.

---

**4. Rollback failure leaves installation permanently corrupted**
`app/src-tauri/src/updater.rs:1146–1154`

If `apply_staged_files()` fails, rollback is triggered. If rollback itself fails (`RollbackFailed`), the function returns the error and the installation is left in an unknown partial state. There is no nested recovery, no escalation path, and no guidance on recovery beyond the error message.

Fix: At minimum, log the state of every file that was applied before failure and every backup file available, so the user (or a recovery routine) has a path forward.

---

**5. `is_recoverable()` incorrectly marks all I/O errors as transient**
`app/src-tauri/src/error.rs:96`

```rust
Self::Io(_) => true,  // ALL I/O errors are "recoverable"
```

`PermissionDenied` is not a transient error — retrying it forever wastes time and confuses users. `NotFound` on a config file is recoverable; `PermissionDenied` on a system path is not.

Fix: Match on `e.kind()` and return false for `PermissionDenied`, `InvalidInput`, and similar permanent failures.

---

**6. No config schema migration**
`app/src-tauri/src/config.rs:410, 454`

`configVersion: u32` exists in `LauncherConfig` with default `1`, but there is zero migration code. If the schema changes in a future release, users upgrading from v0.0.1 will either get a deserialization error (corrupted state) or silently load defaults (lost settings). This is a ticking clock.

Fix: Add a version-aware load path:
```rust
match config.config_version {
    1 => migrate_v1_to_v2(&config),
    2 => Ok(config),
    _ => Ok(Self::default()),
}
```

---

**7. 30-second HTTP timeout applies to entire response, not per-chunk**
`app/src-tauri/src/downloader.rs:205`

reqwest's `.timeout()` is a total request duration timeout. For large game files on a slow connection, this will silently cut off downloads after 30 seconds, causing retry loops that never succeed. A user on a 10 Mbps connection downloading a 500 MB file needs 400+ seconds.

Fix: Use a stall-detection approach (no progress for N seconds) rather than a total timeout. reqwest supports this via per-read timeouts or a custom stream wrapper.

---

### MEDIUM — Should Fix, Not Blocking

**8. No process monitoring after game launch — state gets stuck**
`app/src-tauri/src/commands/launch.rs:136–137`

After the game process is spawned, there is no monitoring. The comment explicitly acknowledges this:
> "If not waiting for exit, we don't know when the game closes. In a real implementation, we might watch the process."

If the game crashes immediately, `is_game_running` stays `true` and the user cannot relaunch without restarting the launcher. The `game_closed()` command exists but requires manual frontend invocation that never triggers on crash.

Fix: Spawn a background task that `wait()`s on the child process and calls `game_closed()` automatically when it exits.

---

**9. TOCTOU race on `is_installing`/`is_updating` flags**
`app/src-tauri/src/state.rs:237–253`

The pattern is: check flag → start operation → set flag. Two concurrent Tauri commands could both observe `is_installing = false` before either sets it to `true`. This is a classic check-then-act race.

Fix: Replace with a `try_start_install()` method that atomically checks and sets inside a single lock acquisition, returning an error if already in progress.

---

**10. No version downgrade prevention despite error type existing**
`app/src-tauri/src/updater.rs:939` / `app/src-tauri/src/error.rs:277–282`

`UpdateError::DowngradeAttempted` is defined but never thrown. A server-side manifest pointing to older files would be accepted and applied as long as it's signed. This enables rollback attacks.

Fix: Compare `manifest.version` against `current_version` in `perform_update()` and return `DowngradeAttempted` if the manifest version is lower.

---

**11. No disk space pre-check in updater**
`app/src-tauri/src/updater.rs`

The installer validates disk space up front, but the updater has no equivalent check before staging begins. A disk-full condition during staging will only be discovered mid-download, potentially after significant time has been spent.

Fix: Before `download_to_staging()`, compute required staging space (sum of all file sizes to be downloaded) and verify against available disk space.

---

**12. No cleanup of partial downloads after network errors**
`app/src-tauri/src/downloader.rs:402–410`

On network errors mid-stream, the partial file is left on disk. Only hash mismatches and pre-download checks delete the file. This means failed downloads accumulate as partial files.

Fix: On any error in `download_file_inner()`, delete the destination file before returning.

---

**13. Files written directly to final destination — no atomic staging per-file**
`app/src-tauri/src/downloader.rs:366–390`

Files are downloaded directly to their final path. Between write completion and hash verification, another process can observe incomplete files. More importantly, if the hash check fails, the corrupted file is deleted — but there's a window where it exists.

Fix: Download to `<dest>.tmp`, verify hash, then `fs::rename()` to final path. This makes each file's existence atomic from the consumer's perspective.

---

**14. Disk space check silently assumes infinite space on API failure**
`app/src-tauri/src/installer.rs:552–586`

If `fs4::available_space()` fails, the function returns `u64::MAX` with a warning message. Validation then passes (`u64::MAX > required_space`), and the installation begins. The warning is included in `PathValidationResult.warning_message` — but only if the frontend displays it prominently.

Verify that the warning is shown to the user and treated as actionable, not just silently logged.

---

**15. PatchNotes component fetches arbitrary URLs from manifest**
`app/src/components/PatchNotes.tsx:92`

```typescript
const response = await fetch(patchNotesUrl);
```

`patchNotesUrl` is taken from the manifest with no origin validation. The fetch has no timeout (browser default is unlimited). A large or slow response could stall the component indefinitely.

Fix: Validate the URL origin against the known update server domain, and add a fetch timeout.

---

### LOW — Clean Up Before or Shortly After Release

**16. No installation cleanup on failure** (`installer.rs:847–866`) — Files already downloaded are left on disk. Safe (re-install overwrites them), but wasteful and confusing.

**17. `Cancelled` marked recoverable in `UpdateError::is_recoverable()`** (`error.rs:292`) — An explicit user cancellation shouldn't trigger retry logic.

**18. `alert()` in `useInstall.ts`** (`useInstall.ts:378`) — Blocks UI thread. Replace with component error state.

**19. Admin elevation uses `std::process::exit(0)`** (`settings.rs:602–603`) — Hard exit bypasses Tauri cleanup. Should return an error and let Tauri handle shutdown gracefully.

**20. Hardcoded placeholder GitHub URL** (`Sidebar.tsx:98`) — `https://github.com/your-repo/ultimaforge` is never replaced.

**21. `parse_hex_signature()` doesn't validate length before hex decode** (`signature.rs:137–139`) — Length is caught one step later (line 78), but cleaner to fail fast here.

**22. Version hardcoded as `"v0.1.0"` in 8+ frontend locations** — Confirmed at `App.tsx:237,253,271,289,307,321`, `StatusBar.tsx:30`, `Settings.tsx:529`.

**23. Non-atomic settings save** (`settings.rs:260–269`) — In-memory state updated before disk write. If disk write fails, memory and disk disagree until next restart.

---

## Crypto/Security: No Issues

The cryptographic system is the strongest part of the codebase:
- Ed25519 with `verify_strict()` throughout — no malleability
- Public key embedded at compile time, cannot be swapped at runtime
- Signature verified **before** JSON parsing in all code paths (`installer.rs:652`, `updater.rs:474`)
- SHA-256 streaming with 64KB buffers — memory-safe for any file size
- CSPRNG (`OsRng`) used for key generation everywhere
- Defense-in-depth path validation at manifest parse, download, backup, and apply phases
- 13 security tests covering the critical attack surface

Key rotation requires a full rebuild — that's a design constraint, not a flaw. Document it.

---

## Summary Checklist

```
CRITICAL (block ship)
 [ ] Validate BrandConfigInput before writing to disk (settings.rs:327)
 [ ] Fix repair button — it re-downloads everything (useSettings.ts:279)

HIGH
 [ ] Add crash recovery sentinel for mid-update corruption (updater.rs)
 [ ] Handle rollback failure with a recovery path (updater.rs:1150)
 [ ] Fix is_recoverable() for I/O errors (error.rs:96)
 [ ] Add config schema migration (config.rs:454)
 [ ] Fix download timeout to be stall-based, not total-duration (downloader.rs:205)

MEDIUM
 [ ] Spawn process monitor task to auto-clear game_running on exit (launcher.rs)
 [ ] Make is_installing/is_updating check-and-set atomic (state.rs)
 [ ] Implement version downgrade check (updater.rs:939)
 [ ] Add disk space pre-check in updater before staging
 [ ] Clean up partial files on network error (downloader.rs:410)
 [ ] Write downloads to .tmp then rename (downloader.rs:386)
 [ ] Verify disk space warning is prominently surfaced in UI (installer.rs:583)
 [ ] Validate patchNotesUrl origin + add fetch timeout (PatchNotes.tsx:92)

LOW
 [ ] Clean up partial install on failure
 [ ] Fix Cancelled in is_recoverable()
 [ ] Replace alert() with error state (useInstall.ts:378)
 [ ] Fix hard exit in relaunch_as_admin (settings.rs:602)
 [ ] Replace placeholder GitHub URL (Sidebar.tsx:98)
 [ ] Fix version hardcoding in frontend (App.tsx, StatusBar.tsx, Settings.tsx)
 [ ] Non-atomic settings save (settings.rs:260)
```

The two criticals and seven highs are the real pre-ship work. The medium items are important for robustness but the system functions correctly in the happy path without them. The lows can follow in a 1.0.1.
