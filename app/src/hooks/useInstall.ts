/**
 * Custom hook for managing the installation process.
 *
 * Provides state and actions for the InstallWizard component,
 * including path validation, installation progress, and error handling.
 */

import { useState, useCallback, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import {
  checkInstallStatus,
  validateInstallPath,
  startInstall,
  onInstallProgress,
  getBrandConfig,
  getLauncherDir,
  relaunchAsAdmin,
  getRecommendedInstallPath,
} from "../lib/api";

import type {
  InstallState,
  InstallProgress,
  PathValidationResult,
  InstallStatusResponse,
} from "../lib/types";

/**
 * Wizard step identifiers.
 */
export type WizardStep =
  | "welcome"
  | "directory"
  | "eula"
  | "installing"
  | "complete"
  | "error";

/**
 * State returned by the useInstall hook.
 */
export interface UseInstallState {
  /** Current wizard step */
  currentStep: WizardStep;
  /** Selected installation path */
  installPath: string;
  /** Path validation result */
  pathValidation: PathValidationResult | null;
  /** Whether path validation is in progress */
  isValidating: boolean;
  /** Installation progress */
  progress: InstallProgress | null;
  /** Error message if installation failed */
  errorMessage: string | null;
  /** Whether the user has accepted the EULA */
  eulaAccepted: boolean;
}

/**
 * Actions returned by the useInstall hook.
 */
export interface UseInstallActions {
  /** Navigate to the next step */
  nextStep: () => void;
  /** Navigate to the previous step */
  prevStep: () => void;
  /** Go to a specific step */
  goToStep: (step: WizardStep) => void;
  /** Open the directory picker dialog */
  pickDirectory: () => Promise<void>;
  /** Set the installation path manually */
  setInstallPath: (path: string) => void;
  /** Accept or reject the EULA */
  setEulaAccepted: (accepted: boolean) => void;
  /** Start the installation process */
  startInstallation: () => Promise<void>;
  /** Retry installation after an error */
  retryInstallation: () => void;
  /** Reset the wizard to the initial state */
  reset: () => void;
  /** Relaunch the app with admin privileges */
  relaunchAsAdmin: () => Promise<void>;
  /** Use a recommended AppData path instead */
  useRecommendedPath: () => Promise<void>;
}

/**
 * Default initial progress state.
 */
const initialProgress: InstallProgress = {
  state: "Idle" as InstallState,
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
 * Default path validation result.
 */
const defaultValidation: PathValidationResult = {
  is_valid: false,
  reason: null,
  exists: false,
  is_empty: true,
  available_space: 0,
  has_sufficient_space: false,
  is_writable: false,
  requires_elevation: false,
};

/**
 * Custom hook for managing the installation wizard.
 *
 * @returns Tuple of [state, actions] for managing installation.
 */
export function useInstall(): [UseInstallState, UseInstallActions] {
  // Wizard state
  const [currentStep, setCurrentStep] = useState<WizardStep>("welcome");
  const [installPath, setInstallPathState] = useState<string>("");
  const [pathValidation, setPathValidation] =
    useState<PathValidationResult | null>(null);
  const [isValidating, setIsValidating] = useState(false);
  const [progress, setProgress] = useState<InstallProgress | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [eulaAccepted, setEulaAcceptedState] = useState(false);

  // Set default install path based on launcher location + branding
  useEffect(() => {
    const setDefaultPath = async () => {
      if (installPath) return; // Don't override if already set

      let pathToSet: string;

      try {
        const brand = await getBrandConfig();
        const serverName = brand.server_name || brand.display_name || "Game";
        const launcherDir = await getLauncherDir();

        // Default to {launcher_dir}\{ServerName}
        // e.g., C:\Program Files\Unchained Patcher\Unchained
        pathToSet = `${launcherDir}\\${serverName}`;
      } catch (err) {
        console.warn("Failed to get default install path:", err);
        // Fallback to generic path
        pathToSet = "C:\\Games\\Game";
      }

      // Set the path state
      setInstallPathState(pathToSet);

      // Validate the default path immediately so user sees validation status on wizard load
      setIsValidating(true);
      try {
        const result = await validateInstallPath(pathToSet);
        setPathValidation(result);
      } catch (error) {
        setPathValidation({
          ...defaultValidation,
          is_valid: false,
          reason:
            error instanceof Error ? error.message : "Failed to validate path",
        });
      } finally {
        setIsValidating(false);
      }
    };

    setDefaultPath();
  }, []); // Run once on mount

  // Subscribe to install progress events
  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const subscribe = async () => {
      unlisten = await onInstallProgress((newProgress) => {
        setProgress(newProgress);

        // Handle state transitions
        if (newProgress.state === "Completed") {
          setCurrentStep("complete");
        } else if (newProgress.state === "Failed") {
          setErrorMessage(
            newProgress.error_message || "Installation failed unexpectedly"
          );
          setCurrentStep("error");
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
   * Validate the installation path.
   */
  const validatePath = useCallback(async (path: string) => {
    if (!path) {
      setPathValidation(null);
      return;
    }

    setIsValidating(true);
    try {
      const result = await validateInstallPath(path);
      setPathValidation(result);
    } catch (error) {
      setPathValidation({
        ...defaultValidation,
        is_valid: false,
        reason:
          error instanceof Error ? error.message : "Failed to validate path",
      });
    } finally {
      setIsValidating(false);
    }
  }, []);

  /**
   * Set the installation path and trigger validation.
   */
  const setInstallPath = useCallback(
    (path: string) => {
      setInstallPathState(path);
      validatePath(path);
    },
    [validatePath]
  );

  /**
   * Open the native directory picker dialog.
   */
  const pickDirectory = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Installation Directory",
      });

      if (selected && typeof selected === "string") {
        setInstallPath(selected);
      }
    } catch (error) {
      // User cancelled or dialog error - ignore
    }
  }, [setInstallPath]);

  /**
   * Navigate to the next wizard step.
   */
  const nextStep = useCallback(() => {
    const stepOrder: WizardStep[] = [
      "welcome",
      "directory",
      "eula",
      "installing",
      "complete",
    ];
    const currentIndex = stepOrder.indexOf(currentStep);

    if (currentIndex < stepOrder.length - 1) {
      setCurrentStep(stepOrder[currentIndex + 1]);
    }
  }, [currentStep]);

  /**
   * Navigate to the previous wizard step.
   */
  const prevStep = useCallback(() => {
    const stepOrder: WizardStep[] = [
      "welcome",
      "directory",
      "eula",
      "installing",
      "complete",
    ];
    const currentIndex = stepOrder.indexOf(currentStep);

    if (currentIndex > 0) {
      setCurrentStep(stepOrder[currentIndex - 1]);
    }
  }, [currentStep]);

  /**
   * Go to a specific wizard step.
   */
  const goToStep = useCallback((step: WizardStep) => {
    setCurrentStep(step);
  }, []);

  /**
   * Set EULA acceptance state.
   */
  const setEulaAccepted = useCallback((accepted: boolean) => {
    setEulaAcceptedState(accepted);
  }, []);

  /**
   * Start the installation process.
   */
  const startInstallation = useCallback(async () => {
    if (!installPath || !pathValidation?.is_valid) {
      setErrorMessage("Please select a valid installation directory");
      return;
    }

    setProgress(initialProgress);
    setErrorMessage(null);
    setCurrentStep("installing");

    try {
      const result = await startInstall(installPath);

      if (result.success) {
        // Progress events will handle the transition to "complete"
      } else {
        setErrorMessage(result.error || "Installation failed");
        setCurrentStep("error");
      }
    } catch (error) {
      setErrorMessage(
        error instanceof Error ? error.message : "Installation failed"
      );
      setCurrentStep("error");
    }
  }, [installPath, pathValidation]);

  /**
   * Retry installation after an error.
   */
  const retryInstallation = useCallback(() => {
    setErrorMessage(null);
    setProgress(null);
    setCurrentStep("directory");
  }, []);

  /**
   * Reset the wizard to initial state.
   */
  const reset = useCallback(() => {
    setCurrentStep("welcome");
    setInstallPathState("");
    setPathValidation(null);
    setIsValidating(false);
    setProgress(null);
    setErrorMessage(null);
    setEulaAcceptedState(false);
  }, []);

  /**
   * Relaunch the app with administrator privileges.
   */
  const handleRelaunchAsAdmin = useCallback(async () => {
    try {
      await relaunchAsAdmin();
      // The app will exit and relaunch, so this won't return
    } catch (error) {
      // Show the error message to the user
      const message = error instanceof Error
        ? error.message
        : "Failed to relaunch as administrator";

      // Use alert for immediate feedback, then also set error state
      alert(message);
      setErrorMessage(message);
    }
  }, []);

  /**
   * Use a recommended AppData path instead of the current path.
   */
  const useRecommendedPath = useCallback(async () => {
    try {
      const recommendedPath = await getRecommendedInstallPath();
      setInstallPath(recommendedPath);
    } catch (error) {
      console.error("Failed to set recommended path:", error);
      setErrorMessage(
        error instanceof Error
          ? error.message
          : "Failed to get recommended path"
      );
    }
  }, [setInstallPath]);

  // Assemble state object
  const state: UseInstallState = {
    currentStep,
    installPath,
    pathValidation,
    isValidating,
    progress,
    errorMessage,
    eulaAccepted,
  };

  // Assemble actions object
  const actions: UseInstallActions = {
    nextStep,
    prevStep,
    goToStep,
    pickDirectory,
    setInstallPath,
    setEulaAccepted,
    startInstallation,
    retryInstallation,
    reset,
    relaunchAsAdmin: handleRelaunchAsAdmin,
    useRecommendedPath,
  };

  return [state, actions];
}

/**
 * Check if installation is needed on app startup.
 *
 * @returns Installation status response.
 */
export async function checkNeedsInstall(): Promise<InstallStatusResponse> {
  return checkInstallStatus();
}
