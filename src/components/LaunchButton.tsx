/**
 * LaunchButton Component
 *
 * Primary call-to-action button for launching the game client.
 * Handles various states: ready, launching, running, updating, etc.
 */

import { useLaunch } from "../hooks/useLaunch";
import type { LaunchGameRequest } from "../lib/types";
import "./LaunchButton.css";

/**
 * Props for the LaunchButton component.
 */
interface LaunchButtonProps {
  /** Whether the button should be disabled */
  disabled?: boolean;
  /** Whether an update is available */
  updateAvailable?: boolean;
  /** Callback when update is requested */
  onUpdateRequest?: () => void;
  /** Callback when launch succeeds */
  onLaunchSuccess?: (pid: number | null, shouldClose: boolean) => void;
  /** Callback when launch fails */
  onLaunchError?: (error: string) => void;
  /** Callback when game state changes */
  onGameStateChange?: (isRunning: boolean) => void;
  /** Additional launch arguments */
  launchArgs?: string[];
  /** Whether to close launcher after launch */
  closeAfterLaunch?: boolean;
}

/**
 * Get button label based on current state.
 */
function getButtonLabel(
  isLaunching: boolean,
  isGameRunning: boolean,
  updateAvailable: boolean
): string {
  if (isLaunching) {
    return "Launching...";
  }
  if (isGameRunning) {
    return "Playing...";
  }
  if (updateAvailable) {
    return "Update & Play";
  }
  return "Play";
}

/**
 * LaunchButton component.
 *
 * Main button for launching the game client. Displays different
 * states and handles validation before launch.
 */
export function LaunchButton({
  disabled = false,
  updateAvailable = false,
  onUpdateRequest,
  onLaunchSuccess,
  onLaunchError,
  onGameStateChange,
  launchArgs,
  closeAfterLaunch,
}: LaunchButtonProps) {
  const [launchState, launchActions] = useLaunch();

  const handleClick = async () => {
    // If update is available, trigger update first
    if (updateAvailable && onUpdateRequest) {
      onUpdateRequest();
      return;
    }

    // Validate client before launching
    const isValid = await launchActions.validateClient();
    if (!isValid) {
      if (launchState.errorMessage && onLaunchError) {
        onLaunchError(launchState.errorMessage);
      }
      return;
    }

    // Build launch request
    const request: LaunchGameRequest = {};
    if (launchArgs && launchArgs.length > 0) {
      request.args = launchArgs;
    }
    if (closeAfterLaunch !== undefined) {
      request.close_after_launch = closeAfterLaunch;
    }

    // Launch the game
    const result = await launchActions.launch(request);

    if (result.success) {
      if (onGameStateChange) {
        onGameStateChange(true);
      }
      if (onLaunchSuccess) {
        onLaunchSuccess(result.pid, result.should_close_launcher);
      }
    } else if (result.error && onLaunchError) {
      onLaunchError(result.error);
    }
  };

  const handleGameClosed = async () => {
    await launchActions.markGameClosed();
    if (onGameStateChange) {
      onGameStateChange(false);
    }
  };

  const isDisabled =
    disabled ||
    launchState.isLaunching ||
    launchState.isValidating ||
    launchState.isGameRunning;

  const buttonLabel = getButtonLabel(
    launchState.isLaunching || launchState.isValidating,
    launchState.isGameRunning,
    updateAvailable
  );

  return (
    <div className="launch-button-container">
      <button
        className="launch-button"
        onClick={handleClick}
        disabled={isDisabled}
        aria-label={buttonLabel}
      >
        {launchState.isLaunching || launchState.isValidating ? (
          <span className="launch-button-content">
            <span className="launch-spinner" />
            <span className="launch-button-text">{buttonLabel}</span>
          </span>
        ) : (
          <span className="launch-button-content">
            <span className="launch-button-text">{buttonLabel}</span>
          </span>
        )}
      </button>

      {/* Show "Mark as Closed" option when game is running */}
      {launchState.isGameRunning && (
        <button
          className="launch-button-secondary"
          onClick={handleGameClosed}
          aria-label="Mark game as closed"
        >
          Game Closed?
        </button>
      )}

      {/* Error message display */}
      {launchState.errorMessage && (
        <div className="launch-error">
          <span className="launch-error-icon">!</span>
          <span className="launch-error-text">{launchState.errorMessage}</span>
          <button
            className="launch-error-dismiss"
            onClick={launchActions.clearError}
            aria-label="Dismiss error"
          >
            &times;
          </button>
        </div>
      )}
    </div>
  );
}

/**
 * Export the useLaunch hook for external use.
 */
export { useLaunch } from "../hooks/useLaunch";
