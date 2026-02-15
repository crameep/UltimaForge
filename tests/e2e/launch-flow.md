# E2E Test: Launch Flow

## Test ID
`E2E-LAUNCH-001`

## Description
Verifies that the game client launches correctly from the launcher, with proper
working directory, arguments, and state management. This test validates the
complete launch lifecycle from validation through process spawning.

## Prerequisites

1. **Complete First-Run Installation (E2E-INSTALL-001)**
   - Version 1.0.0 (or later) must be installed successfully
   - All files verified via hash check
   - `launcher.json` exists with `install_complete: true`

2. **Build Tools:**
   ```bash
   cargo build --release -p host-server -p publish-cli
   ```

3. **Real Executable (for full testing):**
   The mock `client.exe` in test-data won't actually run as a process.
   For complete verification, either:
   - Use a real UO client executable (for production testing)
   - Use a test executable script (see Step 2 below)

4. **Verify Existing Installation:**
   ```bash
   # Check installation directory
   ls -la [install-directory]/

   # Verify config shows installation complete
   cat [config-path]/launcher.json | grep install_complete
   # Should show: "install_complete": true
   ```

## Test Steps

### Step 1: Verify Ready State

Before testing the launch, confirm the launcher is in a ready state:

1. Start the host server (even though not needed for launch, it prevents errors):
   ```bash
   cargo run --release -p host-server -- --dir ./test-updates --port 8080 &
   ```

2. Launch the Tauri app:
   ```bash
   npm run tauri dev
   ```

**Expected Behavior:**
- App starts directly in "Ready" state
- No InstallWizard shown (installation already complete)
- Launch button ("Play") is visible and enabled
- No error messages displayed

**UI Elements to Verify:**
| Element | Expected State |
|---------|---------------|
| Launch button | Visible, enabled, shows "Play" |
| Version display | Shows installed version (e.g., "1.0.0") |
| Error message area | Empty/hidden |
| Settings button | Visible and clickable |

### Step 2: Create Test Executable (Optional)

For full launch testing, create a test script that simulates a game client:

**Windows (test-client.bat):**
```batch
@echo off
echo UO Client Test Executable
echo Working Directory: %CD%
echo Arguments: %*
echo.
echo Press any key to simulate game running...
pause > nul
echo Game exiting with code 0
exit /b 0
```

**Unix (test-client.sh):**
```bash
#!/bin/bash
echo "UO Client Test Executable"
echo "Working Directory: $(pwd)"
echo "Arguments: $@"
echo ""
echo "Press Enter to simulate game running..."
read
echo "Game exiting with code 0"
exit 0
```

Place the test script in the installation directory:
```bash
# Copy to install directory
cp test-client.sh [install-directory]/client.exe

# On Unix, ensure it's executable
chmod +x [install-directory]/client.exe
```

### Step 3: Validate Client Pre-Launch

The launcher validates the client before launching. This happens automatically
when you click "Play", but you can verify the behavior:

**Expected Validation Checks:**
1. Installation directory exists
2. Client executable file exists
3. Executable is a file (not a directory)
4. (Unix) Executable has execute permissions
5. (Windows) Executable has recognized extension (.exe, .bat, .cmd)

**To Test Validation Failure:**
1. Temporarily rename or remove the client executable:
   ```bash
   mv [install-directory]/client.exe [install-directory]/client.exe.bak
   ```
2. Click "Play" button
3. Observe error message displayed

**Expected Error:**
- Error message appears below the launch button
- Message indicates executable not found or invalid
- "Play" button remains enabled for retry

**Restore:**
```bash
mv [install-directory]/client.exe.bak [install-directory]/client.exe
```

### Step 4: Click Launch Button

With a valid installation in place:

1. Click the "Play" button

**Expected Behavior:**
1. Button text changes to "Launching..." with spinner
2. Validation runs (brief, usually not visible)
3. Client process is spawned
4. Button text changes to "Playing..."
5. "Game Closed?" secondary button appears

**Log Events to Observe (in console/terminal):**
```
[INFO] Validating client: [install-path]/client.exe
[INFO] Launching client: [install-path]/client.exe with 0 args
[DEBUG] Working directory: [install-path]
[INFO] Client launched successfully with PID: XXXXX
```

### Step 5: Verify Process Spawned

While the "game" is running, verify the process was correctly spawned:

**Windows:**
```powershell
# List processes matching client
Get-Process | Where-Object { $_.ProcessName -like "*client*" }

# Or using Task Manager:
# Check for client.exe process
```

**Unix/macOS:**
```bash
# Find client processes
ps aux | grep client

# Or check specific PID from launcher log
ps -p XXXXX
```

**Expected:**
- Process is running
- Process ID matches launcher logs
- Process is independent (launcher can be closed while game runs)

### Step 6: Verify Working Directory

The client should be launched with the installation directory as its working
directory. This is crucial for UO clients that load data files from relative paths.

**Verification (with test script):**
If using the test script from Step 2, observe the "Working Directory" output:
```
Working Directory: [install-directory]
```

**Verification (programmatic):**
```bash
# Unix: Check process working directory
lsof -p XXXXX | grep cwd

# Or read from proc filesystem (Linux)
readlink /proc/XXXXX/cwd
```

**Expected:**
- Working directory equals the installation path from launcher config
- NOT the launcher's install directory or system temp

### Step 7: Verify UI State During Launch

While the game is "running":

**Expected UI State:**
| Element | Expected State |
|---------|---------------|
| Launch button | Shows "Playing...", disabled |
| "Game Closed?" button | Visible below launch button |
| Error message | Hidden |
| App window | Still visible (unless close_on_launch is true) |

**User Experience Verification:**
1. Cannot click "Play" while game is running
2. Can navigate to Settings page
3. Can close launcher (game continues running)
4. Can click "Game Closed?" to mark game as closed

### Step 8: Test Game Exit Detection

The launcher tracks when the game is marked as closed:

1. If using the test script: Press Enter/any key to let it exit
2. OR: Close the actual game client normally
3. OR: Click "Game Closed?" button in the launcher

**Expected Behavior:**
After game closes (or "Game Closed?" clicked):
- Button returns to "Play" state
- Button becomes enabled again
- "Game Closed?" button disappears
- App phase returns to "Ready"

**Log Events:**
```
[INFO] Game marked as closed
[DEBUG] App phase changed: GameRunning -> Ready
```

### Step 9: Verify Rapid Launch/Close

Test the launcher's handling of rapid state changes:

1. Click "Play"
2. Wait for "Playing..." state
3. Click "Game Closed?"
4. Immediately click "Play" again

**Expected:**
- No errors or race conditions
- Clean state transitions each time
- Launch succeeds on second attempt

### Step 10: Test Launch with Close-on-Launch Setting

If configured to close after launch:

1. Open Settings
2. Enable "Close launcher after game starts" (if not already)
3. Return to main view
4. Click "Play"

**Expected Behavior:**
- Game launches as before
- Launcher window closes automatically after successful launch
- Game continues running independently

**Verification:**
```bash
# Game should still be running
ps aux | grep client
```

### Step 11: Test Launch with Arguments (Optional)

If the manifest specifies client arguments:

1. Update `branding/brand.json` or manifest to include client args:
   ```json
   {
     "client_args": ["--server", "127.0.0.1"]
   }
   ```

2. Launch the game

**Expected:**
- Arguments passed to client process
- Visible in logs and test script output:
  ```
  Arguments: --server 127.0.0.1
  ```

---

## Edge Cases

### Edge Case 1: Executable Missing

1. Remove or rename the client executable
2. Attempt to launch

**Expected:**
- Error message: "Client executable not found"
- Launch button remains enabled
- No crash or hang

### Edge Case 2: No Execute Permission (Unix)

1. Remove execute permission:
   ```bash
   chmod -x [install-directory]/client.exe
   ```
2. Attempt to launch

**Expected:**
- Error message: "Client is not executable"
- Clear guidance for user

**Restore:**
```bash
chmod +x [install-directory]/client.exe
```

### Edge Case 3: Installation Directory Missing

1. Temporarily move the install directory:
   ```bash
   mv [install-directory] [install-directory].bak
   ```
2. Attempt to launch

**Expected:**
- Error message: "Installation directory not found"
- Suggests repair or reinstall

**Restore:**
```bash
mv [install-directory].bak [install-directory]
```

### Edge Case 4: Corrupted Installation

1. Truncate or corrupt the client executable:
   ```bash
   echo "corrupted" > [install-directory]/client.exe
   ```
2. Attempt to launch

**Expected (Unix):**
- Error on spawn: format error or exec failure
- Error message displayed to user

**Expected (Windows):**
- May show system error dialog
- Launcher captures error and displays user-friendly message

### Edge Case 5: Path with Spaces

Ensure launch works when installation path contains spaces:

1. Install to a path like `/tmp/Ultima Online/Game Files/`
2. Launch the game

**Expected:**
- Launch succeeds without path quoting issues
- Working directory set correctly

### Edge Case 6: Multiple Launch Attempts

1. Start the game
2. While game is "running", try clicking the disabled Play button
3. Use browser dev tools to force-enable and click

**Expected:**
- Additional launch attempts blocked
- No multiple processes spawned
- State remains consistent

### Edge Case 7: Launcher Closed While Game Running

1. Launch the game
2. Close the launcher window
3. Reopen the launcher

**Expected:**
- Game continues running independently
- Launcher starts in Ready state (no game tracking across restarts)
- Can manually launch again (may result in two clients)

---

## Pass/Fail Criteria

| Criterion | Pass Condition |
|-----------|----------------|
| Client validates | Executable check passes (exists, is file, has permissions) |
| Process spawns | Client process starts with correct PID |
| Working directory | Process working dir equals install path |
| Arguments passed | Client receives configured args (if any) |
| UI states correct | Button shows Launching → Playing states |
| Exit detected | "Game Closed?" returns UI to ready |
| Error handling | Clear messages for all failure modes |
| Close-on-launch | Launcher closes after successful launch (if enabled) |
| No memory leaks | Repeated launch/close doesn't increase memory |
| Clean shutdown | No orphan processes or zombie states |

## Platform-Specific Notes

### Windows
- Executable must be `.exe`, `.bat`, or `.cmd` (warning for others)
- Process spawning uses CreateProcess
- Working directory via `set_current_dir`
- UAC may interfere with some client executables

### macOS
- May require security approval for unsigned executables
- Gatekeeper may block first launch
- Use `chmod +x` for shell script executables
- App Sandbox may restrict working directory

### Linux
- Executable permission (chmod +x) required
- Check for missing shared libraries (`ldd`)
- Wine prefix may need configuration for Windows clients
- Working directory affects relative path resolution

## Cleanup

```bash
# Stop host-server if running
kill $(lsof -t -i:8080) 2>/dev/null || true

# Close any running test clients
pkill -f "client" 2>/dev/null || true

# Restore any renamed files
mv [install-directory]/client.exe.bak [install-directory]/client.exe 2>/dev/null || true
```

## Notes

- The mock `client.exe` in test-data is a text file and won't execute
- For realistic testing, use a real UO client or test script
- Launch behavior depends on platform-specific process APIs
- The `wait_for_exit` option is available but not used by default
- Close-on-launch relies on Tauri process management

## Related Tests

- [E2E-INSTALL-001](./first-run-install.md) - Prerequisite for this test
- [E2E-UPDATE-001](./update-flow.md) - Update before launch
- [E2E-SECURITY-001](./security-tests.md) - Validates launch only happens from verified install
