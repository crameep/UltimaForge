#!/usr/bin/env node
/**
 * Sync branding from brand.json to tauri.conf.json and NSIS installer hooks.
 *
 * This script ensures server owners only need to edit branding/brand.json
 * and all config is automatically synced at build time.
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const BRANDING_DIR = path.join(__dirname, '..', 'branding');
const BRAND_JSON = path.join(BRANDING_DIR, 'brand.json');
const TAURI_CONFIG = path.join(__dirname, 'src-tauri', 'tauri.conf.json');
const NSIS_HOOKS = path.join(__dirname, 'src-tauri', 'installer-assets', 'nsis-hooks.nsi');
const PUBLIC_BRANDING_DIR = path.join(__dirname, 'public', 'branding');

console.log('Syncing branding to Tauri config...\n');

// Read brand.json
if (!fs.existsSync(BRAND_JSON)) {
  console.error('Error: branding/brand.json not found!');
  console.error('   Please create branding/brand.json with your server branding.');
  process.exit(1);
}

const brand = JSON.parse(fs.readFileSync(BRAND_JSON, 'utf8'));

// Read tauri.conf.json
if (!fs.existsSync(TAURI_CONFIG)) {
  console.error('Error: src-tauri/tauri.conf.json not found!');
  process.exit(1);
}

const config = JSON.parse(fs.readFileSync(TAURI_CONFIG, 'utf8'));

// Derive fields from brand.json
const productName = brand.ui?.windowTitle || brand.product?.displayName || 'UltimaForge';
const serverName = brand.product?.serverName || '';
// Bundle identifier: com.{servernamelower}.launcher
// Only letters and digits allowed in each segment.
const serverNameLower = serverName.toLowerCase().replace(/[^a-z0-9]/g, '');
const identifier = serverNameLower ? `com.${serverNameLower}.launcher` : config.identifier;

// Update tauri.conf.json
config.productName = productName;
config.identifier = identifier;
config.app.windows[0].title = productName;

fs.writeFileSync(TAURI_CONFIG, JSON.stringify(config, null, 2));

console.log('Updated tauri.conf.json:');
console.log(`   productName: "${productName}"`);
console.log(`   identifier:  "${identifier}"`);
console.log(`   window.title: "${productName}"`);

// Update NSIS installer hooks with SERVER_NAME from brand.json.
// The NSIS uninstaller uses SERVER_NAME to locate the game_path.txt sidecar
// written by the launcher at install time. It must match product.serverName
// from brand.json exactly (the Rust code uses this value for the path).
if (fs.existsSync(NSIS_HOOKS) && serverName) {
  let nsisContent = fs.readFileSync(NSIS_HOOKS, 'utf8');
  const updated = nsisContent.replace(
    /^!define SERVER_NAME ".*"$/m,
    `!define SERVER_NAME "${serverName}"`
  );
  if (updated !== nsisContent) {
    fs.writeFileSync(NSIS_HOOKS, updated);
    console.log(`\nUpdated nsis-hooks.nsi:`);
    console.log(`   SERVER_NAME: "${serverName}"`);
  }
} else if (!serverName) {
  console.log('\nWarning: brand.json missing product.serverName — nsis-hooks.nsi not updated.');
}

// Copy branding assets (brand.json + images) to app/public/branding/
// Vite bundles everything under public/ into the production build.
fs.mkdirSync(PUBLIC_BRANDING_DIR, { recursive: true });

// Always copy brand.json so the frontend gets the current branding config
fs.copyFileSync(BRAND_JSON, path.join(PUBLIC_BRANDING_DIR, 'brand.json'));
console.log('\nCopied brand.json → app/public/branding/brand.json');

// Copy any image assets that exist in branding/
const IMAGE_EXTS = ['.png', '.jpg', '.jpeg', '.gif', '.webp', '.svg', '.ico', '.bmp'];
const brandingFiles = fs.readdirSync(BRANDING_DIR);
const copiedImages = [];
for (const file of brandingFiles) {
  if (IMAGE_EXTS.includes(path.extname(file).toLowerCase())) {
    fs.copyFileSync(
      path.join(BRANDING_DIR, file),
      path.join(PUBLIC_BRANDING_DIR, file)
    );
    copiedImages.push(file);
  }
}
if (copiedImages.length > 0) {
  console.log(`Copied ${copiedImages.length} image(s) → app/public/branding/: ${copiedImages.join(', ')}`);
}

console.log('\nBranding synced successfully!\n');
