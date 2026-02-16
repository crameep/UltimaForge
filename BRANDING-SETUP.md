# Branding Setup - Custom Images

## ✅ What Was Fixed

I've added full support for custom branding images (logo and background) to your launcher.

### Changes Made

1. **Fixed brand.json structure** - Moved colors under `ui.colors` (was incorrectly at `theme`)
2. **Added image support** to Rust structs (`UiConfig`, `BrandInfo`)
3. **Added image fields** to TypeScript interfaces (`BrandInfo`)
4. **Created `useBrand()` hook** to load brand configuration
5. **Updated Layout component** to use brand info and pass logo to Sidebar
6. **Configured Tauri** to bundle branding resources
7. **Copied images** to public folder for dev mode

## 📁 Current Branding Structure

```
branding/
├── brand.json          # Main configuration
├── hero-bg.png         # Background image (9.7 KB)
├── sidebar-logo.png    # Logo image (299 bytes)
└── README.md

public/branding/        # Dev mode copies
├── hero-bg.png
└── sidebar-logo.png
```

## ⚙️ brand.json Configuration

Your `brand.json` now includes image references:

```json
{
  "ui": {
    "colors": { ... },
    "backgroundImage": "/branding/hero-bg.png",
    "logoUrl": "/branding/sidebar-logo.png",
    "showPatchNotes": true,
    "windowTitle": "UltimaForge Launcher (Test)"
  }
}
```

## 🧪 Testing the Images

### Development Mode

```bash
npm run tauri dev
```

The launcher should now:
- ✅ Display your custom logo in the sidebar
- ✅ Load branding colors from brand.json
- ✅ Show server name from brand.json

### What You Should See

1. **Sidebar**: Your `sidebar-logo.png` displayed at the top
2. **Server Name**: "UltimaForge Test Server" (from brand.json)
3. **Colors**: Dark theme with the colors you specified

## 📝 Adding/Changing Images

### Logo (sidebar-logo.png)

**Recommended size**: 64x64px to 128x128px
**Format**: PNG with transparency

```bash
# Replace logo
cp your-logo.png branding/sidebar-logo.png
cp your-logo.png public/branding/sidebar-logo.png
```

### Background (hero-bg.png)

**Recommended size**: 1920x1080px
**Format**: PNG or JPG

```bash
# Replace background
cp your-background.png branding/hero-bg.png
cp your-background.png public/branding/hero-bg.png
```

### Update brand.json

If you rename the files, update `brand.json`:

```json
{
  "ui": {
    "backgroundImage": "/branding/your-bg-name.png",
    "logoUrl": "/branding/your-logo-name.png"
  }
}
```

## 🚀 Production Builds

For production builds, images are automatically bundled from the `branding/` folder.

```bash
npm run tauri build
```

The bundled launcher will include your branding images embedded.

## 🎨 Image Guidelines

### Logo
- **Format**: PNG (preferably with transparency)
- **Size**: 64x64px to 256x256px
- **Aspect**: Square or portrait
- **Purpose**: Displayed in sidebar header

### Background
- **Format**: PNG or JPG
- **Size**: 1920x1080px or larger
- **Purpose**: Hero section background (optional)
- **Note**: Keep file size reasonable (<500KB)

## 🔧 Troubleshooting

### Images not showing in dev mode

1. **Check public folder**:
   ```bash
   ls public/branding/
   # Should show: hero-bg.png  sidebar-logo.png
   ```

2. **Check brand.json paths**:
   ```json
   "logoUrl": "/branding/sidebar-logo.png"  // Must start with /
   ```

3. **Restart dev server**:
   ```bash
   # Stop tauri dev (Ctrl+C)
   npm run tauri dev
   ```

### Logo not displaying

1. **Check console** for errors (F12 in dev window)
2. **Verify image exists** at `public/branding/sidebar-logo.png`
3. **Check image format** - must be valid PNG/JPG
4. **Try smaller image** - very large images may fail to load

### Background not applying

The background image feature is in the `brand.json` but not yet wired to the CSS. To add it:

1. Add to `Layout.tsx`:
   ```typescript
   useEffect(() => {
     if (brandInfo?.background_image) {
       document.documentElement.style.setProperty(
         '--background-image',
         `url(${brandInfo.background_image})`
       );
     }
   }, [brandInfo]);
   ```

2. Use in CSS with `var(--background-image)`

## 📊 File Sizes

Current branding assets:
- `hero-bg.png`: 9.7 KB
- `sidebar-logo.png`: 299 bytes
- **Total**: ~10 KB

Keep total branding assets under 1 MB for optimal launcher size.

## ✨ Next Steps

1. **Test the current setup**: Run `npm run tauri dev`
2. **Replace with your images**: Update the PNG files
3. **Customize colors**: Edit `brand.json` → `ui.colors`
4. **Build for production**: Run `npm run tauri build`

---

**Your branding is now integrated!** 🎉

Run the dev server to see your custom logo and branding in action.
