/**
 * Custom hook for managing the migration flow.
 *
 * Handles scanning for existing installations, presenting choices,
 * and performing file-copy migration with progress tracking.
 */

import { useState, useCallback, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";

import {
  scanForMigrations,
  detectAtPath,
  startMigration,
  useInPlace,
  removeOldInstallation,
  onMigrationProgress,
  getRecommendedInstallPath,
  isRunningAsAdmin,
  relaunchAsAdmin,
  validateInstallPath,
} from "../lib/api";

import type {
  DetectionResult,
  MigrationProgress,
  PathValidationResult,
} from "../lib/types";

export type MigrationStep =
  | "scanning"
  | "decision"
  | "choose_destination"
  | "migrating"
  | "complete"
  | "not_found"
  | "error";

export interface UseMigrationState {
  /** Current step in the migration flow */
  step: MigrationStep;
  /** Detected installations from auto-scan */
  detected: DetectionResult[];
  /** The installation the user selected to migrate from */
  selectedSource: DetectionResult | null;
  /** Destination path for file copy */
  destinationPath: string;
  /** Validation result for the destination path */
  destinationValidation: PathValidationResult | null;
  /** Migration progress */
  progress: MigrationProgress | null;
  /** Error message */
  error: string | null;
  /** Whether the app is running as admin */
  isAdmin: boolean;
}

export interface UseMigrationActions {
  /** Start scanning brand-configured paths */
  scan: () => Promise<void>;
  /** Browse for an installation manually */
  browseForInstallation: () => Promise<void>;
  /** Select a detected installation as the migration source */
  selectSource: (result: DetectionResult) => void;
  /** Set the destination path for file copy */
  setDestinationPath: (path: string) => void;
  /** Navigate to a specific migration step */
  setStep: (step: MigrationStep) => void;
  /** Choose "Copy to new location" */
  copyToNewLocation: () => Promise<void>;
  /** Choose "Use in place" */
  adoptInPlace: () => Promise<void>;
  /** Choose "Skip — install fresh" */
  skip: () => void;
  /** Relaunch as admin for elevation */
  relaunchAsAdmin: () => Promise<void>;
  /** Remove the old installation directory */
  removeOldInstall: () => Promise<void>;
  /** Reset to initial state */
  reset: () => void;
}

export function useMigration(
  onComplete: () => void,
  onSkip: () => void
): [UseMigrationState, UseMigrationActions] {
  const [step, setStep] = useState<MigrationStep>("scanning");
  const [detected, setDetected] = useState<DetectionResult[]>([]);
  const [selectedSource, setSelectedSource] = useState<DetectionResult | null>(null);
  const [destinationPath, setDestinationPathState] = useState<string>("");
  const [destinationValidation, setDestinationValidation] = useState<PathValidationResult | null>(null);
  const [progress, setProgress] = useState<MigrationProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isAdmin, setIsAdmin] = useState(false);

  // Check admin status on mount
  useEffect(() => {
    isRunningAsAdmin().then(setIsAdmin).catch(() => setIsAdmin(false));
  }, []);

  // Set default destination path
  useEffect(() => {
    getRecommendedInstallPath()
      .then(setDestinationPathState)
      .catch(() => setDestinationPathState("C:\\Games\\Game"));
  }, []);

  // Listen for migration progress events
  useEffect(() => {
    let unlisten: (() => void) | null = null;

    const subscribe = async () => {
      unlisten = await onMigrationProgress((p) => {
        setProgress(p);
        if (p.files_copied === p.files_total && p.files_total > 0) {
          setStep("complete");
        }
      });
    };

    subscribe();
    return () => { if (unlisten) unlisten(); };
  }, []);

  // Validate destination when it changes
  useEffect(() => {
    if (!destinationPath) {
      setDestinationValidation(null);
      return;
    }
    validateInstallPath(destinationPath)
      .then(setDestinationValidation)
      .catch(() => setDestinationValidation(null));
  }, [destinationPath]);

  const scan = useCallback(async () => {
    setStep("scanning");
    setError(null);
    try {
      const response = await scanForMigrations();
      if (response.detected.length > 0) {
        setDetected(response.detected);
        // Auto-select the first high/medium confidence result
        setSelectedSource(response.detected[0]);
        setStep("decision");
      } else {
        setStep("not_found");
      }
    } catch {
      setStep("not_found");
    }
  }, []);

  const browseForInstallation = useCallback(async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select Existing Installation Directory",
      });

      if (selected && typeof selected === "string") {
        const result = await detectAtPath(selected);
        if (result.detected) {
          setDetected([result]);
          setSelectedSource(result);
          setStep("decision");
        } else {
          setError("No recognizable UO installation found at that location.");
          setStep("not_found");
        }
      }
    } catch {
      // User cancelled
    }
  }, []);

  const selectSource = useCallback((result: DetectionResult) => {
    setSelectedSource(result);
  }, []);

  const setDestinationPath = useCallback((path: string) => {
    setDestinationPathState(path);
  }, []);

  const copyToNewLocation = useCallback(async () => {
    if (!selectedSource?.install_path || !destinationPath) return;

    setStep("migrating");
    setError(null);
    setProgress(null);

    try {
      await startMigration(selectedSource.install_path, destinationPath);
      setStep("complete");
      onComplete();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setStep("error");
    }
  }, [selectedSource, destinationPath, onComplete]);

  const adoptInPlace = useCallback(async () => {
    if (!selectedSource?.install_path) return;

    setError(null);
    try {
      await useInPlace(selectedSource.install_path);
      onComplete();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      setStep("error");
    }
  }, [selectedSource, onComplete]);

  const skip = useCallback(() => {
    onSkip();
  }, [onSkip]);

  const handleRelaunchAsAdmin = useCallback(async () => {
    try {
      await relaunchAsAdmin();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to relaunch as admin");
    }
  }, []);

  const removeOldInstall = useCallback(async () => {
    if (!selectedSource?.install_path) return;
    try {
      await removeOldInstallation(selectedSource.install_path);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to remove old installation");
    }
  }, [selectedSource]);

  const reset = useCallback(() => {
    setStep("scanning");
    setDetected([]);
    setSelectedSource(null);
    setProgress(null);
    setError(null);
  }, []);

  const state: UseMigrationState = {
    step,
    detected,
    selectedSource,
    destinationPath,
    destinationValidation,
    progress,
    error,
    isAdmin,
  };

  const actions: UseMigrationActions = {
    scan,
    browseForInstallation,
    selectSource,
    setDestinationPath,
    setStep,
    copyToNewLocation,
    adoptInPlace,
    skip,
    relaunchAsAdmin: handleRelaunchAsAdmin,
    removeOldInstall,
    reset,
  };

  return [state, actions];
}
