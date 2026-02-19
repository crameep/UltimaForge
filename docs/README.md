# UltimaForge

**The easiest way to distribute your Ultima Online private server to players.**

UltimaForge creates a professional, branded launcher for your Ultima Online server. Players download a single file, and everything just works—installation, updates, and launching the game.

> **Ready to start?** Jump to the [Quick Start Guide](QUICKSTART.md) and build your launcher in under 30 minutes.

---

## What is UltimaForge?

UltimaForge is a tool that lets you create your own custom game launcher for your Ultima Online private server. Instead of giving players a zip file and a list of instructions, you give them a single download that handles everything automatically.

**You build it once. Your players download it. It stays updated.**

Think of it like creating your own Steam-style launcher, but specifically designed for UO servers—and you maintain full control over it.

## Why Use UltimaForge?

### For You (The Server Owner)

| Challenge Without UltimaForge | How UltimaForge Solves It |
|-------------------------------|---------------------------|
| Players get stuck during setup | One-click installation with directory picker |
| Players run outdated clients | Automatic updates when you publish new files |
| Supporting multiple players with different issues | Everyone gets the same, working installation |
| Worrying about tampered files | Cryptographic signatures verify every download |
| Updating means re-uploading everything | Only changed files are downloaded (differential updates) |

### For Your Players

- **Simple setup**: Download the launcher, pick a folder, click Install
- **Always current**: The launcher quietly updates game files when needed
- **One click to play**: No hunting for the right executable
- **No manual patching**: Updates happen automatically before launch

## How It Works

### The Server Owner's Workflow

1. **Build your launcher** (one time)
   - Run our setup script to install tools
   - Add your server name and logo
   - Build an executable file (.exe, .app, etc.)

2. **Publish your game files** (whenever you update)
   - Put your UO client files in a folder
   - Run a command to generate signed update files
   - Upload to your web server or hosting

3. **Share with players**
   - Give players a link to download your launcher
   - That's it—they're ready to play

### What Happens When a Player Launches

```
Player opens your launcher
        ↓
Launcher checks your update server
        ↓
If first run → installs all files
If update available → downloads only changed files
        ↓
Player clicks "Play"
        ↓
Game launches with correct settings
```

### Security Built In

Every update you publish is **signed with your private key**. The launcher only accepts files that match your signature. This means:

- Players can't accidentally install corrupted files
- No one can trick players into downloading fake updates
- You control exactly what your players receive

## Getting Started

Ready to build your launcher? See:

- **[Root README.md](../README.md)** - Quick Start guide using `ultimaforge.bat`
- **[SETUP.md](SETUP.md)** - Detailed environment setup and configuration
- **[QUICKSTART.md](../QUICKSTART.md)** - Development workflow with ultimaforge.bat

## What Your Players Experience

When players run your launcher for the first time:

1. **Welcome screen** with your server branding
2. **Choose installation folder** (or accept the default)
3. **Automatic download** with progress indicator
4. **Ready to play** when installation completes

On subsequent launches:

1. Open the launcher
2. Launcher checks for updates (takes a few seconds)
3. If updates exist, downloads only what changed
4. Click "Play" to launch the game

Players never need to manually patch, find files, or troubleshoot client versions.

## Documentation

| Guide | What It Covers |
|-------|----------------|
| [Quick Start](QUICKSTART.md) | Build your launcher in 5 steps (~15-30 min) |
| [Full Setup Guide](SETUP.md) | Detailed setup with troubleshooting |
| [Publishing Updates](PUBLISHING.md) | How to push updates to your players |
| [Branding Options](../branding-template/README.md) | Customize colors, logos, and more |

## Common Questions

### Do I need to know how to code?

No! If you can edit a text file and run commands in a terminal, you can use UltimaForge. Our setup scripts handle the technical installation, and you just need to configure your server details.

### What files do my players download?

Just one file—your branded launcher. The launcher is typically 5-15 MB. After that, the launcher downloads the actual game files from your update server.

### How do I push updates to players?

When you update your UO client files:

1. Run the `publish` command on your new files
2. Upload the results to your update server
3. Players automatically get the update next time they launch

See [PUBLISHING.md](PUBLISHING.md) for the full workflow.

### What if I need to change my security keys?

If you rotate to new keys, you'll need to build and distribute a new launcher. Players will need to download the new version. For this reason, keep your private key safe!

### Can I customize the launcher's appearance?

Yes! You can customize:
- Server name and description
- Window title
- Color scheme (primary, secondary, background, text colors)
- Support links (website, Discord, email)

See [branding-template/README.md](../branding-template/README.md) for all options.

### What platforms are supported?

UltimaForge builds launchers for:
- **Windows** (.exe, .msi installer)
- **macOS** (.app, .dmg)
- **Linux** (binary, .deb, .AppImage)

## Getting Help

- **Setup issues?** Run `npm run validate-env` to check your environment
- **Build problems?** Check [SETUP.md](SETUP.md) troubleshooting section
- **Publishing questions?** See [PUBLISHING.md](PUBLISHING.md)
- **Still stuck?** Open an issue on GitHub

---

**Ready to get started?** Head to the [Quick Start Guide](QUICKSTART.md) and you'll have your own launcher built in no time.
