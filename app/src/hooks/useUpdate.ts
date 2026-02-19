/**
 * Custom hook for managing the update process.
 *
 * Provides state and actions for the UpdateProgress component,
 * including update checking, progress tracking, and error handling.
 *
 * Uses React Context to share a single state instance across all consumers.
 */

import React, { useState, useCallback, useEffect, useMemo, createContext, useContext } from "react";

import {
  checkForUpdates,
  startUpdate,
  dismissUpdate,
  onUpdateProgress,
  getUpdateProgress,
} from "../lib/api";

import type {
  UpdateState,
  UpdateProgress,
  UpdateCheckResponse,
} from "../lib/types";

/**
 * State returned by the useUpdate hook.
 */
export interface UseUpdateState {
  /** Whether we're checking for updates */
  isChecking: boolean;
  /** Whether an update is available */
  updateAvailable: boolean;
  /** Whether we're currently updating */
  isUpdating: boolean;
  /** Current update progress */
  progress: UpdateProgress | null;
  /** Update check result */
  checkResult: UpdateCheckResponse | null;
  /** Error message if update failed */
  errorMessage: string | null;
  /** Whether the update completed successfully */
  isComplete: boolean;
  /** Whether the update was rolled back */
  wasRolledBack: boolean;
  /** Whether auto-launch is pending after update completion (one-shot flag) */
  autoLaunchPending: boolean;
}

/**
 * Actions returned by the useUpdate hook.
 */
export interface UseUpdateActions {
  /** Check for available updates */
  checkForUpdates: () => Promise<void>;
  /** Start the update process */
  startUpdate: () => Promise<void>;
  /** Dismiss the update notification */
  dismissUpdate: () => Promise<void>;
  /** Retry update after an error */
  retryUpdate: () => Promise<void>;
  /** Reset the update state */
  reset: () => void;
  /** Set auto-launch pending flag (called when update completes with auto_launch enabled) */
  setAutoLaunchPending: (pending: boolean) => void;
  /** Clear auto-launch pending flag (called after launch attempt) */
  clearAutoLaunchPending: () => void;
}

/**
 * Default initial progress state.
 */
const initialProgress: UpdateProgress = {
  state: "Idle" as UpdateState,
  total_files: 0,
  processed_files: 0,
  total_bytes: 0,
  downloaded_bytes: 0,
  current_file: null,
  speed_bps: 0,
  eta_secs: 0,
  target_version: null,
  error_message: null,
};

/**
 * Context for sharing update state across components.
 */
const UpdateContext = createContext<[UseUpdateState, UseUpdateActions] | null>(null);

/**
 * Internal hook that manages update state.
 * This is the implementation that holds the actual state.
 *
 * @returns Tuple of [state, actions] for managing updates.
 */
function useUpdateInternal(): [UseUpdateState, UseUpdateActions] {
  // Update state
  const [isChecking, setIsChecking] = useState(false);
  const [updateAvailable, setUpdateAvailable] = useState(false);
  const [isUpdating, setIsUpdating] = useState(false);
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [checkResult, setCheckResult] = useState<UpdateCheckResponse | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [isComplete, setIsComplete] = useState(false);
  const [wasRolledBack, setWasRolledBack] = useState(false);
  const [autoLaunchPending, setAutoLaunchPendingState] = useState(false);

  // Subscribe to update progress events
  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const subscribe = async () => {
      unlisten = await onUpdateProgress((newProgress) => {
        setProgress(newProgress);

        // Handle state transitions
        if (newProgress.state === "Completed") {
          setIsUpdating(false);
          setIsComplete(true);
          setUpdateAvailable(false);
        } else if (newProgress.state === "Failed") {
          setIsUpdating(false);
          setErrorMessage(
            newProgress.error_message || "Update failed unexpectedly"
          );
        } else if (newProgress.state === "RollingBack") {
          setWasRolledBack(true);
        }
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
   * Check for available updates.
   */
  const handleCheckForUpdates = useCallback(async () => {
    setIsChecking(true);
    setErrorMessage(null);

    try {
      const result = await checkForUpdates();
      setCheckResult(result);
      setUpdateAvailable(result.update_available);

      if (result.error) {
        setErrorMessage(result.error);
      }
    } catch (error) {
      setErrorMessage(
        error instanceof Error ? error.message : "Failed to check for updates"
      );
    } finally {
      setIsChecking(false);
    }
  }, []);

  /**
   * Start the update process.
   */
  const handleStartUpdate = useCallback(async () => {
    if (!updateAvailable) {
      return;
    }

    setProgress(initialProgress);
    setErrorMessage(null);
    setIsUpdating(true);
    setIsComplete(false);
    setWasRolledBack(false);

    try {
      const result = await startUpdate();

      if (result.success) {
        // Progress events will handle the transition to complete
        setIsComplete(true);
        setUpdateAvailable(false);
      } else {
        setErrorMessage(result.error || "Update failed");
        setIsUpdating(false);

        if (result.rolled_back) {
          setWasRolledBack(true);
        }
      }
    } catch (error) {
      setErrorMessage(
        error instanceof Error ? error.message : "Update failed"
      );
      setIsUpdating(false);
    }
  }, [updateAvailable]);

  /**
   * Dismiss the update notification.
   */
  const handleDismissUpdate = useCallback(async () => {
    try {
      await dismissUpdate();
      setUpdateAvailable(false);
      setCheckResult(null);
    } catch (error) {
      // Ignore dismiss errors
    }
  }, []);

  /**
   * Retry update after an error.
   */
  const handleRetryUpdate = useCallback(async () => {
    setErrorMessage(null);
    setProgress(null);
    setWasRolledBack(false);
    await handleStartUpdate();
  }, [handleStartUpdate]);

  /**
   * Reset the update state.
   */
  const reset = useCallback(() => {
    setIsChecking(false);
    setUpdateAvailable(false);
    setIsUpdating(false);
    setProgress(null);
    setCheckResult(null);
    setErrorMessage(null);
    setIsComplete(false);
    setWasRolledBack(false);
    setAutoLaunchPendingState(false);
  }, []);

  /**
   * Set auto-launch pending flag.
   * Called when update completes and auto_launch setting is enabled.
   */
  const setAutoLaunchPending = useCallback((pending: boolean) => {
    setAutoLaunchPendingState(pending);
  }, []);

  /**
   * Clear auto-launch pending flag.
   * Called after launch attempt to prevent duplicate launches.
   */
  const clearAutoLaunchPending = useCallback(() => {
    setAutoLaunchPendingState(false);
  }, []);

  // Assemble state object
  const state: UseUpdateState = {
    isChecking,
    updateAvailable,
    isUpdating,
    progress,
    checkResult,
    errorMessage,
    isComplete,
    wasRolledBack,
    autoLaunchPending,
  };

  // Memoize actions object to prevent unnecessary re-renders (Bug fix)
  const actions: UseUpdateActions = useMemo(
    () => ({
      checkForUpdates: handleCheckForUpdates,
      startUpdate: handleStartUpdate,
      dismissUpdate: handleDismissUpdate,
      retryUpdate: handleRetryUpdate,
      reset,
      setAutoLaunchPending,
      clearAutoLaunchPending,
    }),
    [handleCheckForUpdates, handleStartUpdate, handleDismissUpdate, handleRetryUpdate, reset, setAutoLaunchPending, clearAutoLaunchPending]
  );

  return [state, actions];
}

/**
 * Props for the UpdateProvider component.
 */
interface UpdateProviderProps {
  children: React.ReactNode;
}

/**
 * Provider component that shares update state across all consumers.
 * Wrap your app with this provider to enable shared update state.
 *
 * @param props - Provider props containing children.
 * @returns Provider component.
 */
export function UpdateProvider({ children }: UpdateProviderProps): React.ReactElement {
  const value = useUpdateInternal();
  return React.createElement(UpdateContext.Provider, { value }, children);
}

/**
 * Consumer hook for accessing shared update state.
 * Must be used within an UpdateProvider.
 *
 * @returns Tuple of [state, actions] for managing updates.
 * @throws Error if used outside of UpdateProvider.
 */
export function useUpdate(): [UseUpdateState, UseUpdateActions] {
  const context = useContext(UpdateContext);
  if (!context) {
    throw new Error("useUpdate must be used within UpdateProvider");
  }
  return context;
}

/**
 * Check for updates and return the result.
 *
 * @returns Update check response.
 */
export async function checkUpdatesOnStartup(): Promise<UpdateCheckResponse> {
  return checkForUpdates();
}

/**
 * Get the current update progress from the backend.
 *
 * @returns Current update progress or null.
 */
export async function getCurrentUpdateProgress(): Promise<UpdateProgress | null> {
  return getUpdateProgress();
}
