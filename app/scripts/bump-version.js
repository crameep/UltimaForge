#!/usr/bin/env node
/**
 * Update version across workspace Cargo.toml, tauri.conf.json, and package.json.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const appDir = path.resolve(__dirname, "..");
const repoRoot = path.resolve(appDir, "..");

function parseArgs(argv) {
  const args = {};
  for (let i = 0; i < argv.length; i += 1) {
    const token = argv[i];
    if (!token.startsWith("--")) {
      continue;
    }
    const key = token.slice(2);
    const value = argv[i + 1];
    args[key] = value === undefined || value.startsWith("--") ? "true" : value;
  }
  return args;
}

function replaceWorkspaceVersion(contents, version) {
  return contents.replace(
    /(\[workspace\.package\][\s\S]*?version\s*=\s*")([^"]+)(")/,
    `$1${version}$3`
  );
}

function updateJsonFile(filePath, updater) {
  const json = JSON.parse(fs.readFileSync(filePath, "utf8"));
  const updated = updater(json);
  fs.writeFileSync(filePath, JSON.stringify(updated, null, 2), "utf8");
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const version = args.version;
  if (!version) {
    console.error("Usage: node app/scripts/bump-version.js --version <x.y.z>");
    process.exit(1);
  }

  const cargoTomlPath = path.join(repoRoot, "Cargo.toml");
  const cargoContents = fs.readFileSync(cargoTomlPath, "utf8");
  const updatedCargo = replaceWorkspaceVersion(cargoContents, version);
  fs.writeFileSync(cargoTomlPath, updatedCargo, "utf8");

  const tauriConfigPath = path.join(appDir, "src-tauri", "tauri.conf.json");
  updateJsonFile(tauriConfigPath, (json) => ({ ...json, version }));

  const packageJsonPath = path.join(appDir, "package.json");
  updateJsonFile(packageJsonPath, (json) => ({ ...json, version }));

  const packageLockPath = path.join(appDir, "package-lock.json");
  if (fs.existsSync(packageLockPath)) {
    updateJsonFile(packageLockPath, (json) => {
      const updated = { ...json, version };
      if (updated.packages && updated.packages[""]) {
        updated.packages[""].version = version;
      }
      return updated;
    });
  }

  console.log(`Version updated to ${version}`);
}

main();
