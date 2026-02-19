#!/bin/bash
#
# Setup v1.1.0 Test Data
#
# This script modifies test files and publishes v1.1.0 for update flow testing.
# Run this after completing a v1.0.0 installation to test the update mechanism.
#
# Usage:
#   ./setup-v1.1.0.sh
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

# Check if v1.0.0 test updates exist
if [ ! -f "./test-updates/manifest.json" ]; then
    log_warning "No v1.0.0 test updates found. Generate them first with:"
    echo "  cargo run -p publish-cli -- publish \\"
    echo "    --source ./test-data/sample-client \\"
    echo "    --output ./test-updates \\"
    echo "    --key ./test-keys/private.key \\"
    echo "    --version 1.0.0"
    exit 1
fi

# Backup current manifest
log_info "Backing up v1.0.0 manifest..."
cp ./test-updates/manifest.json ./test-updates/manifest-v1.0.0.json.bak

# Backup original test data
log_info "Backing up original test data..."
if [ -d "./test-data/sample-client.bak" ]; then
    rm -rf ./test-data/sample-client.bak
fi
cp -r ./test-data/sample-client ./test-data/sample-client.bak

# Modify test files for v1.1.0
log_info "Modifying test files for v1.1.0..."
echo -e "\n[v1.1.0] Updated content for testing - $(date)" >> ./test-data/sample-client/art.mul

# Optional: Add a new file
echo "This is a new configuration file added in v1.1.0" > ./test-data/sample-client/config.ini

# Show what changed
log_info "Changes made:"
echo "  - art.mul: Added v1.1.0 marker"
echo "  - config.ini: New file added"

# Publish v1.1.0
log_info "Publishing version 1.1.0..."
cargo run --release -p publish-cli -- publish \
    --source ./test-data/sample-client \
    --output ./test-updates \
    --key ./test-keys/private.key \
    --version 1.1.0

# Validate
log_info "Validating v1.1.0 release..."
cargo run --release -p publish-cli -- validate \
    --dir ./test-updates \
    --key ./test-keys/public.key

log_success "v1.1.0 test data ready!"
echo ""
echo "Next steps:"
echo "  1. Start the host server:"
echo "     cargo run -p host-server -- --dir ./test-updates --port 8080"
echo ""
echo "  2. Launch the Tauri app:"
echo "     npm run tauri dev"
echo ""
echo "  3. The app should detect v1.1.0 update is available"
echo ""
echo "To restore to v1.0.0:"
echo "  ./tests/e2e/restore-v1.0.0.sh"
