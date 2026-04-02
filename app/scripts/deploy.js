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
  // On Linux/Mac: native rsync on PATH is always correct.
  if (process.platform !== "win32") {
    const probe = spawnSync("rsync", ["--version"], { stdio: "ignore" });
    if (!probe.error) return { bin: "rsync", wsl: false };
    return null;
  }

  // On Windows: prefer WSL rsync over native/cwrsync.
  //
  // Native "rsync" on the Windows PATH is often the WSL interop shim
  // (C:\Windows\System32\rsync), which expects Linux paths — but we'd
  // pass it Cygwin paths, causing protocol error code 12. cwrsync bundles
  // its own Cygwin SSH that also mishandles Windows key paths.
  //
  // WSL rsync is the most reliable choice on Windows: the tryRsync() WSL
  // path converts both the source dir and SSH key to /mnt/... paths.

  // 1. WSL rsync (preferred on Windows)
  const wslProbe = spawnSync("wsl", ["which", "rsync"], { stdio: "pipe" });
  if (!wslProbe.error && wslProbe.status === 0 && wslProbe.stdout.toString().trim()) {
    return { bin: "wsl", wsl: true };
  }

  // 2. Known cwrsync / Scoop install locations (no WSL available)
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

  // -q suppresses SSH banner/MOTD output. Without it, a remote shell that
  // prints anything to stdout on login (DigitalOcean MOTD, /etc/profile.d/
  // scripts, etc.) corrupts the rsync protocol stream, causing error code 12.
  const sshOpts = `-i "${keyPath}" -o StrictHostKeyChecking=accept-new -q -p ${port}`;

  if (rsync.wsl) {
    // Run rsync inside WSL, converting Windows paths to /mnt/... paths.
    // WSL mounts NTFS with 0777 permissions, which makes ssh reject the key.
    // Copy it to a WSL-native temp location with correct permissions first.
    const wslLocalDir = toWslPath(localDir);
    const wslKeyPath = toWslPath(keyPath);
    spawnSync("wsl", ["bash", "-c", `cp "${wslKeyPath}" /tmp/uf-deploy-key && chmod 600 /tmp/uf-deploy-key`], { stdio: "ignore" });
    const wslSshOpts = `-i /tmp/uf-deploy-key -o StrictHostKeyChecking=accept-new -q -p ${port}`;
    bin = "wsl";
    args = [
      "rsync",
      "-avz",
      "--delete",
      "-e",
      `ssh ${wslSshOpts}`,
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
      `ssh ${sshOpts}`,
      srcDir + "/",
      `${user}@${host}:${remotePath}/`,
    ];
  }

  const result = spawnSync(bin, args, { stdio: "inherit" });
  if (result.status !== 0) {
    if (result.status === 12) {
      exitWithError(
        "rsync protocol error (code 12).\n" +
        "rsync is likely not installed on the server.\n" +
        `Fix: ssh ${user}@${host} "apt-get install -y rsync"`
      );
    }
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

/**
 * Returns true if SSH key auth succeeds without a password prompt.
 */
/**
 * On Windows, SSH refuses keys with loose permissions. Copy the key to
 * %USERPROFILE%\.ssh\ and lock down permissions with icacls.
 * Returns the path to use for -i (the secured copy, or the original on non-Windows).
 */
function ensureSecureKeyPath(keyPath) {
  if (process.platform !== "win32") return keyPath;
  const home = process.env.USERPROFILE || process.env.HOME;
  if (!home) return keyPath;
  const sshDir = path.join(home, ".ssh");
  if (!fs.existsSync(sshDir)) fs.mkdirSync(sshDir, { recursive: true });
  const securePath = path.join(sshDir, "ultimaforge-deploy-key");
  try {
    fs.copyFileSync(keyPath, securePath);
    // Remove inherited permissions and grant only the current user
    spawnSync("icacls", [securePath, "/inheritance:r", "/grant:r", `${process.env.USERNAME}:R`], { stdio: "ignore" });
    return securePath;
  } catch {
    return keyPath;
  }
}

function testSshKeyAuth(user, host, port, keyPath) {
  const secureKey = ensureSecureKeyPath(keyPath);
  const result = spawnSync(
    "ssh",
    [
      "-i", secureKey,
      "-o", "BatchMode=yes",
      "-o", "StrictHostKeyChecking=accept-new",
      "-o", "ConnectTimeout=10",
      "-p", String(port),
      `${user}@${host}`,
      "echo OK",
    ],
    { stdio: ["ignore", "pipe", "pipe"] }
  );
  const out = result.stdout ? result.stdout.toString().trim() : "";
  const err = result.stderr ? result.stderr.toString().trim() : "";
  if (result.status !== 0 && err) {
    console.log(`  SSH key auth debug: ${err}`);
  }
  return result.status === 0 && out === "OK";
}

/**
 * Appends the deploy public key to authorized_keys on the server.
 * Prompts for the server password (one-time setup).
 * Returns { ok: true } or { ok: false, reason: string }.
 */
function installDeployKey(user, host, port, pubKeyPath) {
  const pubKey = fs.readFileSync(pubKeyPath, "utf8").trim();
  console.log("You will be prompted for your server password (this is the last time).");
  // Capture stdout+stderr so we can detect the failure reason, but also
  // print them so the user sees the server's diagnostic output.
  const result = spawnSync(
    "ssh",
    [
      "-o", "StrictHostKeyChecking=accept-new",
      "-o", "ConnectTimeout=15",
      "-p", String(port),
      `${user}@${host}`,
      `mkdir -p ~/.ssh && chmod 700 ~/.ssh && echo '${pubKey}' >> ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys && echo "Key installed"`,
    ],
    { stdio: ["inherit", "pipe", "pipe"] }
  );
  const stdout = result.stdout ? result.stdout.toString() : "";
  const stderr = result.stderr ? result.stderr.toString() : "";
  if (stdout) process.stdout.write(stdout);
  if (stderr) process.stderr.write(stderr);
  if (result.status === 0) return { ok: true };
  const combined = stdout + stderr;
  if (combined.includes("No space left on device") || combined.includes("no space")) {
    return { ok: false, reason: "disk_full" };
  }
  return { ok: false, reason: "unknown" };
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

// Verify SSH key auth works before running rsync/scp.
// If it doesn't, auto-install the deploy key (ssh-copy-id equivalent).
const deployKeyPubPath = deployKeyPath + ".pub";
console.log("\nChecking SSH key auth...");
if (!testSshKeyAuth(user, host, port, deployKeyPath)) {
  if (!fs.existsSync(deployKeyPubPath)) {
    exitWithError(
      "SSH key auth failed and deploy-key.pub not found.\n" +
      "Re-run Option 5 (Setup VPS) to generate a new deploy keypair."
    );
  }
  console.log("Deploy key not yet authorized on server.");
  console.log("Installing deploy key on server (one-time setup)...");
  const installed = installDeployKey(user, host, port, deployKeyPubPath);
  if (!installed.ok) {
    if (installed.reason === "disk_full") {
      exitWithError(
        "Server is out of disk space — the deploy key could not be saved.\n" +
        "Free up space on the server, then try again:\n" +
        `  ssh ${user}@${host} -p ${port} "journalctl --vacuum-size=50M && apt-get clean && df -h"`
      );
    }
    exitWithError(
      "Failed to install deploy key.\n" +
      "Check that the server password is correct and SSH is accessible."
    );
  }
  if (!testSshKeyAuth(user, host, port, deployKeyPath)) {
    exitWithError(
      "Deploy key was installed but key auth still fails.\n" +
      "Check server SSH config: PasswordAuthentication and AuthorizedKeysFile settings."
    );
  }
  console.log("Deploy key installed. Future deploys will not require a password.");
} else {
  console.log("SSH key auth: OK");
}

console.log("\nSyncing files...\n");

const securedKeyPath = ensureSecureKeyPath(deployKeyPath);
const rsync = findRsync();
if (rsync) {
  const label = rsync.wsl ? "wsl rsync" : rsync.bin !== "rsync" ? rsync.bin : "rsync";
  console.log(`Using ${label}...`);
  // WSL rsync uses WSL's ssh which reads Linux-style permissions on /mnt/...
  // — the icacls-locked secured key won't be readable from WSL, so use the
  // original key path (toWslPath inside tryRsync handles the conversion).
  const rsyncKey = rsync.wsl ? deployKeyPath : securedKeyPath;
  tryRsync(rsync, user, host, port, rsyncKey, publishDir, remotePath);
} else {
  console.log("rsync not found - using scp instead (re-uploads all files).");
  console.log("Run Option 0 (Install Prerequisites) to install rsync for faster deploys.");
  tryScp(user, host, port, securedKeyPath, publishDir, remotePath);
}

console.log("\n========================================");
console.log("   Deploy Complete!");
console.log("========================================");
console.log(`\nYour update server: ${displayUrl}`);
console.log("Players will get the new version on next launcher startup.");
