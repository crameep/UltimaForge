/**
 * Custom hook for managing user settings.
 *
 * Provides state and actions for the Settings component,
 * including loading, saving, and verification operations.
 */

import { useState, useCallback, useEffect } from "react";

import {
  getSettings,
  saveSettings,
  verifyInstallation,
  clearCache,
  startInstall,
  onVerifyProgress,
  isRunningAsAdmin,
  relaunchAsAdmin,
} from "../lib/api";

import type {
  UserSettings,
  VerifyResponse,
  InstallProgress,
} from "../lib/types";

/**
 * State returned by the useSettings hook.
 */
export interface UseSettingsState {
  /** Whether settings are being loaded */
  isLoading: boolean;
  /** Whether settings are being saved */
  isSaving: boolean;
  /** Whether verification is in progress */
  isVerifying: boolean;
  /** Whether cache is being cleared */
  isClearing: boolean;
  /** Whether repair is in progress */
  isRepairing: boolean;
  /** Whether running with admin privileges */
  isAdmin: boolean;
  /** Current user settings */
  settings: UserSettings | null;
  /** Installation path (read-only) */
  installPath: string | null;
  /** Current installed version (read-only) */
  currentVersion: string | null;
  /** Whether installation is complete */
  installComplete: boolean;
  /** Error message if operation failed */
  errorMessage: string | null;
  /** Success message after operation */
  successMessage: string | null;
  /** Verification result */
  verifyResult: VerifyResponse | null;
  /** Verification progress */
  verifyProgress: InstallProgress | null;
}

/**
 * Actions returned by the useSettings hook.
 */
export interface UseSettingsActions {
  /** Load settings from backend */
  loadSettings: () => Promise<void>;
  /** Update a setting value */
  updateSetting: <K extends keyof UserSettings>(
    key: K,
    value: UserSettings[K]
  ) => void;
  /** Save settings to backend */
  saveSettings: () => Promise<boolean>;
  /** Verify installation integrity */
  verifyInstallation: () => Promise<VerifyResponse | null>;
  /** Repair damaged installation files */
  repairInstallation: () => Promise<boolean>;
  /** Clear cached data */
  clearCache: () => Promise<boolean>;
  /** Check if running with admin privileges */
  checkAdminStatus: () => Promise<void>;
  /** Relaunch app with admin privileges */
  relaunchAsAdmin: () => Promise<void>;
  /** Clear error message */
  clearError: () => void;
  /** Clear success message */
  clearSuccess: () => void;
  /** Reset state */
  reset: () => void;
}

/**
 * Default user settings.
 */
const defaultSettings: UserSettings = {
  auto_launch: false,
  close_on_launch: false,
  check_updates_on_startup: true,
};

/**
 * Custom hook for managing user settings.
 *
 * @returns Tuple of [state, actions] for managing settings.
 */
export function useSettings(): [UseSettingsState, UseSettingsActions] {
  // Loading states
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isVerifying, setIsVerifying] = useState(false);
  const [isClearing, setIsClearing] = useState(false);
  const [isRepairing, setIsRepairing] = useState(false);

  // Admin state
  const [isAdmin, setIsAdmin] = useState(false);

  // Settings data
  const [settings, setSettings] = useState<UserSettings | null>(null);
  const [installPath, setInstallPath] = useState<string | null>(null);
  const [currentVersion, setCurrentVersion] = useState<string | null>(null);
  const [installComplete, setInstallComplete] = useState(false);

  // Messages
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // Verification
  const [verifyResult, setVerifyResult] = useState<VerifyResponse | null>(null);
  const [verifyProgress, setVerifyProgress] = useState<InstallProgress | null>(null);

  // Subscribe to verify progress events
  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const subscribe = async () => {
      unlisten = await onVerifyProgress((progress) => {
        setVerifyProgress(progress);
      });
    };

    subscribe();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  /**
   * Load settings from backend.
   */
  const handleLoadSettings = useCallback(async () => {
    setIsLoading(true);
    setErrorMessage(null);

    try {
      const response = await getSettings();
      setSettings(response.settings);
      setInstallPath(response.install_path);
      setCurrentVersion(response.current_version);
      setInstallComplete(response.install_complete);
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Failed to load settings";
      setErrorMessage(msg);
      // Set defaults on error
      setSettings(defaultSettings);
    } finally {
      setIsLoading(false);
    }
  }, []);

  /**
   * Update a single setting value.
   */
  const handleUpdateSetting = useCallback(
    <K extends keyof UserSettings>(key: K, value: UserSettings[K]) => {
      setSettings((prev) => {
        if (!prev) return prev;
        return { ...prev, [key]: value };
      });
    },
    []
  );

  /**
   * Save settings to backend.
   */
  const handleSaveSettings = useCallback(async (): Promise<boolean> => {
    if (!settings) {
      setErrorMessage("No settings to save");
      return false;
    }

    setIsSaving(true);
    setErrorMessage(null);
    setSuccessMessage(null);

    try {
      const response = await saveSettings({ settings });

      if (response.success) {
        setSuccessMessage("Settings saved successfully");
        return true;
      } else {
        setErrorMessage(response.error || "Failed to save settings");
        return false;
      }
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Failed to save settings";
      setErrorMessage(msg);
      return false;
    } finally {
      setIsSaving(false);
    }
  }, [settings]);

  /**
   * Verify installation integrity.
   */
  const handleVerifyInstallation = useCallback(async (): Promise<VerifyResponse | null> => {
    setIsVerifying(true);
    setErrorMessage(null);
    setSuccessMessage(null);
    setVerifyResult(null);
    setVerifyProgress(null);

    try {
      const result = await verifyInstallation();
      setVerifyResult(result);

      if (result.error) {
        // Verification process itself encountered an error
        setErrorMessage(result.error);
      } else if (result.success && result.invalid_files.length === 0) {
        // All files valid, no repair needed
        setSuccessMessage("All files verified successfully. Your installation is up to date.");
      } else if (result.invalid_files.length > 0) {
        // Some files need repair - this is informational, not an error
        // Don't set errorMessage, let the UI show the repair options
        setSuccessMessage(null);
      } else {
        // Default case: verification complete
        setSuccessMessage(`Verification complete: ${result.valid_files}/${result.total_files} files valid`);
      }

      return result;
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Failed to verify installation";
      setErrorMessage(msg);
      return null;
    } finally {
      setIsVerifying(false);
      setVerifyProgress(null);
    }
  }, []);

  /**
   * Repair damaged installation files.
   */
  const handleRepairInstallation = useCallback(async (): Promise<boolean> => {
    if (!installPath) {
      setErrorMessage("No installation path available. Please complete installation first.");
      return false;
    }

    // Check if there's actually anything to repair
    if (verifyResult && verifyResult.success && verifyResult.invalid_files.length === 0) {
      setSuccessMessage("No repair needed. All files are already valid.");
      return true;
    }

    setIsRepairing(true);
    setErrorMessage(null);
    setSuccessMessage(null);
    setVerifyProgress(null);

    try {
      const response = await startInstall(installPath);

      if (response.success) {
        // Clear the verify result since files have been repaired
        setVerifyResult(null);
        setSuccessMessage("Installation repaired successfully. All files are now valid.");
        // Reload settings to get updated state
        await handleLoadSettings();
        return true;
      } else {
        // Provide more descriptive error message for repair failures
        const errorMsg = response.error || "Failed to repair installation";
        if (errorMsg.toLowerCase().includes("permission") || errorMsg.toLowerCase().includes("access denied")) {
          setErrorMessage(`Repair failed: ${errorMsg}. Try running as administrator.`);
        } else if (errorMsg.toLowerCase().includes("network") || errorMsg.toLowerCase().includes("download")) {
          setErrorMessage(`Repair failed: ${errorMsg}. Check your internet connection and try again.`);
        } else {
          setErrorMessage(`Repair failed: ${errorMsg}`);
        }
        return false;
      }
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Failed to repair installation";
      // Provide context for common error scenarios
      if (msg.toLowerCase().includes("permission") || msg.toLowerCase().includes("access denied")) {
        setErrorMessage(`Repair failed: ${msg}. Try running as administrator.`);
      } else {
        setErrorMessage(`Repair failed: ${msg}`);
      }
      return false;
    } finally {
      setIsRepairing(false);
      setVerifyProgress(null);
    }
  }, [installPath, verifyResult, handleLoadSettings]);

  /**
   * Clear cached data.
   */
  const handleClearCache = useCallback(async (): Promise<boolean> => {
    setIsClearing(true);
    setErrorMessage(null);
    setSuccessMessage(null);

    try {
      const response = await clearCache();

      if (response.success) {
        setSuccessMessage("Cache cleared successfully");
        return true;
      } else {
        setErrorMessage(response.error || "Failed to clear cache");
        return false;
      }
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Failed to clear cache";
      setErrorMessage(msg);
      return false;
    } finally {
      setIsClearing(false);
    }
  }, []);

  /**
   * Check if running with admin privileges.
   */
  const handleCheckAdminStatus = useCallback(async () => {
    try {
      const admin = await isRunningAsAdmin();
      setIsAdmin(admin);
    } catch {
      // Default to false on error
      setIsAdmin(false);
    }
  }, []);

  /**
   * Relaunch app with admin privileges.
   */
  const handleRelaunchAsAdmin = useCallback(async () => {
    setErrorMessage(null);
    setSuccessMessage(null);

    try {
      await relaunchAsAdmin();
      // App will exit and relaunch with admin privileges
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Failed to relaunch as admin";
      // Provide more descriptive error messages for elevation failures
      if (msg.toLowerCase().includes("cancel") || msg.toLowerCase().includes("declined")) {
        setErrorMessage("Administrator access was declined. Some operations may not work correctly.");
      } else if (msg.toLowerCase().includes("uac") || msg.toLowerCase().includes("elevation")) {
        setErrorMessage("Unable to request administrator privileges. Please try running the launcher as administrator manually.");
      } else {
        setErrorMessage(`Failed to restart with administrator privileges: ${msg}`);
      }
    }
  }, []);

  /**
   * Clear error message.
   */
  const handleClearError = useCallback(() => {
    setErrorMessage(null);
  }, []);

  /**
   * Clear success message.
   */
  const handleClearSuccess = useCallback(() => {
    setSuccessMessage(null);
  }, []);

  /**
   * Reset state to initial values.
   */
  const reset = useCallback(() => {
    setIsLoading(false);
    setIsSaving(false);
    setIsVerifying(false);
    setIsClearing(false);
    setIsRepairing(false);
    setIsAdmin(false);
    setSettings(null);
    setInstallPath(null);
    setCurrentVersion(null);
    setInstallComplete(false);
    setErrorMessage(null);
    setSuccessMessage(null);
    setVerifyResult(null);
    setVerifyProgress(null);
  }, []);

  // Load settings and check admin status on mount
  useEffect(() => {
    handleLoadSettings();
    handleCheckAdminStatus();
  }, [handleLoadSettings, handleCheckAdminStatus]);

  // Assemble state object
  const state: UseSettingsState = {
    isLoading,
    isSaving,
    isVerifying,
    isClearing,
    isRepairing,
    isAdmin,
    settings,
    installPath,
    currentVersion,
    installComplete,
    errorMessage,
    successMessage,
    verifyResult,
    verifyProgress,
  };

  // Assemble actions object
  const actions: UseSettingsActions = {
    loadSettings: handleLoadSettings,
    updateSetting: handleUpdateSetting,
    saveSettings: handleSaveSettings,
    verifyInstallation: handleVerifyInstallation,
    repairInstallation: handleRepairInstallation,
    clearCache: handleClearCache,
    checkAdminStatus: handleCheckAdminStatus,
    relaunchAsAdmin: handleRelaunchAsAdmin,
    clearError: handleClearError,
    clearSuccess: handleClearSuccess,
    reset,
  };

  return [state, actions];
}
