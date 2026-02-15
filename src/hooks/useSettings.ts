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
  onVerifyProgress,
} from "../lib/api";

import type {
  UserSettings,
  GetSettingsResponse,
  SaveResponse,
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
  /** Clear cached data */
  clearCache: () => Promise<boolean>;
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

      if (result.success) {
        setSuccessMessage(`Verification complete: ${result.valid_files}/${result.total_files} files valid`);
      } else if (result.error) {
        setErrorMessage(result.error);
      } else {
        setErrorMessage(`${result.invalid_files.length} files need repair`);
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
    setSettings(null);
    setInstallPath(null);
    setCurrentVersion(null);
    setInstallComplete(false);
    setErrorMessage(null);
    setSuccessMessage(null);
    setVerifyResult(null);
    setVerifyProgress(null);
  }, []);

  // Load settings on mount
  useEffect(() => {
    handleLoadSettings();
  }, [handleLoadSettings]);

  // Assemble state object
  const state: UseSettingsState = {
    isLoading,
    isSaving,
    isVerifying,
    isClearing,
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
    clearCache: handleClearCache,
    clearError: handleClearError,
    clearSuccess: handleClearSuccess,
    reset,
  };

  return [state, actions];
}
