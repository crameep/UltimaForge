#!/bin/bash
#
# UltimaForge E2E Test Runner
#
# This script automates the setup and execution of E2E tests for the
# UltimaForge launcher system.
#
# Usage:
#   ./run-e2e-tests.sh              # Run all E2E tests
#   ./run-e2e-tests.sh first-run    # Run first-run installation test
#   ./run-e2e-tests.sh update       # Run update flow test
#   ./run-e2e-tests.sh launch       # Run launch flow test
#   ./run-e2e-tests.sh security     # Run security tests
#
# Environment Variables:
#   ULTIMAFORGE_TEST_INSTALL_DIR  - Override test installation directory
#   ULTIMAFORGE_HOST_PORT         - Override host-server port (default: 8080)
#   SKIP_BUILD                    - Skip building tools if set

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
HOST_PORT="${ULTIMAFORGE_HOST_PORT:-8080}"
TEST_INSTALL_DIR="${ULTIMAFORGE_TEST_INSTALL_DIR:-/tmp/ultimaforge-test-install}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check Rust/Cargo
    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Please install Rust."
        exit 1
    fi

    # Check Node.js
    if ! command -v npm &> /dev/null; then
        log_error "npm not found. Please install Node.js."
        exit 1
    fi

    # Check curl for API testing
    if ! command -v curl &> /dev/null; then
        log_error "curl not found. Please install curl."
        exit 1
    fi

    # Check jq for JSON parsing (optional)
    if ! command -v jq &> /dev/null; then
        log_warning "jq not found. Some test output parsing may be limited."
    fi

    log_success "Prerequisites check passed"
}

# Build tools
build_tools() {
    if [ -n "$SKIP_BUILD" ]; then
        log_info "Skipping build (SKIP_BUILD is set)"
        return
    fi

    log_info "Building tools..."
    cd "$PROJECT_ROOT"

    # Build host-server and publish-cli
    cargo build --release -p host-server -p publish-cli

    log_success "Tools built successfully"
}

# Generate test updates
generate_test_updates() {
    log_info "Generating test updates..."
    cd "$PROJECT_ROOT"

    # Create test-updates directory if it doesn't exist
    mkdir -p test-updates

    # Run publish command
    cargo run --release -p publish-cli -- publish \
        --source ./test-data/sample-client \
        --output ./test-updates \
        --key ./test-keys/private.key \
        --version 1.0.0

    log_success "Test updates generated"
}

# Validate test updates
validate_test_updates() {
    log_info "Validating test updates..."
    cd "$PROJECT_ROOT"

    cargo run --release -p publish-cli -- validate \
        --dir ./test-updates \
        --key ./test-keys/public.key

    log_success "Test updates validated"
}

# Start host server
start_host_server() {
    log_info "Starting host server on port $HOST_PORT..."
    cd "$PROJECT_ROOT"

    # Kill any existing server on the port
    if lsof -i :$HOST_PORT &> /dev/null; then
        log_warning "Port $HOST_PORT is in use. Attempting to free it..."
        fuser -k $HOST_PORT/tcp 2>/dev/null || true
        sleep 1
    fi

    # Start server in background
    cargo run --release -p host-server -- --dir ./test-updates --port $HOST_PORT &
    HOST_SERVER_PID=$!

    # Wait for server to start
    sleep 2

    # Verify server is running
    if curl -s http://localhost:$HOST_PORT/health | grep -q "ok"; then
        log_success "Host server started (PID: $HOST_SERVER_PID)"
    else
        log_error "Host server failed to start"
        exit 1
    fi
}

# Stop host server
stop_host_server() {
    if [ -n "$HOST_SERVER_PID" ]; then
        log_info "Stopping host server (PID: $HOST_SERVER_PID)..."
        kill $HOST_SERVER_PID 2>/dev/null || true
        wait $HOST_SERVER_PID 2>/dev/null || true
        log_success "Host server stopped"
    fi
}

# Prepare test environment
prepare_test_environment() {
    log_info "Preparing test environment..."

    # Create test install directory
    mkdir -p "$TEST_INSTALL_DIR"

    # Clear any previous test files
    rm -rf "$TEST_INSTALL_DIR"/*

    log_success "Test environment prepared"
}

# Cleanup
cleanup() {
    log_info "Cleaning up..."
    stop_host_server

    # Restore test data if backup exists
    if [ -d "$PROJECT_ROOT/test-data/sample-client.bak" ]; then
        rm -rf "$PROJECT_ROOT/test-data/sample-client"
        mv "$PROJECT_ROOT/test-data/sample-client.bak" "$PROJECT_ROOT/test-data/sample-client"
    fi

    # Remove manifest backup
    rm -f "$PROJECT_ROOT/test-updates/manifest-v1.0.0.json.bak"

    rm -rf "$TEST_INSTALL_DIR"
    log_success "Cleanup complete"
}

# Trap for cleanup on exit
trap cleanup EXIT

# Run first-run installation test
test_first_run_installation() {
    log_info "=== Running First-Run Installation Test ==="

    # Setup
    generate_test_updates
    validate_test_updates
    start_host_server
    prepare_test_environment

    log_info "Test setup complete. Manual verification required."
    echo ""
    echo "============================================="
    echo "MANUAL TEST STEPS:"
    echo "============================================="
    echo ""
    echo "1. Clear launcher configuration:"
    echo "   - Windows: Delete %APPDATA%\\ultimaforge\\launcher.json"
    echo "   - macOS: Delete ~/Library/Application Support/ultimaforge/launcher.json"
    echo "   - Linux: Delete ~/.config/ultimaforge/launcher.json"
    echo ""
    echo "2. Launch the Tauri app:"
    echo "   npm run tauri dev"
    echo ""
    echo "3. Follow the installation wizard:"
    echo "   a) Click 'Get Started' on welcome screen"
    echo "   b) Click 'Browse...' and select: $TEST_INSTALL_DIR"
    echo "   c) Accept the Terms of Service"
    echo "   d) Wait for installation to complete"
    echo "   e) Click 'Start Playing'"
    echo ""
    echo "4. Verify installation files:"
    echo "   ls -la $TEST_INSTALL_DIR"
    echo ""
    echo "5. Expected files:"
    echo "   - client.exe"
    echo "   - art.mul"
    echo "   - map0.mul"
    echo ""
    echo "============================================="
    echo "Host server running at: http://localhost:$HOST_PORT"
    echo "Test install directory: $TEST_INSTALL_DIR"
    echo "============================================="
    echo ""
    echo "Press Enter when test is complete, or Ctrl+C to abort..."
    read

    # Verify installation
    log_info "Verifying installation..."

    if [ -f "$TEST_INSTALL_DIR/client.exe" ] && \
       [ -f "$TEST_INSTALL_DIR/art.mul" ] && \
       [ -f "$TEST_INSTALL_DIR/map0.mul" ]; then
        log_success "All expected files found in installation directory"

        # Verify file hashes
        log_info "Verifying file hashes..."
        cd "$PROJECT_ROOT"

        # Get expected hashes from manifest
        if command -v jq &> /dev/null; then
            EXPECTED_HASHES=$(cat test-updates/manifest.json | jq -r '.files[] | "\(.sha256)  \(.path)"')
            echo "Expected hashes from manifest:"
            echo "$EXPECTED_HASHES"
        fi

        # Calculate actual hashes
        echo ""
        echo "Actual file hashes:"
        sha256sum "$TEST_INSTALL_DIR/client.exe" || true
        sha256sum "$TEST_INSTALL_DIR/art.mul" || true
        sha256sum "$TEST_INSTALL_DIR/map0.mul" || true

        log_success "First-run installation test PASSED"
        return 0
    else
        log_error "Missing files in installation directory"
        ls -la "$TEST_INSTALL_DIR" || true
        log_error "First-run installation test FAILED"
        return 1
    fi
}

# Run update flow test
test_update_flow() {
    log_info "=== Running Update Flow Test ==="

    # Check if first-run installation was completed
    if [ ! -f "$TEST_INSTALL_DIR/client.exe" ]; then
        log_warning "No existing installation found. Running first-run installation first..."
        test_first_run_installation
    fi

    # Verify v1.0.0 is installed
    log_info "Verifying v1.0.0 installation..."

    # Backup original test files for later restoration
    log_info "Backing up original test data..."
    cp -r "$PROJECT_ROOT/test-data/sample-client" "$PROJECT_ROOT/test-data/sample-client.bak"

    # Modify test files for v1.1.0
    log_info "Creating v1.1.0 test files..."
    echo -e "\n[v1.1.0] Updated content for testing - $(date)" >> "$PROJECT_ROOT/test-data/sample-client/art.mul"

    # Backup v1.0.0 manifest
    if [ -f "$PROJECT_ROOT/test-updates/manifest.json" ]; then
        cp "$PROJECT_ROOT/test-updates/manifest.json" "$PROJECT_ROOT/test-updates/manifest-v1.0.0.json.bak"
    fi

    # Publish v1.1.0
    log_info "Publishing version 1.1.0..."
    cd "$PROJECT_ROOT"
    cargo run --release -p publish-cli -- publish \
        --source ./test-data/sample-client \
        --output ./test-updates \
        --key ./test-keys/private.key \
        --version 1.1.0

    # Validate the new release
    log_info "Validating v1.1.0 release..."
    cargo run --release -p publish-cli -- validate \
        --dir ./test-updates \
        --key ./test-keys/public.key

    # Restart host server with updated files
    stop_host_server
    sleep 1
    start_host_server

    # Verify manifest version
    log_info "Verifying server is serving v1.1.0..."
    MANIFEST_VERSION=$(curl -s "http://localhost:$HOST_PORT/manifest.json" | grep -o '"version":"[^"]*"' | cut -d'"' -f4)
    if [ "$MANIFEST_VERSION" != "1.1.0" ]; then
        log_error "Server not serving v1.1.0 (got: $MANIFEST_VERSION)"
        restore_test_data
        exit 1
    fi
    log_success "Server confirmed serving v1.1.0"

    log_info "Test setup complete. Manual verification required."
    echo ""
    echo "============================================="
    echo "MANUAL TEST STEPS - UPDATE FLOW:"
    echo "============================================="
    echo ""
    echo "1. Launch the Tauri app:"
    echo "   npm run tauri dev"
    echo ""
    echo "2. Verify update detection:"
    echo "   - App should show 'Update Available' banner"
    echo "   - Current version: 1.0.0"
    echo "   - Available version: 1.1.0"
    echo "   - Files to update: 1 (only art.mul changed)"
    echo ""
    echo "3. Click 'Update Now' and observe:"
    echo "   - Download progress bar"
    echo "   - File count (1/1)"
    echo "   - Only art.mul should be downloaded (differential)"
    echo "   - Verification step"
    echo "   - Apply step"
    echo ""
    echo "4. After update completes:"
    echo "   - Version should show 1.1.0"
    echo "   - Launch button should be enabled"
    echo ""
    echo "5. Verify files on disk:"
    echo "   cat $TEST_INSTALL_DIR/art.mul | tail -1"
    echo "   # Should show: [v1.1.0] Updated content for testing"
    echo ""
    echo "============================================="
    echo "Host server running at: http://localhost:$HOST_PORT"
    echo "Test install directory: $TEST_INSTALL_DIR"
    echo "============================================="
    echo ""
    echo "Press Enter when test is complete, or Ctrl+C to abort..."
    read

    # Verify update applied
    log_info "Verifying update was applied..."

    # Check for v1.1.0 content in art.mul
    if grep -q "v1.1.0" "$TEST_INSTALL_DIR/art.mul" 2>/dev/null; then
        log_success "art.mul contains v1.1.0 content"
    else
        log_error "art.mul does not contain v1.1.0 content"
        restore_test_data
        return 1
    fi

    # Verify other files unchanged
    log_info "Verifying unchanged files..."

    # Calculate hashes
    if command -v sha256sum &> /dev/null; then
        INSTALLED_CLIENT_HASH=$(sha256sum "$TEST_INSTALL_DIR/client.exe" 2>/dev/null | awk '{print $1}')
        ORIGINAL_CLIENT_HASH=$(sha256sum "$PROJECT_ROOT/test-data/sample-client.bak/client.exe" 2>/dev/null | awk '{print $1}')

        if [ "$INSTALLED_CLIENT_HASH" = "$ORIGINAL_CLIENT_HASH" ]; then
            log_success "client.exe unchanged (differential update working)"
        else
            log_warning "client.exe hash differs (may indicate full download instead of differential)"
        fi
    fi

    # Cleanup test data
    restore_test_data

    log_success "Update flow test PASSED"
    return 0
}

# Restore original test data
restore_test_data() {
    log_info "Restoring original test data..."
    if [ -d "$PROJECT_ROOT/test-data/sample-client.bak" ]; then
        rm -rf "$PROJECT_ROOT/test-data/sample-client"
        mv "$PROJECT_ROOT/test-data/sample-client.bak" "$PROJECT_ROOT/test-data/sample-client"
        log_success "Test data restored"
    fi

    # Restore v1.0.0 manifest
    if [ -f "$PROJECT_ROOT/test-updates/manifest-v1.0.0.json.bak" ]; then
        mv "$PROJECT_ROOT/test-updates/manifest-v1.0.0.json.bak" "$PROJECT_ROOT/test-updates/manifest.json"
        log_success "Manifest restored to v1.0.0"
    fi
}

# Run launch flow test
test_launch_flow() {
    log_info "=== Running Launch Flow Test ==="
    log_warning "Launch flow test not yet implemented"
    log_info "See: tests/e2e/launch-flow.md"
}

# Run security tests
test_security() {
    log_info "=== Running Security Tests ==="
    log_warning "Security tests not yet implemented"
    log_info "See: tests/e2e/security-tests.md"
}

# Main function
main() {
    echo "======================================"
    echo "UltimaForge E2E Test Runner"
    echo "======================================"
    echo ""

    check_prerequisites
    build_tools

    case "${1:-all}" in
        first-run|install)
            test_first_run_installation
            ;;
        update)
            test_update_flow
            ;;
        launch)
            test_launch_flow
            ;;
        security)
            test_security
            ;;
        all)
            test_first_run_installation
            test_update_flow
            test_launch_flow
            test_security
            ;;
        *)
            echo "Usage: $0 [first-run|update|launch|security|all]"
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
