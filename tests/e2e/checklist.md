# E2E Test Execution Checklist

Use this checklist to track E2E test execution and results.

## Test Environment

- [ ] Rust toolchain installed
- [ ] Node.js installed
- [ ] Project builds successfully (`cargo build --workspace`)
- [ ] Frontend builds successfully (`npm run build`)
- [ ] Test keys generated in `test-keys/`
- [ ] Test data created in `test-data/sample-client/`
- [ ] Test branding configured in `branding/brand.json`

## Test: First-Run Installation (E2E-INSTALL-001)

**Date Tested:** _________________
**Tester:** _________________
**Build Version:** _________________

### Setup
- [ ] Host-server built and running on port 8080
- [ ] Test updates generated with publish-cli
- [ ] Test updates validated with publish-cli
- [ ] Launcher configuration cleared
- [ ] Test install directory prepared

### Test Steps
- [ ] Step 1: Host server health check passes
- [ ] Step 2: Tauri app launches without errors
- [ ] Step 3: InstallWizard appears on first run
- [ ] Step 4: Welcome screen displays correctly
- [ ] Step 5: Directory picker works
- [ ] Step 6: Path validation works (valid/invalid paths)
- [ ] Step 7: EULA acceptance works
- [ ] Step 8: Installation progress shows:
  - [ ] Download percentage
  - [ ] File count
  - [ ] Current file
  - [ ] Speed/ETA
- [ ] Step 9: Installation completes successfully
- [ ] Step 10: All files present in install directory
- [ ] Step 11: File hashes match manifest
- [ ] Step 12: Configuration saved correctly
- [ ] Step 13: "Start Playing" transitions to Ready state

### Error Scenarios
- [ ] Network error handled gracefully
- [ ] Invalid directory rejected
- [ ] Insufficient space detected

### Result
- [ ] **PASS**
- [ ] **FAIL**

**Notes:**
_____________________________________________
_____________________________________________
_____________________________________________

## Test: Update Flow (E2E-UPDATE-001)

**Date Tested:** _________________
**Tester:** _________________

### Setup
- [ ] Initial version (1.0.0) installed
- [ ] Version 1.1.0 published
- [ ] Host server serving updated files

### Test Steps
- [ ] Update detected on launch
- [ ] Update banner displays correctly
- [ ] Update progress shows correctly
- [ ] Only changed files downloaded
- [ ] Update completes successfully
- [ ] Files verified after update

### Rollback Test
- [ ] Simulate failure during update
- [ ] Rollback restores previous state
- [ ] App remains functional

### Result
- [ ] **PASS**
- [ ] **FAIL**

**Notes:**
_____________________________________________
_____________________________________________

## Test: Launch Flow (E2E-LAUNCH-001)

**Date Tested:** _________________
**Tester:** _________________
**Build Version:** _________________

### Setup
- [ ] Installation complete (E2E-INSTALL-001 passed)
- [ ] Launcher configuration exists with `install_complete: true`
- [ ] Test executable created (via test runner or manually)
- [ ] Host server running (optional but recommended)

### Test Steps - Ready State
- [ ] Step 1: App starts in Ready state (not InstallWizard)
- [ ] Step 2: Launch button ("Play") visible and enabled
- [ ] Step 3: Version number displayed correctly
- [ ] Step 4: No error messages visible
- [ ] Step 5: Settings button accessible

### Test Steps - Launch Process
- [ ] Step 6: Click "Play" button
- [ ] Step 7: Button changes to "Launching..." with spinner
- [ ] Step 8: Validation runs (may be brief)
- [ ] Step 9: Client process spawns
- [ ] Step 10: Button changes to "Playing..."
- [ ] Step 11: "Game Closed?" button appears

### Test Steps - Process Verification
- [ ] Step 12: Process running (visible in task manager/ps)
- [ ] Step 13: Working directory correct (equals install path)
- [ ] Step 14: Arguments passed correctly (if configured)
- [ ] Step 15: Environment variables set (if configured)

### Test Steps - Game Exit
- [ ] Step 16: Game process exits (or "Game Closed?" clicked)
- [ ] Step 17: Button returns to "Play" state
- [ ] Step 18: "Game Closed?" button disappears
- [ ] Step 19: App returns to Ready phase

### Edge Cases
- [ ] Missing executable shows error
- [ ] Non-executable file shows error (Unix)
- [ ] Missing install directory shows error
- [ ] Rapid launch/close works correctly
- [ ] Path with spaces works correctly

### Optional Tests
- [ ] Close-on-launch setting works
- [ ] Launch with custom arguments works
- [ ] Launcher closes while game runs, game continues

### Result
- [ ] **PASS**
- [ ] **FAIL**

**Notes:**
_____________________________________________
_____________________________________________
_____________________________________________

## Test: Security (E2E-SECURITY-001)

**Date Tested:** _________________
**Tester:** _________________

### Signature Verification
- [ ] Valid signature accepted
- [ ] Missing signature rejected
- [ ] Tampered manifest rejected
- [ ] Wrong public key rejected

### Hash Verification
- [ ] Valid hash accepted
- [ ] Corrupted file rejected
- [ ] Re-download attempted on failure

### Directory Traversal
- [ ] Host server rejects `/../` paths
- [ ] Manifest rejects paths with `..`

### Result
- [ ] **PASS**
- [ ] **FAIL**

**Notes:**
_____________________________________________
_____________________________________________

## Summary

| Test | Status | Date |
|------|--------|------|
| E2E-INSTALL-001 | | |
| E2E-UPDATE-001 | | |
| E2E-LAUNCH-001 | | |
| E2E-SECURITY-001 | | |

**Overall E2E Status:**
- [ ] All tests PASSED
- [ ] Some tests FAILED (see notes)

**Sign-off:**
___________________________ Date: ___________
