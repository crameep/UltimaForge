#!/usr/bin/env node
/**
 * Publish game updates + launcher updates in one command.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { execSync } from "node:child_process";
import readline from "node:readline/promises";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const appDir = path.resolve(__dirname, "..");
const repoRoot = path.resolve(appDir, "..");
const legacyKeysDir = path.join(repoRoot, "keys");
const defaultKeysDir = path.join(repoRoot, "server-data", "keys");
const resolvedKeysDir = fs.existsSync(legacyKeysDir) ? legacyKeysDir : defaultKeysDir;
const require = createRequire(import.meta.url);

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function readJsonIfExists(filePath, fallback = null) {
  if (!fs.existsSync(filePath)) {
    return fallback;
  }
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8"));
  } catch (error) {
    return fallback;
  }
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

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

async function promptForValue(rl, label, fallback = "", required = false) {
  const prompt = fallback ? `${label} [${fallback}]: ` : `${label}: `;
  const value = (await rl.question(prompt)).trim();
  if (value) {
    return value;
  }
  if (required && !fallback) {
    return promptForValue(rl, label, fallback, required);
  }
  return fallback;
}

function findSignature(binaryPath) {
  const sigPath = `${binaryPath}.sig`;
  if (fs.existsSync(sigPath)) {
    return fs.readFileSync(sigPath, "utf8").trim();
  }
  return "";
}

function resolvePath(p) {
  return path.isAbsolute(p) ? p : path.resolve(repoRoot, p);
}

function isMissingTauriBinding() {
  try {
    require("@tauri-apps/cli");
    return false;
  } catch (err) {
    const message = err && err.message ? String(err.message) : "";
    if (message.includes("Cannot find native binding")) {
      return true;
    }
    if (message.includes("cli.win32") || message.includes("@tauri-apps/cli-win32")) {
      return true;
    }
    return false;
  }
}

function fixTauriDeps() {
  const lockPath = path.join(appDir, "package-lock.json");
  const modulesPath = path.join(appDir, "node_modules");
  if (fs.existsSync(modulesPath)) {
    fs.rmSync(modulesPath, { recursive: true, force: true });
  }
  if (fs.existsSync(lockPath)) {
    fs.rmSync(lockPath, { force: true });
  }
  execSync("npm install", { cwd: appDir, stdio: "inherit" });
}

function resolveKeyPath(inputPath, filename) {
  const resolved = resolvePath(inputPath);
  if (fs.existsSync(resolved) && fs.statSync(resolved).isDirectory()) {
    return path.join(resolved, filename);
  }
  return resolved;
}

function validateUpdaterKeyPassword(keyPath, passwordPath) {
  if (!fs.existsSync(keyPath)) {
    throw new Error(
      `Missing Tauri updater private key at: ${keyPath}\n` +
      `Fix: Re-run option D to generate updater keys.`
    );
  }
  // Note: Tauri v2 always emits keys in "encrypted" format even with an empty
  // password. We therefore cannot reliably detect encryption from the key text.
  // We always pass TAURI_SIGNING_PRIVATE_KEY_PASSWORD (empty string for
  // no-password keys) so Tauri never falls back to an interactive prompt.
}

/**
 * Normalise the stored key file content to the value Tauri expects for
 * TAURI_SIGNING_PRIVATE_KEY (a base64-encoded blob of the full key text).
 *
 * Two storage formats can exist:
 *  - Raw text (written by `tauri signer generate --write-keys`):
 *      "untrusted comment: rsign ...\n<base64 key material>\n"
 *  - Base64 blob (pasted from the "Private:" line of `tauri signer generate`):
 *      "dW50cnVzdGVkIGNvb..."
 *
 * Tauri always wants the base64 blob. If we detect raw text, we encode it.
 */
function toTauriSigningEnvVar(fileContent) {
  const trimmed = fileContent.trim();
  if (trimmed.startsWith("untrusted comment:")) {
    return Buffer.from(trimmed + "\n").toString("base64");
  }
  return trimmed;
}

/**
 * Validate the updater key + password before running the expensive tauri build.
 * Returns { ok: true } on success, { ok: false } on wrong password,
 * or { ok: true, skipped: true } if the signer command is unavailable.
 */
function validateKeyCanSign(keyEnvVar, password, cwd) {
  const tmpFile = path.join(cwd, ".tauri-key-validate.tmp");
  try {
    fs.writeFileSync(tmpFile, "key-validation-test", "utf8");
    const npxCmd = process.platform === "win32" ? "npx.cmd" : "npx";
    const env = {
      ...process.env,
      TAURI_SIGNING_PRIVATE_KEY: keyEnvVar,
      TAURI_SIGNING_PRIVATE_KEY_PASSWORD: password,
    };
    // Ensure a stale PATH-based key in the inherited env never wins.
    delete env.TAURI_SIGNING_PRIVATE_KEY_PATH;
    execSync(`"${npxCmd}" tauri signer sign "${tmpFile}"`, {
      cwd,
      stdio: "pipe",
      env,
    });
    return { ok: true };
  } catch (err) {
    const combined = [err.stderr, err.stdout]
      .map((b) => (b ? b.toString() : ""))
      .join(" ");
    // Definitively wrong password — bail before the long compile.
    if (
      combined.includes("password") ||
      combined.includes("Wrong") ||
      combined.includes("incorrect") ||
      combined.includes("decode secret key")
    ) {
      return { ok: false };
    }
    // Signer command not found or unrelated error — don't block the build.
    return { ok: true, skipped: true };
  } finally {
    try { fs.unlinkSync(tmpFile); } catch (_) {}
    try { fs.unlinkSync(`${tmpFile}.sig`); } catch (_) {}
  }
}

function findLatestInstaller(bundleDir) {
  if (!fs.existsSync(bundleDir)) {
    return "";
  }

  const installers = [];
  const entries = fs.readdirSync(bundleDir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(bundleDir, entry.name);
    if (entry.isDirectory()) {
      const candidate = findLatestInstaller(fullPath);
      if (candidate) {
        installers.push(candidate);
      }
      continue;
    }
    if (entry.isFile()) {
      const lower = entry.name.toLowerCase();
      if (lower.endsWith(".exe") || lower.endsWith(".msi")) {
        installers.push(fullPath);
      }
    }
  }

  if (!installers.length) {
    return "";
  }

  installers.sort((a, b) => {
    const aStat = fs.statSync(a);
    const bStat = fs.statSync(b);
    return bStat.mtimeMs - aStat.mtimeMs;
  });

  const preferred = installers.find((p) => p.toLowerCase().endsWith(".exe"));
  return preferred || installers[0];
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  const cachePath = path.join(repoRoot, ".publish-all-cache.json");
  const cache = readJsonIfExists(cachePath, {});
  const launcherOnly = (args["launcher-only"] || "").trim() === "true";

  const brandPath = path.join(repoRoot, "branding", "brand.json");
  const brand = fs.existsSync(brandPath) ? readJson(brandPath) : null;
  const defaultUpdateUrl = brand?.updateUrl ?? "http://localhost:8080";

  let gameSource = "";
  let gameKey = "";
  let gameVersion = "";
  let gameExecutable = "client.exe";
  let gamePublicKey = "";

  if (!launcherOnly) {
    gameSource = await promptForValue(
      rl,
      "Game client source folder",
      args["game-source"] || cache.gameSource || "",
      true
    );
    gameKey = await promptForValue(
      rl,
      "Private key path for game updates",
      args["game-key"] || cache.gameKey || "",
      true
    );
    gameVersion = await promptForValue(
      rl,
      "Game version",
      args["game-version"] || cache.gameVersion || "",
      true
    );
    gameExecutable = await promptForValue(
      rl,
      "Game executable (relative to source)",
      args["game-exe"] || cache.gameExecutable || "client.exe",
      true
    );
    gamePublicKey = (args["game-public-key"] || "").trim();
    if (!gamePublicKey) {
      const resolvedKeyPath = resolvePath(gameKey);
      const publicKeyCandidate = fs.existsSync(resolvedKeyPath) &&
          fs.statSync(resolvedKeyPath).isDirectory()
        ? path.join(resolvedKeyPath, "public.key")
        : path.join(path.dirname(resolvedKeyPath), "public.key");
      if (fs.existsSync(publicKeyCandidate)) {
        gamePublicKey = publicKeyCandidate;
      }
    }
  }

  const legacyUpdatesDir = path.join(repoRoot, "updates");
  const defaultUpdatesDir = fs.existsSync(legacyUpdatesDir)
    ? legacyUpdatesDir
    : path.join(repoRoot, "server-data", "updates");
  const updatesDir = resolvePath(
    args["updates-dir"] || cache.updatesDir || defaultUpdatesDir
  );
  ensureDir(updatesDir);

  const earlyCache = {
    gameSource: gameSource || cache.gameSource,
    gameKey: gameKey || cache.gameKey,
    gameVersion: gameVersion || cache.gameVersion,
    gameExecutable: gameExecutable || cache.gameExecutable,
    updatesDir,
  };
  try {
    fs.writeFileSync(cachePath, JSON.stringify(earlyCache, null, 2), "utf8");
  } catch (error) {
    // ignore cache write failures
  }

  if (!launcherOnly) {
    console.log("\nPublishing game updates...");
    execSync(
      `cargo run -p publish-cli -- publish --source "${resolvePath(
        gameSource
      )}" --output "${updatesDir}" --key "${resolveKeyPath(
        gameKey,
        "private.key"
      )}" --version "${gameVersion}" --executable "${gameExecutable}"`,
      { cwd: repoRoot, stdio: "inherit" }
    );

    if (gamePublicKey) {
      console.log("\nValidating game update artifacts...");
      execSync(
        `cargo run -p publish-cli -- validate --dir "${updatesDir}" --key "${resolveKeyPath(
          gamePublicKey,
          "public.key"
        )}"`,
        { cwd: repoRoot, stdio: "inherit" }
      );
    } else {
      console.log("\nSkipping game update validation (no public key provided).");
    }
  } else {
    console.log("\nLauncher-only mode: skipping game update publish.");
  }

  const launcherVersion =
    (args["launcher-version"] || "").trim() ||
    cache.launcherVersion ||
    gameVersion ||
    "1.0.0";
  const bundleDir = path.join(
    repoRoot,
    "app",
    "src-tauri",
    "target",
    "release",
    "bundle"
  );
  const altBundleDir = path.join(repoRoot, "target", "release", "bundle");
  const shouldBuildLauncher = (args["launcher-build"] || "").trim() !== "false";
  if (shouldBuildLauncher) {
    const updaterKeysDir = path.join(resolvedKeysDir, "tauri-updater");
    const updaterKeyPath = path.join(updaterKeysDir, "tauri.key");
    const updaterPassPath = path.join(updaterKeysDir, "password.txt");
    try {
      validateUpdaterKeyPassword(updaterKeyPath, updaterPassPath);
    } catch (error) {
      console.log(`\n${error.message}`);
      console.log("Fix: re-run Option D and generate updater keys with no password, or enter the password when prompted.");
      process.exit(1);
    }

    const autoFixDeps = (args["auto-fix-deps"] || "").trim() === "true";
    if (isMissingTauriBinding()) {
      if (autoFixDeps) {
        console.log("\nTauri CLI native binding missing. Auto-fixing dependencies...");
        fixTauriDeps();
      } else {
        console.log(
          "\nTauri CLI native binding missing. Re-run with --auto-fix-deps true to auto-fix."
        );
        console.log("Manual fix: delete app/node_modules and app/package-lock.json, then run npm install.");
        process.exit(1);
      }
    }
    const env = { ...process.env };
    // Purge all inherited Tauri signing vars. If any were set in the Windows
    // environment from a previous manual attempt, they would override our key
    // files and cause "Wrong password" errors. We set them explicitly below.
    delete env.TAURI_SIGNING_PRIVATE_KEY_PATH;
    delete env.TAURI_SIGNING_PRIVATE_KEY;
    delete env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD;

    if (fs.existsSync(updaterKeyPath)) {
      // Normalise to the base64 blob format Tauri expects for the inline env var.
      // The key file may be raw text (from --write-keys) or already base64
      // (from pasting the "Private:" line). toTauriSigningEnvVar handles both.
      env.TAURI_SIGNING_PRIVATE_KEY = toTauriSigningEnvVar(
        fs.readFileSync(updaterKeyPath, "utf8")
      );
      // Always set the password env var — empty string for no-password keys.
      // Without this, Tauri falls back to an interactive prompt even for keys
      // generated with no password (Tauri v2 always uses the "encrypted" format).
      env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD = fs.existsSync(updaterPassPath)
        ? fs.readFileSync(updaterPassPath, "utf8").trim()
        : "";

      // Validate the key + password BEFORE the expensive 2.5-minute compile.
      console.log("\nValidating updater signing key...");
      const keyCheck = validateKeyCanSign(
        env.TAURI_SIGNING_PRIVATE_KEY,
        env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD,
        appDir
      );
      if (!keyCheck.ok) {
        console.log("\nERROR: Tauri updater key cannot be decrypted with the stored password.");
        console.log("Fix options:");
        console.log("  1. Re-run Option D to regenerate keys (press Enter for no password).");
        console.log(`  2. Edit ${updaterPassPath} with the correct password.`);
        process.exit(1);
      }
      if (keyCheck.skipped) {
        console.log("Note: Pre-build key validation skipped (tauri signer sign not available).");
      } else {
        console.log("Updater key validation passed.");
      }

      // Diagnostic: verify env vars actually reach child processes.
      try {
        const childKeyLen = execSync(
          "node -e \"process.stdout.write(String(process.env.TAURI_SIGNING_PRIVATE_KEY?.length??0))\"",
          { cwd: appDir, env, stdio: "pipe" }
        ).toString().trim();
        const childPassLen = execSync(
          "node -e \"process.stdout.write(String(process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD?.length??-1))\"",
          { cwd: appDir, env, stdio: "pipe" }
        ).toString().trim();
        const childPathSet = execSync(
          "node -e \"process.stdout.write(process.env.TAURI_SIGNING_PRIVATE_KEY_PATH??'(unset)')\"",
          { cwd: appDir, env, stdio: "pipe" }
        ).toString().trim();
        console.log(`  Key visible in child: length=${childKeyLen}`);
        console.log(`  Password visible in child: length=${childPassLen}`);
        console.log(`  PATH var in child: ${childPathSet}`);
      } catch (_) {
        console.log("  (child diagnostic failed — continuing)");
      }

      // Belt-and-suspenders: mutate the parent process.env directly so the
      // signing vars are inherited even if execSync's env parameter is somehow
      // not propagated (observed on some Windows + Node configurations).
      process.env.TAURI_SIGNING_PRIVATE_KEY = env.TAURI_SIGNING_PRIVATE_KEY;
      process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD = env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD;
      delete process.env.TAURI_SIGNING_PRIVATE_KEY_PATH;
    }

    console.log("\nBuilding launcher (tauri build)...");
    try {
      execSync("npm run tauri build", { cwd: appDir, stdio: "inherit", env });
    } finally {
      delete process.env.TAURI_SIGNING_PRIVATE_KEY;
      delete process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD;
    }
  } else {
    console.log("\nSkipping launcher build (launcher-build=false).");
  }

  const detectedLauncherBinary =
    findLatestInstaller(bundleDir) || findLatestInstaller(altBundleDir);
  if (shouldBuildLauncher && detectedLauncherBinary) {
    const sigCandidate = `${detectedLauncherBinary}.sig`;
    if (!fs.existsSync(sigCandidate)) {
      console.log("\nWarning: No .sig found next to the detected installer.");
      console.log("This means the launcher was not signed for auto-updates.");
      console.log("Fix: ensure updater keys exist and are unencrypted (recommended).");
    }
  }
  const launcherBinary = await promptForValue(
    rl,
    "Launcher binary/installer path",
    args["launcher-binary"] ||
      cache.launcherBinary ||
      detectedLauncherBinary ||
      "",
    true
  );
  const launcherTarget =
    (args["launcher-target"] || "").trim() ||
    cache.launcherTarget ||
    "windows";
  const launcherArch =
    (args["launcher-arch"] || "").trim() || cache.launcherArch || "x86_64";
  const launcherBaseUrl =
    (args["launcher-base-url"] || "").trim() ||
    cache.launcherBaseUrl ||
    defaultUpdateUrl;

  let launcherSignature =
    (args["launcher-signature"] || "").trim() ||
    (process.env.TAURI_UPDATER_SIGNATURE || "").trim();
  if (!launcherSignature && args["launcher-signature-file"]) {
    launcherSignature = fs
      .readFileSync(resolvePath(args["launcher-signature-file"]), "utf8")
      .trim();
  }
  if (!launcherSignature) {
    launcherSignature = findSignature(resolvePath(launcherBinary));
  }

  if (!launcherSignature) {
    launcherSignature = await promptForValue(
      rl,
      "Launcher signature (Tauri updater signature)",
      "",
      true
    );
  }

  const updatedCache = {
    gameSource,
    gameKey,
    gameVersion,
    gameExecutable,
    updatesDir,
    launcherBinary,
    launcherVersion,
    launcherTarget,
    launcherArch,
    launcherBaseUrl,
  };
  try {
    fs.writeFileSync(cachePath, JSON.stringify(updatedCache, null, 2), "utf8");
  } catch (error) {
    // ignore cache write failures
  }

  const launcherNotes =
    (args["launcher-notes"] || "").trim() ||
    (args["launcher-notes-file"]
      ? fs
          .readFileSync(resolvePath(args["launcher-notes-file"]), "utf8")
          .trim()
      : "");

  const launcherDir = path.join(updatesDir, "launcher");
  const launcherFilesDir = path.join(launcherDir, "files");
  ensureDir(launcherFilesDir);

  const launcherBinaryPath = resolvePath(launcherBinary);
  const launcherBinaryName = path.basename(launcherBinaryPath);
  fs.copyFileSync(
    launcherBinaryPath,
    path.join(launcherFilesDir, launcherBinaryName)
  );

  const pubDate = new Date().toISOString().replace(/\.\d{3}Z$/, "Z");
  const platformKey = `${launcherTarget}-${launcherArch}`;
  const launcherUrl = `${launcherBaseUrl.replace(/\/$/, "")}/launcher/files/${launcherBinaryName}`;
  const launcherMetadata = {
    version: launcherVersion,
    notes: launcherNotes,
    pub_date: pubDate,
    platforms: {
      [platformKey]: {
        signature: launcherSignature.trim(),
        url: launcherUrl,
      },
    },
  };

  fs.writeFileSync(
    path.join(launcherDir, "latest.json"),
    JSON.stringify(launcherMetadata, null, 2),
    "utf8"
  );
  fs.writeFileSync(
    path.join(launcherDir, `${platformKey}.json`),
    JSON.stringify(launcherMetadata, null, 2),
    "utf8"
  );

  rl.close();

  console.log("\nPublish summary");
  console.log(`- Game updates output: ${updatesDir}`);
  console.log(`- Launcher updates output: ${launcherDir}`);
  console.log(`- Launcher metadata: /launcher/${platformKey}.json (and latest.json)`);
  console.log(`- Launcher binary: /launcher/files/${launcherBinaryName}`);
  console.log("\nSuggested smoke tests:");
  console.log(`- ${defaultUpdateUrl.replace(/\/$/, "")}/manifest.json`);
  console.log(`- ${defaultUpdateUrl.replace(/\/$/, "")}/manifest.sig`);
  console.log(`- ${defaultUpdateUrl.replace(/\/$/, "")}/launcher/${launcherTarget}/${launcherArch}/${launcherVersion}`);
}

main().catch((error) => {
  console.error("Publish failed:", error);
  process.exit(1);
});
