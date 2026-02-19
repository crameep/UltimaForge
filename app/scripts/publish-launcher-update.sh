#!/usr/bin/env bash
set -euo pipefail

VERSION=""
TARGET="windows"
ARCH="x86_64"
BINARY_PATH=""
SIGNATURE=""
SIGNATURE_FILE=""
NOTES=""
NOTES_FILE=""
BASE_URL="http://localhost:8080"
OUTPUT_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      VERSION="$2"
      shift 2
      ;;
    --target)
      TARGET="$2"
      shift 2
      ;;
    --arch)
      ARCH="$2"
      shift 2
      ;;
    --binary)
      BINARY_PATH="$2"
      shift 2
      ;;
    --signature)
      SIGNATURE="$2"
      shift 2
      ;;
    --signature-file)
      SIGNATURE_FILE="$2"
      shift 2
      ;;
    --notes)
      NOTES="$2"
      shift 2
      ;;
    --notes-file)
      NOTES_FILE="$2"
      shift 2
      ;;
    --base-url)
      BASE_URL="$2"
      shift 2
      ;;
    --output-dir)
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --help|-h)
      echo "Usage: $0 --version <ver> --binary <path> [options]"
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

if [[ -z "$OUTPUT_DIR" ]]; then
  OUTPUT_DIR="$REPO_ROOT/updates/launcher"
fi

if [[ -z "$VERSION" ]]; then
  read -r -p "Version (e.g. 1.2.3): " VERSION
fi

if [[ -z "$BINARY_PATH" ]]; then
  read -r -p "Path to launcher binary/installer: " BINARY_PATH
fi

if [[ -z "$SIGNATURE" && -n "$SIGNATURE_FILE" ]]; then
  SIGNATURE="$(cat "$SIGNATURE_FILE")"
fi

if [[ -z "$SIGNATURE" && -n "${TAURI_UPDATER_SIGNATURE:-}" ]]; then
  SIGNATURE="$TAURI_UPDATER_SIGNATURE"
fi

if [[ -z "$SIGNATURE" ]]; then
  read -r -p "Signature string (Tauri updater signature): " SIGNATURE
fi

if [[ -z "$NOTES" && -n "$NOTES_FILE" ]]; then
  NOTES="$(cat "$NOTES_FILE")"
fi

PLATFORM_KEY="${TARGET}-${ARCH}"
FILES_DIR="$OUTPUT_DIR/files"

mkdir -p "$FILES_DIR"
BINARY_PATH="$(cd "$(dirname "$BINARY_PATH")" && pwd)/$(basename "$BINARY_PATH")"
BINARY_NAME="$(basename "$BINARY_PATH")"
cp -f "$BINARY_PATH" "$FILES_DIR/$BINARY_NAME"

python3 - <<PY
import json
import os
import datetime

version = ${VERSION!r}
notes = ${NOTES!r}
pub_date = datetime.datetime.utcnow().strftime("%Y-%m-%dT%H:%M:%SZ")
platform_key = ${PLATFORM_KEY!r}
signature = ${SIGNATURE!r}.strip()
url = f"{${BASE_URL!r}}/launcher/files/{${BINARY_NAME!r}}"

metadata = {
    "version": version,
    "notes": notes,
    "pub_date": pub_date,
    "platforms": {
        platform_key: {
            "signature": signature,
            "url": url,
        }
    },
}

output_dir = ${OUTPUT_DIR!r}
latest_path = os.path.join(output_dir, "latest.json")
platform_path = os.path.join(output_dir, f"{platform_key}.json")

os.makedirs(output_dir, exist_ok=True)
for path in (latest_path, platform_path):
    with open(path, "w", encoding="ascii") as f:
        json.dump(metadata, f, indent=2)
        f.write("\n")

print("Launcher update metadata written:")
print(f" - {latest_path}")
print(f" - {platform_path}")
print("Launcher binary copied to:")
print(f" - {os.path.join(output_dir, 'files', ${BINARY_NAME!r})}")
PY
