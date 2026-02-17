/**
 * InstallWizard Component
 *
 * A multi-step wizard for the first-run installation experience.
 * Steps: Welcome -> Select Directory -> EULA -> Installing -> Complete
 */

import { useInstall, type WizardStep } from "../hooks/useInstall";
import {
  formatBytes,
  formatEta,
  calculatePercentage,
} from "../lib/types";
import "./InstallWizard.css";

interface InstallWizardProps {
  /** Callback when installation completes successfully */
  onComplete?: () => void;
  /** Server/product name for display */
  serverName?: string;
}

/**
 * Progress indicator showing the current wizard step.
 */
function StepIndicator({
  currentStep,
  steps,
}: {
  currentStep: WizardStep;
  steps: { id: WizardStep; label: string }[];
}) {
  const currentIndex = steps.findIndex((s) => s.id === currentStep);

  return (
    <div className="wizard-steps">
      {steps.map((step, index) => (
        <div
          key={step.id}
          className={`wizard-step ${
            index === currentIndex
              ? "active"
              : index < currentIndex
              ? "completed"
              : ""
          }`}
        >
          <div className="wizard-step-number">
            {index < currentIndex ? (
              <span className="wizard-step-check">&#10003;</span>
            ) : (
              index + 1
            )}
          </div>
          <span className="wizard-step-label">{step.label}</span>
        </div>
      ))}
    </div>
  );
}

/**
 * Welcome step - introduces the installation process.
 */
function WelcomeStep({
  serverName,
  onNext,
}: {
  serverName: string;
  onNext: () => void;
}) {
  return (
    <div className="wizard-content">
      <div className="wizard-icon">&#9876;</div>
      <h2 className="wizard-title">Welcome to {serverName}</h2>
      <p className="wizard-description">
        This wizard will guide you through the installation process.
        You'll need to select a directory where the game files will be
        installed.
      </p>
      <p className="wizard-note">
        Make sure you have enough disk space available (approximately 2 GB).
      </p>
      <div className="wizard-actions">
        <button className="wizard-button primary" onClick={onNext}>
          Get Started
        </button>
      </div>
    </div>
  );
}

/**
 * Directory selection step - allows user to choose install location.
 */
function DirectoryStep({
  installPath,
  pathValidation,
  isValidating,
  onPickDirectory,
  onSetPath,
  onNext,
  onPrev,
  onRelaunchAsAdmin,
  onUseRecommendedPath,
}: {
  installPath: string;
  pathValidation: ReturnType<typeof useInstall>[0]["pathValidation"];
  isValidating: boolean;
  onPickDirectory: () => void;
  onSetPath: (path: string) => void;
  onNext: () => void;
  onPrev: () => void;
  onRelaunchAsAdmin: () => void;
  onUseRecommendedPath: () => void;
}) {
  const canProceed = pathValidation?.is_valid && !isValidating;
  const requiresElevation = pathValidation?.requires_elevation ?? false;

  return (
    <div className="wizard-content">
      <h2 className="wizard-title">Select Installation Directory</h2>
      <p className="wizard-description">
        Choose where you want to install the game files. You can create a new
        folder or select an existing empty folder.
      </p>

      <div className="wizard-field">
        <label className="wizard-label">Installation Path</label>
        <div className="wizard-path-input">
          <input
            type="text"
            className="wizard-input"
            value={installPath}
            onChange={(e) => onSetPath(e.target.value)}
            placeholder="Select a directory..."
            readOnly
          />
          <button
            className="wizard-button secondary"
            onClick={onPickDirectory}
          >
            Browse...
          </button>
        </div>

        {/* Validation feedback */}
        {isValidating && (
          <div className="wizard-validation validating">
            <span className="wizard-spinner" />
            Validating directory...
          </div>
        )}

        {!isValidating && pathValidation && (
          <div
            className={`wizard-validation ${
              pathValidation.is_valid ? "valid" : "invalid"
            }`}
          >
            {pathValidation.is_valid ? (
              <>
                <span className="wizard-validation-icon">&#10003;</span>
                <span>
                  Directory is valid. Available space:{" "}
                  {formatBytes(pathValidation.available_space)}
                </span>
              </>
            ) : (
              <>
                <span className="wizard-validation-icon">&#10007;</span>
                <span>{pathValidation.reason || "Invalid directory"}</span>
              </>
            )}
          </div>
        )}

        {/* Elevation required warning */}
        {pathValidation && pathValidation.requires_elevation && (
          <div className="wizard-validation warning">
            <span className="wizard-validation-icon">&#9888;</span>
            <span>
              This location requires administrator rights. Choose an option below:
            </span>
          </div>
        )}

        {/* Non-empty directory warning */}
        {pathValidation && !pathValidation.is_empty && pathValidation.exists && (
          <div className="wizard-validation warning">
            <span className="wizard-validation-icon">&#9888;</span>
            <span>
              This directory is not empty. Files may be overwritten.
            </span>
          </div>
        )}
      </div>

      <div className="wizard-actions">
        <button className="wizard-button secondary" onClick={onPrev}>
          Back
        </button>

        {/* Show elevation options if required */}
        {requiresElevation && canProceed ? (
          <>
            <button
              className="wizard-button secondary"
              onClick={onUseRecommendedPath}
              title="Use a recommended folder in your user directory"
            >
              Use Recommended Folder
            </button>
            <button
              className="wizard-button primary"
              onClick={onRelaunchAsAdmin}
              title="Restart the launcher with administrator rights"
            >
              Run as Administrator
            </button>
          </>
        ) : (
          <button
            className="wizard-button primary"
            onClick={onNext}
            disabled={!canProceed}
          >
            {isValidating ? "Validating..." : "Continue"}
          </button>
        )}
      </div>
    </div>
  );
}

/**
 * EULA step - displays license agreement for user acceptance.
 */
function EulaStep({
  eulaAccepted,
  onSetEulaAccepted,
  onNext,
  onPrev,
}: {
  eulaAccepted: boolean;
  onSetEulaAccepted: (accepted: boolean) => void;
  onNext: () => void;
  onPrev: () => void;
}) {
  return (
    <div className="wizard-content">
      <h2 className="wizard-title">Terms of Service</h2>
      <p className="wizard-description">
        Please read and accept the terms of service to continue.
      </p>

      <div className="wizard-eula">
        <div className="wizard-eula-text">
          <h3>End User License Agreement</h3>
          <p>
            By installing and using this software, you agree to the following
            terms:
          </p>
          <ol>
            <li>
              This launcher is provided for connecting to the specified Ultima
              Online private server.
            </li>
            <li>
              The server operators are not responsible for any data loss or
              damages.
            </li>
            <li>
              You must comply with the server's rules and code of conduct.
            </li>
            <li>
              This software may download and update game files automatically.
            </li>
            <li>
              Your use of this software is at your own risk.
            </li>
          </ol>
          <p>
            For the full terms of service, please visit the server's website.
          </p>
        </div>
      </div>

      <div className="wizard-checkbox-group">
        <label className="wizard-checkbox-label">
          <input
            type="checkbox"
            checked={eulaAccepted}
            onChange={(e) => onSetEulaAccepted(e.target.checked)}
            className="wizard-checkbox"
          />
          <span>I have read and accept the Terms of Service</span>
        </label>
      </div>

      <div className="wizard-actions">
        <button className="wizard-button secondary" onClick={onPrev}>
          Back
        </button>
        <button
          className="wizard-button primary"
          onClick={onNext}
          disabled={!eulaAccepted}
        >
          Install
        </button>
      </div>
    </div>
  );
}

/**
 * Installing step - shows progress during file download and installation.
 */
function InstallingStep({
  progress,
}: {
  progress: ReturnType<typeof useInstall>[0]["progress"];
}) {
  const percentage = progress
    ? calculatePercentage(progress.downloaded_bytes, progress.total_bytes)
    : 0;

  const fileProgress = progress
    ? calculatePercentage(progress.processed_files, progress.total_files)
    : 0;

  return (
    <div className="wizard-content">
      <h2 className="wizard-title">Installing...</h2>
      <p className="wizard-description">
        Please wait while the game files are being downloaded and installed.
        This may take a few minutes depending on your connection speed.
      </p>

      <div className="wizard-progress-section">
        {/* Overall progress bar */}
        <div className="wizard-progress">
          <div className="wizard-progress-header">
            <span className="wizard-progress-label">Download Progress</span>
            <span className="wizard-progress-value">
              {percentage.toFixed(1)}%
            </span>
          </div>
          <div className="wizard-progress-bar">
            <div
              className="wizard-progress-fill"
              style={{ width: `${percentage}%` }}
            />
          </div>
          <div className="wizard-progress-details">
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
        <div className="wizard-progress">
          <div className="wizard-progress-header">
            <span className="wizard-progress-label">Files</span>
            <span className="wizard-progress-value">
              {progress?.processed_files || 0} / {progress?.total_files || 0}
            </span>
          </div>
          <div className="wizard-progress-bar secondary">
            <div
              className="wizard-progress-fill"
              style={{ width: `${fileProgress}%` }}
            />
          </div>
        </div>

        {/* Current file */}
        {progress?.current_file && (
          <div className="wizard-current-file">
            <span className="wizard-current-file-label">Current file:</span>
            <span className="wizard-current-file-name">
              {progress.current_file}
            </span>
          </div>
        )}

        {/* State indicator */}
        <div className="wizard-state">
          <span className="wizard-spinner" />
          <span className="wizard-state-text">
            {getStateText(progress?.state || "Idle")}
          </span>
        </div>
      </div>

      <div className="wizard-actions">
        <p className="wizard-note">
          Please do not close this window during installation.
        </p>
      </div>
    </div>
  );
}

/**
 * Complete step - shows success message after installation finishes.
 */
function CompleteStep({
  serverName,
  installPath,
  version,
  onComplete,
}: {
  serverName: string;
  installPath: string;
  version: string | null;
  onComplete?: () => void;
}) {
  return (
    <div className="wizard-content">
      <div className="wizard-icon success">&#10003;</div>
      <h2 className="wizard-title">Installation Complete!</h2>
      <p className="wizard-description">
        {serverName} has been successfully installed. You can now launch the
        game and start playing.
      </p>

      <div className="wizard-summary">
        <div className="wizard-summary-item">
          <span className="wizard-summary-label">Installed to:</span>
          <span className="wizard-summary-value">{installPath}</span>
        </div>
        {version && (
          <div className="wizard-summary-item">
            <span className="wizard-summary-label">Version:</span>
            <span className="wizard-summary-value">{version}</span>
          </div>
        )}
      </div>

      <div className="wizard-actions">
        <button className="wizard-button primary large" onClick={onComplete}>
          Start Playing
        </button>
      </div>
    </div>
  );
}

/**
 * Error step - shows error message and retry option.
 */
function ErrorStep({
  errorMessage,
  onRetry,
}: {
  errorMessage: string | null;
  onRetry: () => void;
}) {
  return (
    <div className="wizard-content">
      <div className="wizard-icon error">&#10007;</div>
      <h2 className="wizard-title">Installation Failed</h2>
      <p className="wizard-description">
        An error occurred during installation. Please try again.
      </p>

      {errorMessage && (
        <div className="wizard-error-message">
          <span className="wizard-error-label">Error:</span>
          <span className="wizard-error-text">{errorMessage}</span>
        </div>
      )}

      <div className="wizard-actions">
        <button className="wizard-button primary" onClick={onRetry}>
          Try Again
        </button>
      </div>
    </div>
  );
}

/**
 * Get human-readable text for installation state.
 */
function getStateText(state: string): string {
  switch (state) {
    case "Idle":
      return "Preparing...";
    case "ValidatingPath":
      return "Validating installation directory...";
    case "FetchingManifest":
      return "Fetching file manifest...";
    case "Downloading":
      return "Downloading files...";
    case "Verifying":
      return "Verifying files...";
    case "Completed":
      return "Installation complete!";
    case "Failed":
      return "Installation failed";
    default:
      return state;
  }
}

/**
 * Main InstallWizard component.
 */
export function InstallWizard({
  onComplete,
  serverName = "UltimaForge",
}: InstallWizardProps) {
  const [state, actions] = useInstall();

  // Define wizard steps for the indicator
  const steps: { id: WizardStep; label: string }[] = [
    { id: "welcome", label: "Welcome" },
    { id: "directory", label: "Directory" },
    { id: "eula", label: "Terms" },
    { id: "installing", label: "Install" },
    { id: "complete", label: "Done" },
  ];

  // Filter out error step from indicator (it's a special state)
  const visibleSteps = steps.filter((s) => s.id !== "error");

  // Handle the final "Start Playing" button
  const handleComplete = () => {
    if (onComplete) {
      onComplete();
    }
  };

  // Handle EULA acceptance and start installation
  const handleStartInstall = () => {
    actions.startInstallation();
  };

  return (
    <div className="install-wizard">
      <div className="wizard-container">
        {/* Step indicator (hidden on error) */}
        {state.currentStep !== "error" && (
          <StepIndicator currentStep={state.currentStep} steps={visibleSteps} />
        )}

        {/* Render current step */}
        {state.currentStep === "welcome" && (
          <WelcomeStep serverName={serverName} onNext={actions.nextStep} />
        )}

        {state.currentStep === "directory" && (
          <DirectoryStep
            installPath={state.installPath}
            pathValidation={state.pathValidation}
            isValidating={state.isValidating}
            onPickDirectory={actions.pickDirectory}
            onSetPath={actions.setInstallPath}
            onNext={actions.nextStep}
            onPrev={actions.prevStep}
            onRelaunchAsAdmin={actions.relaunchAsAdmin}
            onUseRecommendedPath={actions.useRecommendedPath}
          />
        )}

        {state.currentStep === "eula" && (
          <EulaStep
            eulaAccepted={state.eulaAccepted}
            onSetEulaAccepted={actions.setEulaAccepted}
            onNext={handleStartInstall}
            onPrev={actions.prevStep}
          />
        )}

        {state.currentStep === "installing" && (
          <InstallingStep progress={state.progress} />
        )}

        {state.currentStep === "complete" && (
          <CompleteStep
            serverName={serverName}
            installPath={state.installPath}
            version={state.progress?.target_version || null}
            onComplete={handleComplete}
          />
        )}

        {state.currentStep === "error" && (
          <ErrorStep
            errorMessage={state.errorMessage}
            onRetry={actions.retryInstallation}
          />
        )}
      </div>
    </div>
  );
}
