/**
 * Custom hook for managing the game launch process.
 *
 * Provides state and actions for the LaunchButton component,
 * including validation, launching, and error handling.
 */

import { useState, useCallback } from "react";

import {
  launchGame,
  validateClient,
  gameClosed,
} from "../lib/api";

import type {
  LaunchGameRequest,
  LaunchResponse,
  ValidateClientResponse,
} from "../lib/types";

/**
 * State returned by the useLaunch hook.
 */
export interface UseLaunchState {
  /** Whether we're currently validating the client */
  isValidating: boolean;
  /** Whether the client is valid and launchable */
  isValid: boolean | null;
  /** Whether we're currently launching */
  isLaunching: boolean;
  /** Whether the game is running */
  isGameRunning: boolean;
  /** Error message if validation or launch failed */
  errorMessage: string | null;
  /** Validation result from last check */
  validationResult: ValidateClientResponse | null;
  /** Launch result from last launch */
  launchResult: LaunchResponse | null;
}

/**
 * Actions returned by the useLaunch hook.
 */
export interface UseLaunchActions {
  /** Validate that the client can be launched */
  validateClient: () => Promise<boolean>;
  /** Launch the game */
  launch: (request?: LaunchGameRequest) => Promise<LaunchResponse>;
  /** Mark the game as closed */
  markGameClosed: () => Promise<void>;
  /** Reset the launch state */
  reset: () => void;
  /** Clear error message */
  clearError: () => void;
}

/**
 * Custom hook for managing game launching.
 *
 * @returns Tuple of [state, actions] for managing game launch.
 */
export function useLaunch(): [UseLaunchState, UseLaunchActions] {
  // Launch state
  const [isValidating, setIsValidating] = useState(false);
  const [isValid, setIsValid] = useState<boolean | null>(null);
  const [isLaunching, setIsLaunching] = useState(false);
  const [isGameRunning, setIsGameRunning] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [validationResult, setValidationResult] = useState<ValidateClientResponse | null>(null);
  const [launchResult, setLaunchResult] = useState<LaunchResponse | null>(null);

  /**
   * Validate that the client can be launched.
   */
  const handleValidateClient = useCallback(async (): Promise<boolean> => {
    setIsValidating(true);
    setErrorMessage(null);

    try {
      const result = await validateClient();
      setValidationResult(result);
      setIsValid(result.is_valid);

      if (!result.is_valid && result.error) {
        setErrorMessage(result.error);
      }

      return result.is_valid;
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Failed to validate client";
      setErrorMessage(msg);
      setIsValid(false);
      return false;
    } finally {
      setIsValidating(false);
    }
  }, []);

  /**
   * Launch the game.
   */
  const handleLaunch = useCallback(async (request?: LaunchGameRequest): Promise<LaunchResponse> => {
    setIsLaunching(true);
    setErrorMessage(null);

    try {
      const result = await launchGame(request);
      setLaunchResult(result);

      if (result.success) {
        setIsGameRunning(true);
      } else if (result.error) {
        setErrorMessage(result.error);
      }

      return result;
    } catch (error) {
      const msg = error instanceof Error ? error.message : "Failed to launch game";
      setErrorMessage(msg);

      const errorResult: LaunchResponse = {
        success: false,
        pid: null,
        error: msg,
        should_close_launcher: false,
      };
      setLaunchResult(errorResult);
      return errorResult;
    } finally {
      setIsLaunching(false);
    }
  }, []);

  /**
   * Mark the game as closed.
   */
  const handleGameClosed = useCallback(async () => {
    try {
      await gameClosed();
      setIsGameRunning(false);
      setLaunchResult(null);
    } catch (error) {
      // Ignore errors when marking game as closed
    }
  }, []);

  /**
   * Reset the launch state.
   */
  const reset = useCallback(() => {
    setIsValidating(false);
    setIsValid(null);
    setIsLaunching(false);
    setIsGameRunning(false);
    setErrorMessage(null);
    setValidationResult(null);
    setLaunchResult(null);
  }, []);

  /**
   * Clear the error message.
   */
  const clearError = useCallback(() => {
    setErrorMessage(null);
  }, []);

  // Assemble state object
  const state: UseLaunchState = {
    isValidating,
    isValid,
    isLaunching,
    isGameRunning,
    errorMessage,
    validationResult,
    launchResult,
  };

  // Assemble actions object
  const actions: UseLaunchActions = {
    validateClient: handleValidateClient,
    launch: handleLaunch,
    markGameClosed: handleGameClosed,
    reset,
    clearError,
  };

  return [state, actions];
}
