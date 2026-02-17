import "./StatusBar.css";

type Phase =
  | "Initializing"
  | "NeedsInstall"
  | "Installing"
  | "CheckingUpdates"
  | "UpdateAvailable"
  | "Updating"
  | "Ready"
  | "GameRunning"
  | "Error";

interface StatusBarProps {
  /** Current application phase */
  phase?: Phase | string;
  /** Custom status message */
  message?: string;
  /** Application version */
  version?: string;
}

/**
 * Status bar component displayed at the bottom of the application.
 * Shows current status, connection state, and version information.
 */
export function StatusBar({
  phase = "Ready",
  message,
  version = "v0.1.0",
}: StatusBarProps) {
  // Get status indicator color based on phase
  const getStatusColor = (phase: string): string => {
    switch (phase) {
      case "Ready":
      case "GameRunning":
        return "status-success";
      case "CheckingUpdates":
      case "Updating":
      case "Installing":
        return "status-info";
      case "UpdateAvailable":
        return "status-warning";
      case "Error":
        return "status-error";
      case "Initializing":
      case "NeedsInstall":
      default:
        return "status-neutral";
    }
  };

  // Get human-readable status text
  const getStatusText = (phase: string): string => {
    switch (phase) {
      case "Initializing":
        return "Initializing...";
      case "NeedsInstall":
        return "Installation Required";
      case "Installing":
        return "Installing...";
      case "CheckingUpdates":
        return "Checking for Updates...";
      case "UpdateAvailable":
        return "Update Available";
      case "Updating":
        return "Updating...";
      case "Ready":
        return "Ready to Play";
      case "GameRunning":
        return "Game Running";
      case "Error":
        return "Error";
      default:
        return phase;
    }
  };

  const displayMessage = message || getStatusText(phase);
  const statusClass = getStatusColor(phase);

  return (
    <footer className="statusbar">
      <div className="statusbar-left">
        <span className={`statusbar-indicator ${statusClass}`} />
        <span className="statusbar-message">{displayMessage}</span>
      </div>
      <div className="statusbar-right">
        <span className="statusbar-version">{version}</span>
      </div>
    </footer>
  );
}
