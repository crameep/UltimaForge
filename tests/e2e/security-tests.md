# E2E Test: Security Verification

## Test ID
`E2E-SECURITY-001`

## Description
Verifies security boundaries including Ed25519 signature verification, SHA-256
hash verification, and directory traversal prevention.

## Status
**Not Yet Implemented** - This test plan will be completed in subtask-7-6.

## Prerequisites

1. Host server running with valid test updates
2. Test keypair available
3. Ability to modify server files for testing

## Test Cases

### SEC-001: Signature Verification - Valid Signature

**Steps:**
1. Serve correctly signed manifest
2. Launch app
3. Update/install should proceed

**Expected:** Update proceeds normally

---

### SEC-002: Signature Verification - Missing Signature

**Steps:**
1. Delete `manifest.sig` from server
2. Launch app
3. Attempt update

**Expected:**
- Error message displayed
- "Signature verification failed"
- No files modified

---

### SEC-003: Signature Verification - Tampered Manifest

**Steps:**
1. Modify `manifest.json` after signing
2. Launch app
3. Attempt update

**Expected:**
- Error message displayed
- "Signature verification failed"
- No files modified

---

### SEC-004: Signature Verification - Wrong Public Key

**Steps:**
1. Generate new keypair
2. Sign with new key, but app has old public key
3. Launch app
3. Attempt update

**Expected:**
- Error message displayed
- "Signature verification failed"
- No files modified

---

### SEC-005: Hash Verification - Valid Hash

**Steps:**
1. Download file with correct hash
2. Verify hash

**Expected:** File accepted

---

### SEC-006: Hash Verification - Corrupted File

**Steps:**
1. Modify a file in `/files/` directory on server
2. Hash in manifest no longer matches
3. Download the file

**Expected:**
- Hash mismatch detected
- Error displayed
- File not applied

---

### SEC-007: Hash Verification - Re-download on Failure

**Steps:**
1. Corrupt file detection
2. Retry download

**Expected:** Up to 3 retry attempts

---

### SEC-008: Path Traversal - Host Server

**Steps:**
1. Request: `GET /files/../../../etc/passwd`
2. Request: `GET /files/..%2F..%2F..%2Fetc%2Fpasswd`

**Expected:**
- Returns 404 or 400
- Does NOT return file contents

---

### SEC-009: Path Traversal - Manifest

**Steps:**
1. Create manifest with path: `../../../etc/passwd`
2. Attempt to process manifest

**Expected:**
- Manifest validation fails
- Error: "Invalid file path"

---

### SEC-010: Public Key Embedding

**Verification:**
- Public key must be embedded at build time
- Cannot be changed at runtime
- Check: No config option to override public key

**Expected:** Public key is immutable

## Pass/Fail Criteria

| Test | Description | Expected Result |
|------|-------------|-----------------|
| SEC-001 | Valid signature | Pass - Proceeds |
| SEC-002 | Missing signature | Fail - Blocked |
| SEC-003 | Tampered manifest | Fail - Blocked |
| SEC-004 | Wrong public key | Fail - Blocked |
| SEC-005 | Valid hash | Pass - Accepted |
| SEC-006 | Corrupted file | Fail - Rejected |
| SEC-007 | Retry on failure | Pass - Retries |
| SEC-008 | Host traversal | Fail - 404/400 |
| SEC-009 | Manifest traversal | Fail - Validation |
| SEC-010 | Key embedding | Pass - Immutable |

## Security Checklist

- [ ] Signature verified BEFORE parsing manifest JSON
- [ ] Hash verified BEFORE applying any file
- [ ] Public key embedded at build time
- [ ] Directory traversal prevented in manifest
- [ ] Directory traversal prevented in host server
- [ ] No remote code execution possible
- [ ] No path escaping in file operations
- [ ] Clear error messages (no sensitive info leaked)
