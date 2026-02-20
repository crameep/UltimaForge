# 🚀 UltimaForge Quick Start

## One-Click Development

**Double-click this file:**
```
ultimaforge.bat
```

Or run from command prompt:
```batch
ultimaforge.bat
```

---

## 🎯 What You'll See

A menu with options:

```
========================================
   UltimaForge Development Tool
========================================

What would you like to do?

 [1] Quick Start (Sync + Server + Launcher)
 [2] Sync Branding Only
 [3] Install Dependencies (npm install)
 [4] Generate Test Manifest
 [5] Start Test Server Only
 [6] Start Launcher Only
 [7] Build Production
 [8] Clean Everything
 [9] Run All Tests
 [C] Publish Launcher Update Metadata
 [D] Server Owner Wizard (branding + keys)
 [E] Publish All (game + launcher)
 [F] Dev All-in-One (server + launcher)
 [0] Exit

========================================

Enter your choice (0-9):
```

---

## ⚡ First Time Setup

**Just type `1` and press Enter!**

This will:
1. ✅ Sync branding images
2. ✅ Install npm dependencies (if needed)
3. ✅ Generate test manifest (if needed)
4. ✅ Start test server in new window
5. ✅ Start launcher in new window

**That's it!** Everything runs automatically.

---

## 📖 Menu Options Explained

### [1] Quick Start ⭐ **RECOMMENDED**
- Does everything you need to start developing
- Opens 2 windows: Server + Launcher
- Use this every time you want to start working

### [2] Sync Branding Only
- Copies images from `branding/` to `public/branding/`
- Use after editing logo or background images

### [3] Install Dependencies
- Runs `npm install`
- Use if you get "module not found" errors

### [4] Generate Test Manifest
- Creates test update files
- Use if test-updates folder is missing

### [5] Start Test Server Only
- Runs host server on port 8080
- Use if you only need the server

### [6] Start Launcher Only
- Runs Tauri dev mode
- Use if you only need the launcher

### [7] Build Production
- Creates distributable .exe file
- Output: `src-tauri\target\release\ultimaforge.exe`

### [8] Clean Everything
- Deletes node_modules, target, build files
- Use when things break

### [9] Run All Tests
- Runs Rust and npm tests
- Use to verify everything works

### [C] Publish Launcher Update Metadata
- Generates launcher update metadata + copies launcher binary to `updates/launcher`
- Use after building the launcher to enable self-updates

### [D] Server Owner Wizard
- Guided setup for `branding/brand.json` and key generation

### [E] Publish All (Game + Launcher)
- Publishes game files and launcher update metadata in one flow

### [F] Dev All-in-One
- Starts host server and launcher in a single terminal
- Generates test updates automatically if missing

### [H] Setup VPS ⭐ **First-time only**
- Generates an SSH deploy keypair
- Guides you through VPS + domain setup
- Installs Caddy (automatic HTTPS) on your VPS

### [I] Deploy to VPS
- Syncs `server-data/publish/` to your VPS
- Run after Option E to push updates live

### [0] Exit
- Closes the menu

---

## 🎨 Changing Branding

1. **Edit your images:**
   ```
   branding/
   ├── hero-bg.png       ← Edit this
   └── sidebar-logo.png  ← Edit this
   ```

2. **Run ultimaforge.bat** and choose `[2]` to sync

3. **Restart launcher** to see changes

---

## 🐛 Troubleshooting

### Server won't start
- **Cause**: Port 8080 in use
- **Fix**: Close other apps using port 8080, or change port in `branding/brand.json`

### Launcher won't connect
- **Cause**: Server not running
- **Fix**: Make sure server window is open and running

### Build fails
- **Cause**: Corrupted dependencies
- **Fix**: Choose option `[8]` to clean, then `[1]` to rebuild

### Images not showing
- **Cause**: Not synced to public folder
- **Fix**: Choose option `[2]` to sync branding

---

## 📁 File Locations

**Branding source** (edit here):
```
branding/
├── brand.json
├── hero-bg.png
└── sidebar-logo.png
```

**Dev copy** (auto-generated):
```
public/branding/
├── hero-bg.png
└── sidebar-logo.png
```

**Build output**:
```
src-tauri/target/release/
├── ultimaforge.exe       ← Distributable
└── bundle/msi/*.msi      ← Installer
```

**Server publish output** (deploy this to your VPS):
```
server-data/publish/
├── manifest.json
├── manifest.sig
├── files/          ← content-addressed game file blobs
└── launcher/       ← launcher update metadata
```

**Game client source** (drop your ClassicUO files here):
```
server-data/client/
├── ClassicUO.exe
├── Files/
├── Data/
└── ...
```

---

## 🎯 Common Workflows

### Daily Development
```
1. Double-click ultimaforge.bat
2. Press 1
3. Wait for windows to open
4. Start coding!
```

### One-Terminal Dev
```
npm run dev:all
```

### Update Branding
```
1. Edit branding/*.png
2. Run ultimaforge.bat
3. Press 2 (sync)
4. Restart launcher
```

### Build Release
```
1. Update branding/brand.json (production URLs)
2. Run ultimaforge.bat
3. Press 7 (build)
4. Get .exe from src-tauri/target/release/
```

### Publish Game Update (Production)
```
1. Drop new game files into server-data/client/ (overwrite old ones)
2. Run ultimaforge.bat → Option E (Publish)
3. Run ultimaforge.bat → Option I (Deploy to VPS)
```

### First-Time VPS Setup
```
1. Get a VPS (Digital Ocean, Hetzner, etc.) with Ubuntu 22.04
2. Run ultimaforge.bat → Option H (Setup VPS)
3. Follow the guided prompts - the wizard installs Caddy and configures HTTPS
4. Point your domain's A record to the VPS IP
```

---

## ✨ Tips

- **Keep server window open** - Launcher needs it running
- **Hot reload works** for React changes (no restart needed)
- **Rust changes** require launcher restart
- **First build** takes 10-15 minutes (compiling dependencies)
- **Subsequent builds** are much faster (1-5 minutes)

---

## 🆘 Still Having Issues?

1. Try option `[8]` (Clean Everything)
2. Then option `[1]` (Quick Start)
3. If still broken, check:
   - Node.js installed? (`node --version`)
   - Rust installed? (`cargo --version`)
   - npm installed? (`npm --version`)

---

**Ready?** Just double-click `ultimaforge.bat` and press `1`! 🚀
