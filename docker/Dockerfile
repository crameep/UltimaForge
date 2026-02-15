# UltimaForge Tauri Build Environment
# This Dockerfile creates a Linux build environment for Tauri applications
# NOTE: This only produces Linux binaries. For Windows/macOS builds, use GitHub Actions or native hosts.

# Build arguments for version control
ARG RUST_VERSION=1.77
ARG NODE_VERSION=18

# ============================================
# Stage 1: Rust Build Environment
# ============================================
FROM rust:${RUST_VERSION}-slim-bookworm AS builder

# Re-declare ARG after FROM
ARG NODE_VERSION

# Set environment variables
ENV DEBIAN_FRONTEND=noninteractive
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup
ENV PATH=/usr/local/cargo/bin:$PATH

# Install system dependencies for Tauri
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Build essentials
    build-essential \
    pkg-config \
    # Tauri WebKitGTK dependencies
    libwebkit2gtk-4.1-dev \
    libgtk-3-dev \
    # System tray / app indicator support
    libayatana-appindicator3-dev \
    libappindicator3-dev \
    # Graphics and SVG support
    librsvg2-dev \
    # SSL/TLS
    libssl-dev \
    # Binary patching for AppImage
    patchelf \
    # Download utilities
    curl \
    wget \
    file \
    # Git for potential submodules or dependencies
    git \
    # CA certificates for HTTPS
    ca-certificates \
    # Node.js dependencies
    gnupg \
    && rm -rf /var/lib/apt/lists/*

# Install Node.js via NodeSource
RUN curl -fsSL https://deb.nodesource.com/setup_${NODE_VERSION}.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install Tauri CLI globally
RUN npm install -g @tauri-apps/cli@latest

# Verify installations
RUN echo "=== Build Environment ===" && \
    echo "Rust: $(rustc --version)" && \
    echo "Cargo: $(cargo --version)" && \
    echo "Node.js: $(node --version)" && \
    echo "npm: $(npm --version)" && \
    echo "Tauri CLI: $(tauri --version)"

# Set working directory
WORKDIR /app

# ============================================
# Build Commands
# ============================================
# Default command shows help
CMD ["echo", "Usage: docker run -v $(pwd):/app ultimaforge-builder npm run tauri build"]

# ============================================
# Alternative: Minimal Runtime Stage (optional)
# Uncomment if you need a smaller image for specific use cases
# ============================================
# FROM debian:bookworm-slim AS runtime
# RUN apt-get update && apt-get install -y --no-install-recommends \
#     libwebkit2gtk-4.1-0 \
#     libgtk-3-0 \
#     libayatana-appindicator3-1 \
#     && rm -rf /var/lib/apt/lists/*
# WORKDIR /app
# COPY --from=builder /app/src-tauri/target/release/bundle /app/bundle
