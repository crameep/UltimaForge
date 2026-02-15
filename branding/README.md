# UltimaForge Branding Configuration

This directory contains the branding configuration for the UltimaForge launcher.

## Files

- `brand.json` - Main branding configuration (required)

## Configuration Schema

### brand.json

```json
{
  "product": {
    "displayName": "Server Display Name",
    "serverName": "ServerName",
    "description": "Optional server description",
    "supportEmail": "support@example.com",
    "website": "https://example.com",
    "discord": "https://discord.gg/example"
  },
  "updateUrl": "https://updates.example.com",
  "publicKey": "<64-character hex-encoded Ed25519 public key>",
  "theme": {
    "primary": "#1a1a2e",
    "secondary": "#e94560",
    "background": "#16213e",
    "text": "#eaeaea"
  },
  "ui": {
    "showPatchNotes": true,
    "windowTitle": "Custom Window Title"
  }
}
```

## Required Fields

| Field | Description |
|-------|-------------|
| `product.displayName` | Name shown in UI (e.g., "My UO Server") |
| `product.serverName` | Internal server name (e.g., "MyServer") |
| `updateUrl` | URL where update server is hosted |
| `publicKey` | 64-character hex-encoded Ed25519 public key |

## Generating Keys

Use the publish-cli tool to generate a new keypair:

```bash
cargo run -p publish-cli -- keygen --output ./keys
```

This creates:
- `keys/private.key` - Keep this secret! Used for signing manifests
- `keys/public.key` - Embed this in brand.json

## Test Configuration

The current `brand.json` is configured for local testing:
- `updateUrl`: `http://localhost:8080`
- `publicKey`: RFC 8032 test vector (DO NOT use in production!)

For production, you must:
1. Generate your own keypair
2. Update `publicKey` with your public key
3. Update `updateUrl` with your production server URL

## Theme Colors

All colors should be hex format (e.g., `#1a1a2e`).

| Color | Usage |
|-------|-------|
| `primary` | Main brand color, buttons |
| `secondary` | Accent color, highlights |
| `background` | Window background |
| `text` | Primary text color |
