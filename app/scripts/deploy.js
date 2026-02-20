#!/usr/bin/env node
/**
 * Deploy published game update files to the configured VPS.
 * Requires setup-vps.js to have been run first (server-data/deploy.json must exist).
 * Uses rsync if available (Linux/Mac), falls back to scp on Windows.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const appDir = path.resolve(__dirname, "..");
const repoRoot = path.resolve(appDir, "..");

const deployConfigPath = path.join(repoRoot, "server-data", "deploy.json");
const publishDir = path.join(repoRoot, "server-data", "publish");
const deployKeyPath = path.join(repoRoot, "server-data", "keys", "deploy-key");

function exitWithError(msg) {
  console.error("\nERROR: " + msg);
  process.exit(1);
}

/**
 * Converts a Windows absolute path to a WSL /mnt/ path.
 * e.g. "G:\foo\bar" -> "/mnt/g/foo/bar"
 */
function toWslPath(winPath) {
  return winPath
    .replace(/^([A-Za-z]):/, (_, d) => `/mnt/${d.toLowerCase()}`)
    .replace(/\\/g, "/");
}

/**
 * Converts a Windows absolute path to a Cygwin /cygdrive/ path.
 * Required for cwrsync (Cygwin-based rsync) on Windows.
 * e.g. "G:\foo\bar" -> "/cygdrive/g/foo/bar"
 */
function toCygwinPath(winPath) {
  return winPath
    .replace(/^([A-Za-z]):/, (_, d) => `/cygdrive/${d.toLowerCase()}`)
    .replace(/\\/g, "/");
}

/**
 * Resolves the rsync executable. Returns either:
 *   { bin: "rsync", wsl: false }   - native rsync on PATH or known path
 *   { bin: "wsl", wsl: true }      - rsync available inside WSL
 *   null                           - not found anywhere
 */
function findRsync() {
  // On Windows, prefer WSL rsync first — it handles paths natively and
  // avoids the cygwin/OpenSSH interop issues that cwrsync has.
  if (process.platform === "win32") {
    const wslProbe = spawnSync("wsl", ["which", "rsync"], { stdio: "pipe" });
    if (!wslProbe.error && wslProbe.status === 0 && wslProbe.stdout.toString().trim()) {
      return { bin: "wsl", wsl: true };
    }
  }

  // Check native rsync on PATH (works directly on Linux/Mac, or if user
  // has a working native rsync on Windows PATH)
  const probe = spawnSync("rsync", ["--version"], { stdio: "ignore" });
  if (!probe.error) return { bin: "rsync", wsl: false };

  // Common Windows install locations (Scoop shims, cwRsync)
  const home = process.env.USERPROFILE || "";
  const progFiles = process.env.ProgramFiles || "C:\\Program Files";
  const progFilesX86 = process.env["ProgramFiles(x86)"] || "C:\\Program Files (x86)";

  const candidates = [
    path.join(home, "scoop", "shims", "rsync.exe"),
    path.join(home, "scoop", "shims", "rsync.cmd"),
    path.join(home, "scoop", "apps", "rsync", "current", "rsync.exe"),
    path.join(home, "scoop", "apps", "cwrsync", "current", "rsync.exe"),
    ...[progFiles, progFilesX86].flatMap((base) => {
      try {
        return fs
          .readdirSync(base)
          .filter((d) => d.toLowerCase().startsWith("cwrsync"))
          .map((d) => path.join(base, d, "bin", "rsync.exe"));
      } catch {
        return [];
      }
    }),
  ];

  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) return { bin: candidate, wsl: false };
  }

  return null;
}

function tryRsync(rsync, user, host, port, keyPath, localDir, remotePath) {
  let bin, args;

  if (rsync.wsl) {
    // Run rsync inside WSL, converting Windows paths to /mnt/... paths
    const wslLocalDir = toWslPath(localDir);
    const wslKeyPath = toWslPath(keyPath);
    bin = "wsl";
    args = [
      "rsync",
      "-avz",
      "--delete",
      "-e",
      `ssh -i "${wslKeyPath}" -o StrictHostKeyChecking=accept-new -p ${port}`,
      `${wslLocalDir}/`,
      `${user}@${host}:${remotePath}/`,
    ];
  } else {
    bin = rsync.bin;
    // On Windows, native rsync (cwrsync) is Cygwin-based and chokes on
    // Windows paths like "G:\foo" — the drive letter + colon looks like
    // a remote host spec. Convert source to /cygdrive/g/foo style.
    // The SSH key path is kept as a Windows path because -e ssh invokes
    // Windows OpenSSH (ssh.exe) which doesn't understand cygwin paths.
    const srcDir = process.platform === "win32" ? toCygwinPath(localDir) : localDir;
    args = [
      "-avz",
      "--delete",
      "-e",
      `ssh -i "${keyPath}" -o StrictHostKeyChecking=accept-new -p ${port}`,
      srcDir + "/",
      `${user}@${host}:${remotePath}/`,
    ];
  }

  const result = spawnSync(bin, args, { stdio: "inherit" });
  if (result.status !== 0) {
    exitWithError("rsync failed. Check the output above.");
  }
  return true;
}

function tryScp(user, host, port, keyPath, localDir, remotePath) {
  // scp does not support --delete; first clear the remote dir, then copy
  const clearResult = spawnSync(
    "ssh",
    [
      "-i",
      keyPath,
      "-o",
      "StrictHostKeyChecking=accept-new",
      "-p",
      String(port),
      `${user}@${host}`,
      `rm -rf '${remotePath}/'* 2>/dev/null || true`,
    ],
    { stdio: "inherit" }
  );
  if (clearResult.status !== 0) {
    console.warn("Warning: could not clear remote directory (proceeding anyway).");
  }

  const result = spawnSync(
    "scp",
    [
      "-i",
      keyPath,
      "-o",
      "StrictHostKeyChecking=accept-new",
      "-P",
      String(port),
      "-r",
      localDir + "/.",
      `${user}@${host}:${remotePath}/`,
    ],
    { stdio: "inherit" }
  );
  if (result.status !== 0) {
    exitWithError("scp failed. Check the output above.");
  }
  return true;
}

if (!fs.existsSync(deployConfigPath)) {
  exitWithError("No deploy config found. Run Option H (Setup VPS) first.");
}

const config = JSON.parse(fs.readFileSync(deployConfigPath, "utf8"));
const { host, user, port = 22, remote_path: remotePath, update_url: updateUrl } =
  config;
const displayUrl = updateUrl || `http://${host}`;

if (!host || typeof host !== "string") {
  exitWithError("Invalid host in deploy.json. Re-run Option H (Setup VPS).");
}

if (!user || typeof user !== "string") {
  exitWithError("Invalid user in deploy.json. Re-run Option H (Setup VPS).");
}

if (!remotePath || typeof remotePath !== "string") {
  exitWithError("Invalid remote_path in deploy.json. Re-run Option H (Setup VPS).");
}

const trimmedRemotePath = remotePath.trim();
if (
  trimmedRemotePath === "" ||
  trimmedRemotePath === "/" ||
  trimmedRemotePath === "." ||
  trimmedRemotePath === "~"
) {
  exitWithError(
    "Refusing to deploy to an unsafe remote_path. Set a dedicated path like /var/www/ultimaforge."
  );
}

if (!fs.existsSync(path.join(publishDir, "manifest.json"))) {
  exitWithError(
    "No publish output found at server-data/publish/manifest.json.\n" +
      "Run Option E (Publish) first."
  );
}

if (!fs.existsSync(deployKeyPath)) {
  exitWithError("Deploy key not found. Run Option H (Setup VPS) first.");
}

console.log("\n========================================");
console.log("   Deploying to VPS");
console.log("========================================");
console.log(`\nTarget: ${user}@${host}:${remotePath}`);
console.log(`Update URL: ${displayUrl}`);
console.log("\nSyncing files...\n");

const rsync = findRsync();
if (rsync) {
  const label = rsync.wsl ? "wsl rsync" : rsync.bin !== "rsync" ? rsync.bin : "rsync";
  console.log(`Using ${label}...`);
  tryRsync(rsync, user, host, port, deployKeyPath, publishDir, remotePath);
} else {
  console.log("rsync not found - using scp instead (re-uploads all files).");
  console.log("Run Option 0 (Install Prerequisites) to install rsync for faster deploys.");
  tryScp(user, host, port, deployKeyPath, publishDir, remotePath);
}

console.log("\n========================================");
console.log("   Deploy Complete!");
console.log("========================================");
console.log(`\nYour update server: ${displayUrl}`);
console.log("Players will get the new version on next launcher startup.");
