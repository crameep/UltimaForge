import { useState } from "react";
import "./LauncherUpdateModal.css";

interface LauncherUpdateModalProps {
  version?: string;
  notes?: string;
  date?: string;
  onInstall: () => Promise<void>;
  onDismiss: () => void;
}

export function LauncherUpdateModal({
  version,
  notes,
  date,
  onInstall,
  onDismiss,
}: LauncherUpdateModalProps) {
  const [installing, setInstalling] = useState(false);

  const handleInstall = async () => {
    setInstalling(true);
    await onInstall();
  };

  return (
    <div className="launcher-update-overlay">
      <div className="launcher-update-modal">
        <div className="launcher-update-icon">&#8679;</div>
        <h2 className="launcher-update-title">Launcher Update Available</h2>

        {version && (
          <p className="launcher-update-version">Version {version}</p>
        )}
        {date && (
          <p className="launcher-update-date">Published: {date}</p>
        )}
        {notes && (
          <p className="launcher-update-notes">{notes}</p>
        )}

        <p className="launcher-update-description">
          The launcher will download the update and restart automatically.
        </p>

        <div className="launcher-update-actions">
          <button
            className="launcher-update-button secondary"
            onClick={onDismiss}
            disabled={installing}
          >
            Later
          </button>
          <button
            className="launcher-update-button primary"
            onClick={handleInstall}
            disabled={installing}
          >
            {installing ? "Installing..." : "Update & Restart"}
          </button>
        </div>
      </div>
    </div>
  );
}
