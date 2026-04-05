# UltimaForge

**A self-hosted, secure, and brandable game launcher/patcher for Ultima Online private servers.**

UltimaForge gives server owners a professional launcher that handles client installation, cryptographically-signed updates, and game launching. Built with Rust + React (Tauri), it compiles into a single branded executable that players download and run.

---

## Features

- **Secure Updates** - Ed25519 signed manifests + SHA-256 per-file verification
- **Atomic Updates** - Staged apply with automatic rollback on failure
- **Full Branding** - Colors, logos, window title, hero text, sidebar links, installer graphics
- **Self-Updating Launcher** - The launcher itself updates via Tauri's built-in updater
- **Migration Detection** - Auto-detects existing UO installations and offers copy/adopt/skip
- **ClassicUO Integration** - Auto-configures CUO settings, server selection, and assistant choice
- **Multi-Client** - Launch up to 3 game clients simultaneously
- **Auto-Elevation** - Detects Program Files installs and prompts to relaunch as admin
- **Resume Support** - Interrupted downloads resume where they left off
- **First-Run Wizard** - Guided installation for players
- **VPS Deployment** - Built-in rsync-based deploy to your hosting server
- **Patch Notes** - Displays server patch notes from a configurable URL

---

## Complete Server Owner Guide

This walks you through every step from zero to distributing your branded launcher to players. No programming experience required.

### What You'll Need

- **A Windows 10 or 11 PC** (for building the launcher)
- **Your UO game files** (the client folder you want players to have)
- **A VPS** to host updates (optional but recommended — Digital Ocean, Vultr, Linode, etc.)
- **A domain name** pointed at your VPS (optional — enables HTTPS)
- About **30-60 minutes** for first-time setup

### Overview

The batch file `ultimaforge.bat` is your control panel. Everything goes through it. When you run it, you'll see:

```
  FIRST TIME SETUP

  [1] Install Prerequisites          [...]
  [2] Configure Branding & Keys      [...]
  [3] Generate App Icons             [...]
  [4] Build Launcher                 [...]
  [5] Setup VPS (optional)           [...]

  ONGOING

  [6] Publish Game Update
  [7] Deploy to VPS
  [8] Update Launcher Source

  [D] Developer Tools
  [X] Exit
```

Each step shows `[DONE]` when complete or `[...]` when not yet done. Work through 1-4 in order for your first build.

---

### Step 0: Get the Code

**Option A — With Git (recommended):**

Open a terminal (Command Prompt or PowerShell) and run:

```bash
git clone https://github.com/crameep/UltimaForge.git
cd UltimaForge
ultimaforge.bat
```

**Option B — Download as ZIP:**

1. Go to https://github.com/crameep/UltimaForge
2. Click the green **Code** button, then **Download ZIP**
3. Extract the ZIP somewhere (e.g., `C:\UltimaForge`)
4. Double-click `ultimaforge.bat`

The batch file will auto-initialize a Git repository if you used the ZIP method. This is needed for the source update feature (option 8) to work later.

---

### Step 1: Install Prerequisites

Select **[1]** from the menu.

This installs everything you need to build the launcher:

| Tool | What it does |
|------|-------------|
| **Git** | Tracks code changes, enables source updates |
| **Rust** | The programming language the launcher is built in |
| **Node.js** | Runs the build scripts and frontend tooling |
| **VS Build Tools** | Compiler needed by Rust on Windows |
| **Tauri CLI** | Packages the app into an installer |
| **rsync** | Efficiently syncs files to your VPS (optional) |

The script will ask to relaunch as Administrator if needed (VS Build Tools requires admin to install). **Say yes.**

This step only needs to run once. After it completes, the menu will show `[1] Install Prerequisites [DONE]`.

> **Troubleshooting:** If something fails to install, close the batch file, reopen it, and run option 1 again. It skips things that are already installed.

---

### Step 2: Configure Branding & Keys

Select **[2]** from the menu.

This runs two wizards back-to-back:

#### Part 1: Server Owner Wizard

An interactive wizard that asks you for:

- **Server name** — e.g., "Unchained UO" (shown in the launcher title bar and sidebar)
- **Description** — a short tagline
- **Website / Discord / Support email** — shown in the launcher
- **Colors** — primary, secondary, background, text (hex codes like `#1a1a2e`)
- **Update URL** — where your launcher will check for game updates (set this later if you don't have a VPS yet)
- **CUO settings** — ClassicUO client version, server IP/port, assistant choice

The wizard generates `branding/brand.json`. You can also edit this file by hand — see `branding/brand.example.json` for every available option.

#### Part 2: Key Generation

Two sets of keys are generated automatically:

1. **Ed25519 signing keys** (`keys/private.key` and `keys/public.key`) — used to sign game update manifests so players can verify they haven't been tampered with
2. **Tauri updater keys** (`keys/tauri-updater/`) — used to sign launcher self-updates

> **IMPORTANT:** Keep your `keys/` folder safe. If you lose your private keys, you'll need to rebuild the launcher from scratch with new keys. Never share your private keys.

#### Adding Your Images

Before building, drop your images into `branding/`:

| File | Requirements | Used for |
|------|-------------|----------|
| `sidebar-logo.png` | Square PNG, 1024x1024 recommended, transparent background | Sidebar logo + app icon |
| `hero-bg.png` | Any size, landscape orientation | Main content background |
| `sidebar-texture.png` | Optional, tileable | Sidebar background texture |

---

### Step 3: Generate App Icons

Select **[3]** from the menu.

This takes your `branding/sidebar-logo.png` and generates all the icon sizes needed for the Windows installer, taskbar, and desktop shortcut. If you don't have ImageMagick installed, it will use a built-in converter.

---

### Step 4: Build Launcher

Select **[4]** from the menu.

This compiles everything into a Windows installer (`.exe`). The build takes a few minutes the first time (Rust compiles all dependencies). Subsequent builds are faster.

When it finishes, your installer is at:

```
app\src-tauri\target\release\bundle\nsis\YourServerName_x.y.z_x64-setup.exe
```

This is the file you give to your players. They run it, pick an install folder, and the launcher handles the rest.

> **Note:** `brand.json` is baked into the executable at build time. If you change branding later, you need to rebuild (option 4).

---

### Step 5: Setup VPS (Hosting Your Updates)

Select **[5]** from the menu.

This is where players will download game files and updates from. You need a server on the internet that's always on. If you're just testing locally, you can skip this and come back later.

#### What is a VPS?

A VPS (Virtual Private Server) is a small server you rent from a hosting company. It costs about $4-6/month. Popular providers:

- **[Digital Ocean](https://www.digitalocean.com/)** — simple, $4/mo for basic Droplet
- **[Vultr](https://www.vultr.com/)** — similar pricing
- **[Linode](https://www.linode.com/)** — same tier
- **[Hetzner](https://www.hetzner.com/)** — cheapest option in Europe

You don't need a powerful server. The cheapest tier (1 CPU, 512MB-1GB RAM) is more than enough — it's just serving files.

#### Creating Your VPS (Digital Ocean Example)

1. **Create an account** at digitalocean.com
2. **Create a Droplet:**
   - Click **Create** > **Droplets**
   - Choose **Ubuntu 22.04 LTS** (or 24.04)
   - Pick the **cheapest plan** ($4-6/mo)
   - Choose a **region** close to your players
   - Under **Authentication**, select **SSH Key** (the setup wizard will give you the key — see below)
   - Click **Create Droplet**
3. **Copy the IP address** shown on the Droplets page (e.g., `143.198.42.100`)

#### Running the VPS Setup Wizard

When you select option **[5]**, the wizard walks you through everything:

**1. SSH Deploy Key**

The wizard generates an SSH key pair for secure access to your VPS. It shows your public key:

```
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA... ultimaforge-deploy
```

Copy this key and add it to your VPS:
- **Digital Ocean:** Paste it when creating the Droplet (Authentication > SSH Key > New SSH Key)
- **Other providers:** SSH into the server with your password and run:
  ```bash
  echo 'ssh-ed25519 AAAAC3...' >> ~/.ssh/authorized_keys
  ```

**2. Enter VPS Details**

The wizard asks for:

| Prompt | What to enter | Example |
|--------|--------------|---------|
| VPS IP address | The IP from your hosting provider | `143.198.42.100` |
| SSH user | Usually `root` for fresh servers | `root` |
| SSH port | Usually `22` (the default) | `22` |
| Remote path | Where files are served from | `/var/www/ultimaforge` |
| Domain name | Optional — enables HTTPS | `updates.myserver.com` |

**3. Automatic Server Setup**

The wizard SSHes into your VPS and automatically:

- Updates the system packages
- Installs **Caddy** (a web server that handles HTTPS automatically)
- Opens ports 80 and 443 in the firewall
- Creates the file serving directory
- Writes the Caddy configuration

This takes about 1-2 minutes.

**4. Domain & HTTPS (Optional but Recommended)**

If you have a domain name:

1. Go to your domain registrar (Namecheap, Cloudflare, GoDaddy, etc.)
2. Add an **A record** pointing to your VPS IP:
   - **Host/Name:** `updates` (or whatever subdomain you chose)
   - **Value/Points to:** your VPS IP (e.g., `143.198.42.100`)
   - **TTL:** automatic or 300
3. Wait 5-30 minutes for DNS to propagate
4. Caddy will automatically obtain an HTTPS certificate from Let's Encrypt

If you don't have a domain, the wizard sets up HTTP mode. Your updates are still secure because every file is cryptographically signed — HTTPS just adds an extra layer of transport encryption.

**5. updateUrl**

The wizard offers to update your `branding/brand.json` with the correct `updateUrl`. Say yes. This is the URL your launcher will check for updates:

- With domain: `https://updates.myserver.com`
- Without domain: `http://143.198.42.100`

> **Remember:** After changing `brand.json`, rebuild the launcher (option 4) for the change to take effect.

#### What Gets Saved

The wizard saves your VPS connection info to `server-data/deploy.json`. This file is used by option 7 (Deploy to VPS) so you don't have to re-enter details every time.

---

### Step 6: Publish Game Updates

Select **[6]** from the menu.

This is what you run whenever you update your game files (new maps, art, scripts, etc.). It gives you three choices:

```
  [1] Full (game + launcher) - default
  [2] Game only (fast, skips launcher build)
  [3] Launcher only (fast, skips game files)
```

**How it works:**

1. Scans your game client folder (configured in `brand.json`)
2. Hashes every file with SHA-256
3. Generates `manifest.json` listing all files and their hashes
4. Signs the manifest with your Ed25519 private key
5. Stores changed files as content-addressed blobs (only uploads what changed)
6. Auto-bumps the version number (patch increment)

The output goes to `server-data/publish/`. This is what gets deployed to your VPS.

**For most updates, just press Enter** (option 1, Full) and let it run. Use option 2 (Game only) when you've only changed game files and don't want to wait for a launcher rebuild.

---

### Step 7: Deploy to VPS

Select **[7]** from the menu.

This syncs your published files to your VPS using rsync (or scp as fallback). It:

1. Verifies SSH connection to your VPS
2. Syncs only the files that changed (fast for small updates)
3. Confirms deployment

After this, any player who opens the launcher will see the update and be prompted to download it.

> **First deploy:** If this is the first time deploying from this machine and your deploy key isn't on the server yet, the script will detect this and offer to install the key automatically. You'll be asked for your server password once. After that, all future deploys are password-free.

---

### Step 8: Update Launcher Source

Select **[8]** from the menu.

This pulls the latest UltimaForge code from GitHub. Your branding, keys, and server config are preserved — only the launcher source code is updated.

After updating source, rebuild the launcher (option 4) and optionally publish a launcher update (option 6) so existing players get the new version.

You can undo a source update by pressing **[D]** for Developer Tools and selecting the undo option.

---

## Ongoing Workflow

Once you've done the first-time setup (steps 1-5), your regular workflow is:

1. Update your game files on disk
2. Run `ultimaforge.bat`
3. Press **6** (Publish) > **Enter** (Full)
4. Press **7** (Deploy to VPS)

That's it. Players get the update next time they open the launcher.

---

## For Players

Download and run the installer provided by your server. The launcher will:

1. Guide you through picking an install folder (or detect an existing installation)
2. Download and verify all game files
3. Keep everything up-to-date automatically
4. Launch the game with one click (supports up to 3 clients at once)

If your game is installed in Program Files, the launcher will remind you to run as Administrator so updates can write to that folder.

---

## Branding Reference

All customization lives in `branding/`. See `branding/brand.example.json` for the full schema.

### brand.json Structure

```jsonc
{
  "product": {
    "displayName": "My UO Server",     // Shown in title bar and sidebar
    "serverName": "MyServer",           // Internal name (no spaces)
    "description": "Your description",
    "supportEmail": "support@example.com",
    "website": "https://example.com",
    "discord": "https://discord.gg/example"
  },
  "updateUrl": "https://updates.myserver.com",  // Where the launcher checks for updates
  "publicKey": "YOUR_PUBLIC_KEY_HEX_64_CHARS",  // Generated by option 2
  "ui": {
    "colors": {
      "primary": "#1a1a2e",
      "secondary": "#e94560",
      "background": "#16213e",
      "text": "#eaeaea"
    },
    "backgroundImage": "/branding/hero-bg.png",
    "logoUrl": "/branding/sidebar-logo.png",
    "sidebarBackground": "/branding/sidebar-texture.png",
    "windowTitle": "My UO Server",
    "heroTitle": "Welcome to My UO Server",
    "heroSubtitle": "Your adventure begins here",
    "sidebarSubtitle": "Launcher",
    "showPatchNotes": true,
    "sidebarLinks": [
      { "label": "Home", "icon": "🏠" },
      { "label": "Settings", "icon": "⚙️" },
      { "label": "Discord", "icon": "💬", "url": "https://discord.gg/example" }
    ]
  },
  "cuo": {                              // ClassicUO integration (optional)
    "client_version": "7.0.10.3",
    "live_server": { "label": "My Server", "ip": "your.server.ip", "port": 2593 },
    "test_server": { "label": "Test Center", "ip": "your.server.ip", "port": 2594 },
    "available_assistants": ["razor_enhanced", "razor"],
    "default_assistant": "razor",
    "default_server": "live"
  },
  "migration": {                        // Auto-detect existing installs (optional)
    "searchPaths": ["C:\\Program Files\\MyServer", "C:\\Games\\UO"]
  }
}
```

### Image Files

| File | Requirements | Used for |
|------|-------------|----------|
| `sidebar-logo.png` | Square PNG, 1024x1024 recommended, transparent background | Sidebar logo + app icons |
| `hero-bg.png` | Any size, landscape orientation works best | Main content background |
| `sidebar-texture.png` | Optional, tileable pattern | Sidebar background |

---

## Architecture

```
ultimaforge/
├── ultimaforge.bat              # Server owner tool (run this)
├── branding/                    # Your branding (edit this)
│   ├── brand.json               # Server config, colors, URLs
│   ├── brand.example.json       # Full schema reference
│   └── *.png                    # Logo, background, sidebar texture
├── keys/                        # Generated keys (keep private keys safe!)
│   ├── private.key / public.key # Ed25519 game update signing keys
│   └── tauri-updater/           # Tauri launcher self-update keys
├── server-data/                 # VPS config and publish output
│   ├── deploy.json              # VPS connection info (generated by option 5)
│   ├── keys/deploy-key          # SSH deploy keypair
│   └── publish/                 # Published update artifacts
├── app/
│   ├── src/                     # React/TypeScript frontend
│   ├── src-tauri/src/           # Rust backend
│   ├── tools/
│   │   ├── host-server/         # Axum HTTP server for hosting updates
│   │   └── publish-cli/         # CLI to hash, sign, and package updates
│   └── scripts/                 # Node.js build/setup helpers
└── docs/                        # Additional documentation
```

### How Updates Work

1. **Publish** (option 6): `publish-cli` scans your game folder, hashes every file, generates a signed `manifest.json`, and stores changed files as blobs named by their SHA-256 hash.
2. **Deploy** (option 7): rsync uploads only the changed blobs + manifest to your VPS.
3. **Patch** (player side): The launcher fetches the manifest, verifies the Ed25519 signature, compares hashes against local files, downloads only what changed, and applies everything atomically (all-or-nothing with rollback on failure).

---

## Security

1. **Ed25519 Signatures** - Manifests are signed with your private key; the public key is embedded at compile time in the launcher binary. Nobody can push a fake update without your private key.
2. **SHA-256 Hashing** - Every file is verified by hash after download. If even one byte is wrong, it's rejected.
3. **Path Traversal Protection** - Manifest paths are validated against directory escapes (`../`), UNC paths, and Windows device names.
4. **Atomic Updates** - Files are staged to a temp directory, verified, then moved into place. If anything fails, the update is rolled back.
5. **Auto-Elevation** - The launcher detects when it's installed in a protected folder (like Program Files) and prompts the user to relaunch as Administrator.

---

## Troubleshooting

### "Option 1 fails to install something"

Close the batch file, reopen it, and run option 1 again. It skips things already installed. If VS Build Tools fails, make sure you're running as Administrator (the script will ask).

### "Build fails after changing brand.json"

Run option 3 (Generate Icons) before option 4 (Build). If you changed colors or text only (not images), you can skip straight to option 4.

### "Players can't connect to the update server"

1. Make sure you ran option 7 (Deploy) after option 6 (Publish)
2. Check that your VPS is running: `ssh root@YOUR_IP "curl -s http://localhost/manifest.json | head -1"`
3. If using a domain, verify DNS: `nslookup updates.yourdomain.com` should show your VPS IP
4. Check the Caddy logs on your VPS: `ssh root@YOUR_IP "journalctl -u caddy --no-pager -n 50"`

### "Launcher says 'signature verification failed'"

Your `brand.json` publicKey doesn't match the private key used to sign the manifest. Make sure you're using the keys generated by option 2. If you regenerated keys, rebuild the launcher (option 4) and republish (option 6).

### "Players get 'access denied' errors during updates"

The game is probably installed in Program Files. The launcher will show a banner prompting the user to relaunch as admin. They can also right-click the launcher shortcut and select "Run as administrator."

---

## System Requirements

### For Building (Server Owner)

- Windows 10/11
- 4 GB RAM, 2 GB free disk space
- Internet connection (for downloading dependencies and deploying)

### For Players

- Windows 7+
- ~100 MB for the launcher
- 2-4 GB for game files (varies by server)

---

## Support

- **Docs**: [`docs/`](docs/) directory
- **Issues**: [GitHub Issues](https://github.com/crameep/UltimaForge/issues)

---

Built with [Tauri](https://tauri.app/), [React](https://react.dev/), [Rust](https://www.rust-lang.org/), and [ed25519-dalek](https://github.com/dalek-cryptography/curve25519-dalek).
