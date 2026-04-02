/**
 * Migration wizard for detecting and migrating existing UO installations.
 *
 * Shows detected installations and lets the user choose to copy, adopt in-place,
 * or skip to a fresh install.
 */

import { useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useMigration } from "../hooks/useMigration";
import "./MigrationWizard.css";

interface MigrationWizardProps {
  /** Called when migration completes (copy or adopt). */
  onComplete: () => void;
  /** Called when user skips migration (proceed to fresh install). */
  onSkip: () => void;
  /** Server display name for UI text. */
  serverName: string;
}

export function MigrationWizard({ onComplete, onSkip, serverName: _serverName }: MigrationWizardProps) {
  const [state, actions] = useMigration(onComplete, onSkip);

  // Auto-scan on mount
  useEffect(() => {
    actions.scan();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Scanning state
  if (state.step === "scanning") {
    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Checking for Existing Installations</h2>
          <p className="migration-subtitle">
            Looking for existing Ultima Online files on your system...
          </p>
          <div className="migration-spinner" />
        </div>
      </div>
    );
  }

  // Nothing found
  if (state.step === "not_found") {
    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>No Existing Installations Found</h2>
          <p className="migration-subtitle">
            No existing Ultima Online installations were found at the configured locations.
          </p>
          {state.error && <p className="migration-error">{state.error}</p>}
          <div className="migration-actions">
            <button
              className="migration-btn migration-btn-secondary"
              onClick={actions.browseForInstallation}
            >
              Browse Manually
            </button>
            <button
              className="migration-btn migration-btn-primary"
              onClick={actions.skip}
            >
              Install Fresh
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Decision screen
  if (state.step === "decision" && state.selectedSource) {
    const source = state.selectedSource;
    const isProtected = source.install_path?.toLowerCase().includes("program files") ?? false;

    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Existing Installation Found</h2>
          <div className="migration-detection-info">
            <p className="migration-path">{source.install_path}</p>
            <p className="migration-confidence">
              Confidence: <span className={`confidence-${source.confidence.toLowerCase()}`}>
                {source.confidence}
              </span>
            </p>
            <p className="migration-files">
              Found: {source.found_executables.join(", ")}
              {source.found_data_files.length > 0 &&
                ` + ${source.found_data_files.length} data files`}
            </p>
            {source.missing_files.length > 0 && (
              <p className="migration-missing">
                Missing: {source.missing_files.join(", ")}
                <span className="migration-missing-note">
                  {" "}(will be downloaded during update)
                </span>
              </p>
            )}
          </div>

          <div className="migration-options">
            <button
              className="migration-option migration-option-recommended"
              onClick={() => actions.setStep("choose_destination")}
            >
              <div className="option-header">
                <span className="option-title">Copy to New Location</span>
                <span className="option-badge">Recommended</span>
              </div>
              <p className="option-description">
                Copy files to a safe location. No admin required. Original files untouched.
              </p>
            </button>

            <button
              className="migration-option"
              onClick={actions.adoptInPlace}
            >
              <div className="option-header">
                <span className="option-title">Use in Place</span>
              </div>
              <p className="option-description">
                Use the existing directory for updates.
                {isProtected && (
                  <span className="option-warning">
                    {" "}This location requires administrator privileges for every launch.
                  </span>
                )}
              </p>
              {isProtected && !state.isAdmin && (
                <button
                  className="migration-btn migration-btn-small"
                  onClick={(e) => {
                    e.stopPropagation();
                    actions.relaunchAsAdmin();
                  }}
                >
                  Relaunch as Administrator
                </button>
              )}
            </button>

            <button
              className="migration-option migration-option-skip"
              onClick={actions.skip}
            >
              <div className="option-header">
                <span className="option-title">Skip — Install Fresh</span>
              </div>
              <p className="option-description">
                Ignore existing files and download everything new.
              </p>
            </button>
          </div>

          {state.detected.length > 1 && (
            <div className="migration-other-results">
              <p>Other installations found:</p>
              {state.detected
                .filter((d) => d.install_path !== source.install_path)
                .map((d) => (
                  <button
                    key={d.install_path}
                    className="migration-alt-source"
                    onClick={() => actions.selectSource(d)}
                  >
                    {d.install_path} ({d.confidence})
                  </button>
                ))}
            </div>
          )}
        </div>
      </div>
    );
  }

  // Choose destination for copy
  if (state.step === "choose_destination" && state.selectedSource) {
    const destValid = state.destinationValidation;

    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Choose Destination</h2>
          <p className="migration-subtitle">
            Files will be copied from{" "}
            <strong>{state.selectedSource.install_path}</strong> to:
          </p>
          <div className="migration-dest-input">
            <input
              type="text"
              value={state.destinationPath}
              onChange={(e) => actions.setDestinationPath(e.target.value)}
              className="migration-path-input"
            />
            <button
              className="migration-btn migration-btn-secondary"
              onClick={async () => {
                const selected = await open({
                  directory: true,
                  multiple: false,
                  title: "Select Destination Directory",
                });
                if (selected && typeof selected === "string") {
                  actions.setDestinationPath(selected);
                }
              }}
            >
              Browse
            </button>
          </div>
          {destValid && !destValid.is_valid && destValid.reason && (
            <p className="migration-error">{destValid.reason}</p>
          )}
          {destValid && destValid.requires_elevation && (
            <p className="option-warning">
              This path requires administrator privileges. Consider choosing a
              different location.
            </p>
          )}
          <p className="migration-note">
            Original files will not be modified.
          </p>
          <div className="migration-actions">
            <button
              className="migration-btn migration-btn-secondary"
              onClick={() => actions.setStep("decision")}
            >
              Back
            </button>
            <button
              className="migration-btn migration-btn-primary"
              disabled={!destValid?.is_valid}
              onClick={actions.copyToNewLocation}
            >
              Start Copy
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Migrating — file copy in progress
  if (state.step === "migrating") {
    const pct =
      state.progress && state.progress.files_total > 0
        ? Math.round(
            (state.progress.files_copied / state.progress.files_total) * 100
          )
        : 0;

    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Migrating Files</h2>
          <p className="migration-subtitle">
            Copying files to {state.destinationPath}...
          </p>
          <div className="migration-progress-bar">
            <div
              className="migration-progress-fill"
              style={{ width: `${pct}%` }}
            />
          </div>
          <p className="migration-progress-text">
            {state.progress
              ? `${state.progress.files_copied} / ${state.progress.files_total} files (${pct}%)`
              : "Preparing..."}
          </p>
          {state.progress?.current_file && (
            <p className="migration-current-file">
              {state.progress.current_file}
            </p>
          )}
        </div>
      </div>
    );
  }

  // Complete
  if (state.step === "complete") {
    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Migration Complete</h2>
          <p className="migration-subtitle">
            Files have been copied successfully. The launcher will now check for updates.
          </p>
        </div>
      </div>
    );
  }

  // Error
  if (state.step === "error") {
    return (
      <div className="migration-wizard">
        <div className="migration-card">
          <h2>Migration Failed</h2>
          <p className="migration-error">{state.error}</p>
          <div className="migration-actions">
            <button
              className="migration-btn migration-btn-secondary"
              onClick={actions.reset}
            >
              Try Again
            </button>
            <button
              className="migration-btn migration-btn-primary"
              onClick={actions.skip}
            >
              Install Fresh Instead
            </button>
          </div>
        </div>
      </div>
    );
  }

  return null;
}
