#!/usr/bin/env node
/**
 * Zero-config dev testing: ensure test updates, then run host server + launcher.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawn, execSync } from "node:child_process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const appDir = path.resolve(__dirname, "..");
const repoRoot = path.resolve(appDir, "..");

const testUpdatesDir = path.join(appDir, "test-updates");
const testManifestPath = path.join(testUpdatesDir, "manifest.json");

function ensureTestUpdates() {
  if (fs.existsSync(testManifestPath)) {
    return;
  }

  const sourceDir = path.join(appDir, "test-data", "sample-client");
  const keyPath = path.join(appDir, "test-keys", "private.key");

  console.log("Generating test update manifest...");
  execSync(
    `cargo run -p publish-cli -- publish --source "${sourceDir}" --output "${testUpdatesDir}" --key "${keyPath}" --version "1.0.0" --executable "client.exe"`,
    { cwd: repoRoot, stdio: "inherit" }
  );
}

async function validateTestKeyAlignment() {
  const testPublicKeyPath = path.join(appDir, "test-keys", "public.key");
  const brandJsonPath = path.join(appDir, "public", "branding", "brand.json");

  if (!fs.existsSync(testPublicKeyPath)) {
    console.log("WARNING: app/test-keys/public.key not found.");
    console.log("  Run option [4] to generate test manifests first, or ensure test keys exist.");
    return;
  }

  if (!fs.existsSync(brandJsonPath)) {
    // brand.json in public/ may not exist yet; skip check
    return;
  }

  const testPubKey = fs.readFileSync(testPublicKeyPath, "utf8").trim();
  let brandPubKey = "";
  try {
    const brand = JSON.parse(fs.readFileSync(brandJsonPath, "utf8"));
    brandPubKey = (brand.publicKey ?? "").trim();
  } catch {
    return;
  }

  if (brandPubKey && testPubKey && brandPubKey !== testPubKey) {
    console.log("\nWARNING: Key mismatch detected!");
    console.log("   app/test-keys/public.key does not match the publicKey in app/public/branding/brand.json.");
    console.log("   The launcher will REJECT the test manifest signatures.");
    console.log("");
    console.log("   Fix: copy the test public key into brand.json for local dev:");
    console.log(`     Test public key: ${testPubKey}`);
    console.log("");
    console.log("   Or update app/public/branding/brand.json publicKey to match your test-keys.");
    console.log("   Press Ctrl+C to abort, or continuing in 5 seconds...\n");
    // 5-second pause so the warning is visible
    await new Promise((resolve) => setTimeout(resolve, 5000));
  }
}

async function run() {
  await validateTestKeyAlignment();
  ensureTestUpdates();

  const testPubKeyPath = path.join(appDir, "test-keys", "public.key");
  if (fs.existsSync(testPubKeyPath)) {
    const key = fs.readFileSync(testPubKeyPath, "utf8").trim();
    console.log(`\nDev mode: signing test manifests with test key.`);
    console.log(`Test public key: ${key.substring(0, 16)}...`);
    console.log(`Brand public key must match for launcher to verify updates.\n`);
  }

  const server = spawn(
    "cargo",
    ["run", "-p", "host-server", "--", "--dir", testUpdatesDir, "--port", "8080"],
    { cwd: repoRoot, stdio: "inherit" }
  );

  const launcher = spawn("npm", ["run", "tauri", "dev"], {
    cwd: appDir,
    stdio: "inherit",
  });

  const shutdown = () => {
    if (!server.killed) {
      server.kill("SIGINT");
    }
    if (!launcher.killed) {
      launcher.kill("SIGINT");
    }
  };

  process.on("SIGINT", shutdown);
  process.on("SIGTERM", shutdown);

  launcher.on("exit", (code) => {
    shutdown();
    process.exit(code ?? 0);
  });
}

run().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
