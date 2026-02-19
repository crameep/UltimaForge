# E2E Test: Security Verification

## Test ID
`E2E-SECURITY-001`

## Description
Verifies security boundaries including Ed25519 signature verification, SHA-256
hash verification, and directory traversal prevention.

## Status
**Implemented** - Security verification tests are complete and can be run via Cargo.

## Running Security Tests

### Quick Test (All Security Tests)
```bash
# Run all security tests
cargo test --package ultimaforge security_tests -- --nocapture

# Run with verbose output
cargo test --package ultimaforge security_tests -- --nocapture --test-threads=1
```

### Individual Test Categories
```bash
# Signature bypass tests
cargo test --package ultimaforge test_sec_00 -- --nocapture

# Hash bypass tests
cargo test --package ultimaforge test_sec_006 -- --nocapture

# Path traversal tests
cargo test --package ultimaforge test_sec_008 -- --nocapture
```

## Prerequisites

1. Rust toolchain installed (`cargo --version`)
2. Project builds successfully (`cargo check --workspace`)
3. Test keys available in `test-keys/` (for E2E tests)

## Test Cases

### SEC-001: Signature Verification - Valid Signature

**Test:** `test_sec_001_valid_signature_accepted`

**Steps:**
1. Generate Ed25519 keypair
2. Sign manifest with private key
3. Verify with corresponding public key

**Expected:** Verification succeeds ✓

---

### SEC-002: Signature Verification - Missing Signature

**Test:** `test_sec_002_missing_signature_rejected`

**Steps:**
1. Provide empty/missing signature bytes
2. Attempt verification

**Expected:**
- Returns `InvalidSignatureLength(0)` error
- Update blocked

---

### SEC-003: Signature Verification - Tampered Manifest

**Tests:**
- `test_sec_003_tampered_manifest_rejected`
- `test_sec_003b_single_byte_modification_detected`
- `test_sec_003c_added_whitespace_detected`

**Steps:**
1. Sign original manifest
2. Modify manifest content (version, files, whitespace)
3. Verify modified manifest with original signature

**Expected:**
- Returns `VerificationFailed` error
- Single-byte modifications detected
- Whitespace additions detected
- No files modified

---

### SEC-004: Signature Verification - Wrong Public Key

**Tests:**
- `test_sec_004_wrong_public_key_rejected`
- `test_sec_004b_invalid_public_key_format_rejected`
- `test_sec_004c_invalid_signature_length_rejected`
- `test_sec_004d_corrupted_signature_rejected`
- `test_sec_004e_zero_signature_rejected`

**Steps:**
1. Sign manifest with key A
2. Verify with key B (different key)
3. Test with invalid key formats
4. Test with corrupted signatures

**Expected:**
- Wrong key: `VerificationFailed`
- Invalid format: `InvalidPublicKeyLength`
- Invalid signature: `InvalidSignatureLength` or `VerificationFailed`

---

### SEC-005: Hash Verification - Valid Hash

**Test:** `test_sec_005_valid_hash_accepted`

**Steps:**
1. Create file with known content
2. Compute SHA-256 hash
3. Verify hash matches

**Expected:** Verification returns `true` ✓

---

### SEC-006: Hash Verification - Corrupted File

**Tests:**
- `test_sec_006_corrupted_file_rejected`
- `test_sec_006b_single_byte_corruption_detected`
- `test_sec_006c_appended_data_detected`
- `test_sec_006d_truncated_file_detected`
- `test_sec_006e_invalid_hash_format_rejected`
- `test_sec_006f_empty_file_hash`

**Steps:**
1. Create file with content
2. Modify content (corrupt, append, truncate)
3. Verify against original hash

**Expected:**
- Hash mismatch returns `false`
- Single-byte corruption detected
- Appended data detected
- Truncation detected
- Invalid hash format returns error

---

### SEC-007: Hash Verification - Re-download on Failure

**Verification:**
- Updater retries download up to 3 times on failure
- Configuration: `DownloaderConfig::max_retries`

**Expected:** Up to 3 retry attempts ✓

---

### SEC-008: Path Traversal - Manifest Validation

**Tests:**
- `test_sec_008_path_traversal_rejected`
- `test_sec_008b_absolute_paths_rejected`
- `test_sec_008c_embedded_traversal_rejected`
- `test_sec_008d_valid_subdirectory_accepted`

**Paths Tested:**
- `../../../etc/passwd` → Rejected
- `..\..\Windows\System32` → Rejected
- `/etc/passwd` → Rejected
- `\Windows\System32` → Rejected
- `data/../../../etc/passwd` → Rejected
- `data/maps/map0.mul` → Accepted
- `client.exe` → Accepted

**Expected:**
- Path traversal attempts return `InvalidPath` error
- Absolute paths rejected
- Valid relative paths accepted

---

### SEC-009: Manifest Validation

**Tests:**
- `test_sec_009_invalid_hash_in_manifest_rejected`
- `test_sec_009b_total_size_mismatch_detected`
- `test_sec_009c_executable_not_in_files_rejected`

**Validations:**
- Hash format (64 hex characters)
- Hash characters (0-9, a-f only)
- Total size matches sum of file sizes
- Client executable exists in files list

---

### SEC-010: Public Key Embedding

**Test:** `test_sec_010_hex_parsing_security`

**Verification:**
- Public key embedded at build time
- Cannot be changed at runtime
- Invalid hex formats rejected
- Wrong-length keys rejected

**Expected:** Public key is immutable ✓

---

### SEC-011: Signature Malleability Prevention

**Test:** `test_sec_011_signature_malleability_prevented`

**Description:**
Uses `verify_strict()` instead of `verify()` to prevent signature malleability attacks.

**Expected:** Malleable signatures rejected ✓

---

### SEC-012: Signature Uniqueness

**Test:** `test_sec_012_signature_uniqueness`

**Description:**
Verifies that different messages produce different signatures and
cross-verification fails.

**Expected:** Signatures are unique per message ✓

---

### SEC-013: Hash Determinism

**Test:** `test_sec_013_hash_determinism`

**Description:**
Verifies SHA-256 hashing is deterministic and collision-resistant.

**Expected:** Same content always produces same hash ✓

---

## Manual E2E Security Testing

For comprehensive security testing, also perform these manual tests:

### Test: Missing Signature File
```bash
# Start host server
cargo run -p host-server -- --dir ./test-updates --port 8080

# Remove signature file
mv test-updates/manifest.sig test-updates/manifest.sig.bak

# Launch Tauri app and check for update
# Expected: "Signature verification failed" error

# Restore signature
mv test-updates/manifest.sig.bak test-updates/manifest.sig
```

### Test: Tampered Manifest
```bash
# Backup and modify manifest
cp test-updates/manifest.json test-updates/manifest.json.bak
echo '{}' > test-updates/manifest.json

# Launch Tauri app
# Expected: "Signature verification failed" error

# Restore manifest
mv test-updates/manifest.json.bak test-updates/manifest.json
```

### Test: Corrupted Blob File
```bash
# Get a file hash from manifest
HASH=$(cat test-updates/manifest.json | grep -o '"sha256": "[^"]*"' | head -1 | cut -d'"' -f4)

# Corrupt the file
echo "corrupted" > test-updates/files/$HASH

# Download should fail
# Expected: Hash mismatch error, re-download attempted

# Restore by re-publishing
cargo run -p publish-cli -- publish ...
```

### Test: Host Server Path Traversal
```bash
# Start host server
cargo run -p host-server -- --dir ./test-updates --port 8080

# Try path traversal attacks
curl -s "http://localhost:8080/files/../../../etc/passwd"
curl -s "http://localhost:8080/files/..%2F..%2F..%2Fetc%2Fpasswd"
curl -s "http://localhost:8080/../manifest.json"

# Expected: 404 Not Found (not file contents)
```

## Pass/Fail Criteria

| Test | Description | Expected Result | Implementation |
|------|-------------|-----------------|----------------|
| SEC-001 | Valid signature | Pass - Proceeds | ✅ Unit test |
| SEC-002 | Missing signature | Fail - Blocked | ✅ Unit test |
| SEC-003 | Tampered manifest | Fail - Blocked | ✅ Unit test |
| SEC-004 | Wrong public key | Fail - Blocked | ✅ Unit test |
| SEC-005 | Valid hash | Pass - Accepted | ✅ Unit test |
| SEC-006 | Corrupted file | Fail - Rejected | ✅ Unit test |
| SEC-007 | Retry on failure | Pass - Retries | ✅ Downloader |
| SEC-008 | Path traversal | Fail - 404/400 | ✅ Unit test |
| SEC-009 | Manifest validation | Fail - Validation | ✅ Unit test |
| SEC-010 | Key embedding | Pass - Immutable | ✅ Build-time |
| SEC-011 | Malleability | Fail - Rejected | ✅ Unit test |
| SEC-012 | Sig uniqueness | Pass - Unique | ✅ Unit test |
| SEC-013 | Hash determinism | Pass - Deterministic | ✅ Unit test |

## Security Checklist

- [x] Signature verified BEFORE parsing manifest JSON
- [x] Hash verified BEFORE applying any file
- [x] Public key embedded at build time
- [x] Directory traversal prevented in manifest
- [x] Directory traversal prevented in host server
- [x] No remote code execution possible
- [x] No path escaping in file operations
- [x] Clear error messages (no sensitive info leaked)
- [x] Uses verify_strict() for malleability protection
- [x] Invalid formats rejected early

## Test Files

| File | Description |
|------|-------------|
| `src-tauri/src/security_tests.rs` | Comprehensive security test module |
| `src-tauri/src/signature.rs` | Signature verification with tests |
| `src-tauri/src/hash.rs` | Hash verification with tests |
| `src-tauri/src/manifest.rs` | Manifest validation with tests |

## Running All Security Tests

```bash
# Full security test suite
cargo test --package ultimaforge security_tests -- --nocapture

# Expected output:
# ╔══════════════════════════════════════════════════════════╗
# ║         ULTIMAFORGE SECURITY VERIFICATION TESTS          ║
# ╠══════════════════════════════════════════════════════════╣
# ║  ✓ SEC-001: Valid signature accepted                     ║
# ║  ✓ SEC-002: Missing signature rejected                   ║
# ║  ✓ SEC-003: Tampered manifest rejected                   ║
# ║  ✓ SEC-004: Wrong public key rejected                    ║
# ║  ✓ SEC-005: Valid file hash accepted                     ║
# ║  ✓ SEC-006: Corrupted file rejected                      ║
# ║  ✓ SEC-008: Path traversal rejected                      ║
# ║  ✓ SEC-009: Invalid manifest rejected                    ║
# ║  ✓ SEC-010: Hex parsing security enforced                ║
# ║  ✓ SEC-011: Signature malleability prevented             ║
# ║  ✓ SEC-012: Signature uniqueness verified                ║
# ║  ✓ SEC-013: Hash determinism verified                    ║
# ╚══════════════════════════════════════════════════════════╝
```
