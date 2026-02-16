# 🚀 Batch File Guide

Windows batch files to simplify UltimaForge development.

## 📋 Quick Start

**Easiest way to start developing:**

```batch
start-dev.bat
```

This single command will:
1. ✅ Sync branding assets
2. ✅ Generate test manifest (if needed)
3. ✅ Start host server (new window)
4. ✅ Start launcher dev mode (new window)

---

## 📁 Available Batch Files

### 🎯 Main Commands

| File | Purpose | When to Use |
|------|---------|-------------|
| **start-dev.bat** | Start everything | 🌟 **Start here!** Opens server + launcher |
| **build.bat** | Build production launcher | When ready to distribute |
| **sync-branding.bat** | Sync branding assets | After changing images in `branding/` |
| **clean.bat** | Clean build artifacts | When things break or for fresh start |

### 🔧 Individual Components

| File | Purpose |
|------|---------|
| **dev-server.bat** | Start only the host server |
| **dev-launcher.bat** | Start only the launcher |

---

## 📖 Usage Examples

### Start Development Environment

The easiest way - starts everything you need:

```batch
start-dev.bat
```

**What happens:**
1. Syncs branding images to `public/branding/`
2. Generates test manifest (if missing)
3. Opens **Window 1**: Host server on port 8080
4. Opens **Window 2**: Tauri launcher in dev mode

**When to use:** Every time you want to start developing!

---

### Update Branding Images

After changing images in the `branding/` folder:

```batch
sync-branding.bat
```

**What it does:**
- Copies `branding/*.png` → `public/branding/`
- Copies `branding/*.jpg` → `public/branding/`

**Then restart dev launcher** to see changes.

---

### Build Production Launcher

When you're ready to distribute:

```batch
build.bat
```

**What it does:**
1. Syncs branding assets
2. Installs npm dependencies
3. Builds React frontend
4. Builds Tauri application

**Output location:**
- `src-tauri\target\release\ultimaforge.exe`
- `src-tauri\target\release\bundle\msi\*.msi`

---

### Clean Everything

When you want a fresh start:

```batch
clean.bat
```

**Removes:**
- `node_modules/`
- `target/`
- `dist/`
- `Cargo.lock`
- `package-lock.json`

**Use when:**
- Build errors that won't go away
- Switching branches
- Want to free up disk space

---

## 🎮 Common Workflows

### 🌅 Starting Your Day

```batch
# Start everything
start-dev.bat

# Wait for both windows to open
# The launcher window will auto-open when ready
```

### 🎨 Changing Branding

```batch
# 1. Edit images in branding/ folder
# (Use any image editor)

# 2. Sync to public folder
sync-branding.bat

# 3. Restart launcher (Ctrl+C in launcher window, then re-run)
```

### 📦 Building for Release

```batch
# 1. Update branding/brand.json with production settings
# 2. Generate production keys (if not done)
# 3. Build
build.bat

# 4. Test the built executable
cd src-tauri\target\release
ultimaforge.exe
```

### 🧹 Troubleshooting

```batch
# If build fails or things are broken:

# 1. Clean everything
clean.bat

# 2. Start fresh
start-dev.bat
```

---

## ⚙️ Manual Control

### Run Components Separately

If you prefer more control:

**Terminal 1 - Server:**
```batch
dev-server.bat
```

**Terminal 2 - Launcher:**
```batch
dev-launcher.bat
```

---

## 🎯 Workflow Chart

```
Daily Development:
    start-dev.bat
    ↓
    Two windows open
    ↓
    Make changes to code
    ↓
    Hot reload (automatic)

Changing Branding:
    Edit branding/*.png
    ↓
    sync-branding.bat
    ↓
    Restart launcher

Building Release:
    Update brand.json
    ↓
    build.bat
    ↓
    Test .exe in target/release/

Something Broken:
    clean.bat
    ↓
    start-dev.bat
```

---

## 📝 Notes

### Port in Use Error

If you see "Address already in use" error:

1. **Find what's using port 8080:**
   ```batch
   netstat -ano | findstr :8080
   ```

2. **Kill the process:**
   ```batch
   taskkill /PID <process_id> /F
   ```

3. **Or use different port:**
   Edit `dev-server.bat` and change `--port 8080` to `--port 8081`
   Then update `branding/brand.json` to match.

### Hot Reload

- **Frontend changes (React)**: Auto-reload ✅
- **Rust changes**: Requires launcher restart ⚠️
- **brand.json changes**: Requires launcher restart ⚠️
- **Image changes**: Run `sync-branding.bat` + restart ⚠️

### Build Times

- **First build**: 5-15 minutes (compiling Rust dependencies)
- **Subsequent builds**: 1-5 minutes (incremental)
- **Production build**: 10-20 minutes (optimized)

---

## 🆘 Quick Troubleshooting

| Problem | Solution |
|---------|----------|
| Server won't start | Check if port 8080 is in use |
| Launcher won't connect | Make sure server is running first |
| Images not showing | Run `sync-branding.bat` |
| Build fails | Try `clean.bat` then rebuild |
| "Command not found" | Make sure you're in the project root |

---

## 🎉 You're Ready!

Just run:
```batch
start-dev.bat
```

And start building! 🚀
