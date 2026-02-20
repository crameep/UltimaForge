#!/usr/bin/env node
/**
 * Guided VPS setup wizard for UltimaForge.
 * Generates an SSH deploy keypair, installs Caddy on a remote VPS,
 * and saves connection config to server-data/deploy.json.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import readline from "node:readline/promises";

let rl = null;

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const appDir = path.resolve(__dirname, "..");
const repoRoot = path.resolve(appDir, "..");

const serverDataDir = path.join(repoRoot, "server-data");
const keysDir = path.join(serverDataDir, "keys");
const deployKeyPath = path.join(keysDir, "deploy-key");
const deployKeyPubPath = path.join(keysDir, "deploy-key.pub");
const deployConfigPath = path.join(serverDataDir, "deploy.json");
const brandPath = path.join(repoRoot, "branding", "brand.json");

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function readJsonIfExists(p, fallback = {}) {
  try {
    return JSON.parse(fs.readFileSync(p, "utf8"));
  } catch {
    return fallback;
  }
}

async function prompt(rl, label, fallback = "") {
  const suffix = fallback ? ` [${fallback}]` : "";
  const answer = (await rl.question(`${label}${suffix}: `)).trim();
  return answer || fallback;
}

function generateDeployKey() {
  console.log("\nGenerating ed25519 SSH deploy keypair...");
  const result = spawnSync(
    "ssh-keygen",
    ["-t", "ed25519", "-f", deployKeyPath, "-N", "", "-C", "ultimaforge-deploy"],
    { stdio: "inherit" }
  );
  if (result.status !== 0) {
    throw new Error(
      "ssh-keygen failed. Make sure OpenSSH is installed.\n" +
        "  Windows: Settings -> Apps -> Optional Features -> OpenSSH Client\n" +
        "  Linux:   sudo apt install openssh-client"
    );
  }
  console.log("Deploy keypair generated.");
}

function ensureDeployKeypair() {
  if (fs.existsSync(deployKeyPath) && fs.existsSync(deployKeyPubPath)) {
    return;
  }

  if (fs.existsSync(deployKeyPath) && !fs.existsSync(deployKeyPubPath)) {
    console.warn(
      "\nDeploy key exists but the public key is missing. Regenerating the keypair..."
    );
    try {
      fs.unlinkSync(deployKeyPath);
    } catch {
      // ignore
    }
  }

  generateDeployKey();
}

function testSshConnection(host, user, port, keyPath) {
  console.log(`\nTesting SSH connection to ${user}@${host}:${port} ...`);
  const result = spawnSync(
    "ssh",
    [
      "-i",
      keyPath,
      "-o",
      "StrictHostKeyChecking=accept-new",
      "-o",
      "ConnectTimeout=10",
      "-p",
      String(port),
      `${user}@${host}`,
      "echo OK",
    ],
    { stdio: ["ignore", "pipe", "pipe"] }
  );
  const out = result.stdout ? result.stdout.toString().trim() : "";
  return result.status === 0 && out === "OK";
}

function runRemoteSetup(host, user, port, keyPath, remotePath, domain, hasDomain) {
  // hasDomain=true  -> Caddy listens on the domain (automatic HTTPS via Let's Encrypt)
  // hasDomain=false -> Caddy listens on :80 (HTTP-only, no domain required)
  const caddyDirective = hasDomain ? domain : ":80";
  const caddyNote = hasDomain
    ? 'echo "Caddy is running. HTTPS will activate once DNS propagates to this IP."'
    : 'echo "Caddy is running in HTTP mode on port 80."';

  const script = `#!/bin/bash
set -e
export DEBIAN_FRONTEND=noninteractive
echo "[1/6] Updating package index..."
apt-get update -y -qq
echo "[2/6] Installing Caddy dependencies..."
apt-get install -y -qq debian-keyring debian-archive-keyring apt-transport-https curl gnupg
echo "[3/6] Adding Caddy apt repository..."
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/gpg.key' \\
  | gpg --dearmor -o /usr/share/keyrings/caddy-stable-archive-keyring.gpg 2>/dev/null
curl -1sLf 'https://dl.cloudsmith.io/public/caddy/stable/debian.deb.txt' \\
  | tee /etc/apt/sources.list.d/caddy-stable.list > /dev/null
apt-get update -y -qq
echo "[4/6] Installing Caddy..."
apt-get install -y -qq caddy
echo "[5/6] Creating serve directory: ${remotePath}"
mkdir -p '${remotePath}'
chown -R caddy:caddy '${remotePath}' 2>/dev/null || true
echo "[6/6] Writing Caddyfile..."
cat > /etc/caddy/Caddyfile << 'CADDYEOF'
${caddyDirective} {
    root * ${remotePath}
    file_server
    encode gzip
}
CADDYEOF
systemctl enable caddy
systemctl restart caddy
${caddyNote}
`;

  console.log("\nRunning remote setup (this may take 1-2 minutes)...");
  const result = spawnSync(
    "ssh",
    [
      "-i",
      keyPath,
      "-o",
      "StrictHostKeyChecking=accept-new",
      "-p",
      String(port),
      `${user}@${host}`,
      "bash -s",
    ],
    { input: script, stdio: ["pipe", "inherit", "inherit"] }
  );
  return result.status === 0;
}

async function main() {
  rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  console.log("\n========================================");
  console.log("   UltimaForge VPS Setup Wizard");
  console.log("========================================");
  console.log("\nThis wizard will:");
  console.log("  1. Generate an SSH deploy key for secure access");
  console.log("  2. Guide you through creating a VPS on Digital Ocean (or any provider)");
  console.log("  3. Install Caddy (automatic HTTPS) on your VPS");
  console.log("  4. Save your VPS config for future deploys");

  ensureDir(keysDir);
  ensureDir(path.join(serverDataDir, "client"));
  ensureDir(path.join(serverDataDir, "publish"));

  // Step 1: Generate keypair if needed
  if (fs.existsSync(deployKeyPath) && fs.existsSync(deployKeyPubPath)) {
    console.log("\nDeploy key already exists at: " + deployKeyPath);
  } else {
    ensureDeployKeypair();
  }

  const pubKey = fs.readFileSync(deployKeyPubPath, "utf8").trim();

  // Step 2: Guide user through DO setup
  console.log("\n========================================");
  console.log("   STEP 1: Add your deploy key to the VPS");
  console.log("========================================");
  console.log("\nYour public deploy key (copy this entire line):");
  console.log("\n" + pubKey + "\n");
  console.log("If using Digital Ocean:");
  console.log("  -> Create a new Droplet (Ubuntu 22.04 LTS recommended)");
  console.log("  -> In 'Authentication', choose 'SSH Key'");
  console.log("  -> Click 'New SSH Key' and paste the key above");
  console.log("  -> Finish creating the Droplet and note its IP address");
  console.log("\nIf using another provider:");
  console.log("  -> Add the key to ~/.ssh/authorized_keys on the VPS");
  console.log("    (ssh in with a password, then: echo '<key>' >> ~/.ssh/authorized_keys)");

  console.log("\nDigital Ocean quick steps:");
  console.log("  1. Create Droplet -> Ubuntu 22.04 LTS");
  console.log("  2. Authentication: SSH Key -> New SSH Key");
  console.log("  3. Paste the key above and create the Droplet");
  console.log("  4. Copy the Droplet IP address");

  await prompt(rl, "\nPress Enter when the key is on your VPS and it is running");

  // Step 3: Collect VPS details
  console.log("\n========================================");
  console.log("   STEP 2: Enter your VPS details");
  console.log("========================================");

  const existing = readJsonIfExists(deployConfigPath);
  const host = await prompt(rl, "VPS IP address", existing.host || "");
  const user = await prompt(rl, "SSH user", existing.user || "root");
  let port = 22;
  while (true) {
    const portInput = await prompt(rl, "SSH port", String(existing.port || 22));
    const parsed = parseInt(portInput, 10);
    if (!Number.isNaN(parsed) && parsed > 0 && parsed <= 65535) {
      port = parsed;
      break;
    }
    console.warn("Invalid SSH port. Please enter a number between 1 and 65535.");
  }
  const remotePath = await prompt(
    rl,
    "Remote path to serve files from",
    existing.remote_path || "/var/www/ultimaforge"
  );

  if (!host) {
    console.error("\nERROR: VPS IP is required.");
    rl.close();
    process.exit(1);
  }

  if (remotePath.includes(" ")) {
    console.error("\nERROR: Remote path must not contain spaces (Caddy does not support spaces in root paths).");
    rl.close();
    process.exit(1);
  }

  // Domain is optional - HTTPS requires a domain; bare IP gets HTTP-only mode
  const domainAnswer = (
    await rl.question("\nDo you have a domain name pointed at this VPS? (Y/n): ")
  )
    .trim()
    .toLowerCase();
  const hasDomain = domainAnswer !== "n";

  let domain = "";
  if (hasDomain) {
    domain = await prompt(
      rl,
      "Domain name (e.g. updates.myserver.com)",
      existing.domain || ""
    );
    if (!domain) {
      console.error(
        "\nERROR: Domain name is required when using HTTPS mode. Re-run and answer 'n' to use HTTP mode."
      );
      rl.close();
      process.exit(1);
    }
    console.log("\nHTTPS mode: Caddy will obtain a Let's Encrypt cert once DNS propagates.");
  } else {
    console.log(
      "\nHTTP mode: Caddy will serve on port 80. Updates are still signed (Ed25519) so integrity is protected."
    );
  }

  const updateUrl = hasDomain ? `https://${domain}` : `http://${host}`;

  // Step 4: Test connection
  const connected = testSshConnection(host, user, port, deployKeyPath);
  if (!connected) {
    console.error(
      "\nERROR: Could not connect to " +
        user +
        "@" +
        host +
        ":" +
        port +
        "\n" +
        "Check that:\n" +
        "  - The VPS is running and the IP is correct\n" +
        "  - The deploy key was added before the Droplet was created\n" +
        "  - Port " +
        port +
        " is open (check firewall rules)\n" +
        "  - The SSH user is correct (try 'root' for fresh Droplets)"
    );
    rl.close();
    process.exit(1);
  }
  console.log("SSH connection: OK");

  // Step 5: Remote Caddy install
  const ok = runRemoteSetup(
    host,
    user,
    port,
    deployKeyPath,
    remotePath,
    domain,
    hasDomain
  );
  if (!ok) {
    console.error("\nERROR: Remote setup failed. Check the output above for details.");
    rl.close();
    process.exit(1);
  }

  // Step 6: Save deploy config
  // update_url is the canonical URL deploy.js and the launcher should use
  const config = {
    host,
    user,
    port,
    remote_path: remotePath,
    domain,
    update_url: updateUrl,
  };
  fs.writeFileSync(deployConfigPath, JSON.stringify(config, null, 2), "utf8");
  console.log("\nDeploy config saved to server-data/deploy.json");

  // Step 7: Offer to update brand.json updateUrl
  console.log("\n========================================");
  console.log("   STEP 3: Update launcher config");
  console.log("========================================");
  console.log("\nYour launcher's updateUrl in branding/brand.json should point to:");
  console.log("  " + updateUrl);

  if (fs.existsSync(brandPath)) {
    const brand = JSON.parse(fs.readFileSync(brandPath, "utf8"));
    const currentUrl = brand.updateUrl || "(not set)";
    console.log("\nCurrent updateUrl: " + currentUrl);
    const answer = (await rl.question("Update it now? (Y/n): ")).trim().toLowerCase();
    if (answer !== "n") {
      brand.updateUrl = updateUrl;
      fs.writeFileSync(brandPath, JSON.stringify(brand, null, 2), "utf8");
      console.log("brand.json updated. Rebuild the launcher (Option 7) for this to take effect.");
    }
  }

  rl.close();

  console.log("\n========================================");
  console.log("   VPS Setup Complete!");
  console.log("========================================");
  console.log("\nNext steps:");
  console.log("  1. Run Option E to publish your game files");
  console.log("  2. Run Option I to deploy them to your VPS");
  if (hasDomain) {
    console.log("  3. Point your domain's DNS A record to: " + host);
    console.log("  4. Caddy will get an HTTPS cert automatically once DNS propagates");
  } else {
    console.log(
      "  3. Your server is HTTP-only (no domain). Updates are Ed25519-signed so integrity is protected."
    );
    console.log("     To upgrade to HTTPS later, point a domain at this IP and re-run Option H.");
  }
  console.log("\nYour update server will be at: " + updateUrl);
}

main().catch((err) => {
  console.error("\nSetup failed:", err.message);
  if (rl) rl.close();
  process.exit(1);
});
