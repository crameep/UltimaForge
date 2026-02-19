/**
 * Settings Component
 *
 * User preferences and installation management interface.
 * Displays install path, user settings toggles, and maintenance actions.
 */

import { useState } from "react";
import { useSettings } from "../hooks/useSettings";
import { checkForLauncherUpdate } from "../lib/launcherUpdater";
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
  loading,
  variant,
  onClick,
}: {
  label: string;
  description: string;
  icon: string;
  disabled?: boolean;
  loading?: boolean;
  variant?: "default" | "repairing";
  onClick: () => void;
}) {
  const buttonClass = [
    "settings-action-button",
    variant === "repairing" ? "repairing" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const iconClass = ["settings-action-icon", loading ? "spinning" : ""]
    .filter(Boolean)
    .join(" ");

  return (
    <button className={buttonClass} disabled={disabled} onClick={onClick}>
      <div className="settings-action-content">
        <span className="settings-action-label">{label}</span>
        <span className="settings-action-description">{description}</span>
      </div>
      <span className={iconClass}>{icon}</span>
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
 * Admin privilege banner component.
 * Displays when the application is running without administrator privileges.
 */
function AdminBanner({
  onRelaunchAsAdmin,
  isElevating,
}: {
  onRelaunchAsAdmin: () => void;
  isElevating?: boolean;
}) {
  return (
    <div className="settings-admin-banner">
      <div className="settings-admin-banner-content">
        <span className="settings-admin-banner-icon">{"\u26A0"}</span>
        <div className="settings-admin-banner-text">
          <span className="settings-admin-banner-title">
            Running without administrator privileges
          </span>
          <span className="settings-admin-banner-description">
            File repairs and some maintenance operations may require elevation.
            Click &quot;Run as Admin&quot; to restart with full permissions.
          </span>
        </div>
      </div>
      <button
        className="settings-admin-banner-button"
        onClick={onRelaunchAsAdmin}
        disabled={isElevating}
      >
        {isElevating ? "Requesting..." : "Run as Admin"}
      </button>
    </div>
  );
}

/**
 * Main Settings component.
 */
export function Settings({ onBack }: SettingsProps) {
  const [state, actions] = useSettings();
  const [launcherUpdateMessage, setLauncherUpdateMessage] = useState<{
    type: "success" | "error";
    message: string;
  } | null>(null);
  const [isCheckingLauncherUpdate, setIsCheckingLauncherUpdate] =
    useState(false);

  const handleLauncherUpdateCheck = async () => {
    setIsCheckingLauncherUpdate(true);
    setLauncherUpdateMessage(null);

    const result = await checkForLauncherUpdate({ interactive: true });

    if (result.error) {
      setLauncherUpdateMessage({
        type: "error",
        message: result.error,
      });
    } else if (!result.updateAvailable) {
      setLauncherUpdateMessage({
        type: "success",
        message: "Launcher is up to date.",
      });
    } else {
      const versionText = result.version ? ` (v${result.version})` : "";
      setLauncherUpdateMessage({
        type: "success",
        message: `Launcher update available${versionText}.`,
      });
    }

    setIsCheckingLauncherUpdate(false);
  };

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

  // Track if any maintenance operation is running (for mutual exclusion)
  const isAnyOperationRunning =
    state.isVerifying || state.isClearing || state.isRepairing;

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
        {launcherUpdateMessage && (
          <Message
            type={launcherUpdateMessage.type}
            message={launcherUpdateMessage.message}
            onDismiss={() => setLauncherUpdateMessage(null)}
          />
        )}

        {/* Admin Privilege Banner */}
        {!state.isAdmin && (
          <AdminBanner
            onRelaunchAsAdmin={actions.relaunchAsAdmin}
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
              loading={state.isVerifying}
              disabled={
                isAnyOperationRunning ||
                !state.installComplete ||
                !state.installPath
              }
              onClick={actions.verifyInstallation}
            />

            <ActionButton
              label="Clear Cache"
              description="Remove cached manifests and temporary files"
              icon={state.isClearing ? "\u21BB" : "\u2672"}
              loading={state.isClearing}
              disabled={isAnyOperationRunning}
              onClick={actions.clearCache}
            />

            <ActionButton
              label="Repair Installation"
              description="Re-download and fix corrupted or damaged game files"
              icon={state.isRepairing ? "\u21BB" : "\u2699"}
              loading={state.isRepairing}
              variant={state.isRepairing ? "repairing" : "default"}
              disabled={
                isAnyOperationRunning ||
                !state.installComplete ||
                !state.installPath
              }
              onClick={actions.repairInstallation}
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
          {state.verifyResult && !state.isVerifying && !state.isRepairing && (
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

              {/* Success state: All files valid, no repair needed */}
              {state.verifyResult.success && state.verifyResult.invalid_files.length === 0 && (
                <div className="settings-verify-success">
                  <span className="settings-verify-success-text">
                    Your installation is healthy. No files need repair.
                  </span>
                </div>
              )}

              {/* Failure state: Show files needing repair */}
              {state.verifyResult.invalid_files.length > 0 && (
                <div className="settings-verify-invalid">
                  <span className="settings-verify-invalid-label">
                    {state.verifyResult.invalid_files.length === 1
                      ? "1 file needs repair:"
                      : `${state.verifyResult.invalid_files.length} files need repair:`}
                  </span>
                  <div className="settings-verify-invalid-list">
                    {state.verifyResult.invalid_files.map((file) => (
                      <span key={file} className="settings-verify-invalid-file">
                        {file}
                      </span>
                    ))}
                  </div>
                  <button
                    className="settings-repair-now-button"
                    disabled={isAnyOperationRunning || !state.installPath}
                    onClick={actions.repairInstallation}
                  >
                    {state.isRepairing ? "Repairing..." : "Repair Now"}
                  </button>
                  {!state.isAdmin && (
                    <span className="settings-verify-admin-hint">
                      Note: Repair may require administrator privileges
                    </span>
                  )}
                </div>
              )}

              {/* Error state: Verification itself failed */}
              {state.verifyResult.error && (
                <div className="settings-verify-error">
                  <span className="settings-verify-error-text">
                    Verification failed: {state.verifyResult.error}
                  </span>
                </div>
              )}
            </div>
          )}

          {/* Repair Progress */}
          {state.isRepairing && state.verifyProgress && (
            <div className="settings-progress repair">
              <div className="settings-progress-header">
                <span className="settings-progress-label">
                  Repairing files...
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
        </section>

        {/* Launcher Updates */}
        <section className="settings-section">
          <h2 className="settings-section-title">Launcher Updates</h2>

          <div className="settings-actions">
            <ActionButton
              label="Check for launcher updates"
              description="Download and install the latest launcher build"
              icon={isCheckingLauncherUpdate ? "\u21BB" : "\u2191"}
              loading={isCheckingLauncherUpdate}
              disabled={isCheckingLauncherUpdate}
              onClick={handleLauncherUpdateCheck}
            />
          </div>
        </section>

        {/* About */}
        <section className="settings-section">
          <h2 className="settings-section-title">About</h2>

          <div className="settings-about">
            <div className="settings-about-header">
              <span className="settings-about-name">UltimaForge Launcher</span>
              <span className="settings-about-version">v0.1.0</span>
            </div>
            <p className="settings-about-description">
              A modern game launcher for managing your UltimaForge installation.
            </p>
            <div className="settings-about-credits">
              <span className="settings-about-credits-label">Built by</span>
              <span className="settings-about-credits-value">UltimaForge Team</span>
            </div>
            <div className="settings-about-links">
              <a
                href="https://ultimaforge.com"
                target="_blank"
                rel="noopener noreferrer"
                className="settings-about-link"
              >
                Website
              </a>
              <a
                href="https://discord.gg/ultimaforge"
                target="_blank"
                rel="noopener noreferrer"
                className="settings-about-link"
              >
                Discord
              </a>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
}

// Re-export hook for convenience
export { useSettings } from "../hooks/useSettings";
