/**
 * LaunchButton Component
 *
 * Primary call-to-action button for launching the game client.
 * Handles various states: ready, launching, running, updating, etc.
 */

import type { UseLaunchActions, UseLaunchState } from "../hooks/useLaunch";
import type { LaunchGameRequest } from "../lib/types";
import type { ReactNode } from "react";
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
  onLaunchSuccess?: (pid: number | null, shouldClose: boolean, runningClients: number) => void;
  /** Callback when launch fails */
  onLaunchError?: (error: string) => void;
  /** Callback when game state changes */
  onGameStateChange?: (isRunning: boolean) => void;
  /** Additional launch arguments */
  launchArgs?: string[];
  /** Whether to close launcher after launch */
  closeAfterLaunch?: boolean;
  /** Launch state from parent */
  launchState: UseLaunchState;
  /** Launch actions from parent */
  launchActions: UseLaunchActions;
  /** Number of client instances to open (1-3) */
  clientCount?: number;
  /** Callback when client count changes */
  onClientCountChange?: (count: number) => void;
  /** Optional controls shown between the Play button and client count */
  controlsBelowButton?: ReactNode;
}

/**
 * Get button label based on current state.
 */
function getButtonLabel(
  isLaunching: boolean,
  runningClients: number,
  updateAvailable: boolean
): string {
  if (isLaunching) {
    return "Launching...";
  }
  if (runningClients >= 3) {
    return "Playing...";
  }
  if (updateAvailable) {
    return "Update & Play";
  }
  if (runningClients > 0) {
    return "Play Another";
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
  launchState,
  launchActions,
  clientCount,
  onClientCountChange,
  controlsBelowButton,
}: LaunchButtonProps) {
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
        onLaunchSuccess(result.pid, result.should_close_launcher, result.running_clients);
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
    launchState.runningClients >= 3;

  const buttonLabel = getButtonLabel(
    launchState.isLaunching || launchState.isValidating,
    launchState.runningClients,
    updateAvailable
  );

  return (
    <div className="launch-button-container">
      <div className="launch-row">
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

      </div>

      {controlsBelowButton}

      {onClientCountChange && (
        <div className="client-count-spinner">
          <button
            className="client-count-btn"
            onClick={() => onClientCountChange(Math.max(1, (clientCount ?? 1) - 1))}
            disabled={isDisabled || (clientCount ?? 1) <= 1}
            aria-label="Decrease client count"
          >
            −
          </button>
          <span className="client-count-value">{clientCount ?? 1}</span>
          <button
            className="client-count-btn"
            onClick={() => onClientCountChange(Math.min(3, (clientCount ?? 1) + 1))}
            disabled={isDisabled || (clientCount ?? 1) >= 3}
            aria-label="Increase client count"
          >
            +
          </button>
        </div>
      )}



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
