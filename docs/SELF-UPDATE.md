# Launcher Self-Update

This document describes the UltimaForge update architecture, explains the distinction between game file updates and launcher self-updates, and provides an implementation roadmap for adding launcher self-update capability.

## Overview

UltimaForge has **two distinct update concerns**:

| Update Type | Description | Status |
|-------------|-------------|--------|
| **Game File Updates** | Updates to the UO client files players need to play | Fully implemented |
| **Launcher Self-Update** | Updates to the launcher application itself | Implemented |

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

## Launcher Self-Update (Implemented)

UltimaForge now supports **self-updating the launcher** using Tauri's updater plugin. This enables the launcher to check for new versions, download them, and restart automatically when approved by the user.

### What It Includes

| Component | Description |
|-----------|-------------|
| Version checking | Launcher checks for newer versions at startup or manually |
| Update metadata | Tauri-compatible update manifest hosted by the server |
| Download mechanism | Built-in updater handles download and verification |
| Apply mechanism | Running executable replaced safely by the updater |
| Restart handling | Automatic restart after successful update |

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

## Implementation Details

### Tauri Updater Plugin (Implemented)

Tauri provides a built-in updater plugin that handles the complexity of self-updates across platforms.

The current implementation:

1. **Enables the updater plugin** in `app/src-tauri/Cargo.toml` and `app/src-tauri/src/lib.rs`
2. **Configures update endpoints and pubkey** in `app/src-tauri/tauri.conf.json`
3. **Checks for updates in the UI** and prompts users to download and restart
4. **Supports manual update checks** via the Settings screen

### Hosting With the Built-In Host Server

The host server can serve launcher updates alongside game updates. Place metadata and binaries here:

```
updates/
└── launcher/
    ├── latest.json
    ├── windows-x86_64.json
    └── files/
        └── YourLauncher-1.2.0-x64-setup.exe
```

The updater endpoint should be:

```
http://your-server/launcher/{{target}}/{{arch}}/{{current_version}}
```

Use the helper scripts to generate metadata:

```bash
app/scripts/publish-launcher-update.ps1
app/scripts/publish-launcher-update.sh
```

By default the scripts write to `updates/launcher`.
You can also set `TAURI_UPDATER_SIGNATURE` to avoid manual prompts.

The scripts prompt for the **Tauri updater signature** for the binary. Use your
release signing key to generate this signature (typically via the Tauri CLI or
CI pipeline), then paste it when prompted.

### Update Metadata Format

The update endpoint must return Tauri-compatible metadata:

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

### Custom Self-Update (Optional)

If you later need custom update behaviors that Tauri's plugin does not support, you can build a bespoke updater. This is not currently required.

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
