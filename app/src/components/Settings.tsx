/**
 * Settings Component
 *
 * User preferences and installation management interface.
 * Displays install path, user settings toggles, and maintenance actions.
 */

import { useSettings } from "../hooks/useSettings";
import { calculatePercentage } from "../lib/types";
import "./Settings.css";

interface SettingsProps {
  /** Callback when user wants to go back to main view */
  onBack?: () => void;
}

/**
 * Toggle switch component for boolean settings.
 */
function ToggleSwitch({
  id,
  label,
  description,
  checked,
  disabled,
  onChange,
}: {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  disabled?: boolean;
  onChange: (checked: boolean) => void;
}) {
  return (
    <div className="settings-toggle-item">
      <div className="settings-toggle-content">
        <label htmlFor={id} className="settings-toggle-label">
          {label}
        </label>
        <span className="settings-toggle-description">{description}</span>
      </div>
      <label className="settings-toggle-switch">
        <input
          type="checkbox"
          id={id}
          className="settings-toggle-input"
          checked={checked}
          disabled={disabled}
          onChange={(e) => onChange(e.target.checked)}
        />
        <span className="settings-toggle-slider" />
      </label>
    </div>
  );
}

/**
 * Action button component for maintenance operations.
 */
function ActionButton({
  label,
  description,
  icon,
  disabled,
  onClick,
}: {
  label: string;
  description: string;
  icon: string;
  disabled?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      className="settings-action-button"
      disabled={disabled}
      onClick={onClick}
    >
      <div className="settings-action-content">
        <span className="settings-action-label">{label}</span>
        <span className="settings-action-description">{description}</span>
      </div>
      <span className="settings-action-icon">{icon}</span>
    </button>
  );
}

/**
 * Message component for displaying success/error messages.
 */
function Message({
  type,
  message,
  onDismiss,
}: {
  type: "success" | "error";
  message: string;
  onDismiss: () => void;
}) {
  return (
    <div className={`settings-message ${type}`}>
      <span className="settings-message-icon">
        {type === "success" ? "\u2713" : "\u2717"}
      </span>
      <span>{message}</span>
      <button
        className="settings-message-dismiss"
        onClick={onDismiss}
        aria-label="Dismiss"
      >
        &times;
      </button>
    </div>
  );
}

/**
 * Main Settings component.
 */
export function Settings({ onBack }: SettingsProps) {
  const [state, actions] = useSettings();

  // Show loading state
  if (state.isLoading && !state.settings) {
    return (
      <div className="settings">
        <div className="settings-container">
          <div className="settings-loading">
            <div className="settings-spinner" />
            <span className="settings-loading-text">Loading settings...</span>
          </div>
        </div>
      </div>
    );
  }

  // Calculate verify progress percentage
  const verifyPercentage = state.verifyProgress
    ? calculatePercentage(
        state.verifyProgress.processed_files,
        state.verifyProgress.total_files
      )
    : 0;

  return (
    <div className="settings">
      <div className="settings-container">
        {/* Header */}
        <div className="settings-header">
          {onBack && (
            <button
              className="settings-back-button"
              onClick={onBack}
              aria-label="Go back"
            >
              &larr;
            </button>
          )}
          <h1 className="settings-title">Settings</h1>
        </div>

        {/* Messages */}
        {state.errorMessage && (
          <Message
            type="error"
            message={state.errorMessage}
            onDismiss={actions.clearError}
          />
        )}
        {state.successMessage && (
          <Message
            type="success"
            message={state.successMessage}
            onDismiss={actions.clearSuccess}
          />
        )}

        {/* Installation Info */}
        <section className="settings-section">
          <h2 className="settings-section-title">Installation</h2>

          <div className="settings-info-item">
            <span className="settings-info-label">Install Path</span>
            <span
              className={`settings-info-value ${
                !state.installPath ? "empty" : ""
              }`}
            >
              {state.installPath || "Not installed"}
            </span>
          </div>

          <div className="settings-info-item">
            <span className="settings-info-label">Version</span>
            <span
              className={`settings-info-value ${
                !state.currentVersion ? "empty" : ""
              }`}
            >
              {state.currentVersion || "Unknown"}
            </span>
          </div>
        </section>

        {/* User Preferences */}
        <section className="settings-section">
          <h2 className="settings-section-title">Preferences</h2>

          <ToggleSwitch
            id="check-updates"
            label="Check for updates on startup"
            description="Automatically check for new updates when the launcher starts"
            checked={state.settings?.check_updates_on_startup ?? true}
            disabled={!state.settings || state.isSaving}
            onChange={(checked) =>
              actions.updateSetting("check_updates_on_startup", checked)
            }
          />

          <ToggleSwitch
            id="auto-launch"
            label="Auto-launch after update"
            description="Automatically start the game after a successful update"
            checked={state.settings?.auto_launch ?? false}
            disabled={!state.settings || state.isSaving}
            onChange={(checked) =>
              actions.updateSetting("auto_launch", checked)
            }
          />

          <ToggleSwitch
            id="close-on-launch"
            label="Close launcher when game starts"
            description="Minimize the launcher to system tray when the game is running"
            checked={state.settings?.close_on_launch ?? false}
            disabled={!state.settings || state.isSaving}
            onChange={(checked) =>
              actions.updateSetting("close_on_launch", checked)
            }
          />

          <div className="settings-save-section">
            <button
              className="settings-save-button"
              disabled={!state.settings || state.isSaving}
              onClick={actions.saveSettings}
            >
              {state.isSaving ? "Saving..." : "Save Settings"}
            </button>
          </div>
        </section>

        {/* Maintenance */}
        <section className="settings-section">
          <h2 className="settings-section-title">Maintenance</h2>

          <div className="settings-actions">
            <ActionButton
              label="Verify Installation"
              description="Check all game files for corruption or missing files"
              icon={state.isVerifying ? "\u21BB" : "\u2713"}
              disabled={
                state.isVerifying ||
                !state.installComplete ||
                !state.installPath
              }
              onClick={actions.verifyInstallation}
            />

            <ActionButton
              label="Clear Cache"
              description="Remove cached manifests and temporary files"
              icon={state.isClearing ? "\u21BB" : "\u2672"}
              disabled={state.isClearing}
              onClick={actions.clearCache}
            />
          </div>

          {/* Verify Progress */}
          {state.isVerifying && state.verifyProgress && (
            <div className="settings-progress">
              <div className="settings-progress-header">
                <span className="settings-progress-label">
                  Verifying files...
                </span>
                <span className="settings-progress-value">
                  {state.verifyProgress.processed_files} /{" "}
                  {state.verifyProgress.total_files}
                </span>
              </div>
              <div className="settings-progress-bar">
                <div
                  className="settings-progress-fill"
                  style={{ width: `${verifyPercentage}%` }}
                />
              </div>
            </div>
          )}

          {/* Verify Result */}
          {state.verifyResult && !state.isVerifying && (
            <div className="settings-verify-result">
              <div
                className={`settings-verify-summary ${
                  state.verifyResult.success ? "success" : "error"
                }`}
              >
                <span className="settings-verify-icon">
                  {state.verifyResult.success ? "\u2713" : "\u2717"}
                </span>
                <span>
                  {state.verifyResult.valid_files} of{" "}
                  {state.verifyResult.total_files} files verified
                </span>
              </div>

              {state.verifyResult.invalid_files.length > 0 && (
                <div className="settings-verify-invalid">
                  <span className="settings-verify-invalid-label">
                    Files needing repair:
                  </span>
                  <div className="settings-verify-invalid-list">
                    {state.verifyResult.invalid_files.map((file) => (
                      <span key={file} className="settings-verify-invalid-file">
                        {file}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}

// Re-export hook for convenience
export { useSettings } from "../hooks/useSettings";
