# Publishing Updates

This guide explains how to publish updates for your Ultima Online server using UltimaForge.

## Overview

UltimaForge uses a content-addressed update system with cryptographic signing:

1. **Manifest**: JSON file listing all files with their SHA-256 hashes
2. **Signature**: Ed25519 signature of the manifest
3. **Blobs**: Content-addressed files stored by their SHA-256 hash

## Tools

UltimaForge provides two tools for publishing:

| Tool | Purpose |
|------|---------|
| `publish-cli` | Create manifests, sign, and generate blobs |
| `host-server` | Serve update files to clients |

## Quick Start

### 1. Generate Keypair (One Time)

```bash
cargo run -p publish-cli -- keygen --output ./keys
```

Keep `keys/private.key` secure! Add the public key to your `brand.json`.

### 2. Publish Your Client Files

```bash
cargo run -p publish-cli -- publish \
  --source ./uo-client \
  --output ./updates \
  --key ./keys/private.key \
  --version 1.0.0 \
  --executable client.exe
```

### 3. Host the Update Files

```bash
cargo run -p host-server -- --dir ./updates --port 8080
```

Your update server is now running at `http://localhost:8080`.

## Publish CLI Commands

### keygen

Generate an Ed25519 keypair for signing manifests.

```bash
publish-cli keygen [OPTIONS]

Options:
  -o, --output <DIR>    Output directory for key files [default: ./keys]
  -f, --force           Overwrite existing key files
```

**Example:**

```bash
cargo run -p publish-cli -- keygen --output ./my-keys --force
```

**Output Files:**
- `private.key` - 32-byte private key (hex-encoded)
- `public.key` - 32-byte public key (hex-encoded)

### manifest

Create a manifest from a source directory (without signing).

```bash
publish-cli manifest [OPTIONS]

Options:
  -s, --source <DIR>       Source directory containing files
  -o, --output <FILE>      Output path for manifest.json [default: ./manifest.json]
  -v, --version <VERSION>  Version string [default: 1.0.0]
  -e, --executable <PATH>  Client executable path (relative to source) [default: client.exe]
```

**Example:**

```bash
cargo run -p publish-cli -- manifest \
  --source ./uo-client \
  --output ./updates/manifest.json \
  --version 2.1.0 \
  --executable ClassicUO.exe
```

### sign

Sign a manifest file with your private key.

```bash
publish-cli sign [OPTIONS]

Options:
  -m, --manifest <FILE>  Path to manifest.json
  -k, --key <FILE>       Path to private key file
  -o, --output <FILE>    Output path for signature [default: ./manifest.sig]
```

**Example:**

```bash
cargo run -p publish-cli -- sign \
  --manifest ./updates/manifest.json \
  --key ./keys/private.key \
  --output ./updates/manifest.sig
```

### blob

Copy files to content-addressed blob storage.

```bash
publish-cli blob [OPTIONS]

Options:
  -s, --source <DIR>  Source directory containing original files
  -o, --output <DIR>  Output directory for content-addressed blobs [default: ./files]
```

**Example:**

```bash
cargo run -p publish-cli -- blob \
  --source ./uo-client \
  --output ./updates/files
```

### validate

Validate an update folder structure.

```bash
publish-cli validate [OPTIONS]

Options:
  -d, --dir <DIR>  Directory containing update files
  -k, --key <FILE> Path to public key for signature verification
```

**Example:**

```bash
cargo run -p publish-cli -- validate \
  --dir ./updates \
  --key ./keys/public.key
```

### publish (Recommended)

All-in-one workflow: manifest + sign + blob.

```bash
publish-cli publish [OPTIONS]

Options:
  -s, --source <DIR>       Source directory containing files to publish
  -o, --output <DIR>       Output directory for update artifacts [default: ./updates]
  -k, --key <FILE>         Path to private key file
  -v, --version <VERSION>  Version string [default: 1.0.0]
  -e, --executable <PATH>  Client executable path [default: client.exe]
```

**Example:**

```bash
cargo run -p publish-cli -- publish \
  --source ./uo-client \
  --output ./updates \
  --key ./keys/private.key \
  --version 1.0.0 \
  --executable client.exe
```

**Output Structure:**

```
updates/
├── manifest.json     # File list with hashes
├── manifest.sig      # Ed25519 signature
└── files/            # Content-addressed blobs
    ├── a1b2c3d4...   # File stored by SHA-256 hash
    ├── e5f6g7h8...
    └── ...
```

## Host Server

The host server serves update files to clients.

### Starting the Server

```bash
cargo run -p host-server -- [OPTIONS]

Options:
  -d, --dir <DIR>    Directory to serve files from [default: ./updates]
  -p, --port <PORT>  Port to listen on [default: 8080]
  --host <HOST>      Host address to bind to [default: 0.0.0.0]
```

**Example:**

```bash
# Development (local only)
cargo run -p host-server -- --dir ./updates --port 8080

# Production (all interfaces)
cargo run -p host-server -- --dir ./updates --port 80 --host 0.0.0.0
```

### Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /` | Server info and status |
| `GET /health` | Health check (returns 200 OK) |
| `GET /validate` | Validate update folder structure |
| `GET /manifest.json` | Current manifest |
| `GET /manifest.sig` | Manifest signature |
| `GET /files/{hash}` | Content-addressed file by SHA-256 hash |

### Testing the Server

```bash
# Health check
curl http://localhost:8080/health

# Get manifest
curl http://localhost:8080/manifest.json

# Validate structure
curl http://localhost:8080/validate
```

## Update Workflow

### Initial Release

1. Prepare your UO client files in a directory
2. Generate keypair (if not already done)
3. Run `publish` command
4. Deploy update files to your server
5. Start host-server

### Subsequent Updates

1. Update files in your source directory
2. Increment version number
3. Run `publish` command again
4. Deploy updated files to server

The launcher will automatically detect the new version and download only changed files.

## Manifest Format

```json
{
  "version": "1.0.0",
  "timestamp": "2026-02-15T12:00:00Z",
  "client_executable": "client.exe",
  "client_args": [],
  "files": [
    {
      "path": "client.exe",
      "sha256": "a1b2c3d4e5f6...",
      "size": 12345678,
      "required": true
    },
    {
      "path": "data/map0.mul",
      "sha256": "e5f6g7h8i9j0...",
      "size": 87654321,
      "required": true
    }
  ],
  "total_size": 100000000,
  "patch_notes_url": "patchnotes.md"
}
```

## Production Deployment

### Using nginx (Recommended)

You can use nginx to serve update files instead of host-server for production:

```nginx
server {
    listen 443 ssl;
    server_name updates.yourserver.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    root /var/www/updates;

    # Manifest and signature
    location /manifest.json {
        add_header Cache-Control "no-cache";
    }
    location /manifest.sig {
        add_header Cache-Control "no-cache";
    }

    # Content-addressed files (immutable, cache forever)
    location /files/ {
        add_header Cache-Control "public, max-age=31536000, immutable";
    }

    # Health check
    location /health {
        return 200 'OK';
        add_header Content-Type text/plain;
    }
}
```

### Using a CDN

For large player bases, serve files through a CDN:

1. Set up nginx as origin server
2. Configure CDN to cache `/files/*` aggressively (immutable content)
3. Configure CDN to not cache `/manifest.json` and `/manifest.sig`
4. Update `updateUrl` in brand.json to CDN URL

### Using Cloud Storage

You can host update files on S3, Azure Blob, or Google Cloud Storage:

1. Upload `updates/` directory contents
2. Set proper CORS headers
3. Configure public read access
4. Update `updateUrl` in brand.json

## Security Best Practices

### Key Management

- Store private key encrypted at rest
- Never commit private key to version control
- Use separate keys for development and production
- Rotate keys periodically (requires launcher update)

### File Access

- Content-addressed storage prevents tampering
- Signature verification is mandatory
- Hash verification for every downloaded file

### Server Hardening

- Use HTTPS in production
- Rate limit manifest endpoint
- Monitor for unusual access patterns

## Troubleshooting

### "Signature verification failed"

- Ensure you're using the correct private key
- Verify manifest.json hasn't been modified after signing
- Check that public key in brand.json matches your private key

### "Missing blob" Errors

- Run `validate` command to check for missing files
- Re-run `publish` command to regenerate blobs

### Large File Uploads Timeout

For very large client distributions:

1. Increase timeouts in host-server
2. Use chunked uploads
3. Consider a CDN with resumable uploads

### Version Not Detected

- Ensure version string changed in manifest
- Clear launcher cache (Settings > Clear Cache)
- Check that manifest.json was updated on server

## Differential Updates

UltimaForge automatically calculates differential updates:

1. Launcher downloads new manifest
2. Compares file hashes with local installation
3. Downloads only changed files
4. Applies update atomically

This means a 1GB client with a 10MB patch only downloads 10MB.

## Rollback Support

If an update fails mid-apply:

1. Launcher backs up modified files
2. Attempts to apply staged files
3. On failure, restores from backup
4. Installation remains in working state

## Example CI/CD Integration

### GitHub Actions

```yaml
name: Publish Update

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Build publish-cli
        run: cargo build --release -p publish-cli

      - name: Publish update
        run: |
          ./target/release/publish-cli publish \
            --source ./client \
            --output ./updates \
            --key ${{ secrets.PRIVATE_KEY_PATH }} \
            --version ${{ github.ref_name }}

      - name: Upload to server
        run: rsync -avz ./updates/ user@updates.server.com:/var/www/updates/
```

## Support

For issues with publishing tools, please open a GitHub issue with:

- Command you ran
- Full error output
- Tool versions (`cargo --version`, `rustc --version`)
