#!/usr/bin/env node
/**
 * Prints the TAURI_SIGNING_PRIVATE_KEY value for a given key file.
 *
 * Tauri expects TAURI_SIGNING_PRIVATE_KEY to be the base64-encoded content
 * of the minisign private key file. Keys written by `tauri signer generate
 * --write-keys` are raw text starting with "untrusted comment:". This script
 * converts that raw text to the base64 blob format Tauri requires.
 *
 * Usage: node scripts/print-signing-key.js <path-to-tauri.key>
 */
import fs from "node:fs";

const keyPath = process.argv[2];
if (!keyPath || !fs.existsSync(keyPath)) {
  process.exit(1);
}

const content = fs.readFileSync(keyPath, "utf8");
const trimmed = content.trim();
const value = trimmed.startsWith("untrusted comment:")
  ? Buffer.from(trimmed + "\n").toString("base64")
  : trimmed;

process.stdout.write(value);
