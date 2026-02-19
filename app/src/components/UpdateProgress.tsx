/**
 * UpdateProgress Component
 *
 * Displays progress visualization during the update process.
 * Shows download progress, file counts, speed, and ETA.
 */

import { useUpdate } from "../hooks/useUpdate";
import type { UpdateProgress as UpdateProgressType, UpdateCheckResponse } from "../lib/types";
import {
  formatBytes,
  formatEta,
  calculatePercentage,
} from "../lib/types";
import "./UpdateProgress.css";

interface UpdateProgressProps {
  /** Callback when update completes successfully */
  onComplete?: () => void;
  /** Callback when user dismisses the update */
  onDismiss?: () => void;
  /** Update check result for display */
  checkResult?: UpdateCheckResponse | null;
  /** Whether to show the update available banner */
  showBanner?: boolean;
}

/**
 * Get human-readable text for update state.
 */
function getStateText(state: string): string {
  switch (state) {
    case "Idle":
      return "Preparing...";
    case "Checking":
      return "Checking for updates...";
    case "Downloading":
      return "Downloading files...";
    case "Verifying":
      return "Verifying files...";
    case "BackingUp":
      return "Creating backup...";
    case "Applying":
      return "Applying update...";
    case "RollingBack":
      return "Rolling back changes...";
    case "Completed":
      return "Update complete!";
    case "Failed":
      return "Update failed";
    default:
      return state;
  }
}

/**
 * Banner showing that an update is available.
 */
function UpdateAvailableBanner({
  checkResult,
  onUpdate,
  onDismiss,
}: {
  checkResult: UpdateCheckResponse;
  onUpdate: () => void;
  onDismiss: () => void;
}) {
  return (
    <div className="update-banner">
      <div className="update-banner-content">
        <div className="update-banner-icon">&#8679;</div>
        <div className="update-banner-text">
          <h3 className="update-banner-title">Update Available</h3>
          <p className="update-banner-description">
            Version {checkResult.server_version} is ready to install.
            {checkResult.files_to_update > 0 && (
              <span className="update-banner-details">
                {" "}
                ({checkResult.files_to_update} files, {checkResult.download_size_formatted})
              </span>
            )}
          </p>
        </div>
      </div>
      <div className="update-banner-actions">
        <button
          className="update-banner-button secondary"
          onClick={onDismiss}
        >
          Later
        </button>
        <button
          className="update-banner-button primary"
          onClick={onUpdate}
        >
          Update Now
        </button>
      </div>
    </div>
  );
}

/**
 * Progress display during update.
 */
function UpdatingProgress({
  progress,
}: {
  progress: UpdateProgressType | null;
}) {
  const percentage = progress
    ? calculatePercentage(progress.downloaded_bytes, progress.total_bytes)
    : 0;

  const fileProgress = progress
    ? calculatePercentage(progress.processed_files, progress.total_files)
    : 0;

  return (
    <div className="update-progress-container">
      <div className="update-progress-header">
        <div className="update-progress-icon">
          <span className="update-spinner" />
        </div>
        <h2 className="update-progress-title">Updating...</h2>
        <p className="update-progress-description">
          Please wait while the update is being downloaded and applied.
        </p>
      </div>

      <div className="update-progress-section">
        {/* Overall download progress bar */}
        <div className="update-progress">
          <div className="update-progress-bar-header">
            <span className="update-progress-label">Download Progress</span>
            <span className="update-progress-value">
              {percentage.toFixed(1)}%
            </span>
          </div>
          <div className="update-progress-bar">
            <div
              className="update-progress-fill"
              style={{ width: `${percentage}%` }}
            />
          </div>
          <div className="update-progress-details">
            <span>
              {progress ? formatBytes(progress.downloaded_bytes) : "0 bytes"} /{" "}
              {progress ? formatBytes(progress.total_bytes) : "0 bytes"}
            </span>
            {progress && progress.speed_bps > 0 && (
              <span>{formatBytes(progress.speed_bps)}/s</span>
            )}
            {progress && progress.eta_secs > 0 && (
              <span>ETA: {formatEta(progress.eta_secs)}</span>
            )}
          </div>
        </div>

        {/* File progress */}
        <div className="update-progress">
          <div className="update-progress-bar-header">
            <span className="update-progress-label">Files</span>
            <span className="update-progress-value">
              {progress?.processed_files || 0} / {progress?.total_files || 0}
            </span>
          </div>
          <div className="update-progress-bar secondary">
            <div
              className="update-progress-fill"
              style={{ width: `${fileProgress}%` }}
            />
          </div>
        </div>

        {/* Current file */}
        {progress?.current_file && (
          <div className="update-current-file">
            <span className="update-current-file-label">Current file:</span>
            <span className="update-current-file-name">
              {progress.current_file}
            </span>
          </div>
        )}

        {/* State indicator */}
        <div className="update-state">
          <span className="update-state-text">
            {getStateText(progress?.state || "Idle")}
          </span>
        </div>

        {/* Target version */}
        {progress?.target_version && (
          <div className="update-version">
            Updating to version {progress.target_version}
          </div>
        )}
      </div>

      <div className="update-progress-note">
        <p>Please do not close this window during the update.</p>
      </div>
    </div>
  );
}

/**
 * Update complete display.
 */
function UpdateComplete({
  version,
  onComplete,
}: {
  version: string | null;
  onComplete?: () => void;
}) {
  return (
    <div className="update-complete">
      <div className="update-complete-icon">&#10003;</div>
      <h2 className="update-complete-title">Update Complete!</h2>
      <p className="update-complete-description">
        {version
          ? `Successfully updated to version ${version}.`
          : "The update has been successfully applied."}
      </p>
      <div className="update-complete-actions">
        <button className="update-complete-button primary" onClick={onComplete}>
          Continue
        </button>
      </div>
    </div>
  );
}

/**
 * Update error display.
 */
function UpdateError({
  errorMessage,
  wasRolledBack,
  onRetry,
  onDismiss,
}: {
  errorMessage: string | null;
  wasRolledBack: boolean;
  onRetry: () => void;
  onDismiss: () => void;
}) {
  return (
    <div className="update-error">
      <div className="update-error-icon">&#10007;</div>
      <h2 className="update-error-title">Update Failed</h2>
      <p className="update-error-description">
        {wasRolledBack
          ? "The update was rolled back to restore the previous version."
          : "An error occurred during the update process."}
      </p>

      {errorMessage && (
        <div className="update-error-message">
          <span className="update-error-label">Error:</span>
          <span className="update-error-text">{errorMessage}</span>
        </div>
      )}

      <div className="update-error-actions">
        <button className="update-error-button secondary" onClick={onDismiss}>
          Skip Update
        </button>
        <button className="update-error-button primary" onClick={onRetry}>
          Try Again
        </button>
      </div>
    </div>
  );
}

/**
 * Main UpdateProgress component.
 */
export function UpdateProgress({
  onComplete,
  onDismiss,
  checkResult: externalCheckResult,
  showBanner = false,
}: UpdateProgressProps) {
  const [state, actions] = useUpdate();

  // Use external check result if provided, otherwise use internal state
  const checkResult = externalCheckResult ?? state.checkResult;

  // Handle update completion
  const handleComplete = () => {
    actions.reset();
    if (onComplete) {
      onComplete();
    }
  };

  // Handle dismiss
  const handleDismiss = () => {
    actions.dismissUpdate();
    if (onDismiss) {
      onDismiss();
    }
  };

  // Handle retry
  const handleRetry = () => {
    actions.retryUpdate();
  };

  // Handle start update
  const handleStartUpdate = () => {
    actions.startUpdate();
  };

  // Show error state
  if (state.errorMessage && !state.isUpdating) {
    return (
      <UpdateError
        errorMessage={state.errorMessage}
        wasRolledBack={state.wasRolledBack}
        onRetry={handleRetry}
        onDismiss={handleDismiss}
      />
    );
  }

  // Show complete state
  if (state.isComplete) {
    return (
      <UpdateComplete
        version={state.progress?.target_version || null}
        onComplete={handleComplete}
      />
    );
  }

  // Show updating progress
  if (state.isUpdating) {
    return <UpdatingProgress progress={state.progress} />;
  }

  // Show update available banner
  if (showBanner && checkResult && checkResult.update_available) {
    return (
      <UpdateAvailableBanner
        checkResult={checkResult}
        onUpdate={handleStartUpdate}
        onDismiss={handleDismiss}
      />
    );
  }

  // No update state to show
  return null;
}

/**
 * Standalone progress display without the full state machine.
 * Use this when you want to show progress but manage state externally.
 */
export function UpdateProgressDisplay({
  progress,
}: {
  progress: UpdateProgressType;
}) {
  return <UpdatingProgress progress={progress} />;
}

/**
 * Export the useUpdate hook for external use.
 */
export { useUpdate } from "../hooks/useUpdate";
