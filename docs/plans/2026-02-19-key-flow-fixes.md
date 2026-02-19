# Key Flow Fixes (D/E/F Paths) Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix five key management bugs in the server owner wizard (D), publish-all (E), and dev all-in-one (F) scripts that cause immediate crashes, silent data loss, and broken dev environments.

**Architecture:** All five issues live in Node.js ES module scripts under `app/scripts/`. Fixes are surgical — add guards, shared helpers, and validation logic. No new dependencies. No restructuring. Each fix is independently deployable.

**Tech Stack:** Node.js ESM, `fs`, `path`, `child_process`, `readline/promises`

---

## Issue Map

| # | Issue | Script | Impact |
|---|-------|--------|--------|
| 1 | Encrypted Tauri key, missing `password.txt` | `publish-all.js` + `configure-launcher-updater.js` | Immediate crash on path E |
| 2 | Re-running D silently overwrites game keypair | `server-owner-wizard.js` | Breaks all existing launchers permanently |
| 3 | `publish-all.js` hardcodes `keys/tauri-updater/` | `publish-all.js` | Breaks on fresh setup if `keys/` doesn't exist first |
| 4 | Tauri key capture can silently write empty files | `configure-launcher-updater.js` | Empty key files, no error shown |
| 5 | Test keys ≠ brand.json public key for dev | `dev-all-in-one.js` | Signature verification failure in dev mode |

---

## Task 1: Fix encrypted Tauri key / missing password.txt crash (Issue 1)

**Root cause:** `configure-launcher-updater.js` prompts for a password when generating an encrypted key, but only *if* no `password.txt` exists yet at the time of generation. If the key was generated in a previous session and the password was never stored, subsequent runs skip the prompt (the key already exists, generation is skipped) and `publish-all.js` crashes.

**Files:**
- Modify: `app/scripts/configure-launcher-updater.js`
- Modify: `app/scripts/publish-all.js` (improve error message + add recovery hint)

---

### Step 1: Add a post-setup validation function to `configure-launcher-updater.js`

At the bottom of `configure-launcher-updater.js`, before `main()` closes, add a call to validate the final state:

```js
function validateFinalKeyState(privateKeyPath, passwordPath) {
  if (!fs.existsSync(privateKeyPath)) {
    console.log("\nWARNING: Private key file was not created. Re-run and generate keys.");
    return false;
  }
  const keyText = fs.readFileSync(privateKeyPath, "utf8");
  const encrypted = isEncryptedKey(keyText);
  const hasPassword = fs.existsSync(passwordPath) &&
    fs.readFileSync(passwordPath, "utf8").trim().length > 0;

  if (encrypted && !hasPassword) {
    console.log("\nERROR: The Tauri updater key is encrypted but no password was saved.");
    console.log("Fix option 1: Re-run this wizard and regenerate keys — press Enter at both");
    console.log("             password prompts to create an unencrypted key (recommended).");
    console.log("Fix option 2: Create keys/tauri-updater/password.txt containing your password.");
    return false;
  }
  return true;
}
```

**Where to call it** — at the end of `main()` in `configure-launcher-updater.js`, just before the final console.log summary:

```js
  // After all key writing is done:
  const valid = validateFinalKeyState(privateKeyPath, privateKeyPasswordPath);
  if (!valid) {
    process.exit(1);
  }
```

---

### Step 2: Improve the error message in `publish-all.js:validateUpdaterKeyPassword`

The current error says "Re-run the wizard and enter the password". Make it more specific:

**Current (lines 120-125):**
```js
if (isEncrypted && !hasPassword) {
  throw new Error(
    "Updater key is encrypted but no password.txt was found. " +
      "Re-run the wizard and enter the password, or regenerate keys with no password."
  );
}
```

**Replace with:**
```js
if (isEncrypted && !hasPassword) {
  throw new Error(
    `Updater key is encrypted but no password file found at: ${passwordPath}\n` +
    `Fix option 1 (recommended): Re-run option D and regenerate keys,\n` +
    `  pressing Enter at BOTH password prompts to create an unencrypted key.\n` +
    `Fix option 2: Create ${passwordPath} containing the password you used.`
  );
}
```

---

### Step 3: Manual verification

```
1. Ensure keys/tauri-updater/ has an encrypted tauri.key and NO password.txt
2. Run: node app/scripts/configure-launcher-updater.js
   Expected: Wizard detects encrypted key, no password saved → prints clear error → exits 1
3. Regenerate keys without a password (press Enter at both prompts)
4. Run: node app/scripts/publish-all.js --launcher-build false
   Expected: No crash on key validation
```

---

### Step 4: Commit

```bash
git add app/scripts/configure-launcher-updater.js app/scripts/publish-all.js
git commit -m "fix: detect and error on encrypted Tauri key without saved password"
```

---

## Task 2: Prevent silent overwrite of game signing keypair (Issue 2)

**Root cause:** `server-owner-wizard.js:130` passes `--force` unconditionally and defaults the keygen prompt to `true`. A server owner who re-runs the wizard for any other reason (update URL, colors, etc.) can accidentally overwrite their production keypair, permanently breaking update verification for all existing players.

**Files:**
- Modify: `app/scripts/server-owner-wizard.js`

---

### Step 1: Change the default for the keygen prompt based on key existence

**Find this block (around line 126-140):**
```js
let publicKey = existing?.publicKey ?? "";
const wantKeygen = await confirm(rl, "Generate a new Ed25519 keypair now?", true);
if (wantKeygen) {
  try {
    execSync(`cargo run -p publish-cli -- keygen --output "${keysDir}" --force`, {
      cwd: repoRoot,
      stdio: "inherit",
    });
```

**Replace with:**
```js
let publicKey = existing?.publicKey ?? "";
const gameKeysExist = fs.existsSync(publicKeyPath);
const keygenPrompt = gameKeysExist
  ? "A keypair already exists. Regenerate? (overwrites existing keys — breaks launchers already distributed)"
  : "Generate a new Ed25519 keypair now?";
const wantKeygen = await confirm(rl, keygenPrompt, !gameKeysExist);
```

This makes the default `false` (no regenerate) when keys already exist, and `true` (generate) on first run.

---

### Step 2: Add typed confirmation when keys already exist

Immediately after the `wantKeygen` assignment, add:

```js
let confirmedKeygen = wantKeygen;
if (wantKeygen && gameKeysExist) {
  console.log("\n  WARNING: Overwriting keys is IRREVERSIBLE.");
  console.log("  Any launcher already distributed to players will stop verifying updates.");
  console.log("  Only proceed if this is a brand new setup.\n");
  const typed = (await rl.question('  Type "REGENERATE" to confirm, or press Enter to cancel: ')).trim();
  if (typed !== "REGENERATE") {
    console.log("  Aborted. Existing keys preserved.");
    confirmedKeygen = false;
  }
}
```

---

### Step 3: Back up existing keys before overwriting, and remove `--force`

Replace the keygen execution block:

```js
if (confirmedKeygen) {
  try {
    // Back up existing keys first
    if (gameKeysExist) {
      const timestamp = new Date().toISOString().replace(/[:.]/g, "-");
      const backupDir = path.join(keysDir, `backup-${timestamp}`);
      fs.mkdirSync(backupDir, { recursive: true });
      for (const filename of ["public.key", "private.key"]) {
        const src = path.join(keysDir, filename);
        if (fs.existsSync(src)) {
          fs.copyFileSync(src, path.join(backupDir, filename));
        }
      }
      console.log(`\nExisting keys backed up to: ${backupDir}`);
    }

    // Generate without --force; publish-cli will error if keys exist (belt-and-suspenders)
    execSync(`cargo run -p publish-cli -- keygen --output "${keysDir}"`, {
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
```

> **Note:** Check whether `publish-cli keygen` without `--force` will error if keys exist. If it does not, keep `--force` here since the typed confirmation above is the safety gate. If it does error without `--force`, this is belt-and-suspenders protection.

---

### Step 4: Manual verification

```
Scenario A — First run (no keys/):
1. Delete keys/ directory if it exists
2. Run: node app/scripts/server-owner-wizard.js
   Expected: Prompt defaults to YES, keys generated, no backup created

Scenario B — Re-run with existing keys (normal case):
1. Ensure keys/ has public.key and private.key
2. Run: node app/scripts/server-owner-wizard.js
   Expected: keygen prompt defaults to NO, pressing Enter skips keygen

Scenario C — Re-run and type REGENERATE:
1. Ensure keys/ has existing keys
2. Run wizard, answer yes to keygen, type "REGENERATE" when prompted
   Expected: Backup created in keys/backup-YYYY-MM-DDTHH-MM-SS/, new keys generated

Scenario D — Re-run and type anything else:
1. Ensure keys/ has existing keys
2. Run wizard, answer yes to keygen, type "oops" when prompted
   Expected: "Aborted. Existing keys preserved." — no keygen
```

---

### Step 5: Commit

```bash
git add app/scripts/server-owner-wizard.js
git commit -m "fix: prevent accidental game keypair overwrite in server owner wizard"
```

---

## Task 3: Fix hardcoded `keys/tauri-updater/` path in publish-all.js (Issue 3)

**Root cause:** `configure-launcher-updater.js` uses dynamic legacy/default path resolution but `publish-all.js:281` hardcodes `path.join(repoRoot, "keys", "tauri-updater")`. On a fresh machine where `keys/` hasn't been created yet, the wizard writes to `server-data/keys/tauri-updater/` but publish-all looks in `keys/tauri-updater/`.

**Files:**
- Modify: `app/scripts/publish-all.js`

---

### Step 1: Add the shared key directory resolution to `publish-all.js`

At the top of `publish-all.js`, after the path constants are defined (around line 16), add the same resolution logic used in the other scripts:

```js
const legacyKeysDir = path.join(repoRoot, "keys");
const defaultKeysDir = path.join(repoRoot, "server-data", "keys");
const resolvedKeysDir = fs.existsSync(legacyKeysDir) ? legacyKeysDir : defaultKeysDir;
```

---

### Step 2: Replace the hardcoded path

**Find (around line 281-283):**
```js
const updaterKeysDir = path.join(repoRoot, "keys", "tauri-updater");
const updaterKeyPath = path.join(updaterKeysDir, "tauri.key");
const updaterPassPath = path.join(updaterKeysDir, "password.txt");
```

**Replace with:**
```js
const updaterKeysDir = path.join(resolvedKeysDir, "tauri-updater");
const updaterKeyPath = path.join(updaterKeysDir, "tauri.key");
const updaterPassPath = path.join(updaterKeysDir, "password.txt");
```

---

### Step 3: Manual verification

```
1. Rename keys/ to keys-backup/ to simulate a fresh setup
2. Create server-data/keys/tauri-updater/ with a tauri.key and tauri.pub
3. Run: node app/scripts/publish-all.js --launcher-build false
   Expected: Key validation succeeds (finds key in server-data/keys/tauri-updater/)
4. Restore: rename keys-backup/ back to keys/
```

---

### Step 4: Commit

```bash
git add app/scripts/publish-all.js
git commit -m "fix: use dynamic key directory resolution in publish-all.js"
```

---

## Task 4: Error on empty/invalid Tauri key capture (Issue 4)

**Root cause:** `runSignerGenerate()` in `configure-launcher-updater.js` parses `Private:` and `Public:` lines from the combined stdout+stderr of `tauri signer generate`. If the CLI output format changes or the capture fails, it silently returns empty strings, which are written to `tauri.key` and `tauri.pub` as blank files. The wizard continues without error.

**Files:**
- Modify: `app/scripts/configure-launcher-updater.js`

---

### Step 1: Add key format validation helpers

Add these two functions to `configure-launcher-updater.js` after `parseGeneratedKeys`:

```js
function isValidTauriPrivateKey(keyText) {
  if (!keyText || keyText.trim().length === 0) return false;
  // Tauri minisign keys are base64-encoded blocks, always multi-line
  // Minimum length check (unencrypted ~110 chars, encrypted ~340 chars)
  if (keyText.trim().length < 80) return false;
  return true;
}

function isValidTauriPublicKey(keyText) {
  if (!keyText || keyText.trim().length === 0) return false;
  // Public key is a single base64 line, typically 100-160 chars
  if (keyText.trim().length < 40) return false;
  return true;
}
```

---

### Step 2: Validate the captured keys immediately after `runSignerGenerate()`

**Find (around line 244-256):**
```js
if (wantGenerate) {
  fs.mkdirSync(updaterDir, { recursive: true });
  const generated = await runSignerGenerate();
  if (generated.privateKey) {
    writeKeyFile(privateKeyPath, generated.privateKey);
  }
  if (generated.publicKey) {
    writeKeyFile(path.join(updaterDir, "tauri.pub"), generated.publicKey);
  }
  if (!generated.privateKey || !generated.publicKey) {
    console.log("Failed to capture Tauri updater keys from CLI output.");
  }
}
```

**Replace with:**
```js
if (wantGenerate) {
  fs.mkdirSync(updaterDir, { recursive: true });
  const generated = await runSignerGenerate();

  const privateValid = isValidTauriPrivateKey(generated.privateKey);
  const publicValid = isValidTauriPublicKey(generated.publicKey);

  if (!privateValid || !publicValid) {
    console.log("\nERROR: Failed to capture Tauri updater keys from CLI output.");
    console.log("The tauri signer generate command ran but the key output could not be parsed.");
    console.log("\nTroubleshooting:");
    console.log("  1. Run manually: npx tauri signer generate");
    console.log("  2. Copy the output into keys/tauri-updater/tauri.key (private) and tauri.pub (public)");
    console.log("  3. Re-run this wizard — it will detect the existing keys");
    console.log("\nNo key files were written.");
    process.exit(1);
  }

  writeKeyFile(privateKeyPath, generated.privateKey);
  writeKeyFile(path.join(updaterDir, "tauri.pub"), generated.publicKey);
  console.log("Tauri updater keys captured and saved.");
}
```

---

### Step 3: Manual verification

```
Simulate failure: temporarily patch parseGeneratedKeys to return empty strings,
then run: node app/scripts/configure-launcher-updater.js
Expected: Error message with troubleshooting steps, process exits 1, no key files written

Normal flow:
Run: node app/scripts/configure-launcher-updater.js
Choose to regenerate keys, press Enter at password prompts
Expected: Keys captured, validated, written — no errors
```

---

### Step 4: Commit

```bash
git add app/scripts/configure-launcher-updater.js
git commit -m "fix: error loudly when Tauri key generation output cannot be parsed"
```

---

## Task 5: Fix dev mode signature mismatch between test keys and brand.json (Issue 5)

**Root cause:** `dev-all-in-one.js` signs the test manifest with `app/test-keys/private.key`, but the launcher in dev mode verifies signatures using the public key embedded in `branding/brand.json`. If `brand.json` has the production public key (which it does after running the wizard), the test manifest signatures will never verify.

**Files:**
- Modify: `app/scripts/dev-all-in-one.js`

---

### Step 1: Read test public key and compare against brand.json at startup

Add a validation check at the top of `run()` in `dev-all-in-one.js`:

```js
function validateTestKeyAlignment() {
  const testPublicKeyPath = path.join(appDir, "test-keys", "public.key");
  const brandJsonPath = path.join(appDir, "public", "branding", "brand.json");

  if (!fs.existsSync(testPublicKeyPath)) {
    console.log("WARNING: app/test-keys/public.key not found.");
    console.log("  Run option [4] to generate test manifests first, or ensure test keys exist.");
    return;
  }

  if (!fs.existsSync(brandJsonPath)) {
    // brand.json in public/ may not exist yet; skip check
    return;
  }

  const testPubKey = fs.readFileSync(testPublicKeyPath, "utf8").trim();
  let brandPubKey = "";
  try {
    const brand = JSON.parse(fs.readFileSync(brandJsonPath, "utf8"));
    brandPubKey = (brand.publicKey ?? "").trim();
  } catch {
    return;
  }

  if (brandPubKey && testPubKey && brandPubKey !== testPubKey) {
    console.log("\n⚠  WARNING: Key mismatch detected!");
    console.log("   app/test-keys/public.key does not match the publicKey in branding/brand.json.");
    console.log("   The launcher will REJECT the test manifest signatures.");
    console.log("");
    console.log("   Fix: copy the test public key into brand.json for local dev:");
    console.log(`     Test public key: ${testPubKey}`);
    console.log("");
    console.log("   Or update branding/brand.json publicKey to match your test-keys.");
    console.log("   Press Ctrl+C to abort, or wait 5 seconds to continue anyway...\n");
    // Give them a moment to see the warning and abort if needed
    execSync("node -e \"setTimeout(()=>{},5000)\"", { stdio: "inherit" });
  }
}
```

---

### Step 2: Call the validation at the start of `run()`

**Find:**
```js
function run() {
  ensureTestUpdates();
```

**Replace with:**
```js
function run() {
  validateTestKeyAlignment();
  ensureTestUpdates();
```

---

### Step 3: Add a note to the console output about which key is in use

Just before spawning the server and launcher, add:

```js
  const testPubKeyPath = path.join(appDir, "test-keys", "public.key");
  if (fs.existsSync(testPubKeyPath)) {
    const key = fs.readFileSync(testPubKeyPath, "utf8").trim();
    console.log(`\nDev mode: signing test manifests with test key.`);
    console.log(`Test public key: ${key.substring(0, 16)}...`);
    console.log(`Brand public key must match for launcher to verify updates.\n`);
  }
```

---

### Step 4: Manual verification

```
Scenario A — Keys match:
1. Copy contents of app/test-keys/public.key into branding/brand.json publicKey field
2. Run: node app/scripts/dev-all-in-one.js
   Expected: No warning, server and launcher start normally

Scenario B — Keys mismatch (current state):
1. Ensure brand.json has production public key (different from test-keys)
2. Run: node app/scripts/dev-all-in-one.js
   Expected: Warning printed showing the mismatch and the test public key value
             5-second countdown shown, then continues
```

---

### Step 5: Commit

```bash
git add app/scripts/dev-all-in-one.js
git commit -m "fix: warn on test key / brand.json public key mismatch in dev-all-in-one"
```

---

## Implementation Order

Do these in sequence — each is independent but ordered by impact:

```
Task 1  →  Task 2  →  Task 3  →  Task 4  →  Task 5
(crash)   (data loss) (fresh setup) (silent fail) (dev UX)
```

## Testing the Full D → E → F Flow After All Fixes

```bash
# Clean state test
# 1. Delete keys/ and server-data/ directories
# 2. Run D: node app/scripts/server-owner-wizard.js
#    + node app/scripts/configure-launcher-updater.js
#    Expected: Keys created in server-data/keys/ (since keys/ doesn't exist yet)

# 3. Run E: node app/scripts/publish-all.js --launcher-build false
#    Expected: Finds keys in server-data/keys/tauri-updater/ correctly

# 4. Run F: node app/scripts/dev-all-in-one.js
#    Expected: Key alignment check runs, server and launcher start

# Re-run D without regenerating (common case)
# 5. Run D again, press Enter for keygen (default: No)
#    Expected: Keys untouched, brand.json updated with new values only
```
