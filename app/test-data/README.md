# UltimaForge Test Data

This directory contains test data for integration testing of the UltimaForge
update and publishing system.

## Contents

### sample-client/

A minimal mock UO client installation containing:

- `client.exe` - Mock client executable
- `art.mul` - Mock art data file
- `map0.mul` - Mock map data file

These files contain placeholder content for testing the manifest generation,
signing, and blob creation workflow.

### ../test-keys/

Ed25519 keypair for testing (RFC 8032 test vector):

- `private.key` - Hex-encoded 32-byte private key seed
- `public.key` - Hex-encoded 32-byte public key

**WARNING:** These are TEST KEYS from RFC 8032. Never use these in production!
Generate new keys with: `cargo run -p publish-cli -- keygen --output ./keys`

### ../test-updates/

Output directory for generated update artifacts:

- `manifest.json` - Signed manifest file
- `manifest.sig` - Ed25519 signature
- `files/` - Content-addressed file blobs

## Usage

### Generate test update:

```bash
cargo run -p publish-cli -- publish \
  --source ./test-data/sample-client \
  --output ./test-updates \
  --key ./test-keys/private.key \
  --version 1.0.0
```

### Validate test update:

```bash
cargo run -p publish-cli -- validate \
  --dir ./test-updates \
  --key ./test-keys/public.key
```

### Serve test updates:

```bash
cargo run -p host-server -- --dir ./test-updates --port 8080
```

## Testing Notes

The sample client files are small text files that simulate real UO client files.
This allows for fast testing of the entire publishing workflow without needing
actual multi-gigabyte UO client files.
