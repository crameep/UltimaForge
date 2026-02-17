# Branding Template

This directory contains a template for customizing the UltimaForge launcher for your Ultima Online server.

## Quick Start

1. Copy this entire directory to your launcher project as `branding/`
2. Edit `brand.json` with your server details
3. Generate a keypair (see below)
4. Build your customized launcher

## Files

| File | Purpose |
|------|---------|
| `brand.json` | Main branding configuration (required) |

## Configuring brand.json

### Required Fields

These fields must be set for your launcher to build:

```json
{
  "product": {
    "displayName": "Your Server Name",
    "serverName": "YourServer"
  },
  "updateUrl": "https://updates.yourserver.com",
  "publicKey": "<your-64-char-hex-public-key>"
}
```

| Field | Description | Example |
|-------|-------------|---------|
| `product.displayName` | Name shown in the launcher UI | `"Avalon Legends"` |
| `product.serverName` | Internal identifier (no spaces) | `"AvalonLegends"` |
| `updateUrl` | Base URL for your update server | `"https://updates.avalon.gg"` |
| `publicKey` | 64-character hex Ed25519 public key | See below |

### Optional Fields

```json
{
  "product": {
    "description": "A brief description of your server",
    "supportEmail": "support@yourserver.com",
    "website": "https://yourserver.com",
    "discord": "https://discord.gg/yourserver"
  },
  "ui": {
    "colors": {
      "primary": "#1a1a2e",
      "secondary": "#e94560",
      "background": "#16213e",
      "text": "#ffffff"
    },
    "showPatchNotes": true,
    "windowTitle": "Custom Window Title"
  }
}
```

## Generating Your Keypair

Before building your launcher, you must generate an Ed25519 keypair:

```bash
# Generate keypair
cargo run -p publish-cli -- keygen --output ./keys

# This creates:
# - keys/private.key (KEEP SECRET - for signing manifests)
# - keys/public.key (for embedding in launcher)
```

**Copy the public key to your brand.json:**

The command will output your public key as a 64-character hex string. Copy this value to the `publicKey` field in your `brand.json`.

## Security

- **Private Key**: Keep `private.key` secure and never distribute it. This key is used to sign your update manifests.
- **Public Key**: The public key is safe to embed in your launcher and distribute. It is used only for verifying signatures.
- **Never share your private key** - anyone with access to it can sign malicious updates that your launcher will accept.

## Theme Colors

All colors use CSS hex format (e.g., `#1a1a2e` or shorthand `#abc`).

| Color | Default | Usage |
|-------|---------|-------|
| `primary` | `#1a1a2e` | Main brand color, button backgrounds |
| `secondary` | `#e94560` | Accent color, highlights, progress bars |
| `background` | `#16213e` | Main window background |
| `text` | `#ffffff` | Primary text color |

### Example Themes

**Dark Ocean:**
```json
{
  "primary": "#0f0f23",
  "secondary": "#00bfff",
  "background": "#1a1a35",
  "text": "#e0e0e0"
}
```

**Forest Green:**
```json
{
  "primary": "#1a2e1a",
  "secondary": "#45e960",
  "background": "#162116",
  "text": "#ffffff"
}
```

**Classic Red:**
```json
{
  "primary": "#2e1a1a",
  "secondary": "#e94545",
  "background": "#211616",
  "text": "#ffffff"
}
```

## Validation

The launcher validates brand.json at build time. Build will fail if:

- `product.displayName` is missing or empty
- `product.serverName` is missing or empty
- `updateUrl` is missing or doesn't start with `http://` or `https://`
- `publicKey` is missing, not 64 characters, or contains non-hex characters
- Any color value doesn't follow `#RGB` or `#RRGGBB` format

## Production Checklist

Before distributing your launcher:

- [ ] Generated your own keypair (never use test keys)
- [ ] Updated `publicKey` with your real public key
- [ ] Set `updateUrl` to your production server URL
- [ ] Customized `displayName` and `serverName`
- [ ] Tested the launcher with your update server
- [ ] Stored private key securely (not in version control)

## Example Complete Configuration

```json
{
  "product": {
    "displayName": "Avalon Legends",
    "serverName": "AvalonLegends",
    "description": "A Renaissance-era Ultima Online freeshard",
    "supportEmail": "support@avalon-legends.com",
    "website": "https://avalon-legends.com",
    "discord": "https://discord.gg/avalonuo"
  },
  "updateUrl": "https://updates.avalon-legends.com",
  "publicKey": "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
  "ui": {
    "colors": {
      "primary": "#2d1b4e",
      "secondary": "#9b59b6",
      "background": "#1a0f2e",
      "text": "#f0e6ff"
    },
    "showPatchNotes": true,
    "windowTitle": "Avalon Legends Launcher"
  },
  "brandVersion": "1.0"
}
```
