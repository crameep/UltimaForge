# UltimaForge deployment recovery, 2026-11-20

## What was on the server

The Vultr server was serving a prebuilt client and launcher update tree, not a source checkout. No Git checkout was found in the web root or the usual project locations checked. The original Caddy document root was `/var/www/ultimaforge`.

| Artifact | Observed metadata |
| --- | --- |
| Game client manifest | `2026.04.02`, published 2026-04-02 03:21:38 UTC; 1,161 entries; declares 4,515,980,928 bytes |
| Windows x64 launcher | `0.1.34`, published 2026-04-06 14:24:38 UTC |
| Complete deployment tree | 1,250 regular files, including game blobs, launcher files, update manifests, and the game manifest signature |

GitHub `main` declared launcher version `0.1.29` in `app/src-tauri/tauri.conf.json` at the time of comparison. The server's `0.1.34` binary and metadata are not a substitute for its source. Do not bump the version in source or claim that this release is reproducible without locating the corresponding checkout or build provenance.

## Original web-server configuration

```caddyfile
:80 {
    root * /var/www/ultimaforge
    file_server
    encode gzip
}
```

This describes the old host, not a recommendation to publish a replacement over plain HTTP. On a new host, use a TLS-enabled hostname and verify the launcher and game update URLs before cutting over. The old launcher manifest pointed to the old server, so it should not be copied unchanged into a new live updater without checking that URL and preserving the signature relationship with the installer.

## Integrity references

SHA-256 observed on the server:

| Relative path | SHA-256 |
| --- | --- |
| `manifest.json` | `71c8e97543e2ca83f019d8b6911111a8ccc738937f08142076a800314cc7e442` |
| `manifest.sig` | `21e67eb3af2f3c823b41efb0c81862e4321a97857997cf271c20291cfcc58319` |
| `launcher/latest.json` and `launcher/windows-x86_64.json` | `832d0b807e40c3a37ed8f7c75c637986143945d789c695e0fa565e01ba43a8e4` |

The full deployment checksum list was exported separately. Its SHA-256 is `279e3583d397732be594c0ffc48866e06b6fad38331153fbbf74959ac1257e94`. From the root of a restored payload, check every file with `sha256sum -c /path/to/SHA256SUMS` (Linux) or `shasum -a 256 -c /path/to/SHA256SUMS` (macOS).

Do not check the payload, raw deployment manifests, client binaries or data, passwords, or signing/deployment keys into this public repository.
