# Quick Start Guide

Get your branded UltimaForge launcher built in 5 steps.

> **Time to complete:** ~15-30 minutes (depending on download speeds)

## Prerequisites

You'll need Git installed. Everything else is handled by our setup scripts.

## Step 1: Clone and Setup

```bash
git clone https://github.com/your-org/ultimaforge.git
cd ultimaforge
```

**Run the automated setup script:**

```powershell
# Windows (PowerShell as Administrator)
.\scripts\setup.ps1
```

```bash
# Linux/macOS
./scripts/setup.sh
```

The setup script installs Rust, Node.js, and all required dependencies automatically.

## Step 2: Install Project Dependencies

```bash
npm install
```

## Step 3: Configure Your Branding

Copy the template and edit with your server's details:

```bash
# Copy branding template
cp -r branding-template/* branding/

# Edit branding/brand.json with your details
```

**Required fields in `branding/brand.json`:**

```json
{
  "product": {
    "displayName": "Your Server Launcher",
    "serverName": "YourServer"
  },
  "updateUrl": "https://updates.yourserver.com"
}
```

## Step 4: Generate Your Keypair

Generate keys for signing your update manifests:

```bash
cargo run -p publish-cli -- keygen --output ./keys
```

Copy the public key (64 hex characters) to `branding/brand.json`:

```json
{
  "product": { ... },
  "updateUrl": "...",
  "publicKey": "<your-64-char-public-key>"
}
```

> **Important:** Keep `keys/private.key` secret! Never commit it to version control.

## Step 5: Build Your Launcher

```bash
npm run tauri build
```

Your built launcher will be in `src-tauri/target/release/`:
- **Windows:** `ultimaforge.exe`, `ultimaforge.msi`
- **macOS:** `ultimaforge.app`, `ultimaforge.dmg`
- **Linux:** `ultimaforge`, `.deb`, `.AppImage`

## Verify Your Environment

Having issues? Validate your setup:

```bash
npm run validate-env
```

This checks all dependencies and provides fix instructions for any problems.

## Next Steps

- **Set up your update server:** See [PUBLISHING.md](PUBLISHING.md)
- **Detailed configuration:** See [SETUP.md](SETUP.md)
- **Branding options:** See [branding-template/README.md](../branding-template/README.md)

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Setup script fails | Run as Administrator (Windows) or check error messages |
| Build fails | Run `npm run validate-env` to check dependencies |
| Missing branding | Ensure `branding/brand.json` exists with required fields |

For more help, see the full [SETUP.md](SETUP.md) guide or open a GitHub issue.
