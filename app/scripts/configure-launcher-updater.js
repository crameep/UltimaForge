#!/usr/bin/env node
/**
 * Configure Tauri updater keys and embed the public key into tauri.conf.json.
 *
 * This is a helper for server owners so launcher self-updates work out of the box.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execSync, spawn } from "node:child_process";
import readline from "node:readline/promises";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const appDir = path.resolve(__dirname, "..");
const repoRoot = path.resolve(appDir, "..");

const legacyKeysDir = path.join(repoRoot, "keys");
const defaultKeysDir = path.join(repoRoot, "server-data", "keys");
const keysDir = fs.existsSync(legacyKeysDir) ? legacyKeysDir : defaultKeysDir;

const updaterDir = path.join(keysDir, "tauri-updater");
const privateKeyPath = path.join(updaterDir, "tauri.key");
const privateKeyPasswordPath = path.join(updaterDir, "password.txt");
const tauriConfigPath = path.join(appDir, "src-tauri", "tauri.conf.json");

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function writeJson(filePath, data) {
  fs.writeFileSync(filePath, JSON.stringify(data, null, 2), "utf8");
}

function findPublicKey(dir) {
  const candidates = [
    path.join(dir, "tauri.pub"),
    path.join(dir, "public.key"),
    path.join(dir, "updater.pub"),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return fs.readFileSync(candidate, "utf8").trim();
    }
  }
  return "";
}

function findPrivateKey(dir) {
  const candidates = [
    path.join(dir, "tauri.key"),
    path.join(dir, "private.key"),
    path.join(dir, "updater.key"),
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate)) {
      return fs.readFileSync(candidate, "utf8").trim();
    }
  }
  return "";
}

function writeKeyFile(filePath, content) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${content.trim()}\n`, "utf8");
}

function isEncryptedKey(keyText) {
  return keyText.includes("encrypted secret key");
}

function parseGeneratedKeys(output) {
  const lines = output.split(/\r?\n/).map((line) => line.trim());
  let privateKey = "";
  let publicKey = "";
  for (let i = 0; i < lines.length; i += 1) {
    if (lines[i].startsWith("Private:")) {
      privateKey = (lines[i + 1] || "").trim();
    }
    if (lines[i].startsWith("Public:")) {
      publicKey = (lines[i + 1] || "").trim();
    }
  }
  return { privateKey, publicKey };
}

function isValidTauriPrivateKey(keyText) {
  if (!keyText || keyText.trim().length === 0) return false;
  // Tauri minisign keys are base64-encoded blocks, always multi-line
  // Minimum length check (unencrypted ~110 chars, encrypted ~340 chars)
  if (keyText.trim().length < 80) return false;
  return true;
}

function isValidTauriPublicKey(keyText) {
  if (!keyText || keyText.trim().length === 0) return false;
  // Public key is a single base64 line, typically 100-160 chars
  if (keyText.trim().length < 40) return false;
  return true;
}

function tryReadWrittenKeys(keyBase) {
  // Tauri --write-keys <base> writes <base>.key (private) and <base>.pub (public).
  // Check both that pattern and a plain-path variant.
  const candidates = [
    [path.join(updaterDir, "tauri.key"), path.join(updaterDir, "tauri.pub")],
    [`${keyBase}.key`, `${keyBase}.pub`],
    [keyBase, `${keyBase}.pub`],
  ];
  for (const [privPath, pubPath] of candidates) {
    if (!fs.existsSync(privPath) || !fs.existsSync(pubPath)) continue;
    const priv = fs.readFileSync(privPath, "utf8").trim();
    const pub = fs.readFileSync(pubPath, "utf8").trim();
    if (isValidTauriPrivateKey(priv) && isValidTauriPublicKey(pub)) {
      return { privateKey: priv, publicKey: pub };
    }
  }
  return null;
}

async function runSignerGenerate(rl) {
  const npxCommand = process.platform === "win32" ? "npx.cmd" : "npx";
  const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
  // Base path passed to --write-keys; Tauri appends .key / .pub
  const keyBase = path.join(updaterDir, "tauri");

  // Attempt 1: --write-keys writes key files directly — no stdout capture needed.
  // This avoids the Windows limitation where interactive programs write through
  // the console API and bypass Node.js stdout/stderr pipes.
  console.log("Generating Tauri updater keys...");
  for (const [cmd, baseArgs] of [
    [npxCommand, ["tauri", "signer", "generate"]],
    [npmCommand, ["exec", "--", "tauri", "signer", "generate"]],
  ]) {
    try {
      execSync(
        [cmd, ...baseArgs, "--password", "", "--write-keys", keyBase].join(" "),
        { cwd: appDir, stdio: "ignore" }
      );
      const keys = tryReadWrittenKeys(keyBase);
      if (keys) return keys;
    } catch (e) {
      // flag not supported or command failed — try next
    }
  }

  // Attempt 2: run interactively so the user can interact with password prompts,
  // then ask them to paste the key values they see in the terminal.
  console.log("\nAutomatic generation failed. Running interactively instead.");
  console.log("Press Enter at BOTH password prompts for a no-password key.\n");

  let ran = false;
  for (const [cmd, args] of [
    [npxCommand, ["tauri", "signer", "generate"]],
    [npmCommand, ["exec", "--", "tauri", "signer", "generate"]],
  ]) {
    try {
      execSync([cmd, ...args].join(" "), { cwd: appDir, stdio: "inherit" });
      ran = true;
      break;
    } catch (e) {
      // try next
    }
  }

  if (!ran) {
    return { privateKey: "", publicKey: "" };
  }

  // The keys were printed to the terminal above. Ask the user to paste them.
  console.log("\nPaste the key values shown above:");
  const privateKey = (
    await rl.question("  Private key (base64 line after 'Private:'): ")
  ).trim();
  const publicKey = (
    await rl.question("  Public key  (base64 line after 'Public:'):  ")
  ).trim();

  return { privateKey, publicKey };
}

function findPublicKeyInPaths(paths) {
  for (const filePath of paths) {
    if (fs.existsSync(filePath)) {
      return fs.readFileSync(filePath, "utf8").trim();
    }
  }
  return "";
}

function getHomeDir() {
  return process.env.HOME || process.env.USERPROFILE || "";
}

function guessDefaultKeyPaths() {
  const home = getHomeDir();
  const homeTauri = home ? path.join(home, ".tauri") : "";
  const cwd = process.cwd();
  return [
    path.join(cwd, "tauri.pub"),
    path.join(cwd, "tauri_signing.pub"),
    path.join(cwd, "tauri-signing.pub"),
    homeTauri ? path.join(homeTauri, "tauri.pub") : "",
    homeTauri ? path.join(homeTauri, "tauri_signing.pub") : "",
    homeTauri ? path.join(homeTauri, "tauri-signing.pub") : "",
  ].filter(Boolean);
}

function copyPublicKey(sourcePath, destDir) {
  const destPath = path.join(destDir, "tauri.pub");
  fs.mkdirSync(destDir, { recursive: true });
  fs.copyFileSync(sourcePath, destPath);
  return destPath;
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

async function confirm(rl, label, fallback = true) {
  const suffix = fallback ? "Y/n" : "y/N";
  const value = (await rl.question(`${label} (${suffix}): `)).trim().toLowerCase();
  if (!value) {
    return fallback;
  }
  return value === "y" || value === "yes";
}

function validateFinalKeyState(privateKeyPath, passwordPath) {
  if (!fs.existsSync(privateKeyPath)) {
    console.log("\nWARNING: Private key file was not created. Re-run and generate keys.");
    return false;
  }
  // password.txt must exist (even if empty) so publish-all can pass
  // TAURI_SIGNING_PRIVATE_KEY_PASSWORD without triggering an interactive prompt.
  if (!fs.existsSync(passwordPath)) {
    console.log(`\nERROR: ${passwordPath} is missing.`);
    console.log("Re-run this wizard to create it.");
    return false;
  }
  return true;
}

async function main() {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  console.log("Launcher Updater Key Setup");
  console.log("This configures Tauri updater keys for launcher self-updates.\n");
  console.log("Password guidance:");
  console.log("- Recommended: press Enter at both key prompts to avoid passwords.");
  console.log("- If you set a password, we will store it for signing.\n");

  let publicKey = findPublicKey(updaterDir);

  const wantGenerate = await confirm(
    rl,
    publicKey
      ? "Tauri updater keypair found. Regenerate it?"
      : "Generate a new Tauri updater keypair now?",
    !publicKey
  );

  if (wantGenerate) {
    fs.mkdirSync(updaterDir, { recursive: true });
    const generated = await runSignerGenerate(rl);

    const privateValid = isValidTauriPrivateKey(generated.privateKey);
    const publicValid = isValidTauriPublicKey(generated.publicKey);

    if (!privateValid || !publicValid) {
      console.log("\nERROR: Failed to capture Tauri updater keys from CLI output.");
      console.log("The tauri signer generate command ran but the key output could not be parsed.");
      console.log("\nTroubleshooting:");
      console.log("  1. Run manually: npx tauri signer generate");
      console.log("  2. Copy the output into keys/tauri-updater/tauri.key (private) and tauri.pub (public)");
      console.log("  3. Re-run this wizard — it will detect the existing keys");
      console.log("\nNo key files were written.");
      rl.close();
      process.exit(1);
    }

    writeKeyFile(privateKeyPath, generated.privateKey);
    writeKeyFile(path.join(updaterDir, "tauri.pub"), generated.publicKey);
    // Write empty password.txt — generated keys have no password.
    // publish-all.js always passes TAURI_SIGNING_PRIVATE_KEY_PASSWORD from this
    // file (empty string = no password), preventing an interactive prompt.
    writeKeyFile(privateKeyPasswordPath, "");
    console.log("Tauri updater keys captured and saved.");
  }

  publicKey = findPublicKey(updaterDir);
  if (!publicKey) {
    const guessPaths = guessDefaultKeyPaths();
    const guessed = findPublicKeyInPaths(guessPaths);
    if (guessed) {
      publicKey = guessed;
    } else {
      const guessList = guessPaths.join("\n  - ");
      if (guessList) {
        console.log("\nCould not find a Tauri updater public key.");
        console.log("Looked for common locations:");
        console.log(`  - ${guessList}`);
      }
    }
  }

  if (!publicKey) {
    const keyPath = await promptForValue(
      rl,
      "Tauri updater public key file path (or paste key)",
      "",
      true
    );
    if (fs.existsSync(keyPath)) {
      const copiedPath = copyPublicKey(keyPath, updaterDir);
      publicKey = fs.readFileSync(copiedPath, "utf8").trim();
    } else {
      publicKey = keyPath.trim();
    }
  }

  if (publicKey) {
    writeKeyFile(path.join(updaterDir, "tauri.pub"), publicKey);
  }

  let privateKey = findPrivateKey(updaterDir);
  if (!privateKey) {
    const privateKeyInput = await promptForValue(
      rl,
      "Tauri updater private key file path (or paste key)",
      "",
      true
    );
    if (fs.existsSync(privateKeyInput)) {
      privateKey = fs.readFileSync(privateKeyInput, "utf8").trim();
    } else {
      privateKey = privateKeyInput.trim();
    }
  }

  if (privateKey) {
    writeKeyFile(privateKeyPath, privateKey);
  }

  if (!fs.existsSync(tauriConfigPath)) {
    console.log(`Could not find ${tauriConfigPath}`);
    rl.close();
    return;
  }

  if (!privateKey) {
    console.log("\nUpdater keys are missing. Please re-run and generate keys.");
    rl.close();
    return;
  }

  // Tauri v2 always emits the "encrypted" key format even for empty-password
  // keys, so we cannot detect encryption from the key text. Instead, we ensure
  // password.txt always exists so publish-all can set
  // TAURI_SIGNING_PRIVATE_KEY_PASSWORD without falling back to an interactive
  // prompt. For generated keys this file was already written above. For
  // manually pasted keys we ask once; blank = no password.
  if (!fs.existsSync(privateKeyPasswordPath)) {
    const password = await promptForValue(
      rl,
      "Key password (press Enter if this key has no password)",
      "",
      false
    );
    writeKeyFile(privateKeyPasswordPath, password);
  }

  const savedPassword = fs.readFileSync(privateKeyPasswordPath, "utf8").trim();
  if (savedPassword) {
    console.log("\nPassword stored for updater key.");
  } else {
    console.log("\nNo password stored (key has no password — recommended).");
  }

  rl.close();

  const tauriConfig = readJson(tauriConfigPath);
  if (!tauriConfig.plugins) {
    tauriConfig.plugins = {};
  }
  if (!tauriConfig.plugins.updater) {
    tauriConfig.plugins.updater = { endpoints: [] };
  }

  tauriConfig.plugins.updater.pubkey = publicKey;
  writeJson(tauriConfigPath, tauriConfig);

  const valid = validateFinalKeyState(privateKeyPath, privateKeyPasswordPath);
  if (!valid) {
    rl.close();
    process.exit(1);
  }

  console.log("\nUpdated Tauri updater public key:");
  console.log(`- ${tauriConfigPath}`);
  console.log(`- Key source: ${publicKey ? "configured" : "missing"}`);
  console.log(`- Key directory: ${updaterDir}`);
  if (fs.existsSync(privateKeyPath)) {
    console.log(`- Private key saved: ${privateKeyPath}`);
  }
  if (fs.existsSync(privateKeyPasswordPath)) {
    console.log(`- Private key password saved: ${privateKeyPasswordPath}`);
  }
}

main().catch((error) => {
  console.error("Updater key setup failed:", error);
});
