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

function run() {
  ensureTestUpdates();

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

run();
