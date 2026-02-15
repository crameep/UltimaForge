# E2E Test: First-Run Installation Flow

## Test ID
`E2E-INSTALL-001`

## Description
Verifies that a fresh installation of the UltimaForge launcher correctly handles
the first-run experience, including directory selection, file download, and
verification.

## Prerequisites

1. Build the host-server tool:
   ```bash
   cargo build --release -p host-server
   ```

2. Build the publish-cli tool:
   ```bash
   cargo build --release -p publish-cli
   ```

3. Generate test updates (if not already done):
   ```bash
   cargo run -p publish-cli -- publish \
     --source ./test-data/sample-client \
     --output ./test-updates \
     --key ./test-keys/private.key \
     --version 1.0.0
   ```

4. Ensure branding is configured with test public key:
   - Update `branding/brand.json` with `publicKey` from `test-keys/public.key`
   - Update `updateUrl` to `http://localhost:8080`

5. Clear any existing launcher configuration:
   - **Windows:** Delete `%APPDATA%\ultimaforge\launcher.json`
   - **macOS:** Delete `~/Library/Application Support/ultimaforge/launcher.json`
   - **Linux:** Delete `~/.config/ultimaforge/launcher.json`

## Test Steps

### Step 1: Start the Test Update Server

```bash
cargo run -p host-server -- --dir ./test-updates --port 8080
```

**Expected Output:**
```
Starting UltimaForge Update Server
Serving files from: ./test-updates
Server URL: http://localhost:8080
Endpoints:
  GET /                 - Server info
  GET /health          - Health check
  GET /manifest.json   - Update manifest
  GET /manifest.sig    - Manifest signature
  GET /files/{hash}    - Content-addressed file
  GET /validate        - Validate server setup
```

**Verification:**
```bash
curl http://localhost:8080/health
# Should return: {"status":"ok"}

curl http://localhost:8080/validate
# Should return validation results with all checks passing
```

### Step 2: Launch Tauri App (Fresh State)

Start the launcher with no existing installation:

```bash
npm run tauri dev
```

**Or use the built executable:**
```bash
# Windows
./src-tauri/target/release/ultimaforge.exe

# macOS
./src-tauri/target/release/bundle/macos/UltimaForge.app/Contents/MacOS/ultimaforge

# Linux
./src-tauri/target/release/ultimaforge
```

### Step 3: Verify InstallWizard Appears

**Expected Behavior:**
- The InstallWizard component should appear immediately
- Welcome screen shows "Welcome to [ServerName]"
- Step indicator shows: Welcome (active) > Directory > Terms > Install > Done

**Screenshot Checkpoint:** Capture the welcome screen

### Step 4: Navigate to Directory Selection

1. Click "Get Started" button on welcome screen

**Expected Behavior:**
- Directory selection step becomes active
- "Select Installation Directory" header visible
- Browse button available
- Continue button disabled (no directory selected)

### Step 5: Select Installation Directory

1. Click "Browse..." button
2. Select or create a new directory for installation
   - Recommended: Use a temporary directory like `/tmp/ultimaforge-test-install`

**Expected Behavior:**
- Directory picker dialog opens
- After selection, path appears in the input field
- Validation runs automatically:
  - Spinner appears: "Validating directory..."
  - On success: Green checkmark with available space info
  - On failure: Red X with reason (permissions, space, etc.)
- Continue button becomes enabled

**Verification Checks:**
| Check | Expected |
|-------|----------|
| Directory exists | Shows available space |
| Directory empty | Valid (recommended) |
| Directory not empty | Warning displayed |
| No write permission | Invalid with reason |
| Insufficient space | Invalid with reason |

### Step 6: Accept Terms of Service

1. Click "Continue" button
2. Read the Terms of Service
3. Check the acceptance checkbox
4. Click "Install" button

**Expected Behavior:**
- EULA step becomes active
- Terms of Service text visible
- Install button disabled until checkbox checked
- After checking, Install button enables

### Step 7: Monitor Installation Progress

**Expected Behavior:**
- Installing step becomes active
- Progress bars visible:
  - Download progress (percentage, bytes downloaded/total)
  - File count (X / Y files)
- Current file being downloaded shown
- Speed and ETA displayed (when available)
- State text updates: "Fetching manifest...", "Downloading files...", "Verifying files..."

**Progress Events to Observe:**
1. "Fetching file manifest..." - Downloading manifest.json
2. "Downloading files..." - Streaming file downloads
3. "Verifying files..." - Hash verification
4. Progress percentage increases smoothly
5. File count increments

### Step 8: Verify Installation Complete

**Expected Behavior:**
- Complete step shows success message
- Checkmark icon displayed
- "Installation Complete!" header
- Summary shows:
  - Installed to: [selected directory path]
  - Version: 1.0.0
- "Start Playing" button visible

### Step 9: Verify Files on Disk

After installation completes, verify the files were correctly installed:

```bash
# Check files exist
ls -la [install-directory]/

# Expected files:
# - client.exe
# - art.mul
# - map0.mul

# Verify file hashes match manifest
sha256sum [install-directory]/client.exe
sha256sum [install-directory]/art.mul
sha256sum [install-directory]/map0.mul

# Compare with manifest.json hashes
cat ./test-updates/manifest.json | jq '.files[] | {path, sha256}'
```

### Step 10: Verify Configuration Saved

Check that launcher configuration was saved correctly:

```bash
# Find config file based on platform
# Windows: %APPDATA%\ultimaforge\launcher.json
# macOS: ~/Library/Application Support/ultimaforge/launcher.json
# Linux: ~/.config/ultimaforge/launcher.json

cat [config-path]/launcher.json
```

**Expected Configuration:**
```json
{
  "install_path": "[selected-directory]",
  "current_version": "1.0.0",
  "install_complete": true,
  "auto_launch": false,
  "close_on_launch": true,
  "check_updates_on_startup": true
}
```

### Step 11: Click "Start Playing"

1. Click the "Start Playing" button

**Expected Behavior:**
- Wizard closes
- Main launcher UI appears (Ready state)
- Launch button visible
- No update banner (already on latest version)

## Pass/Fail Criteria

| Criterion | Pass Condition |
|-----------|----------------|
| Wizard appears on first run | InstallWizard component renders |
| Directory validation works | Valid paths accepted, invalid rejected |
| Manifest download succeeds | No network errors, manifest parsed |
| Signature verification passes | Invalid signatures would fail |
| Files download correctly | All files downloaded with progress |
| Hash verification passes | All file hashes match manifest |
| Configuration saved | launcher.json contains correct state |
| UI transitions correctly | Each step renders without errors |

## Error Scenarios to Test

### Network Error
1. Stop the host-server during download
2. Observe error handling
3. Verify retry button appears
4. Restart server and retry

**Expected:** Error message displayed, retry option available

### Invalid Directory
1. Select a read-only directory
2. Observe validation failure

**Expected:** Red X with "No write permission" or similar message

### Disk Full Simulation
1. Select a directory on a nearly-full disk
2. Observe space validation

**Expected:** Invalid with insufficient space warning

## Cleanup

After the test:

```bash
# Remove test installation
rm -rf [install-directory]

# Stop host-server (Ctrl+C in terminal)

# Optionally remove launcher config
rm [config-path]/launcher.json
```

## Notes

- This test should be run with a fresh configuration (no existing launcher.json)
- The test public key (`d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a`)
  is from RFC 8032 test vectors and should NEVER be used in production
- Test files are small text files simulating UO client files
- Real UO client files are multi-gigabyte; test with actual files for realistic timing

## Related Tests

- [E2E-UPDATE-001](./update-flow.md) - Update flow after installation
- [E2E-LAUNCH-001](./launch-flow.md) - Game launching
- [E2E-SECURITY-001](./security-tests.md) - Signature verification
