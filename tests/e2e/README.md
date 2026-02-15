# UltimaForge E2E Test Suite

This directory contains end-to-end tests for the UltimaForge launcher system.

## Overview

E2E tests verify the complete integration of all components:
- **Launcher** (Tauri app with React frontend)
- **Host Server** (static file server for updates)
- **Publish CLI** (manifest generation and signing)

## Test Plans

| Test | Description | Status |
|------|-------------|--------|
| [First-Run Installation](./first-run-install.md) | Tests new installation flow | Manual |
| [Update Flow](./update-flow.md) | Tests differential updates | Manual |
| [Launch Flow](./launch-flow.md) | Tests game client launching | Manual |
| [Security](./security-tests.md) | Tests signature/hash verification | Manual |

## Prerequisites

Before running E2E tests:

1. **Build all tools:**
   ```bash
   cargo build --release -p host-server -p publish-cli
   ```

2. **Build the Tauri app:**
   ```bash
   npm install
   npm run tauri build
   ```

3. **Generate test updates:**
   ```bash
   cargo run -p publish-cli -- publish \
     --source ./test-data/sample-client \
     --output ./test-updates \
     --key ./test-keys/private.key \
     --version 1.0.0
   ```

## Running Tests

### Option 1: Automated Script (Recommended)

```bash
# Run all E2E tests
./tests/e2e/run-e2e-tests.sh

# Run specific test
./tests/e2e/run-e2e-tests.sh first-run
```

### Option 2: Manual Testing

Follow the step-by-step instructions in each test plan document.

## Test Data

- `test-data/sample-client/` - Mock UO client files
- `test-keys/` - Ed25519 keypair for testing (RFC 8032 test vector)
- `test-updates/` - Generated update artifacts
- `branding/` - Test branding configuration

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ULTIMAFORGE_UPDATE_URL` | `http://localhost:8080` | Override update server URL |
| `ULTIMAFORGE_TEST_INSTALL_DIR` | System temp dir | Override installation directory |
| `RUST_LOG` | `info` | Logging verbosity |

## Cleanup

After running tests, clean up test artifacts:

```bash
# Remove test installation
rm -rf /tmp/ultimaforge-test-install

# Reset test updates
rm -rf ./test-updates/files ./test-updates/manifest.json ./test-updates/manifest.sig
```
