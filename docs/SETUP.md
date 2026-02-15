# UltimaForge Setup Guide

This guide walks you through setting up UltimaForge for your Ultima Online server.

## Overview

UltimaForge is a self-hosted installer, patcher, and launcher for Ultima Online private servers. Server owners build a branded launcher executable that players download as a single file. The launcher handles:

- First-run installation with directory selection
- Automatic updates via cryptographically signed manifests
- Game launching

## Prerequisites

### Development Tools

**Rust (1.77.2+)**

```bash
# Linux/macOS
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup update stable

# Windows
# Download and run rustup-init.exe from https://rustup.rs
```

**Node.js (18+)**

```bash
# Using nvm (recommended)
nvm install 18
nvm use 18

# Or download from https://nodejs.org
```

**Linux Only - WebKit Dependencies**

```bash
# Debian/Ubuntu
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
    libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev

# Fedora
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
    gtk3-devel libappindicator-gtk3-devel librsvg2-devel

# Arch
sudo pacman -S webkit2gtk-4.1 base-devel curl wget file \
    gtk3 libappindicator-gtk3 librsvg
```

### Tauri CLI

```bash
npm install -g @tauri-apps/cli
```

## Project Setup

### 1. Clone the Repository

```bash
git clone https://github.com/your-org/ultimaforge.git
cd ultimaforge
```

### 2. Install Dependencies

```bash
# Install Node.js dependencies
npm install

# Rust dependencies are installed automatically on build
```

### 3. Configure Your Branding

Copy the branding template and customize it:

```bash
# Copy template to branding directory
cp -r branding-template/* branding/

# Edit branding configuration
# (Use your preferred editor)
nano branding/brand.json
```

See [branding-template/README.md](../branding-template/README.md) for configuration details.

### 4. Generate Your Keypair

Generate an Ed25519 keypair for signing manifests:

```bash
cargo run -p publish-cli -- keygen --output ./keys
```

This creates:
- `keys/private.key` - Keep secret! Used for signing manifests
- `keys/public.key` - Public key to embed in your launcher

Copy the public key (64-character hex string) to `branding/brand.json`:

```json
{
  "publicKey": "<paste-your-64-char-public-key-here>"
}
```

### 5. Development Mode

Run the launcher in development mode:

```bash
npm run tauri dev
```

This starts:
- Vite dev server on port 1420 (frontend)
- Tauri development window with hot reload

### 6. Build for Distribution

Build your branded launcher:

```bash
npm run tauri build
```

Build outputs are in `src-tauri/target/release/`:
- Windows: `ultimaforge.exe` + `ultimaforge.msi`
- macOS: `ultimaforge.app` + `ultimaforge.dmg`
- Linux: `ultimaforge` + `.deb` / `.AppImage`

## Project Structure

```
ultimaforge/
├── src/                      # Frontend (React)
│   ├── components/           # UI components
│   ├── hooks/                # React hooks
│   └── styles/               # CSS styles
├── src-tauri/                # Backend (Rust)
│   ├── src/
│   │   ├── commands/         # Tauri IPC commands
│   │   ├── config.rs         # Configuration handling
│   │   ├── downloader.rs     # HTTP downloads
│   │   ├── installer.rs      # Installation logic
│   │   ├── launcher.rs       # Game launching
│   │   ├── manifest.rs       # Manifest parsing
│   │   ├── signature.rs      # Signature verification
│   │   ├── state.rs          # Application state
│   │   └── updater.rs        # Update mechanism
│   └── tauri.conf.json       # Tauri configuration
├── tools/
│   ├── publish-cli/          # Manifest publishing CLI
│   └── host-server/          # Update hosting server
├── branding/                 # Your branding (customize)
├── branding-template/        # Template for branding
└── docs/                     # Documentation
```

## Configuration Files

### branding/brand.json

Your server's branding configuration. See [branding-template/README.md](../branding-template/README.md).

### src-tauri/tauri.conf.json

Tauri application configuration. Key fields:

```json
{
  "productName": "YourServerLauncher",
  "identifier": "com.yourserver.launcher",
  "app": {
    "windows": [{
      "title": "Your Server Launcher",
      "width": 900,
      "height": 600
    }]
  }
}
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `ULTIMAFORGE_UPDATE_URL` | Override update server URL | Uses brand.json |
| `RUST_LOG` | Set logging level | `info` |

## Update Server Setup

See [PUBLISHING.md](PUBLISHING.md) for setting up your update server.

## Security Considerations

### Key Management

- **Private Key**: Store securely, never in version control
- **Public Key**: Safe to embed in launcher and distribute
- Use separate keys for development and production

### Manifest Signing

All update manifests must be signed with your private key. The launcher will reject any unsigned or incorrectly signed manifests.

### Content Verification

Every file download is verified against its SHA-256 hash from the manifest. Corrupted or tampered files are rejected.

## Troubleshooting

### Build Fails with "branding/brand.json not found"

Ensure you have created the branding configuration:

```bash
cp -r branding-template/* branding/
```

### "Invalid public key" Error

The public key must be exactly 64 hexadecimal characters:

```json
{
  "publicKey": "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
}
```

### WebKit Errors on Linux

Install the required WebKit dependencies (see Prerequisites).

### "Port already in use" During Development

The Vite dev server uses port 1420. Kill any existing process:

```bash
# Linux/macOS
lsof -i :1420 | grep LISTEN | awk '{print $2}' | xargs kill

# Windows
netstat -ano | findstr :1420
taskkill /PID <pid> /F
```

### Launcher Won't Connect to Update Server

1. Verify `updateUrl` in brand.json is correct
2. Ensure your update server is running and accessible
3. Check firewall rules
4. Test with: `curl https://your-update-url/health`

## Next Steps

1. **Set up update server**: See [PUBLISHING.md](PUBLISHING.md)
2. **Publish your first update**: Use the publish-cli
3. **Distribute your launcher**: Share the built executable with players

## Support

For issues and feature requests, please open a GitHub issue.
