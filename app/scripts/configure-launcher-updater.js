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

function runSignerGenerateCommand(command, args) {
  return new Promise((resolve) => {
    try {
      const child = spawn(command, args, {
        cwd: appDir,
        stdio: ["inherit", "pipe", "pipe"],
      });

      let stdout = "";
      let stderr = "";
      child.stdout.on("data", (chunk) => {
        const text = chunk.toString();
        stdout += text;
        process.stdout.write(text);
      });
      child.stderr.on("data", (chunk) => {
        const text = chunk.toString();
        stderr += text;
        process.stderr.write(text);
      });

      child.on("close", () => {
        const combined = `${stdout}\n${stderr}`;
        resolve(parseGeneratedKeys(combined));
      });

      child.on("error", () => {
        resolve({ privateKey: "", publicKey: "" });
      });
    } catch (error) {
      resolve({ privateKey: "", publicKey: "" });
    }
  });
}

async function runSignerGenerate() {
  const npxCommand = process.platform === "win32" ? "npx.cmd" : "npx";
  let generated = await runSignerGenerateCommand(npxCommand, [
    "tauri",
    "signer",
    "generate",
  ]);
  if (generated.privateKey || generated.publicKey) {
    return generated;
  }

  const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
  generated = await runSignerGenerateCommand(npmCommand, [
    "exec",
    "--",
    "tauri",
    "signer",
    "generate",
  ]);
  if (generated.privateKey || generated.publicKey) {
    return generated;
  }

  const outputFile = path.join(updaterDir, "tauri-signer-output.txt");
  try {
    const command = `${npmCommand} exec -- tauri signer generate > "${outputFile}" 2>&1`;
    execSync(command, {
      cwd: appDir,
      stdio: "ignore",
      shell: true,
    });
    if (fs.existsSync(outputFile)) {
      const output = fs.readFileSync(outputFile, "utf8");
      fs.unlinkSync(outputFile);
      return parseGeneratedKeys(output);
    }
  } catch (error) {
    if (fs.existsSync(outputFile)) {
      fs.unlinkSync(outputFile);
    }
  }

  return { privateKey: "", publicKey: "" };
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
  const keyText = fs.readFileSync(privateKeyPath, "utf8");
  const encrypted = isEncryptedKey(keyText);
  const hasPassword = fs.existsSync(passwordPath) &&
    fs.readFileSync(passwordPath, "utf8").trim().length > 0;

  if (encrypted && !hasPassword) {
    console.log("\nERROR: The Tauri updater key is encrypted but no password was saved.");
    console.log("Fix option 1: Re-run this wizard and regenerate keys — press Enter at both");
    console.log("             password prompts to create an unencrypted key (recommended).");
    console.log("Fix option 2: Create keys/tauri-updater/password.txt containing your password.");
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
    const generated = await runSignerGenerate();

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

  const encryptedKey = isEncryptedKey(privateKey);
  const hasPasswordFile =
    fs.existsSync(privateKeyPasswordPath) &&
    fs.readFileSync(privateKeyPasswordPath, "utf8").trim().length > 0;

  if (encryptedKey && !hasPasswordFile) {
    const password = await promptForValue(
      rl,
      "Updater key is encrypted. Enter the same password (required)",
      "",
      true
    );
    writeKeyFile(privateKeyPasswordPath, password);
  }

  if (!encryptedKey && fs.existsSync(privateKeyPasswordPath)) {
    fs.unlinkSync(privateKeyPasswordPath);
  }

  if (encryptedKey) {
    console.log("\nPassword stored for encrypted updater key.");
  } else {
    console.log("\nNo password stored. Key is unencrypted (recommended).");
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
