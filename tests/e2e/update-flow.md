# E2E Test: Update Flow

## Test ID
`E2E-UPDATE-001`

## Description
Verifies the differential update mechanism, including update detection, download
of only changed files, atomic application, and rollback on failure.

## Status
**Not Yet Implemented** - This test plan will be completed in subtask-7-3.

## Prerequisites

1. Complete a fresh installation (E2E-INSTALL-001)
2. Modify test files and publish version 1.1.0:
   ```bash
   # Modify test files
   echo "Updated content" >> ./test-data/sample-client/art.mul

   # Publish new version
   cargo run -p publish-cli -- publish \
     --source ./test-data/sample-client \
     --output ./test-updates \
     --key ./test-keys/private.key \
     --version 1.1.0
   ```

## Test Steps

### Step 1: Verify Initial Installation
- Version 1.0.0 installed
- All files verified

### Step 2: Publish Updated Version
- Modify one or more files
- Publish as version 1.1.0

### Step 3: Launch App and Detect Update
- Update banner appears
- Shows file count and download size
- Only modified files listed

### Step 4: Apply Update
- Start update
- Progress shows correctly
- Only changed files downloaded

### Step 5: Verify Update Complete
- Version now shows 1.1.0
- Modified files have new content
- Unchanged files remain unchanged

### Step 6: Test Rollback (Optional)
- Simulate failure during apply phase
- Verify backup restores correctly

## Pass/Fail Criteria

| Criterion | Pass Condition |
|-----------|----------------|
| Update detected | Version difference shown |
| Differential download | Only modified files downloaded |
| Atomic application | All or nothing applies |
| Rollback works | Failure restores previous state |
| Version updated | New version in config |
