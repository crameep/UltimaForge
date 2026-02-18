#!/usr/bin/env node
/**
 * Publish game updates + launcher updates in one command.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { execSync } from "node:child_process";
import readline from "node:readline/promises";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const appDir = path.resolve(__dirname, "..");
const repoRoot = path.resolve(appDir, "..");

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
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

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  const brandPath = path.join(repoRoot, "branding", "brand.json");
  const brand = fs.existsSync(brandPath) ? readJson(brandPath) : null;
  const defaultUpdateUrl = brand?.updateUrl ?? "http://localhost:8080";

  const gameSource = await promptForValue(
    rl,
    "Game client source folder",
    args["game-source"] || "",
    true
  );
  const gameKey = await promptForValue(
    rl,
    "Private key path for game updates",
    args["game-key"] || "",
    true
  );
  const gameVersion = await promptForValue(
    rl,
    "Game version",
    args["game-version"] || "",
    true
  );
  const gameExecutable = await promptForValue(
    rl,
    "Game executable (relative to source)",
    args["game-exe"] || "client.exe",
    true
  );
  const gamePublicKey = (args["game-public-key"] || "").trim();

  const updatesDir = resolvePath(args["updates-dir"] || "updates");
  ensureDir(updatesDir);

  console.log("\nPublishing game updates...");
  execSync(
    `cargo run -p publish-cli -- publish --source "${resolvePath(
      gameSource
    )}" --output "${updatesDir}" --key "${resolvePath(
      gameKey
    )}" --version "${gameVersion}" --executable "${gameExecutable}"`,
    { cwd: repoRoot, stdio: "inherit" }
  );

  if (gamePublicKey) {
    console.log("\nValidating game update artifacts...");
    execSync(
      `cargo run -p publish-cli -- validate --dir "${updatesDir}" --key "${resolvePath(
        gamePublicKey
      )}"`,
      { cwd: repoRoot, stdio: "inherit" }
    );
  } else {
    console.log("\nSkipping game update validation (no public key provided).");
  }

  const launcherVersion =
    (args["launcher-version"] || "").trim() || gameVersion;
  const launcherBinary = await promptForValue(
    rl,
    "Launcher binary/installer path",
    args["launcher-binary"] || "",
    true
  );
  const launcherTarget =
    (args["launcher-target"] || "").trim() || "windows";
  const launcherArch = (args["launcher-arch"] || "").trim() || "x86_64";
  const launcherBaseUrl =
    (args["launcher-base-url"] || "").trim() || defaultUpdateUrl;

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
