#!/usr/bin/env node
// UltimaForge Environment Validation Script
// Checks all dependencies and their versions, reporting any issues
//
// Usage:
//   node scripts/validate-env.js           # Run validation
//   node scripts/validate-env.js --help    # Show help
//   node scripts/validate-env.js --json    # Output as JSON
//   node scripts/validate-env.js --ci      # CI mode (no colors, exit code only)

const { execSync } = require('child_process');
const os = require('os');

// Version requirements
const REQUIREMENTS = {
  rust: { cmd: 'rustc --version', name: 'Rust', minVersion: '1.77.2' },
  node: { cmd: 'node --version', name: 'Node.js', minVersion: '18.0.0' },
  npm: { cmd: 'npm --version', name: 'npm', minVersion: '8.0.0' },
  tauriCli: { cmd: 'npm list @tauri-apps/cli', name: 'Tauri CLI', minVersion: '2.0.0', optional: false },
};

// Platform-specific requirements
const PLATFORM_REQUIREMENTS = {
  win32: [
    { name: 'VS Build Tools', check: checkVSBuildTools }
  ],
  linux: [
    { name: 'WebKitGTK', check: checkWebKitGTK }
  ],
  darwin: [
    { name: 'Xcode CLI Tools', check: checkXcodeTools }
  ]
};

// Colors for terminal output
const COLORS = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  red: '\x1b[31m',
  cyan: '\x1b[36m',
  white: '\x1b[37m',
  bold: '\x1b[1m',
};

// Configuration
let useColors = true;
let outputJson = false;
let ciMode = false;

// Parse command line arguments
function parseArgs() {
  const args = process.argv.slice(2);

  for (const arg of args) {
    switch (arg) {
      case '--help':
      case '-h':
        showHelp();
        process.exit(0);
        break;
      case '--json':
        outputJson = true;
        useColors = false;
        break;
      case '--ci':
        ciMode = true;
        useColors = false;
        break;
      case '--no-color':
        useColors = false;
        break;
      default:
        console.error(`Unknown option: ${arg}`);
        showHelp();
        process.exit(1);
    }
  }

  // Disable colors if not a TTY
  if (!process.stdout.isTTY) {
    useColors = false;
  }
}

function showHelp() {
  console.log(`
UltimaForge Environment Validation Script

USAGE:
    node scripts/validate-env.js [OPTIONS]
    npm run validate-env [-- OPTIONS]

OPTIONS:
    --help, -h     Show this help message
    --json         Output results as JSON
    --ci           CI mode (no colors, minimal output)
    --no-color     Disable colored output

DESCRIPTION:
    Validates that all required dependencies for building UltimaForge are
    installed and meet minimum version requirements.

DEPENDENCIES CHECKED:
    - Rust (>= 1.77.2)
    - Node.js (>= 18.0.0)
    - npm (>= 8.0.0)
    - Tauri CLI (>= 2.0.0)
    - Platform-specific dependencies:
      - Windows: Visual Studio Build Tools
      - Linux: WebKitGTK 4.1
      - macOS: Xcode Command Line Tools

EXIT CODES:
    0    All dependencies satisfied
    1    One or more dependencies missing or outdated

EXAMPLES:
    # Run validation
    node scripts/validate-env.js

    # Output as JSON for tooling
    node scripts/validate-env.js --json

    # CI mode (for automated pipelines)
    node scripts/validate-env.js --ci
`);
}

// Utility functions
function color(text, colorName) {
  if (!useColors) return text;
  return `${COLORS[colorName] || ''}${text}${COLORS.reset}`;
}

function writeStatus(message, type = 'info') {
  if (outputJson || ciMode) return;

  const symbols = {
    success: color('✓', 'green'),
    warning: color('⚠', 'yellow'),
    error: color('✗', 'red'),
    info: color('→', 'cyan'),
    step: color('▶', 'white'),
  };

  const symbol = symbols[type] || symbols.info;
  console.log(`${symbol} ${message}`);
}

function execCommand(cmd) {
  try {
    const output = execSync(cmd, {
      encoding: 'utf8',
      stdio: ['pipe', 'pipe', 'pipe'],
      timeout: 30000
    });
    return { success: true, output: output.trim() };
  } catch (error) {
    return { success: false, output: error.message };
  }
}

// Version comparison (returns true if actual >= required)
function compareVersion(actual, required) {
  if (!actual) return false;

  const parse = (v) => {
    // Clean version string: remove 'v' prefix and any trailing info
    const cleaned = v.replace(/^v/i, '').replace(/[^0-9.].*$/, '');
    return cleaned.split('.').map(n => parseInt(n, 10) || 0);
  };

  const actualParts = parse(actual);
  const requiredParts = parse(required);

  for (let i = 0; i < 3; i++) {
    const a = actualParts[i] || 0;
    const r = requiredParts[i] || 0;

    if (a > r) return true;
    if (a < r) return false;
  }

  return true; // Equal versions
}

// Extract version from command output
function extractVersion(output, name) {
  if (!output) return null;

  // Handle different output formats
  switch (name) {
    case 'Rust':
      // "rustc 1.77.2 (25ef9e3d8 2024-04-09)"
      const rustMatch = output.match(/rustc\s+(\d+\.\d+\.\d+)/);
      return rustMatch ? rustMatch[1] : null;

    case 'Node.js':
      // "v18.19.0"
      const nodeMatch = output.match(/v?(\d+\.\d+\.\d+)/);
      return nodeMatch ? nodeMatch[1] : null;

    case 'npm':
      // "10.2.3"
      const npmMatch = output.match(/(\d+\.\d+\.\d+)/);
      return npmMatch ? npmMatch[1] : null;

    case 'Tauri CLI':
      // "@tauri-apps/cli@2.0.0" in npm list output
      const tauriMatch = output.match(/@tauri-apps\/cli@(\d+\.\d+\.\d+)/);
      return tauriMatch ? tauriMatch[1] : null;

    default:
      // Generic version pattern
      const genericMatch = output.match(/(\d+\.\d+\.\d+)/);
      return genericMatch ? genericMatch[1] : null;
  }
}

// Platform-specific checks
function checkVSBuildTools() {
  // Check using vswhere
  const vsWherePath = 'C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\vswhere.exe';

  // Try vswhere first
  const vsWhereResult = execCommand(`"${vsWherePath}" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`);
  if (vsWhereResult.success && vsWhereResult.output.trim()) {
    return { installed: true, version: '2022', path: vsWhereResult.output.trim() };
  }

  // Check common paths
  const buildToolsPaths = [
    'C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools',
    'C:\\Program Files\\Microsoft Visual Studio\\2022\\BuildTools',
    'C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\Community',
    'C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\Professional',
    'C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\Enterprise',
  ];

  const fs = require('fs');
  for (const vsPath of buildToolsPaths) {
    try {
      if (fs.existsSync(vsPath)) {
        return { installed: true, version: '2022', path: vsPath };
      }
    } catch (e) {
      // Ignore access errors
    }
  }

  return { installed: false };
}

function checkWebKitGTK() {
  // Check using pkg-config
  const result = execCommand('pkg-config --modversion webkit2gtk-4.1');
  if (result.success) {
    return { installed: true, version: result.output.trim() };
  }

  // Fallback check for older webkit
  const fallbackResult = execCommand('pkg-config --modversion webkit2gtk-4.0');
  if (fallbackResult.success) {
    return {
      installed: true,
      version: fallbackResult.output.trim(),
      warning: 'WebKitGTK 4.0 found, but 4.1 is recommended for Tauri 2.0'
    };
  }

  return { installed: false };
}

function checkXcodeTools() {
  const result = execCommand('xcode-select -p');
  if (result.success && result.output.trim()) {
    return { installed: true, path: result.output.trim() };
  }
  return { installed: false };
}

// Get installation instructions
function getInstallInstructions(name) {
  const platform = os.platform();

  const instructions = {
    'Rust': {
      win32: 'Install: winget install Rustlang.Rustup\nOr visit: https://rustup.rs',
      linux: 'Install: curl --proto \'=https\' --tlsv1.2 -sSf https://sh.rustup.rs | sh',
      darwin: 'Install: curl --proto \'=https\' --tlsv1.2 -sSf https://sh.rustup.rs | sh'
    },
    'Node.js': {
      win32: 'Install: winget install OpenJS.NodeJS.LTS\nOr visit: https://nodejs.org',
      linux: 'Install: Use your package manager or nvm (https://github.com/nvm-sh/nvm)',
      darwin: 'Install: brew install node@20\nOr visit: https://nodejs.org'
    },
    'npm': {
      all: 'npm is included with Node.js. Update with: npm install -g npm'
    },
    'Tauri CLI': {
      all: 'Install: npm install @tauri-apps/cli\nOr globally: npm install -g @tauri-apps/cli'
    },
    'VS Build Tools': {
      win32: 'Install: winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"\nOr download: https://visualstudio.microsoft.com/visual-cpp-build-tools/'
    },
    'WebKitGTK': {
      linux: 'Ubuntu/Debian: sudo apt install libwebkit2gtk-4.1-dev\nFedora: sudo dnf install webkit2gtk4.1-devel\nArch: sudo pacman -S webkit2gtk-4.1'
    },
    'Xcode CLI Tools': {
      darwin: 'Install: xcode-select --install'
    }
  };

  const depInstructions = instructions[name];
  if (!depInstructions) return 'Visit the project documentation for installation instructions.';

  return depInstructions[platform] || depInstructions.all || 'Visit the project documentation for installation instructions.';
}

// Main validation function
function validateEnvironment() {
  const results = {
    platform: os.platform(),
    arch: os.arch(),
    timestamp: new Date().toISOString(),
    dependencies: [],
    platformDependencies: [],
    allPassed: true,
    summary: {
      total: 0,
      passed: 0,
      failed: 0,
      warnings: 0
    }
  };

  if (!ciMode && !outputJson) {
    console.log('');
    console.log(color('╔═══════════════════════════════════════════════════════╗', 'cyan'));
    console.log(color('║       UltimaForge Environment Validation              ║', 'cyan'));
    console.log(color('╚═══════════════════════════════════════════════════════╝', 'cyan'));
    console.log('');
    writeStatus(`Platform: ${os.platform()} (${os.arch()})`, 'info');
    console.log('');
  }

  // Check core dependencies
  if (!ciMode && !outputJson) {
    console.log(color('Checking core dependencies...', 'white'));
  }

  for (const [key, req] of Object.entries(REQUIREMENTS)) {
    results.summary.total++;

    const depResult = {
      name: req.name,
      required: req.minVersion,
      status: 'unknown',
      version: null,
      message: ''
    };

    const cmdResult = execCommand(req.cmd);

    if (!cmdResult.success) {
      depResult.status = 'missing';
      depResult.message = `${req.name} is not installed`;
      results.allPassed = false;
      results.summary.failed++;

      writeStatus(`${req.name}: ${color('Not found', 'red')}`, 'error');
      writeStatus(getInstallInstructions(req.name), 'info');
    } else {
      const version = extractVersion(cmdResult.output, req.name);
      depResult.version = version;

      if (!version) {
        depResult.status = 'unknown';
        depResult.message = `Could not determine ${req.name} version`;
        results.summary.warnings++;

        writeStatus(`${req.name}: ${color('Version unknown', 'yellow')} (installed but version could not be determined)`, 'warning');
      } else if (compareVersion(version, req.minVersion)) {
        depResult.status = 'ok';
        depResult.message = `${req.name} ${version} meets requirement (>= ${req.minVersion})`;
        results.summary.passed++;

        writeStatus(`${req.name}: ${color(version, 'green')} (>= ${req.minVersion} required)`, 'success');
      } else {
        depResult.status = 'outdated';
        depResult.message = `${req.name} ${version} is below required version ${req.minVersion}`;
        results.allPassed = false;
        results.summary.failed++;

        writeStatus(`${req.name}: ${color(version, 'red')} (${req.minVersion} or newer required)`, 'error');
        writeStatus(`Update ${req.name} to version ${req.minVersion} or newer`, 'info');
      }
    }

    results.dependencies.push(depResult);
  }

  // Check platform-specific dependencies
  const platformReqs = PLATFORM_REQUIREMENTS[os.platform()] || [];

  if (platformReqs.length > 0) {
    if (!ciMode && !outputJson) {
      console.log('');
      console.log(color('Checking platform dependencies...', 'white'));
    }

    for (const req of platformReqs) {
      results.summary.total++;

      const checkResult = req.check();
      const depResult = {
        name: req.name,
        status: checkResult.installed ? 'ok' : 'missing',
        version: checkResult.version || null,
        path: checkResult.path || null,
        warning: checkResult.warning || null
      };

      if (checkResult.installed) {
        results.summary.passed++;

        let statusMsg = req.name;
        if (checkResult.version) {
          statusMsg += `: ${color(checkResult.version, 'green')}`;
        } else {
          statusMsg += `: ${color('Installed', 'green')}`;
        }

        writeStatus(statusMsg, 'success');

        if (checkResult.warning) {
          writeStatus(checkResult.warning, 'warning');
          results.summary.warnings++;
        }
      } else {
        results.allPassed = false;
        results.summary.failed++;
        depResult.message = `${req.name} is not installed`;

        writeStatus(`${req.name}: ${color('Not found', 'red')}`, 'error');
        writeStatus(getInstallInstructions(req.name), 'info');
      }

      results.platformDependencies.push(depResult);
    }
  }

  return results;
}

// Output summary
function showSummary(results) {
  if (outputJson) {
    console.log(JSON.stringify(results, null, 2));
    return;
  }

  if (ciMode) {
    // Minimal CI output
    if (results.allPassed) {
      console.log('Environment validation passed');
    } else {
      console.log(`Environment validation failed: ${results.summary.failed} issue(s)`);
    }
    return;
  }

  console.log('');
  console.log(color('═══════════════════════════════════════════════════════', 'cyan'));
  console.log(color('                    Validation Summary                   ', 'cyan'));
  console.log(color('═══════════════════════════════════════════════════════', 'cyan'));
  console.log('');

  const { passed, failed, warnings, total } = results.summary;

  console.log(`  ${color('Total checks:', 'white')} ${total}`);
  console.log(`  ${color('Passed:', 'green')} ${passed}`);
  console.log(`  ${color('Failed:', 'red')} ${failed}`);
  if (warnings > 0) {
    console.log(`  ${color('Warnings:', 'yellow')} ${warnings}`);
  }

  console.log('');
  console.log(color('═══════════════════════════════════════════════════════', 'cyan'));

  if (results.allPassed) {
    console.log('');
    console.log(color('All dependencies are satisfied!', 'green'));
    console.log('');
    console.log(color('Next steps:', 'white'));
    console.log('  1. Run: npm install');
    console.log('  2. Run: npm run tauri dev');
    console.log('');
  } else {
    console.log('');
    console.log(color('Some dependencies are missing or outdated.', 'yellow'));
    console.log(color('Please install the missing dependencies listed above.', 'yellow'));
    console.log('');
    console.log(color('Quick fix:', 'white'));

    const platform = os.platform();
    if (platform === 'win32') {
      console.log('  Run: .\\scripts\\setup.ps1');
    } else {
      console.log('  Run: ./scripts/setup.sh');
    }
    console.log('');
  }
}

// Main entry point
function main() {
  parseArgs();

  const results = validateEnvironment();
  showSummary(results);

  // Exit with appropriate code
  process.exit(results.allPassed ? 0 : 1);
}

main();
