/**
 * API wrapper for Tauri command invocations.
 *
 * This module provides type-safe wrappers around Tauri's invoke() function
 * for communication with the Rust backend.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

import type {
  AppStatus,
  BrandInfo,
  CuoConfig,
  DetectionResult,
  GetSettingsResponse,
  InstallProgress,
  InstallResponse,
  InstallStatusResponse,
  LaunchGameRequest,
  LaunchResponse,
  MigrationProgress,
  PathValidationResult,
  SaveResponse,
  SaveSettingsRequest,
  ScanMigrationResponse,
  ThemeColors,
  UpdateCheckResponse,
  UpdateProgress,
  UpdateResponse,
  ValidateClientResponse,
  VerifyResponse,
  TauriEventName,
} from "./types";

import { TauriEvents } from "./types";

// ============================================================================
// Install Commands
// ============================================================================

/**
 * Checks the current installation status.
 *
 * @returns Information about whether installation is needed and the current state.
 */
export async function checkInstallStatus(): Promise<InstallStatusResponse> {
  return invoke<InstallStatusResponse>("check_install_status");
}

/**
 * Validates a proposed installation path.
 *
 * @param path - The path to validate.
 * @returns Validation result indicating whether the path is suitable.
 */
export async function validateInstallPath(
  path: string
): Promise<PathValidationResult> {
  return invoke<PathValidationResult>("validate_install_path", { path });
}

/**
 * Starts the installation process.
 *
 * @param installPath - Path to install the game to.
 * @returns Result of the installation operation.
 */
export async function startInstall(
  installPath: string
): Promise<InstallResponse> {
  return invoke<InstallResponse>("start_install", {
    request: { install_path: installPath },
  });
}

/**
 * Gets the current application status.
 *
 * @returns A snapshot of the current application state.
 */
export async function getAppStatus(): Promise<AppStatus> {
  return invoke<AppStatus>("get_app_status");
}

// ============================================================================
// Migration Commands
// ============================================================================

/**
 * Scans brand-configured paths for existing installations.
 */
export async function scanForMigrations(): Promise<ScanMigrationResponse> {
  return invoke<ScanMigrationResponse>("scan_for_migrations");
}

/**
 * Detects an existing installation at a user-specified path.
 */
export async function detectAtPath(path: string): Promise<DetectionResult> {
  return invoke<DetectionResult>("detect_at_path", { path });
}

/**
 * Starts a file-copy migration from source to destination.
 */
export async function startMigration(
  sourcePath: string,
  destinationPath: string
): Promise<void> {
  return invoke<void>("start_migration", {
    request: { source_path: sourcePath, destination_path: destinationPath },
  });
}

/**
 * Adopts an existing installation directory in-place.
 */
export async function useInPlace(installPath: string): Promise<void> {
  return invoke<void>("use_in_place", {
    request: { install_path: installPath },
  });
}

/**
 * Listens for migration progress events.
 */
export async function onMigrationProgress(
  callback: (progress: MigrationProgress) => void
): Promise<UnlistenFn> {
  return listen<MigrationProgress>(TauriEvents.MIGRATION_PROGRESS, (event) => {
    callback(event.payload);
  });
}

// ============================================================================
// Update Commands
// ============================================================================

/**
 * Checks for available updates.
 *
 * Downloads and verifies the manifest from the update server, then
 * compares it against the current installation.
 *
 * @returns Update check result with version and file information.
 */
export async function checkForUpdates(): Promise<UpdateCheckResponse> {
  return invoke<UpdateCheckResponse>("check_for_updates");
}

/**
 * Starts the update process.
 *
 * Downloads and applies all pending updates with atomic application
 * and rollback support.
 *
 * @returns Result of the update operation.
 */
export async function startUpdate(): Promise<UpdateResponse> {
  return invoke<UpdateResponse>("start_update");
}

/**
 * Gets the current update progress.
 *
 * @returns The cached update progress, or null if no update is in progress.
 */
export async function getUpdateProgress(): Promise<UpdateProgress | null> {
  return invoke<UpdateProgress | null>("get_update_progress");
}

/**
 * Dismisses the update notification without applying.
 *
 * User can still apply the update later.
 */
export async function dismissUpdate(): Promise<void> {
  return invoke<void>("dismiss_update");
}

// ============================================================================
// Launch Commands
// ============================================================================

/**
 * Launches the game client.
 *
 * @param request - Optional launch options including args and close behavior.
 * @returns Result of the launch operation.
 */
export async function launchGame(
  request?: LaunchGameRequest
): Promise<LaunchResponse> {
  return invoke<LaunchResponse>("launch_game", { request: request ?? null });
}

/**
 * Validates that the game client can be launched.
 *
 * @returns Validation result indicating whether the client is valid.
 */
export async function validateClient(): Promise<ValidateClientResponse> {
  return invoke<ValidateClientResponse>("validate_client");
}

/**
 * Marks the game as no longer running.
 *
 * Should be called when the game process exits or the user indicates
 * the game has closed.
 */
export async function gameClosed(): Promise<void> {
  return invoke<void>("game_closed");
}

// ============================================================================
// Settings Commands
// ============================================================================

/**
 * Gets the current user settings.
 *
 * @returns Current settings and read-only installation info.
 */
export async function getSettings(): Promise<GetSettingsResponse> {
  return invoke<GetSettingsResponse>("get_settings");
}

/**
 * Saves user settings.
 *
 * @param settings - The updated user settings.
 * @returns Result of the save operation.
 */
export async function saveSettings(
  request: SaveSettingsRequest
): Promise<SaveResponse> {
  return invoke<SaveResponse>("save_settings", { request });
}

/**
 * Gets the brand configuration for display.
 *
 * @returns Brand information including theme colors and server details.
 */
export async function getBrandConfig(): Promise<BrandInfo> {
  return invoke<BrandInfo>("get_brand_config");
}

/**
 * Fetches the CUO config block from brand.json. Returns null if not configured.
 */
export async function getCuoConfig(): Promise<CuoConfig | null> {
  return invoke<CuoConfig | null>("get_cuo_config");
}

/**
 * Gets the full theme colors for styling.
 *
 * @returns Theme colors for the launcher.
 */
export async function getThemeColors(): Promise<ThemeColors> {
  return invoke<ThemeColors>("get_theme_colors");
}

/**
 * Gets the launcher's installation directory.
 *
 * @returns Path to the directory where the launcher is installed.
 */
export async function getLauncherDir(): Promise<string> {
  return invoke<string>("get_launcher_dir");
}

/**
 * Verifies the installation integrity.
 *
 * @returns Verification result with list of any invalid files.
 */
export async function verifyInstallation(): Promise<VerifyResponse> {
  return invoke<VerifyResponse>("verify_installation");
}

/**
 * Clears cached data (manifests, etc.).
 *
 * @returns Result of the clear operation.
 */
export async function clearCache(): Promise<SaveResponse> {
  return invoke<SaveResponse>("clear_cache");
}

/**
 * Gets the repair list for damaged installation.
 *
 * @returns List of file paths that need repair.
 */
export async function getRepairList(): Promise<string[]> {
  return invoke<string[]>("get_repair_list");
}

/**
 * Checks if the application is currently running with administrator privileges.
 *
 * @returns True if running as admin, false otherwise.
 */
export async function isRunningAsAdmin(): Promise<boolean> {
  return invoke<boolean>("is_running_as_admin");
}

/**
 * Gets a recommended installation path in the user's AppData directory.
 *
 * @returns Path like C:\Users\{User}\AppData\Local\{ServerName}
 */
export async function getRecommendedInstallPath(): Promise<string> {
  return invoke<string>("get_recommended_install_path");
}

/**
 * Relaunches the application with administrator privileges.
 *
 * On Windows, this requests UAC elevation. The current app will exit
 * and a new elevated instance will start.
 */
export async function relaunchAsAdmin(): Promise<void> {
  return invoke<void>("relaunch_as_admin");
}

/**
 * Opens the game installation folder in the system file manager.
 */
export async function openInstallFolder(): Promise<void> {
  return invoke<void>("open_install_folder");
}

/**
 * Removes all game files from the installation directory and resets install state.
 */
export async function removeGameFiles(): Promise<SaveResponse> {
  return invoke<SaveResponse>("remove_game_files");
}

// ============================================================================
// Event Listeners
// ============================================================================

/**
 * Listens for install progress events.
 *
 * @param callback - Function to call when progress is received.
 * @returns Function to call to stop listening.
 */
export async function onInstallProgress(
  callback: (progress: InstallProgress) => void
): Promise<UnlistenFn> {
  return listen<InstallProgress>(TauriEvents.INSTALL_PROGRESS, (event) => {
    callback(event.payload);
  });
}

/**
 * Listens for update progress events.
 *
 * @param callback - Function to call when progress is received.
 * @returns Function to call to stop listening.
 */
export async function onUpdateProgress(
  callback: (progress: UpdateProgress) => void
): Promise<UnlistenFn> {
  return listen<UpdateProgress>(TauriEvents.UPDATE_PROGRESS, (event) => {
    callback(event.payload);
  });
}

/**
 * Listens for verify progress events.
 *
 * @param callback - Function to call when progress is received.
 * @returns Function to call to stop listening.
 */
export async function onVerifyProgress(
  callback: (progress: InstallProgress) => void
): Promise<UnlistenFn> {
  return listen<InstallProgress>(TauriEvents.VERIFY_PROGRESS, (event) => {
    callback(event.payload);
  });
}

/**
 * Listens for client count change events.
 *
 * Fired by the backend whenever a client process exits. The payload is the
 * number of client instances still running (0 means all clients have closed).
 *
 * @param callback - Function to call with the new running count.
 * @returns Function to call to stop listening.
 */
export async function onClientCountChanged(
  callback: (runningClients: number) => void
): Promise<UnlistenFn> {
  return listen<number>(TauriEvents.CLIENT_COUNT_CHANGED, (event) => {
    callback(event.payload);
  });
}

/**
 * Generic event listener for any Tauri event.
 *
 * @param eventName - Name of the event to listen for.
 * @param callback - Function to call when the event is received.
 * @returns Function to call to stop listening.
 */
export async function onEvent<T>(
  eventName: TauriEventName | string,
  callback: (payload: T) => void
): Promise<UnlistenFn> {
  return listen<T>(eventName, (event) => {
    callback(event.payload);
  });
}

// ============================================================================
// Composite Operations
// ============================================================================

/**
 * Performs a full update check and returns the status.
 *
 * This is a convenience function that combines getting the app status
 * and checking for updates.
 *
 * @returns Object containing app status and update check results.
 */
export async function getFullStatus(): Promise<{
  appStatus: AppStatus;
  updateCheck: UpdateCheckResponse | null;
}> {
  const appStatus = await getAppStatus();

  // Only check for updates if we have a valid installation
  let updateCheck: UpdateCheckResponse | null = null;
  if (
    appStatus.install_path &&
    !appStatus.is_installing &&
    !appStatus.is_updating
  ) {
    try {
      updateCheck = await checkForUpdates();
    } catch (error) {
      // Update check failed, but we can still return the app status
      updateCheck = null;
    }
  }

  return { appStatus, updateCheck };
}

/**
 * Converts a hex color to rgba() with the given alpha.
 */
function hexToRgba(hex: string, alpha: number): string {
  const h = hex.replace("#", "");
  const r = parseInt(h.slice(0, 2), 16);
  const g = parseInt(h.slice(2, 4), 16);
  const b = parseInt(h.slice(4, 6), 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/**
 * Lightens or darkens a hex color by adjusting each channel.
 * Positive amount = darker, negative = lighter.
 */
function adjustHex(hex: string, amount: number): string {
  const h = hex.replace("#", "");
  const r = Math.min(255, Math.max(0, parseInt(h.slice(0, 2), 16) - amount));
  const g = Math.min(255, Math.max(0, parseInt(h.slice(2, 4), 16) - amount));
  const b = Math.min(255, Math.max(0, parseInt(h.slice(4, 6), 16) - amount));
  return `#${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
}

/**
 * Applies theme colors to CSS custom properties.
 *
 * Brand color semantics:
 *   secondary → --color-primary   (action/accent color: buttons, links, active states)
 *   primary   → --color-surface   (identity/surface color: panels, sidebar background)
 *   background → --color-background
 *   text      → --color-text
 *
 * Derived colors (hover, active, light, glow) are computed from the accent color
 * so the full design system updates automatically from brand.json.
 *
 * @param colors - Theme colors from brand.json.
 */
export function applyThemeColors(colors: ThemeColors): void {
  const root = document.documentElement;
  const accent = colors.secondary; // crimson / action color
  const surface = colors.primary;  // dark navy / surface color

  // Action / accent color and its derived variants
  root.style.setProperty("--color-primary", accent);
  root.style.setProperty("--color-primary-hover", adjustHex(accent, 20));
  root.style.setProperty("--color-primary-active", adjustHex(accent, 40));
  root.style.setProperty("--color-primary-light", hexToRgba(accent, 0.15));
  root.style.setProperty("--shadow-glow", `0 0 20px ${hexToRgba(accent, 0.4)}`);

  // Surface / identity color
  root.style.setProperty("--color-surface", surface);
  root.style.setProperty("--color-surface-hover", adjustHex(surface, -11));
  root.style.setProperty("--color-surface-active", adjustHex(surface, -19));

  // Base colors
  root.style.setProperty("--color-background", colors.background);
  root.style.setProperty("--color-text", colors.text);
}

/**
 * Initializes the application by loading brand config and applying theme.
 *
 * @returns Brand information if successful.
 */
export async function initializeApp(): Promise<BrandInfo | null> {
  try {
    const brandInfo = await getBrandConfig();
    applyThemeColors(brandInfo.colors);
    return brandInfo;
  } catch (error) {
    // Brand config not available - might be in development mode
    return null;
  }
}

// ============================================================================
// Re-exports
// ============================================================================

// Re-export types for convenience
export * from "./types";
