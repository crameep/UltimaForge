# Launcher Self-Update

This document describes the UltimaForge update architecture, explains the distinction between game file updates and launcher self-updates, and provides an implementation roadmap for adding launcher self-update capability.

## Overview

UltimaForge has **two distinct update concerns**:

| Update Type | Description | Status |
|-------------|-------------|--------|
| **Game File Updates** | Updates to the UO client files players need to play | Fully implemented |
| **Launcher Self-Update** | Updates to the launcher application itself | Not yet implemented |

The launcher can currently update game files (art assets, maps, executables, etc.) but **cannot update itself**. This document explains both systems and outlines the path to implementing self-update.

---

## Current Status: Game File Updates (Implemented)

The launcher has a **fully functional game file update system** that handles all player-side content updates.

### Architecture

```
Update Server                          Player's Launcher
─────────────────                      ─────────────────────
manifest.json    ───────────────────>  Fetch & verify signature
manifest.sig                           Compare file hashes
files/                                 Download only changed files
  ├── <sha256>   ───────────────────>  Verify hash matches
  └── ...                              Apply atomically with backup
```

### Components

| File | Purpose |
|------|---------|
| `app/src-tauri/src/updater.rs` | Atomic update engine with backup/rollback (~1400 lines) |
| `app/src-tauri/src/commands/update.rs` | Tauri commands for frontend interaction |
| `app/src-tauri/src/downloader.rs` | HTTP downloads with progress reporting |
| `app/src-tauri/src/manifest.rs` | Manifest parsing and file comparison |
| `app/src-tauri/src/signature.rs` | Ed25519 signature verification |

### Features

- **Differential updates**: Only changed files are downloaded (content-addressed by SHA-256)
- **Cryptographic verification**: Ed25519 signatures ensure manifest integrity
- **Atomic application**: All files applied together or rolled back on failure
- **Backup and rollback**: Current files backed up before update; restored on failure
- **Transaction logging**: All operations logged for troubleshooting
- **Progress reporting**: Real-time progress events sent to UI

### Update Flow

```
1. Check for Updates
   └── Fetch manifest.json and manifest.sig from update server
   └── Verify Ed25519 signature against embedded public key
   └── Compare local file hashes with manifest hashes
   └── Calculate files needing update

2. Download (if updates available)
   └── Download files to staging directory (.update-staging/)
   └── Report progress: bytes, file count, speed, ETA
   └── Verify each downloaded file's hash

3. Apply
   └── Backup current files to .update-backup/
   └── Move staged files to installation directory
   └── On failure: restore from backup (rollback)
   └── Clean up staging and backup directories

4. Complete
   └── Update version in launcher configuration
   └── Emit completion event to UI
```

### Security Model

The game update system uses a trust model based on cryptographic signing:

- **Server owner** holds the private key (generated via `publish-cli keygen`)
- **Public key** is embedded in `brand.json` and compiled into the launcher
- **Manifests** are signed with the private key before deployment
- **Launcher** verifies the signature before trusting any file list
- **Individual files** are verified by SHA-256 hash after download

This ensures players only receive files you've explicitly signed and published.

---

## What's Missing: Launcher Self-Update (Not Implemented)

The launcher **cannot update itself**. When you release a new version of the launcher with bug fixes, UI improvements, or new features, players must:

1. Manually download the new launcher
2. Replace their existing installation
3. Re-launch

This creates friction for both server owners and players.

### Missing Components

| Component | Description |
|-----------|-------------|
| Version checking | Launcher doesn't check for newer launcher versions |
| Update manifest | No manifest format for launcher binaries (separate from game files) |
| Download mechanism | No code to download launcher updates |
| Apply mechanism | Cannot replace the running executable |
| Restart handling | No mechanism to restart after self-update |

### Why It's Different from Game Updates

Game file updates and launcher self-updates are fundamentally different:

| Aspect | Game Updates | Launcher Self-Update |
|--------|--------------|---------------------|
| Files updated | Game assets (read/write at any time) | Running executable (locked while running) |
| Update timing | Can apply immediately | Must restart to complete |
| Platform concerns | Same files for all platforms | Platform-specific binaries |
| Signature source | Server owner's key | Possible additional signing (code signing) |
| User expectation | Automatic, seamless | May require user confirmation |

---

## Implementation Roadmap

### Option 1: Tauri Updater Plugin (Recommended)

Tauri provides a built-in updater plugin that handles the complexity of self-updates across platforms.

#### Pros

- Battle-tested implementation across Windows, macOS, and Linux
- Handles executable replacement and restart automatically
- Supports code signing verification
- Well-documented and maintained by the Tauri team
- Minimal implementation effort

#### Cons

- Less control over update flow
- Requires hosting update metadata in Tauri's expected format
- May have different signing requirements from game updates

#### Implementation Steps

1. **Add the updater plugin dependency**:
   ```toml
   # app/src-tauri/Cargo.toml
   [dependencies]
   tauri-plugin-updater = "2"
   ```

2. **Configure the updater in tauri.conf.json**:
   ```json
   {
     "plugins": {
       "updater": {
         "endpoints": [
           "https://your-server.com/launcher-updates/{{target}}/{{arch}}/{{current_version}}"
         ],
         "pubkey": "<your-public-key>"
       }
     }
   }
   ```

3. **Register the plugin**:
   ```rust
   // main.rs
   fn main() {
       tauri::Builder::default()
           .plugin(tauri_plugin_updater::Builder::new().build())
           .run(tauri::generate_context!())
           .expect("error while running application");
   }
   ```

4. **Add frontend check**:
   ```typescript
   import { check } from '@tauri-apps/plugin-updater';

   async function checkForLauncherUpdate() {
     const update = await check();
     if (update) {
       await update.downloadAndInstall();
       // App will restart automatically
     }
   }
   ```

5. **Create update endpoint** that returns Tauri-compatible update metadata:
   ```json
   {
     "version": "1.2.0",
     "notes": "Bug fixes and performance improvements",
     "pub_date": "2024-01-15T12:00:00Z",
     "platforms": {
       "windows-x86_64": {
         "signature": "<signature>",
         "url": "https://your-server.com/launcher-updates/launcher-1.2.0-x64-setup.exe"
       }
     }
   }
   ```

### Option 2: Custom Self-Update Implementation

Build a custom updater using the same patterns as the game file updater.

#### Pros

- Full control over update flow
- Can reuse existing signing infrastructure
- Unified update experience for game files and launcher

#### Cons

- Significant implementation effort
- Must handle platform-specific executable replacement
- Must handle process restart carefully
- Higher maintenance burden

#### Implementation Steps

1. **Create separate launcher manifest format**:
   ```json
   {
     "launcher_version": "1.2.0",
     "min_supported_version": "1.0.0",
     "platforms": {
       "windows-x86_64": {
         "url": "https://your-server.com/launcher/1.2.0/launcher.exe",
         "sha256": "abc123...",
         "size": 15000000
       },
       "darwin-aarch64": {
         "url": "https://your-server.com/launcher/1.2.0/launcher.app.tar.gz",
         "sha256": "def456...",
         "size": 18000000
       }
     },
     "signature": "..."
   }
   ```

2. **Add version checking endpoint**:
   ```
   GET /launcher/latest.json
   ```

3. **Implement download to temporary location**:
   - Download new launcher binary to a temp directory
   - Verify signature and hash
   - Do NOT replace the running executable yet

4. **Implement restart-and-replace mechanism**:

   **Windows approach**:
   - Write a small update script/batch file
   - Launch the script as a detached process
   - Script waits for main process to exit
   - Script replaces the executable
   - Script launches the new version
   - Script deletes itself

   **macOS/Linux approach**:
   - Similar shell script approach
   - Or use a separate updater binary

5. **Handle edge cases**:
   - User declines update → remember to ask later
   - Download fails → retry logic
   - Replace fails → preserve original
   - Version rollback → detect and handle

### Recommendation

**Start with Tauri's built-in updater plugin (Option 1).** It handles the complex platform-specific concerns and provides a well-tested foundation. The implementation effort is minimal compared to a custom solution.

If you later need features the built-in plugin doesn't support, you can migrate to a custom solution while maintaining the same user experience.

---

## Security Considerations

### Code Signing

For production launcher distribution, consider platform code signing:

| Platform | Signing Method | Why It Matters |
|----------|----------------|----------------|
| Windows | Authenticode certificate | Avoids SmartScreen warnings |
| macOS | Apple Developer ID | Required for Gatekeeper |
| Linux | GPG signatures | Optional but recommended |

Code signing is **separate from** the Ed25519 manifest signing used for game updates. Both can coexist.

### Update Channel Security

Self-update introduces a new attack surface:

- **Endpoint security**: HTTPS is mandatory for launcher update endpoints
- **Signature verification**: Launcher updates should be signed (Tauri plugin handles this)
- **Version pinning**: Prevent downgrade attacks by tracking minimum supported versions

### Key Management

If using Tauri's updater, you'll have **two key pairs**:

1. **Game update keys** (Ed25519): For signing game file manifests
2. **Launcher update keys**: For signing launcher binaries (Tauri uses its own format)

Keep both private keys secure and consider separate keys for development vs. production.

---

## Hosting Considerations

### Separate Endpoints

Consider hosting launcher updates separately from game updates:

```
Game updates:     https://updates.yourserver.com/game/
Launcher updates: https://updates.yourserver.com/launcher/
```

This allows:
- Different caching strategies (launcher updates are less frequent)
- Clearer separation of concerns
- Easier rollback of launcher vs. game updates

### CDN Recommendations

Launcher binaries can be 10-20+ MB. Use a CDN for global distribution:

- Cloudflare (free tier available)
- AWS CloudFront
- GitHub Releases (simple, free for open source)

---

## UI/UX Considerations

### When to Check

- **On startup**: Check for launcher updates before game updates
- **Background**: Optionally check periodically while idle
- **Manual**: Provide "Check for Updates" option in settings

### User Notification

Unlike game updates (which can be applied automatically), launcher updates should:

1. **Notify the user** that an update is available
2. **Show what's new** (release notes)
3. **Allow deferral** ("Remind me later")
4. **Require confirmation** before restarting

### Progress Indication

Show clear progress during:
- Download (with percentage and speed)
- Verification
- Restart countdown (if auto-restarting)

---

## Next Steps

To implement launcher self-update:

1. **Choose approach**: Tauri plugin (recommended) or custom implementation
2. **Set up hosting**: Create launcher update endpoint with proper metadata
3. **Implement version check**: Add check on startup
4. **Add UI**: Update available notification and progress
5. **Test thoroughly**: Verify update works on all target platforms
6. **Document**: Update this file with implementation details

## Related Documentation

- [PUBLISHING.md](PUBLISHING.md) - Game file update publishing
- [SETUP.md](SETUP.md) - Development environment setup
- [tests/e2e/update-flow.md](../tests/e2e/update-flow.md) - Game update E2E tests
