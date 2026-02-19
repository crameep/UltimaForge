#!/usr/bin/env node
/**
 * Server owner wizard to generate branding/brand.json and optional keys.
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

const brandingDir = path.join(repoRoot, "branding");
const brandPath = path.join(brandingDir, "brand.json");
const legacyKeysDir = path.join(repoRoot, "keys");
const defaultKeysDir = path.join(repoRoot, "server-data", "keys");
const keysDir = fs.existsSync(legacyKeysDir) ? legacyKeysDir : defaultKeysDir;
const publicKeyPath = path.join(keysDir, "public.key");

function sanitizeServerName(name) {
  return name.replace(/[^a-zA-Z0-9]/g, "");
}

function readJsonIfExists(filePath) {
  if (!fs.existsSync(filePath)) {
    return null;
  }
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
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

async function main() {
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  const existing = readJsonIfExists(brandPath);

  console.log("UltimaForge Server Owner Wizard");
  console.log("This will create branding/brand.json for your launcher.\n");

  const displayName = await promptForValue(
    rl,
    "Server display name",
    existing?.product?.displayName ?? "",
    true
  );

  const defaultServerName = sanitizeServerName(displayName) || existing?.product?.serverName || "";
  const serverName = await promptForValue(
    rl,
    "Server name (no spaces)",
    defaultServerName,
    true
  );

  const updateUrl = await promptForValue(
    rl,
    "Update URL (https://updates.yourserver.com)",
    existing?.updateUrl ?? "",
    true
  );

  const supportEmail = await promptForValue(
    rl,
    "Support email (optional)",
    existing?.product?.supportEmail ?? ""
  );

  const website = await promptForValue(
    rl,
    "Website URL (optional)",
    existing?.product?.website ?? ""
  );

  const discord = await promptForValue(
    rl,
    "Discord invite URL (optional)",
    existing?.product?.discord ?? ""
  );

  const description = await promptForValue(
    rl,
    "Short description (optional)",
    existing?.product?.description ?? ""
  );

  const wantColors = await confirm(rl, "Customize theme colors?", false);
  const primary = wantColors
    ? await promptForValue(rl, "Primary color (#RRGGBB)", existing?.ui?.colors?.primary ?? "")
    : "";
  const secondary = wantColors
    ? await promptForValue(rl, "Secondary color (#RRGGBB)", existing?.ui?.colors?.secondary ?? "")
    : "";
  const background = wantColors
    ? await promptForValue(rl, "Background color (#RRGGBB)", existing?.ui?.colors?.background ?? "")
    : "";
  const text = wantColors
    ? await promptForValue(rl, "Text color (#RRGGBB)", existing?.ui?.colors?.text ?? "")
    : "";

  let publicKey = existing?.publicKey ?? "";
  const wantKeygen = await confirm(rl, "Generate a new Ed25519 keypair now?", true);
  if (wantKeygen) {
    try {
      execSync(`cargo run -p publish-cli -- keygen --output "${keysDir}" --force`, {
        cwd: repoRoot,
        stdio: "inherit",
      });
      if (fs.existsSync(publicKeyPath)) {
        publicKey = fs.readFileSync(publicKeyPath, "utf8").trim();
      }
    } catch (error) {
      console.log("Key generation failed or cargo not available.");
    }
  }

  if (!publicKey) {
    publicKey = await promptForValue(
      rl,
      "Public key (64-char hex)",
      existing?.publicKey ?? "",
      true
    );
  }

  rl.close();

  const brand = {
    ...(existing ?? {}),
    product: {
      ...(existing?.product ?? {}),
      displayName,
      serverName,
      ...(description ? { description } : {}),
      ...(supportEmail ? { supportEmail } : {}),
      ...(website ? { website } : {}),
      ...(discord ? { discord } : {}),
    },
    updateUrl,
    publicKey,
    ui: {
      ...(existing?.ui ?? {}),
      ...(wantColors
        ? {
            colors: {
              ...(existing?.ui?.colors ?? {}),
              ...(primary ? { primary } : {}),
              ...(secondary ? { secondary } : {}),
              ...(background ? { background } : {}),
              ...(text ? { text } : {}),
            },
          }
        : {}),
      windowTitle: displayName,
      heroTitle: existing?.ui?.heroTitle ?? `Welcome to ${displayName}`,
      heroSubtitle: existing?.ui?.heroSubtitle ?? "Your adventure begins here",
      sidebarSubtitle: existing?.ui?.sidebarSubtitle ?? "Game Launcher",
    },
    brandVersion: existing?.brandVersion ?? "1.0",
  };

  fs.mkdirSync(brandingDir, { recursive: true });
  fs.writeFileSync(brandPath, JSON.stringify(brand, null, 2), "utf8");

  console.log(`\nSaved branding config: ${brandPath}`);

  try {
    execSync("node sync-branding-config.js", { cwd: appDir, stdio: "inherit" });
  } catch (error) {
    console.log("Branding sync skipped (node or script not available).");
  }
}

main().catch((error) => {
  console.error("Wizard failed:", error);
  process.exit(1);
});
