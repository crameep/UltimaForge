# UltimaForge

**A self-hosted, secure, and brandable game launcher/patcher for Ultima Online private servers.**

UltimaForge provides server owners with a professional, turnkey solution for distributing and updating their custom UO client. Built with Rust and React, it features cryptographic signature verification, atomic updates with rollback, and complete branding customization.

---

## ✨ Features

- 🔐 **Secure Updates** - Ed25519 signature verification prevents tampering
- ⚡ **Atomic Updates** - All-or-nothing updates with automatic rollback on failure
- 🎨 **Full Branding** - Customize colors, logos, product name, and installer graphics
- 📦 **Self-Contained** - Single executable with embedded branding
- 🔄 **Resume Support** - Interrupted downloads automatically resume
- 🎯 **First-Run Wizard** - Guided installation experience for players
- 🛡️ **Path Traversal Protection** - Security-hardened file handling
- 📊 **Progress Tracking** - Real-time download and installation progress

---

## 🚀 Quick Start

### For Server Owners

1. **Clone the repository:**
   ```bash
   git clone https://github.com/your-org/ultimaforge.git
   cd ultimaforge
   ```

2. **Install prerequisites (first-time only):**
   ```bash
   ultimaforge.bat
   # Select option 0: Install Prerequisites
   ```

3. **Configure your branding:**
   - Edit `branding/brand.json` with your server details
   - Add your logo to `branding/sidebar-logo.png`
   - Add background image to `branding/hero-bg.png`

4. **Build your installer:**
   ```bash
   ultimaforge.bat
   # Select option 7: Build Production Installer
   ```

Your branded installer will be in `app/src-tauri/target/release/bundle/nsis/`

**Optional shortcuts:**
- Run the **Server Owner Wizard** (option `D`) to generate `branding/brand.json`
- Use **Publish All** (option `E`) to publish game + launcher updates together

### For Players

Download and run the installer provided by your server administrator. The launcher will:
1. Guide you through selecting an installation directory
2. Download and verify all game files
3. Keep your client up-to-date automatically
4. Launch the game with a single click

---

## 📚 Documentation

- **[Setup Guide](docs/SETUP.md)** - Detailed environment setup and configuration
- **[Branding Guide](app/branding-template/README.md)** - Customizing your launcher
- **[Publishing Guide](docs/PUBLISHING.md)** - Hosting updates and generating manifests
- **[Security Guide](docs/SECURITY.md)** - Understanding the security model

---

## 🛠️ Development

### Prerequisites

- **Rust** 1.77.2 or later
- **Node.js** 18+ and npm
- **Windows Build Tools** (MSVC on Windows, build-essential on Linux)

### Development Workflow

```bash
# Start development server (launcher + host server)
ultimaforge.bat
# Select option 1: Start Development Environment

# Or run components separately:
ultimaforge.bat  # Select option 5 for host server
ultimaforge.bat  # Select option 6 for launcher only
```

### Project Structure

```
ultimaforge/
├── ultimaforge.bat          # "Run this" - main development tool
├── README.md                # "Read this" - getting started
├── branding/                # "Edit YOUR branding here"
│   ├── brand.json           # Server name, colors, URLs
│   ├── sidebar-logo.png     # Your server logo
│   └── hero-bg.png          # Background image
├── docs/                    # Reference documentation
└── app/                     # "Don't touch" - all technical internals
    ├── src/                 # React frontend (TypeScript)
    ├── src-tauri/           # Rust backend (Tauri)
    │   └── src/
    │       ├── commands/    # Tauri IPC commands
    │       ├── installer.rs # Installation logic
    │       ├── updater.rs   # Atomic update system
    │       ├── launcher.rs  # Game process spawning
    │       └── signature.rs # Ed25519 verification
    ├── public/              # Static assets
    ├── scripts/             # Build/setup scripts
    ├── branding-template/   # Template for new servers
    ├── package.json         # Node.js config
    └── Cargo.toml           # Rust config
```

**For Server Owners:** You only need to interact with `branding/` and `ultimaforge.bat`. All technical build files are in `app/`.

---

## 🔒 Security

UltimaForge employs multiple security layers:

1. **Ed25519 Signatures** - All manifests are cryptographically signed
2. **Hash Verification** - SHA-256 verification for every downloaded file
3. **Path Validation** - Protection against path traversal attacks
4. **Atomic Updates** - Rollback on verification failure
5. **Offline Public Key** - Public key embedded at compile-time

See [docs/SECURITY.md](docs/SECURITY.md) for details.

---

## 📋 System Requirements

### For Building

- Windows 10/11, macOS 10.15+, or Linux
- 4 GB RAM
- 2 GB disk space

### For Players

- Windows 7+ or compatible OS
- 100 MB free space (launcher)
- 2-4 GB free space (game files, varies by server)

---

## 🤝 Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## 📜 License

[Add your license here - MIT, GPL, etc.]

---

## 🆘 Support

- **Documentation**: [docs/](docs/)
- **Issues**: [GitHub Issues](https://github.com/your-org/ultimaforge/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/ultimaforge/discussions)

---

## 🙏 Acknowledgments

Built with:
- [Tauri](https://tauri.app/) - Desktop application framework
- [React](https://react.dev/) - Frontend UI
- [Rust](https://www.rust-lang.org/) - Backend logic
- [ed25519-dalek](https://github.com/dalek-cryptography/curve25519-dalek) - Cryptographic signatures

---

**Ready to launch your Ultima Online server with confidence!** 🚀
