#!/usr/bin/env bash
# UltimaForge Linux/macOS Setup Script
# Installs Rust, Node.js, platform dependencies, and Tauri CLI
#
# Usage:
#   ./scripts/setup.sh                    # Interactive mode
#   ./scripts/setup.sh --skip-prompts     # Non-interactive (CI mode)
#   ./scripts/setup.sh -y                 # Same as --skip-prompts
#   ./scripts/setup.sh --help             # Show help

set -e

# Version requirements
RUST_MIN_VERSION="1.77.2"
NODE_MIN_VERSION="18.0.0"
NPM_MIN_VERSION="8.0.0"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
NC='\033[0m' # No Color

# Flags
SKIP_PROMPTS=false

# Parse arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-prompts|-y)
                SKIP_PROMPTS=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                echo -e "${RED}Unknown option: $1${NC}"
                show_help
                exit 1
                ;;
        esac
    done
}

show_help() {
    cat << 'EOF'
UltimaForge Linux/macOS Setup Script

USAGE:
    ./scripts/setup.sh [OPTIONS]

OPTIONS:
    --skip-prompts, -y    Run without user prompts (for CI/automation)
    --help, -h            Show this help message

DESCRIPTION:
    This script installs all dependencies required to build UltimaForge:
    - Rust (via rustup)
    - Node.js LTS
    - Platform-specific dependencies (WebKitGTK on Linux, Xcode tools on macOS)
    - Tauri CLI (via npm)

SUPPORTED PLATFORMS:
    - Ubuntu/Debian Linux (apt)
    - Fedora Linux (dnf)
    - Arch Linux (pacman)
    - openSUSE Linux (zypper)
    - macOS (Homebrew)

EXAMPLES:
    # Interactive installation
    ./scripts/setup.sh

    # Non-interactive installation for CI
    ./scripts/setup.sh --skip-prompts

    # Same as above
    ./scripts/setup.sh -y

EOF
}

# Output helpers
write_status() {
    local message="$1"
    local type="${2:-info}"

    case $type in
        success) echo -e "${GREEN}✓ $message${NC}" ;;
        warning) echo -e "${YELLOW}⚠ $message${NC}" ;;
        error)   echo -e "${RED}✗ $message${NC}" ;;
        info)    echo -e "${CYAN}→ $message${NC}" ;;
        step)    echo -e "\n${WHITE}▶ $message${NC}" ;;
    esac
}

# Check if command exists
check_command() {
    if command -v "$1" &> /dev/null; then
        return 0
    else
        return 1
    fi
}

# Compare semantic versions
# Returns 0 if actual >= required, 1 otherwise
compare_version() {
    local actual="$1"
    local required="$2"

    # Clean version strings (remove 'v' prefix and trailing info)
    actual=$(echo "$actual" | sed 's/^v//' | sed 's/[^0-9.].*$//')
    required=$(echo "$required" | sed 's/^v//')

    # Split into arrays
    IFS='.' read -ra actual_parts <<< "$actual"
    IFS='.' read -ra required_parts <<< "$required"

    for i in 0 1 2; do
        local a="${actual_parts[$i]:-0}"
        local r="${required_parts[$i]:-0}"

        if (( a > r )); then
            return 0
        elif (( a < r )); then
            return 1
        fi
    done

    return 0  # Equal versions
}

# Get user confirmation
get_confirmation() {
    local message="$1"

    if [[ "$SKIP_PROMPTS" == "true" ]]; then
        return 0
    fi

    read -rp "$message (Y/n) " response
    if [[ -z "$response" || "$response" =~ ^[Yy] ]]; then
        return 0
    fi
    return 1
}

# Detect OS and distribution
detect_os() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "macos"
    elif [[ -f /etc/os-release ]]; then
        # shellcheck disable=SC1091
        source /etc/os-release
        case "$ID" in
            ubuntu|debian|pop|linuxmint|elementary)
                echo "debian"
                ;;
            fedora|rhel|centos|rocky|alma)
                echo "fedora"
                ;;
            arch|manjaro|endeavouros)
                echo "arch"
                ;;
            opensuse*|sles)
                echo "opensuse"
                ;;
            *)
                echo "unknown"
                ;;
        esac
    else
        echo "unknown"
    fi
}

# Detect architecture
detect_arch() {
    local arch
    arch=$(uname -m)
    case "$arch" in
        x86_64|amd64)
            echo "x86_64"
            ;;
        aarch64|arm64)
            echo "aarch64"
            ;;
        *)
            echo "$arch"
            ;;
    esac
}

# Install Rust
install_rust() {
    write_status "Checking Rust installation..." "step"

    if check_command rustc; then
        local version
        version=$(rustc --version | awk '{print $2}')
        if compare_version "$version" "$RUST_MIN_VERSION"; then
            write_status "Rust $version already installed (>= $RUST_MIN_VERSION required)" "success"
            return 0
        else
            write_status "Rust $version installed but $RUST_MIN_VERSION or newer required" "warning"
            if ! get_confirmation "Update Rust?"; then
                return 1
            fi

            write_status "Updating Rust..." "info"
            rustup update stable
            return $?
        fi
    fi

    write_status "Installing Rust..." "info"

    if ! curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable; then
        write_status "Failed to install Rust" "error"
        write_status "Try manual installation: https://rustup.rs" "info"
        return 1
    fi

    # Source cargo environment
    if [[ -f "$HOME/.cargo/env" ]]; then
        # shellcheck disable=SC1091
        source "$HOME/.cargo/env"
    fi

    # Verify installation
    if check_command rustc; then
        local version
        version=$(rustc --version | awk '{print $2}')
        write_status "Rust $version installed successfully" "success"
        return 0
    else
        write_status "Rust installed but not found in PATH" "warning"
        write_status "Please restart your terminal or run: source ~/.cargo/env" "info"
        return 0
    fi
}

# Install Node.js
install_nodejs() {
    write_status "Checking Node.js installation..." "step"

    if check_command node; then
        local version
        version=$(node --version | sed 's/^v//')
        if compare_version "$version" "$NODE_MIN_VERSION"; then
            write_status "Node.js v$version already installed (>= v$NODE_MIN_VERSION required)" "success"
            return 0
        else
            write_status "Node.js v$version installed but v$NODE_MIN_VERSION or newer required" "warning"
        fi
    fi

    write_status "Installing Node.js LTS..." "info"

    local os
    os=$(detect_os)

    case "$os" in
        macos)
            if ! check_command brew; then
                write_status "Homebrew not found. Installing Homebrew first..." "info"
                if ! /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"; then
                    write_status "Failed to install Homebrew" "error"
                    return 1
                fi
                # Add Homebrew to PATH for this session
                if [[ -f "/opt/homebrew/bin/brew" ]]; then
                    eval "$(/opt/homebrew/bin/brew shellenv)"
                elif [[ -f "/usr/local/bin/brew" ]]; then
                    eval "$(/usr/local/bin/brew shellenv)"
                fi
            fi

            write_status "Installing Node.js via Homebrew..." "info"
            if ! brew install node@20; then
                write_status "Failed to install Node.js via Homebrew" "error"
                return 1
            fi

            # Link node@20 if it's not already linked
            brew link --overwrite node@20 2>/dev/null || true
            ;;

        debian)
            write_status "Installing Node.js via NodeSource..." "info"
            if ! curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -; then
                write_status "Failed to add NodeSource repository" "error"
                return 1
            fi
            if ! sudo apt-get install -y nodejs; then
                write_status "Failed to install Node.js" "error"
                return 1
            fi
            ;;

        fedora)
            write_status "Installing Node.js via dnf..." "info"
            if ! sudo dnf install -y nodejs npm; then
                write_status "Failed to install Node.js" "error"
                return 1
            fi
            ;;

        arch)
            write_status "Installing Node.js via pacman..." "info"
            if ! sudo pacman -S --noconfirm nodejs npm; then
                write_status "Failed to install Node.js" "error"
                return 1
            fi
            ;;

        opensuse)
            write_status "Installing Node.js via zypper..." "info"
            if ! sudo zypper install -y nodejs npm; then
                write_status "Failed to install Node.js" "error"
                return 1
            fi
            ;;

        *)
            write_status "Unknown OS. Please install Node.js 18+ manually." "error"
            write_status "Visit: https://nodejs.org/en/download/" "info"
            return 1
            ;;
    esac

    # Verify installation
    if check_command node; then
        local version
        version=$(node --version)
        write_status "Node.js $version installed successfully" "success"
        return 0
    else
        write_status "Node.js installation may have failed" "warning"
        return 1
    fi
}

# Install platform-specific dependencies
install_platform_deps() {
    write_status "Checking platform dependencies..." "step"

    local os
    os=$(detect_os)

    case "$os" in
        macos)
            # Check for Xcode Command Line Tools
            if ! xcode-select -p &> /dev/null; then
                write_status "Installing Xcode Command Line Tools..." "info"
                if ! xcode-select --install 2>/dev/null; then
                    write_status "Please install Xcode Command Line Tools manually" "warning"
                    write_status "Run: xcode-select --install" "info"
                fi
            else
                write_status "Xcode Command Line Tools already installed" "success"
            fi
            return 0
            ;;

        debian)
            write_status "Installing Linux dependencies (WebKitGTK, etc.)..." "info"
            if ! sudo apt-get update; then
                write_status "Failed to update package lists" "error"
                return 1
            fi

            local packages=(
                libwebkit2gtk-4.1-dev
                build-essential
                curl
                wget
                file
                libgtk-3-dev
                libayatana-appindicator3-dev
                librsvg2-dev
                libssl-dev
                pkg-config
            )

            if ! sudo apt-get install -y "${packages[@]}"; then
                write_status "Failed to install dependencies" "error"
                return 1
            fi

            write_status "Linux dependencies installed successfully" "success"
            ;;

        fedora)
            write_status "Installing Linux dependencies (WebKitGTK, etc.)..." "info"
            local packages=(
                webkit2gtk4.1-devel
                openssl-devel
                curl
                wget
                file
                gtk3-devel
                libappindicator-gtk3-devel
                librsvg2-devel
                gcc
            )

            if ! sudo dnf install -y "${packages[@]}"; then
                write_status "Failed to install dependencies" "error"
                return 1
            fi

            write_status "Linux dependencies installed successfully" "success"
            ;;

        arch)
            write_status "Installing Linux dependencies (WebKitGTK, etc.)..." "info"
            local packages=(
                webkit2gtk-4.1
                base-devel
                curl
                wget
                file
                openssl
                gtk3
                libappindicator-gtk3
                librsvg
            )

            if ! sudo pacman -S --noconfirm "${packages[@]}"; then
                write_status "Failed to install dependencies" "error"
                return 1
            fi

            write_status "Linux dependencies installed successfully" "success"
            ;;

        opensuse)
            write_status "Installing Linux dependencies (WebKitGTK, etc.)..." "info"
            local packages=(
                webkit2gtk3-devel
                libopenssl-devel
                curl
                wget
                file
                gtk3-devel
                libappindicator3-devel
                librsvg-devel
                gcc
            )

            if ! sudo zypper install -y "${packages[@]}"; then
                write_status "Failed to install dependencies" "error"
                return 1
            fi

            write_status "Linux dependencies installed successfully" "success"
            ;;

        *)
            write_status "Unknown OS. Please install WebKitGTK and dependencies manually." "warning"
            write_status "See: https://v2.tauri.app/start/prerequisites/" "info"
            return 0
            ;;
    esac

    return 0
}

# Install Tauri CLI
install_tauri_cli() {
    write_status "Checking Tauri CLI installation..." "step"

    if ! check_command npm; then
        write_status "npm not found. Please install Node.js first." "error"
        return 1
    fi

    # Check for global Tauri CLI
    local tauri_version
    tauri_version=$(npm list -g @tauri-apps/cli 2>/dev/null | grep "@tauri-apps/cli@" | sed 's/.*@tauri-apps\/cli@//' | sed 's/[^0-9.].*$//' || echo "")

    if [[ -n "$tauri_version" ]] && compare_version "$tauri_version" "2.0.0"; then
        write_status "Tauri CLI v$tauri_version already installed globally" "success"
        return 0
    fi

    # Check for local installation
    local local_version
    local_version=$(npm list @tauri-apps/cli 2>/dev/null | grep "@tauri-apps/cli@" | sed 's/.*@tauri-apps\/cli@//' | sed 's/[^0-9.].*$//' || echo "")

    if [[ -n "$local_version" ]]; then
        write_status "Tauri CLI v$local_version found in project dependencies" "success"
        return 0
    fi

    write_status "Installing Tauri CLI globally..." "info"

    if ! get_confirmation "Install @tauri-apps/cli globally via npm?"; then
        write_status "Skipping Tauri CLI global installation" "warning"
        write_status "You can install it locally with: npm install @tauri-apps/cli" "info"
        return 0
    fi

    if ! npm install -g @tauri-apps/cli; then
        write_status "Failed to install Tauri CLI" "error"
        write_status "Try: npm install -g @tauri-apps/cli" "info"
        return 1
    fi

    # Verify installation
    tauri_version=$(npm list -g @tauri-apps/cli 2>/dev/null | grep "@tauri-apps/cli@" | sed 's/.*@tauri-apps\/cli@//' | sed 's/[^0-9.].*$//' || echo "")
    if [[ -n "$tauri_version" ]]; then
        write_status "Tauri CLI v$tauri_version installed successfully" "success"
        return 0
    fi

    write_status "Tauri CLI installation may have failed" "warning"
    return 1
}

# Show installation summary
show_summary() {
    local rust_ok="$1"
    local node_ok="$2"
    local deps_ok="$3"
    local tauri_ok="$4"

    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}                  Installation Summary                   ${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
    echo ""

    local all_success=true

    if [[ "$rust_ok" == "true" ]]; then
        echo -e "  ${GREEN}✓ Rust${NC}"
    else
        echo -e "  ${RED}✗ Rust${NC}"
        all_success=false
    fi

    if [[ "$node_ok" == "true" ]]; then
        echo -e "  ${GREEN}✓ Node.js${NC}"
    else
        echo -e "  ${RED}✗ Node.js${NC}"
        all_success=false
    fi

    if [[ "$deps_ok" == "true" ]]; then
        echo -e "  ${GREEN}✓ Platform Dependencies${NC}"
    else
        echo -e "  ${RED}✗ Platform Dependencies${NC}"
        all_success=false
    fi

    if [[ "$tauri_ok" == "true" ]]; then
        echo -e "  ${GREEN}✓ Tauri CLI${NC}"
    else
        echo -e "  ${RED}✗ Tauri CLI${NC}"
        all_success=false
    fi

    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"

    if [[ "$all_success" == "true" ]]; then
        echo -e "\n${GREEN}All dependencies installed successfully!${NC}"
        echo -e "\n${WHITE}Next steps:${NC}"
        echo -e "${WHITE}  1. Open a new terminal (to refresh PATH)${NC}"
        echo -e "${WHITE}  2. Run: npm install${NC}"
        echo -e "${WHITE}  3. Run: npm run tauri dev${NC}"
        echo ""
        return 0
    else
        echo -e "\n${YELLOW}Some dependencies failed to install.${NC}"
        echo -e "${YELLOW}Please check the errors above and try again.${NC}"
        echo ""
        return 1
    fi
}

# Main function
main() {
    parse_args "$@"

    echo ""
    echo -e "${CYAN}╔═══════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║         UltimaForge Setup Script (Linux/macOS)        ║${NC}"
    echo -e "${CYAN}╚═══════════════════════════════════════════════════════╝${NC}"
    echo ""

    # Detect environment
    local os
    local arch
    os=$(detect_os)
    arch=$(detect_arch)
    write_status "Detected OS: $os ($arch)" "info"

    # Detect CI environment
    if [[ -n "$CI" || -n "$GITHUB_ACTIONS" || -n "$TRAVIS" || -n "$CIRCLECI" ]]; then
        SKIP_PROMPTS=true
        write_status "CI environment detected, running non-interactively" "info"
    fi

    # Track results
    local rust_ok=false
    local node_ok=false
    local deps_ok=false
    local tauri_ok=false

    # Install dependencies in order
    if install_rust; then
        rust_ok=true
    fi

    if install_nodejs; then
        node_ok=true
    fi

    if install_platform_deps; then
        deps_ok=true
    fi

    if install_tauri_cli; then
        tauri_ok=true
    fi

    # Show summary and exit
    if show_summary "$rust_ok" "$node_ok" "$deps_ok" "$tauri_ok"; then
        exit 0
    else
        exit 1
    fi
}

# Run main function
main "$@"
