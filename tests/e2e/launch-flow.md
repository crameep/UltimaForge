# E2E Test: Launch Flow

## Test ID
`E2E-LAUNCH-001`

## Description
Verifies that the game client launches correctly from the launcher, with proper
working directory, arguments, and state management.

## Status
**Not Yet Implemented** - This test plan will be completed in subtask-7-4.

## Prerequisites

1. Complete installation (E2E-INSTALL-001)
2. Ensure launcher is in Ready state
3. (For real testing) Have an actual UO client executable

## Test Steps

### Step 1: Verify Ready State
- Launch button visible and enabled
- No pending updates

### Step 2: Validate Client
- Client executable exists
- Client passes validation checks

### Step 3: Launch Game
- Click Launch button
- Client process spawns

### Step 4: Verify Working Directory
- Check process working directory
- Should be the install path

### Step 5: UI State During Launch
- Button shows "Launching..."
- Then shows "Game Running"
- "Game Closed?" button available

### Step 6: Game Exit
- Close game client
- Or click "Game Closed?"
- UI returns to Ready state

## Pass/Fail Criteria

| Criterion | Pass Condition |
|-----------|----------------|
| Client validates | Executable check passes |
| Process spawns | Client process starts |
| Working directory | Set to install path |
| UI updates | Shows running state |
| Exit detection | Returns to ready state |

## Notes

- For test purposes, the mock `client.exe` won't actually run
- Test with a real executable for complete verification
- On Windows, executable should be .exe, .bat, or .cmd
- On Unix, executable needs execute permissions
