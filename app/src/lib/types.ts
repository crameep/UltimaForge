/**
 * Type definitions for Tauri communication in UltimaForge.
 *
 * These types mirror the Rust types used in the backend commands.
 * They are used for type-safe communication between the frontend and backend.
 */

// ============================================================================
// Application State Types
// ============================================================================

/**
 * Current phase of the application lifecycle.
 */
export type AppPhase =
  | "Initializing"
  | "NeedsInstall"
  | "Installing"
  | "CheckingUpdates"
  | "UpdateAvailable"
  | "Updating"
  | "Ready"
  | "GameRunning"
  | "Error";

/**
 * Serializable status summary for the frontend.
 * Contains the current state of the application.
 */
export interface AppStatus {
  /** Current application phase */
  phase: AppPhase;
  /** Path to the UO client installation directory */
  install_path: string | null;
  /** Current installed version */
  current_version: string | null;
  /** Whether an update is available */
  update_available: boolean;
  /** Version available on the server */
  available_version: string | null;
  /** Number of files that need updating */
  files_to_update: number;
  /** Total download size for the update (bytes) */
  update_download_size: number;
  /** Whether an installation is in progress */
  is_installing: boolean;
  /** Whether an update is in progress */
  is_updating: boolean;
  /** Whether the game is currently running */
  is_game_running: boolean;
  /** Number of game client instances currently running */
  running_clients: number;
  /** Current error message (if any) */
  error_message: string | null;
  /** Installation progress percentage (0-100) */
  install_progress: number;
  /** Current operation description */
  current_operation: string | null;
}

// ============================================================================
// Installation Types
// ============================================================================

/**
 * Current state of an installation operation.
 */
export type InstallState =
  | "Idle"
  | "ValidatingPath"
  | "FetchingManifest"
  | "Downloading"
  | "Verifying"
  | "Completed"
  | "Failed";

/**
 * Progress information for an ongoing installation.
 */
export interface InstallProgress {
  /** Current installation state */
  state: InstallState;
  /** Total number of files to install */
  total_files: number;
  /** Number of files processed so far */
  processed_files: number;
  /** Total bytes to download */
  total_bytes: number;
  /** Bytes downloaded so far */
  downloaded_bytes: number;
  /** Current file being processed (if any) */
  current_file: string | null;
  /** Download speed in bytes per second */
  speed_bps: number;
  /** Estimated time remaining in seconds */
  eta_secs: number;
  /** Target version being installed */
  target_version: string | null;
  /** Error message if state is Failed */
  error_message: string | null;
}

/**
 * Response for install status check.
 */
export interface InstallStatusResponse {
  /** Whether installation is required */
  needs_install: boolean;
  /** Current install path if set */
  install_path: string | null;
  /** Current installed version if set */
  current_version: string | null;
  /** Whether installation is complete */
  install_complete: boolean;
  /** Whether the installation was auto-detected */
  was_detected: boolean;
}

/**
 * Request for starting installation.
 */
export interface StartInstallRequest {
  /** Path to install the game to */
  install_path: string;
}

/**
 * Response for installation operation.
 */
export interface InstallResponse {
  /** Whether the operation was successful */
  success: boolean;
  /** Error message if failed */
  error: string | null;
  /** Installed version if successful */
  version: string | null;
}

/**
 * Result of validating an installation path.
 */
export interface PathValidationResult {
  /** Whether the path is valid for installation */
  is_valid: boolean;
  /** Reason why the path is invalid (if applicable) */
  reason: string | null;
  /** Whether the directory exists */
  exists: boolean;
  /** Whether the directory is empty */
  is_empty: boolean;
  /** Available disk space in bytes */
  available_space: number;
  /** Whether there's sufficient space for installation */
  has_sufficient_space: boolean;
  /** Whether we have write permissions */
  is_writable: boolean;
  /** Whether the path requires elevation (admin rights) */
  requires_elevation: boolean;
}

// ============================================================================
// Update Types
// ============================================================================

/**
 * Current state of an update operation.
 */
export type UpdateState =
  | "Idle"
  | "Checking"
  | "Downloading"
  | "Verifying"
  | "BackingUp"
  | "Applying"
  | "RollingBack"
  | "Completed"
  | "Failed";

/**
 * Progress information for an ongoing update.
 */
export interface UpdateProgress {
  /** Current update state */
  state: UpdateState;
  /** Total number of files to update */
  total_files: number;
  /** Number of files processed so far */
  processed_files: number;
  /** Total bytes to download */
  total_bytes: number;
  /** Bytes downloaded so far */
  downloaded_bytes: number;
  /** Current file being processed (if any) */
  current_file: string | null;
  /** Download speed in bytes per second */
  speed_bps: number;
  /** Estimated time remaining in seconds */
  eta_secs: number;
  /** Version being updated to */
  target_version: string | null;
  /** Error message if state is Failed */
  error_message: string | null;
}

/**
 * Response for update check.
 */
export interface UpdateCheckResponse {
  /** Whether an update is available */
  update_available: boolean;
  /** Current installed version */
  current_version: string | null;
  /** Server version available */
  server_version: string | null;
  /** Number of files that need updating */
  files_to_update: number;
  /** Total download size in bytes */
  download_size: number;
  /** Human-readable download size */
  download_size_formatted: string;
  /** URL to patch notes if available */
  patch_notes_url: string | null;
  /** Error message if check failed */
  error: string | null;
}

/**
 * Response for update operation.
 */
export interface UpdateResponse {
  /** Whether the operation was successful */
  success: boolean;
  /** Error message if failed */
  error: string | null;
  /** New version if update was successful */
  new_version: string | null;
  /** Whether a rollback occurred */
  rolled_back: boolean;
}

// ============================================================================
// Launch Types
// ============================================================================

/**
 * Request for launching the game.
 */
export interface LaunchGameRequest {
  /** Additional command-line arguments (optional) */
  args?: string[];
  /** Whether to close the launcher after launching */
  close_after_launch?: boolean;
  /** Number of client instances to open (1-5) */
  client_count?: number;
  /** Which server to connect to */
  server_choice?: ServerChoice;
  /** Which assistant to use */
  assistant_choice?: AssistantKind;
}

/**
 * Response for launch operations.
 */
export interface LaunchResponse {
  /** Whether the launch was successful */
  success: boolean;
  /** Process ID of the launched client */
  pid: number | null;
  /** Error message if launch failed */
  error: string | null;
  /** Whether the launcher should close */
  should_close_launcher: boolean;
  /** Number of clients that launched successfully */
  running_clients: number;
}

/**
 * Response for client validation.
 */
export interface ValidateClientResponse {
  /** Whether the client is valid and launchable */
  is_valid: boolean;
  /** Path to the executable */
  executable_path: string | null;
  /** Error message if invalid */
  error: string | null;
}

// ============================================================================
// Settings Types
// ============================================================================

/**
 * User-editable settings.
 */
export interface UserSettings {
  /** Auto-launch client after successful update */
  auto_launch: boolean;
  /** Close launcher after launching game */
  close_on_launch: boolean;
  /** Check for updates on startup */
  check_updates_on_startup: boolean;
}

/** Which assistant is active. Mirrors Rust AssistantKind. */
export type AssistantKind = "razor_enhanced" | "razor" | "none";

/** Which server to connect to. Mirrors Rust ServerChoice. */
export type ServerChoice = "live" | "test";

/** Single server endpoint from brand.json. */
export interface ServerConfig {
  label: string;
  ip: string;
  port: number;
}

/** CUO block from brand.json. Null if server owner didn't configure it. */
export interface CuoConfig {
  client_version: string;
  live_server: ServerConfig;
  test_server: ServerConfig | null;
  available_assistants: AssistantKind[];
  default_assistant: AssistantKind;
  default_server: ServerChoice;
}

/**
 * Response for getting settings.
 */
export interface GetSettingsResponse {
  /** Current user settings */
  settings: UserSettings;
  /** Installation path (read-only for display) */
  install_path: string | null;
  /** Current installed version (read-only for display) */
  current_version: string | null;
  /** Whether installation is complete */
  install_complete: boolean;
}

/**
 * Request for saving settings.
 */
export interface SaveSettingsRequest {
  /** Updated user settings */
  settings: UserSettings;
}

/**
 * Response for save operations.
 */
export interface SaveResponse {
  /** Whether the save was successful */
  success: boolean;
  /** Error message if failed */
  error: string | null;
}

/**
 * Theme colors for styling.
 */
export interface ThemeColors {
  /** Primary brand color (hex, e.g., "#1a1a2e") */
  primary: string;
  /** Secondary/accent color (hex, e.g., "#e94560") */
  secondary: string;
  /** Background color (hex, e.g., "#16213e") */
  background: string;
  /** Text color (hex, e.g., "#ffffff") */
  text: string;
}

/**
 * Brand information for display.
 */
/** Sidebar navigation link */
export interface SidebarLink {
  /** Link label text */
  label: string;
  /** Icon emoji or character */
  icon?: string;
  /** External URL to open */
  url?: string;
}

export interface BrandInfo {
  /** Display name of the server */
  display_name: string;
  /** Server name identifier */
  server_name: string;
  /** Server description */
  description: string | null;
  /** Support email address */
  support_email: string | null;
  /** Server website URL */
  website: string | null;
  /** Discord invite link */
  discord: string | null;
  /** Theme colors */
  colors: ThemeColors;
  /** Background image URL/path */
  background_image: string | null;
  /** Logo image URL/path */
  logo_url: string | null;
  /** Sidebar background texture URL/path */
  sidebar_background: string | null;
  /** Whether to show patch notes */
  show_patch_notes: boolean;
  /** Window title */
  window_title: string;
  /** Main hero title text */
  hero_title: string | null;
  /** Hero subtitle text */
  hero_subtitle: string | null;
  /** Sidebar subtitle text */
  sidebar_subtitle: string | null;
  /** Custom sidebar navigation links */
  sidebar_links: SidebarLink[] | null;
}

/**
 * Response for verification operation.
 */
export interface VerifyResponse {
  /** Whether all files are valid */
  success: boolean;
  /** Total number of files checked */
  total_files: number;
  /** Number of valid files */
  valid_files: number;
  /** List of invalid file paths */
  invalid_files: string[];
  /** Error message if verification failed */
  error: string | null;
}

// ============================================================================
// Event Types
// ============================================================================

/**
 * Event names used for Tauri event communication.
 */
export const TauriEvents = {
  /** Download progress event */
  DOWNLOAD_PROGRESS: "download-progress",
  /** Install progress event */
  INSTALL_PROGRESS: "install-progress",
  /** Update progress event */
  UPDATE_PROGRESS: "update-progress",
  /** Verify progress event */
  VERIFY_PROGRESS: "verify-progress",
} as const;

/**
 * Type for Tauri event names.
 */
export type TauriEventName = (typeof TauriEvents)[keyof typeof TauriEvents];

// ============================================================================
// Utility Types
// ============================================================================

/**
 * Helper to calculate percentage from progress types.
 */
export function calculatePercentage(downloaded: number, total: number): number {
  if (total === 0) return 0;
  return (downloaded / total) * 100;
}

/**
 * Helper to format bytes into human-readable size.
 */
export function formatBytes(bytes: number): string {
  const KB = 1024;
  const MB = KB * 1024;
  const GB = MB * 1024;

  if (bytes >= GB) {
    return `${(bytes / GB).toFixed(2)} GB`;
  } else if (bytes >= MB) {
    return `${(bytes / MB).toFixed(2)} MB`;
  } else if (bytes >= KB) {
    return `${(bytes / KB).toFixed(2)} KB`;
  } else {
    return `${bytes} bytes`;
  }
}

/**
 * Helper to format seconds into human-readable time.
 */
export function formatEta(seconds: number): string {
  if (seconds < 60) {
    return `${Math.round(seconds)}s`;
  } else if (seconds < 3600) {
    const mins = Math.floor(seconds / 60);
    const secs = Math.round(seconds % 60);
    return `${mins}m ${secs}s`;
  } else {
    const hours = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${mins}m`;
  }
}
