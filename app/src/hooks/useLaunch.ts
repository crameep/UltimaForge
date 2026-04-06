/**
 * Custom hook for managing the game launch process.
 *
 * Provides state and actions for the LaunchButton component,
 * including validation, launching, and error handling.
 */

import { useState, useCallback, useEffect } from "react";

import {
  getCuoConfig,
  getSettings,
  launchGame,
  validateClient,
  gameClosed,
  onClientCountChanged,
} from "../lib/api";

import type {
  AssistantKind,
  CuoConfig,
  LaunchGameRequest,
  LaunchResponse,
  ServerChoice,
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
  /** CUO config if configured */
  cuoConfig: CuoConfig | null;
  /** Selected server choice */
  selectedServer: ServerChoice;
  /** Selected assistant choice */
  selectedAssistant: AssistantKind;
  /** Selected client count */
  clientCount: number;
  /** Number of clients currently running */
  runningClients: number;
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
  /** Update selected server */
  setSelectedServer: (server: ServerChoice) => void;
  /** Update selected assistant */
  setSelectedAssistant: (assistant: AssistantKind) => void;
  /** Update client count (1-3) */
  setClientCount: (count: number) => void;
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
  const [cuoConfig, setCuoConfig] = useState<CuoConfig | null>(null);
  const [selectedServer, setSelectedServer] = useState<ServerChoice>("live");
  const [selectedAssistant, setSelectedAssistant] = useState<AssistantKind>("razor_enhanced");
  const [clientCount, setClientCount] = useState<number>(1);
  const [runningClients, setRunningClients] = useState<number>(0);

  useEffect(() => {
    // Load CUO config for available options
    getCuoConfig()
      .then((cfg) => {
        if (cfg) setCuoConfig(cfg);
      })
      .catch(() => {});

    // Load saved preferences (assistant, server, client count)
    getSettings()
      .then((s) => {
        if (s.selected_assistant) setSelectedAssistant(s.selected_assistant);
        if (s.selected_server) setSelectedServer(s.selected_server);
        if (s.client_count) setClientCount(s.client_count);
      })
      .catch(() => {
        // Fallback to CUO defaults if settings unavailable
        getCuoConfig()
          .then((cfg) => {
            if (cfg) {
              setSelectedAssistant(cfg.default_assistant);
              setSelectedServer(cfg.default_server);
            }
          })
          .catch(() => {});
      });
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | null = null;

    onClientCountChanged((remaining) => {
      setRunningClients(remaining);
      setIsGameRunning(remaining > 0);
    }).then((fn) => {
      unlisten = fn;
    }).catch(() => {});

    return () => {
      if (unlisten) unlisten();
    };
  }, []);

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
      const msg = typeof error === "string" ? error : error instanceof Error ? error.message : "Failed to validate client";
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
      const mergedRequest: LaunchGameRequest = {
        ...request,
        client_count: clientCount,
        server_choice: selectedServer,
        assistant_choice: selectedAssistant,
      };

      const result = await launchGame(mergedRequest);
      setLaunchResult(result);

      if (result.success) {
        setIsGameRunning(result.running_clients > 0);
        setRunningClients(result.running_clients);
      } else if (result.error) {
        setErrorMessage(result.error);
      }

      return result;
    } catch (error) {
      const msg = typeof error === "string" ? error : error instanceof Error ? error.message : "Failed to launch game";
      setErrorMessage(msg);

      const errorResult: LaunchResponse = {
        success: false,
        pid: null,
        error: msg,
        should_close_launcher: false,
        running_clients: 0,
      };
      setLaunchResult(errorResult);
      return errorResult;
    } finally {
      setIsLaunching(false);
    }
  }, [clientCount, selectedAssistant, selectedServer]);

  /**
   * Mark the game as closed.
   */
  const handleGameClosed = useCallback(async () => {
    try {
      await gameClosed();
      setIsGameRunning(false);
      setRunningClients(0);
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
    setRunningClients(0);
  }, []);

  /**
   * Clear the error message.
   */
  const clearError = useCallback(() => {
    setErrorMessage(null);
  }, []);

  const handleSetClientCount = useCallback((count: number) => {
    setClientCount(Math.min(3, Math.max(1, count)));
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
    cuoConfig,
    selectedServer,
    selectedAssistant,
    clientCount,
    runningClients,
  };

  // Assemble actions object
  const actions: UseLaunchActions = {
    validateClient: handleValidateClient,
    launch: handleLaunch,
    markGameClosed: handleGameClosed,
    reset,
    clearError,
    setSelectedServer,
    setSelectedAssistant,
    setClientCount: handleSetClientCount,
  };

  return [state, actions];
}
