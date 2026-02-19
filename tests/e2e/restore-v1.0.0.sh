#!/bin/bash
#
# Restore v1.0.0 Test Data
#
# This script restores the test data and manifest back to v1.0.0 state
# after running update flow tests.
#
# Usage:
#   ./restore-v1.0.0.sh
#

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

cd "$PROJECT_ROOT"

# Restore test data
if [ -d "./test-data/sample-client.bak" ]; then
    log_info "Restoring original test data..."
    rm -rf ./test-data/sample-client
    mv ./test-data/sample-client.bak ./test-data/sample-client
    log_success "Test data restored"
else
    log_warning "No backup found at ./test-data/sample-client.bak"
fi

# Restore v1.0.0 manifest
if [ -f "./test-updates/manifest-v1.0.0.json.bak" ]; then
    log_info "Restoring v1.0.0 manifest..."
    mv ./test-updates/manifest-v1.0.0.json.bak ./test-updates/manifest.json
    log_success "Manifest restored to v1.0.0"
else
    log_warning "No manifest backup found. Re-publishing v1.0.0..."
    cargo run --release -p publish-cli -- publish \
        --source ./test-data/sample-client \
        --output ./test-updates \
        --key ./test-keys/private.key \
        --version 1.0.0
    log_success "v1.0.0 published"
fi

log_success "Test environment restored to v1.0.0 state"
echo ""
echo "You may need to restart the host-server to serve the restored files."
