#!/usr/bin/env node
/**
 * Sync branding from brand.json to tauri.conf.json
 *
 * This script ensures server owners only need to edit branding/brand.json
 * and all config is automatically synced at build time.
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const BRAND_JSON = path.join(__dirname, '..', 'branding', 'brand.json');
const TAURI_CONFIG = path.join(__dirname, 'src-tauri', 'tauri.conf.json');

console.log('🎨 Syncing branding to Tauri config...\n');

// Read brand.json
if (!fs.existsSync(BRAND_JSON)) {
  console.error('❌ Error: branding/brand.json not found!');
  console.error('   Please create branding/brand.json with your server branding.');
  process.exit(1);
}

const brand = JSON.parse(fs.readFileSync(BRAND_JSON, 'utf8'));

// Read tauri.conf.json
if (!fs.existsSync(TAURI_CONFIG)) {
  console.error('❌ Error: src-tauri/tauri.conf.json not found!');
  process.exit(1);
}

const config = JSON.parse(fs.readFileSync(TAURI_CONFIG, 'utf8'));

// Update config with branding
const productName = brand.ui?.windowTitle || brand.product?.displayName || 'UltimaForge';
const windowTitle = brand.ui?.windowTitle || brand.product?.displayName || 'UltimaForge';

config.productName = productName;
config.app.windows[0].title = windowTitle;

// Write updated config
fs.writeFileSync(TAURI_CONFIG, JSON.stringify(config, null, 2));

console.log('✅ Updated tauri.conf.json:');
console.log(`   productName: "${productName}"`);
console.log(`   window.title: "${windowTitle}"`);
console.log('\n✨ Branding synced successfully!\n');
