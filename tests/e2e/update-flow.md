# E2E Test: Update Flow

## Test ID
`E2E-UPDATE-001`

## Description
Verifies the differential update mechanism, including update detection, download
of only changed files, atomic application with backup, and rollback on failure.
This test validates the core value proposition of UltimaForge: efficient, secure,
and atomic updates that never leave an installation in a broken state.

## Prerequisites

1. **Complete First-Run Installation (E2E-INSTALL-001)**
   - Version 1.0.0 must be installed successfully
   - All files verified via hash check

2. **Build Tools:**
   ```bash
   cargo build --release -p host-server -p publish-cli
   ```

3. **Existing Test Environment:**
   - Test keys present in `test-keys/`
   - Initial installation in test directory
   - Launcher configuration saved with version 1.0.0

4. **Verify v1.0.0 Installation:**
   ```bash
   # Check current installation
   ls -la [install-directory]/

   # Verify config shows 1.0.0
   cat [config-path]/launcher.json | grep current_version
   # Should show: "current_version": "1.0.0"
   ```

## Test Steps

### Step 1: Verify Initial State (v1.0.0)

Before testing updates, confirm the baseline installation:

```bash
# Verify installed files
sha256sum [install-directory]/client.exe
sha256sum [install-directory]/art.mul
sha256sum [install-directory]/map0.mul

# Record the original hashes for comparison
cd [project-root]
cat test-updates/manifest.json | jq '.files[] | {path, sha256, size}'
```

**Expected Output:**
```json
{
  "path": "client.exe",
  "sha256": "...",
  "size": ...
}
{
  "path": "art.mul",
  "sha256": "...",
  "size": ...
}
{
  "path": "map0.mul",
  "sha256": "...",
  "size": ...
}
```

### Step 2: Create Modified Files for v1.1.0

Modify one or more files to simulate an update:

```bash
cd [project-root]

# Option A: Modify an existing file
echo -e "\n[v1.1.0] Updated content for testing" >> ./test-data/sample-client/art.mul

# Option B: Add a new file (optional)
echo "New file in version 1.1.0" > ./test-data/sample-client/config.ini
```

**Verification:**
```bash
# Check that art.mul was modified
cat ./test-data/sample-client/art.mul | tail -1
# Should show: [v1.1.0] Updated content for testing
```

### Step 3: Publish Version 1.1.0

Generate the new update package:

```bash
cd [project-root]

# Backup v1.0.0 manifest for comparison
cp ./test-updates/manifest.json ./test-updates/manifest-v1.0.0.json.bak

# Publish v1.1.0
cargo run --release -p publish-cli -- publish \
    --source ./test-data/sample-client \
    --output ./test-updates \
    --key ./test-keys/private.key \
    --version 1.1.0

# Validate the new release
cargo run --release -p publish-cli -- validate \
    --dir ./test-updates \
    --key ./test-keys/public.key
```

**Expected Output:**
```
Publishing version 1.1.0...
Processing files from ./test-data/sample-client
  - client.exe (unchanged, reusing blob)
  - art.mul (modified, new hash)
  - map0.mul (unchanged, reusing blob)
  - config.ini (new file, if added)
Generating manifest...
Signing manifest...
Validation complete: OK
  Manifest version: 1.1.0
  Total files: 3-4
  Total size: XXX bytes
```

### Step 4: Compare Manifests (Verify Differential)

Compare v1.0.0 and v1.1.0 manifests to verify only changed files are different:

```bash
# View v1.1.0 manifest
cat ./test-updates/manifest.json | jq '.version, .files[].path'

# Compare with v1.0.0 backup
diff <(cat ./test-updates/manifest-v1.0.0.json.bak | jq -S '.files | sort_by(.path)') \
     <(cat ./test-updates/manifest.json | jq -S '.files | sort_by(.path)')
```

**Expected Differences:**
- `version` field changed from "1.0.0" to "1.1.0"
- `timestamp` updated
- `art.mul` entry has new `sha256` and possibly different `size`
- `client.exe` and `map0.mul` entries should be identical (same hash)
- Optional: new `config.ini` entry if added

### Step 5: Start Host Server with Updated Files

```bash
cd [project-root]

# Start the update server
cargo run --release -p host-server -- --dir ./test-updates --port 8080
```

**Verify Server:**
```bash
# Health check
curl http://localhost:8080/health
# Expected: {"status":"ok"}

# Verify manifest version
curl http://localhost:8080/manifest.json | jq '.version'
# Expected: "1.1.0"

# Verify signature
curl http://localhost:8080/manifest.sig | head -c 20
# Expected: Hex-encoded Ed25519 signature (128 characters)
```

### Step 6: Launch Tauri App and Detect Update

```bash
npm run tauri dev
```

**Expected Behavior:**
1. App launches with existing installation (does not show InstallWizard)
2. Automatic update check begins (if `check_updates_on_startup: true`)
3. Update banner appears showing:
   - Current version: 1.0.0
   - Available version: 1.1.0
   - Files to update: 1 (or 2 if new file added)
   - Download size: Shows only the bytes needed to download

**UI Elements to Verify:**
| Element | Expected Value |
|---------|----------------|
| Update banner visible | Yes |
| Current version display | 1.0.0 |
| New version display | 1.1.0 |
| Files to update | 1-2 |
| Download size | < Total package size (differential) |
| "Update Now" button | Visible and enabled |
| "Later" button | Visible and enabled |

### Step 7: Start the Update

1. Click "Update Now" button

**Expected Behavior:**
- Update banner transitions to progress view
- Progress bar appears
- File count shows: "X / Y files"
- Current file being downloaded is displayed
- Speed and ETA shown when available

**Progress Events to Observe:**
```
State: CheckingUpdates
  "Checking for updates..."

State: Downloading
  "Downloading art.mul..."
  Progress: 0% -> 100%
  Files: 1 / 1 (or 2 / 2)

State: Verifying
  "Verifying downloaded files..."

State: BackingUp
  "Backing up current files..."

State: Applying
  "Applying update..."

State: Completed
  "Update complete!"
```

### Step 8: Verify Update Progress (Differential Download)

While the update is in progress, monitor the console/logs:

```bash
# Check Tauri logs for differential behavior
# Look for messages like:
# - "File client.exe unchanged, skipping"
# - "File art.mul changed, downloading"
# - "Downloading 1 of 1 files"
```

**Key Verification:**
- **Unchanged files NOT downloaded**: `client.exe` and `map0.mul` should be skipped
- **Only modified files downloaded**: `art.mul` should be the only downloaded file
- **Download size matches**: Progress should show size of only the changed file

### Step 9: Verify Update Completion

**Expected UI State:**
- Update complete message displayed
- Version number updated to 1.1.0
- "Ready to Play" state visible
- Launch button enabled

**Verify from Logs/Console:**
```
[INFO] Update completed successfully
[INFO] Version updated: 1.0.0 -> 1.1.0
[INFO] Files updated: 1
[INFO] Staging directory cleaned
[INFO] Backup directory cleaned
```

### Step 10: Verify Files on Disk

```bash
# Verify files exist with correct content
cat [install-directory]/art.mul | tail -1
# Expected: [v1.1.0] Updated content for testing

# Verify file hashes match new manifest
sha256sum [install-directory]/client.exe  # Should match v1.0.0 (unchanged)
sha256sum [install-directory]/art.mul      # Should match v1.1.0 (new hash)
sha256sum [install-directory]/map0.mul     # Should match v1.0.0 (unchanged)

# Compare with manifest
curl -s http://localhost:8080/manifest.json | jq '.files[] | {path, sha256}'
```

### Step 11: Verify Configuration Updated

```bash
cat [config-path]/launcher.json
```

**Expected Configuration:**
```json
{
  "install_path": "[install-directory]",
  "current_version": "1.1.0",
  "install_complete": true,
  "auto_launch": false,
  "close_on_launch": true,
  "check_updates_on_startup": true
}
```

### Step 12: Verify Staging/Backup Cleanup

After successful update, temporary directories should be cleaned:

```bash
# Verify staging directory removed
ls [install-directory]/.update-staging 2>/dev/null
# Should fail with "No such file or directory"

# Verify backup directory removed
ls [install-directory]/.update-backup 2>/dev/null
# Should fail with "No such file or directory"

# Verify no leftover temp files
ls [install-directory]/ | grep -E '\.(tmp|bak|staging)$'
# Should return nothing
```

### Step 13: Verify No Re-download on Next Launch

Close and relaunch the app:

```bash
npm run tauri dev
```

**Expected Behavior:**
- App starts directly in "Ready" state
- No update banner shown (already on v1.1.0)
- Update check shows "No updates available"

---

## Rollback Testing (Optional but Recommended)

### Step R1: Setup Rollback Test

This test verifies the atomic rollback mechanism when an update fails mid-apply.

```bash
# Ensure you have a clean v1.0.0 installation
# (Run E2E-INSTALL-001 again if needed)

# Publish v1.1.0 as before
cargo run --release -p publish-cli -- publish \
    --source ./test-data/sample-client \
    --output ./test-updates \
    --key ./test-keys/private.key \
    --version 1.1.0
```

### Step R2: Simulate Apply Failure

There are several ways to simulate a failure during the apply phase:

**Option A: Lock a file (simulates "file in use")**
```bash
# On Windows, open a file in a program
# On Linux/macOS, lock the file:
flock [install-directory]/art.mul sleep 60 &
```

**Option B: Remove write permissions**
```bash
chmod -w [install-directory]/art.mul
```

**Option C: Fill disk (advanced)**
```bash
# Create a large file to fill the disk before apply
```

### Step R3: Attempt Update

1. Launch the Tauri app
2. Accept the update when prompted
3. Wait for the apply phase to fail

**Expected Behavior:**
1. Download completes successfully
2. Verification passes
3. Backup created successfully
4. Apply phase fails with error
5. Rollback automatically triggered
6. Original files restored from backup
7. Error message displayed to user with retry option

**Error Messages to Verify:**
```
[ERROR] Failed to apply update: <specific error>
[INFO] Rolling back to previous version...
[INFO] Restoring art.mul from backup
[INFO] Rollback completed successfully
[ERROR] Update failed: <user-friendly message>
```

### Step R4: Verify Rollback Success

```bash
# Verify original files restored
cat [install-directory]/art.mul | tail -1
# Should NOT contain "[v1.1.0]" content

# Verify version unchanged
cat [config-path]/launcher.json | grep current_version
# Expected: "current_version": "1.0.0"

# Verify all files match v1.0.0 hashes
sha256sum [install-directory]/*
```

### Step R5: Remove Lock and Retry

```bash
# Remove the file lock
chmod +w [install-directory]/art.mul
# or kill the flock process
```

1. Click "Retry" in the error dialog
2. Update should proceed normally

---

## Edge Case Tests

### Edge Case 1: Large Number of Files

Test update with many files to verify progress reporting:

```bash
# Create many test files
for i in {1..50}; do
    echo "File $i content" > ./test-data/sample-client/data/file$i.dat
done

# Modify half of them for the update
for i in {26..50}; do
    echo "Modified in v1.1.0" >> ./test-data/sample-client/data/file$i.dat
done

# Publish and test
cargo run --release -p publish-cli -- publish \
    --source ./test-data/sample-client \
    --output ./test-updates \
    --key ./test-keys/private.key \
    --version 1.1.0
```

**Expected:** Progress shows correct file count (25 files to update)

### Edge Case 2: Network Interruption

1. Start update
2. During download, kill the host-server
3. Verify error handling and retry

**Expected:**
- Error displayed: "Network error: Connection refused"
- Retry button available
- Staging directory preserved for resume

### Edge Case 3: Concurrent Launch Attempt

1. Start an update
2. While updating, try to click "Launch Game"

**Expected:**
- Launch button disabled during update
- Or: Warning "Please wait for update to complete"

### Edge Case 4: App Quit During Update

1. Start an update
2. Close the app window during download

**Expected on next launch:**
- App detects incomplete update
- Offers to resume or cancel
- Staging directory cleaned on cancel

---

## Pass/Fail Criteria

| Criterion | Pass Condition |
|-----------|----------------|
| Update detected | Banner shows correct version difference |
| Differential download | Only modified files downloaded (verify download size) |
| Progress reporting | Percentage, file count, speed all update correctly |
| Atomic application | All files updated together or none |
| Backup created | .update-backup directory exists during apply |
| Rollback works | Failure restores previous state completely |
| Cleanup performed | No staging/backup directories after success |
| Version updated | Config shows new version after update |
| Files verified | All hashes match new manifest |
| UI transitions | Each state renders without errors |
| Error handling | Clear messages for all failure modes |

## Known Limitations

1. **Partial resume not implemented**: If download fails, it restarts from beginning
2. **No delta compression**: Files are downloaded in full, not as binary diffs
3. **Single update path**: No A/B rollback to versions older than previous

## Cleanup

```bash
# Restore test data to original state
cd [project-root]
git checkout -- test-data/sample-client/

# Remove backup manifest
rm -f ./test-updates/manifest-v1.0.0.json.bak

# Stop host-server (Ctrl+C)

# Optionally reset installation for fresh tests
rm -rf [install-directory]/*
rm [config-path]/launcher.json
```

## Notes

- This test assumes a successful E2E-INSTALL-001 was completed first
- The differential download is verified by observing that unchanged file hashes are skipped
- Rollback testing is optional but highly recommended before production use
- For realistic timing tests, use actual UO client files (multi-GB)
- The test public key (`d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a`)
  from RFC 8032 should NEVER be used in production

## Related Tests

- [E2E-INSTALL-001](./first-run-install.md) - Prerequisite for this test
- [E2E-LAUNCH-001](./launch-flow.md) - Test launching after update
- [E2E-SECURITY-001](./security-tests.md) - Signature verification during update
