/**
 * SetupWizard Component
 *
 * A multi-step wizard for server owner branding configuration.
 * Steps: Welcome -> Branding -> Keypair -> Review -> Complete
 */

import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./SetupWizard.css";

/**
 * Brand configuration structure matching brand.json format.
 */
interface BrandConfig {
  product: {
    displayName: string;
    serverName: string;
    description?: string;
    supportEmail?: string;
    website?: string;
    discord?: string;
  };
  updateUrl: string;
  publicKey: string;
  ui?: {
    colors?: {
      primary?: string;
      secondary?: string;
      background?: string;
      text?: string;
    };
    showPatchNotes?: boolean;
    windowTitle?: string;
  };
  migration?: {
    autoDetectPath?: string;
    autoMigrateOnFirstLaunch?: boolean;
  };
  brandVersion?: string;
}

/**
 * Keypair result from Rust backend.
 */
interface KeypairResult {
  publicKey: string;
  privateKey: string;
}

/**
 * Wizard step identifiers.
 */
type SetupWizardStep = "welcome" | "branding" | "keypair" | "review" | "complete" | "error";

interface SetupWizardProps {
  /** Callback when setup completes successfully */
  onComplete?: (config: BrandConfig) => void;
  /** Initial configuration to edit (optional) */
  initialConfig?: Partial<BrandConfig>;
}

/**
 * Progress indicator showing the current wizard step.
 */
function StepIndicator({
  currentStep,
  steps,
}: {
  currentStep: SetupWizardStep;
  steps: { id: SetupWizardStep; label: string }[];
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
 * Welcome step - introduces the setup process.
 */
function WelcomeStep({ onNext }: { onNext: () => void }) {
  return (
    <div className="wizard-content">
      <div className="wizard-icon">&#9881;</div>
      <h2 className="wizard-title">Welcome to Setup Wizard</h2>
      <p className="wizard-description">
        This wizard will guide you through configuring your branded launcher.
        You'll set up your server name, update URL, and generate a secure keypair
        for signing updates.
      </p>
      <p className="wizard-note">
        Make sure you have your server details ready before proceeding.
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
 * Branding step - collects server name and update URL.
 */
function BrandingStep({
  config,
  onChange,
  errors,
  onNext,
  onPrev,
}: {
  config: BrandConfig;
  onChange: (config: BrandConfig) => void;
  errors: Record<string, string>;
  onNext: () => void;
  onPrev: () => void;
}) {
  const updateProduct = (field: keyof BrandConfig["product"], value: string) => {
    onChange({
      ...config,
      product: {
        ...config.product,
        [field]: value,
      },
    });
  };

  const isValid =
    config.product.displayName.trim() !== "" &&
    config.product.serverName.trim() !== "" &&
    config.updateUrl.trim() !== "" &&
    !Object.keys(errors).length;

  return (
    <div className="wizard-content">
      <h2 className="wizard-title">Configure Branding</h2>
      <p className="wizard-description">
        Enter your server details. These will be displayed in the launcher UI.
      </p>

      <div className="wizard-field">
        <label className="wizard-label" htmlFor="displayName">
          Display Name *
        </label>
        <input
          id="displayName"
          type="text"
          className={`wizard-input ${errors.displayName ? "invalid" : ""}`}
          value={config.product.displayName}
          onChange={(e) => updateProduct("displayName", e.target.value)}
          placeholder="My Awesome Server"
        />
        {errors.displayName && (
          <div className="wizard-validation invalid">
            <span className="wizard-validation-icon">&#10007;</span>
            <span>{errors.displayName}</span>
          </div>
        )}
      </div>

      <div className="wizard-field">
        <label className="wizard-label" htmlFor="serverName">
          Server Name (no spaces) *
        </label>
        <input
          id="serverName"
          type="text"
          className={`wizard-input ${errors.serverName ? "invalid" : ""}`}
          value={config.product.serverName}
          onChange={(e) =>
            updateProduct("serverName", e.target.value.replace(/\s/g, ""))
          }
          placeholder="MyAwesomeServer"
        />
        {errors.serverName && (
          <div className="wizard-validation invalid">
            <span className="wizard-validation-icon">&#10007;</span>
            <span>{errors.serverName}</span>
          </div>
        )}
        <p className="wizard-hint">
          Internal identifier used for configuration. No spaces allowed.
        </p>
      </div>

      <div className="wizard-field">
        <label className="wizard-label" htmlFor="updateUrl">
          Update Server URL *
        </label>
        <input
          id="updateUrl"
          type="text"
          className={`wizard-input ${errors.updateUrl ? "invalid" : ""}`}
          value={config.updateUrl}
          onChange={(e) => onChange({ ...config, updateUrl: e.target.value })}
          placeholder="https://updates.yourserver.com"
        />
        {errors.updateUrl && (
          <div className="wizard-validation invalid">
            <span className="wizard-validation-icon">&#10007;</span>
            <span>{errors.updateUrl}</span>
          </div>
        )}
        <p className="wizard-hint">
          The base URL where your update server is hosted.
        </p>
      </div>

      <div className="wizard-field">
        <label className="wizard-label" htmlFor="description">
          Description (optional)
        </label>
        <input
          id="description"
          type="text"
          className="wizard-input"
          value={config.product.description || ""}
          onChange={(e) => updateProduct("description", e.target.value)}
          placeholder="A brief description of your server"
        />
      </div>

      <div className="wizard-field">
        <label className="wizard-label" htmlFor="autoMigrateOnFirstLaunch">
          Legacy Migration (optional)
        </label>
        <label className="wizard-checkbox-row">
          <input
            id="autoMigrateOnFirstLaunch"
            type="checkbox"
            checked={config.migration?.autoMigrateOnFirstLaunch ?? false}
            onChange={(e) =>
              onChange({
                ...config,
                migration: {
                  ...config.migration,
                  autoMigrateOnFirstLaunch: e.target.checked,
                },
              })
            }
          />
          <span>Attempt automatic migration on first launch</span>
        </label>
        <input
          id="autoDetectPath"
          type="text"
          className={`wizard-input ${errors.autoDetectPath ? "invalid" : ""}`}
          value={config.migration?.autoDetectPath || ""}
          onChange={(e) =>
            onChange({
              ...config,
              migration: {
                ...config.migration,
                autoDetectPath: e.target.value,
              },
            })
          }
          placeholder="%PROGRAMFILES%\\OldLauncher\\ClassicUO"
        />
        {errors.autoDetectPath && (
          <div className="wizard-validation invalid">
            <span className="wizard-validation-icon">&#10007;</span>
            <span>{errors.autoDetectPath}</span>
          </div>
        )}
        <p className="wizard-hint">
          Folder to scan on first launch. Supports `%ENV_VAR%` and
          {" {serverName} "}placeholders.
        </p>
      </div>

      <div className="wizard-actions">
        <button className="wizard-button secondary" onClick={onPrev}>
          Back
        </button>
        <button
          className="wizard-button primary"
          onClick={onNext}
          disabled={!isValid}
        >
          Continue
        </button>
      </div>
    </div>
  );
}

/**
 * Keypair step - generates or imports Ed25519 keypair.
 */
function KeypairStep({
  publicKey,
  privateKey,
  isGenerating,
  error,
  onGenerate,
  onImportPublicKey,
  onNext,
  onPrev,
}: {
  publicKey: string;
  privateKey: string;
  isGenerating: boolean;
  error: string | null;
  onGenerate: () => void;
  onImportPublicKey: (key: string) => void;
  onNext: () => void;
  onPrev: () => void;
}) {
  const [importMode, setImportMode] = useState(false);
  const [importedKey, setImportedKey] = useState("");

  const handleImport = () => {
    if (importedKey.trim().length === 64 && /^[0-9a-fA-F]+$/.test(importedKey.trim())) {
      onImportPublicKey(importedKey.trim().toLowerCase());
      setImportMode(false);
    }
  };

  const isValidKey = publicKey.length === 64 && /^[0-9a-fA-F]+$/.test(publicKey);

  return (
    <div className="wizard-content">
      <h2 className="wizard-title">Security Keypair</h2>
      <p className="wizard-description">
        Generate an Ed25519 keypair for signing your update manifests.
        The public key will be embedded in your launcher for verification.
      </p>

      {!importMode ? (
        <>
          <div className="wizard-keypair-section">
            <button
              className="wizard-button primary large"
              onClick={onGenerate}
              disabled={isGenerating}
            >
              {isGenerating ? (
                <>
                  <span className="wizard-spinner" />
                  Generating...
                </>
              ) : (
                "Generate New Keypair"
              )}
            </button>

            <button
              className="wizard-button secondary"
              onClick={() => setImportMode(true)}
            >
              Import Existing Key
            </button>
          </div>

          {error && (
            <div className="wizard-validation invalid">
              <span className="wizard-validation-icon">&#10007;</span>
              <span>{error}</span>
            </div>
          )}

          {publicKey && (
            <div className="wizard-key-display">
              <div className="wizard-key-item">
                <span className="wizard-key-label">Public Key</span>
                <code className="wizard-key-value">{publicKey}</code>
                <p className="wizard-hint">
                  This key will be embedded in your launcher. Safe to share.
                </p>
              </div>

              {privateKey && (
                <div className="wizard-key-item warning">
                  <span className="wizard-key-label">Private Key</span>
                  <code className="wizard-key-value">{privateKey}</code>
                  <div className="wizard-validation warning">
                    <span className="wizard-validation-icon">&#9888;</span>
                    <span>
                      Save this key securely! You'll need it to sign update manifests.
                      Never share or commit this key.
                    </span>
                  </div>
                </div>
              )}
            </div>
          )}
        </>
      ) : (
        <div className="wizard-import-section">
          <div className="wizard-field">
            <label className="wizard-label" htmlFor="importKey">
              Public Key (64-character hex string)
            </label>
            <input
              id="importKey"
              type="text"
              className="wizard-input"
              value={importedKey}
              onChange={(e) => setImportedKey(e.target.value)}
              placeholder="d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
            />
            <p className="wizard-hint">
              Paste your existing public key if you already have one.
            </p>
          </div>

          <div className="wizard-actions">
            <button
              className="wizard-button secondary"
              onClick={() => {
                setImportMode(false);
                setImportedKey("");
              }}
            >
              Cancel
            </button>
            <button
              className="wizard-button primary"
              onClick={handleImport}
              disabled={
                importedKey.trim().length !== 64 ||
                !/^[0-9a-fA-F]+$/.test(importedKey.trim())
              }
            >
              Import Key
            </button>
          </div>
        </div>
      )}

      {!importMode && (
        <div className="wizard-actions">
          <button className="wizard-button secondary" onClick={onPrev}>
            Back
          </button>
          <button
            className="wizard-button primary"
            onClick={onNext}
            disabled={!isValidKey}
          >
            Continue
          </button>
        </div>
      )}
    </div>
  );
}

/**
 * Review step - shows configuration summary before saving.
 */
function ReviewStep({
  config,
  privateKey,
  onSave,
  onPrev,
  isSaving,
}: {
  config: BrandConfig;
  privateKey: string;
  onSave: () => void;
  onPrev: () => void;
  isSaving: boolean;
}) {
  return (
    <div className="wizard-content">
      <h2 className="wizard-title">Review Configuration</h2>
      <p className="wizard-description">
        Please review your configuration before saving.
      </p>

      <div className="wizard-summary">
        <div className="wizard-summary-item">
          <span className="wizard-summary-label">Display Name</span>
          <span className="wizard-summary-value">
            {config.product.displayName}
          </span>
        </div>
        <div className="wizard-summary-item">
          <span className="wizard-summary-label">Server Name</span>
          <span className="wizard-summary-value">
            {config.product.serverName}
          </span>
        </div>
        <div className="wizard-summary-item">
          <span className="wizard-summary-label">Update URL</span>
          <span className="wizard-summary-value">{config.updateUrl}</span>
        </div>
        {config.product.description && (
          <div className="wizard-summary-item">
            <span className="wizard-summary-label">Description</span>
            <span className="wizard-summary-value">
              {config.product.description}
            </span>
          </div>
        )}
        <div className="wizard-summary-item">
          <span className="wizard-summary-label">Auto Migration</span>
          <span className="wizard-summary-value">
            {config.migration?.autoMigrateOnFirstLaunch ? "Enabled" : "Disabled"}
          </span>
        </div>
        {config.migration?.autoDetectPath && (
          <div className="wizard-summary-item">
            <span className="wizard-summary-label">Auto-Detect Path</span>
            <span className="wizard-summary-value">
              {config.migration.autoDetectPath}
            </span>
          </div>
        )}
        <div className="wizard-summary-item">
          <span className="wizard-summary-label">Public Key</span>
          <span className="wizard-summary-value wizard-key-truncated">
            {config.publicKey.substring(0, 16)}...{config.publicKey.substring(48)}
          </span>
        </div>
      </div>

      {privateKey && (
        <div className="wizard-validation warning">
          <span className="wizard-validation-icon">&#9888;</span>
          <span>
            Make sure you've saved your private key before proceeding!
          </span>
        </div>
      )}

      <div className="wizard-actions">
        <button className="wizard-button secondary" onClick={onPrev}>
          Back
        </button>
        <button
          className="wizard-button primary"
          onClick={onSave}
          disabled={isSaving}
        >
          {isSaving ? "Saving..." : "Save Configuration"}
        </button>
      </div>
    </div>
  );
}

/**
 * Complete step - shows success message after configuration is saved.
 */
function CompleteStep({
  config: _config,
  privateKey,
  onComplete,
}: {
  config: BrandConfig;
  privateKey: string;
  onComplete?: () => void;
}) {
  const [privateKeyCopied, setPrivateKeyCopied] = useState(false);

  const copyPrivateKey = async () => {
    try {
      await navigator.clipboard.writeText(privateKey);
      setPrivateKeyCopied(true);
      setTimeout(() => setPrivateKeyCopied(false), 2000);
    } catch {
      // Clipboard API not available
    }
  };

  return (
    <div className="wizard-content">
      <div className="wizard-icon success">&#10003;</div>
      <h2 className="wizard-title">Setup Complete!</h2>
      <p className="wizard-description">
        Your branded launcher configuration has been saved.
        You can now build your customized launcher.
      </p>

      <div className="wizard-summary">
        <div className="wizard-summary-item">
          <span className="wizard-summary-label">Configuration saved to:</span>
          <span className="wizard-summary-value">branding/brand.json</span>
        </div>
      </div>

      {privateKey && (
        <div className="wizard-private-key-reminder">
          <h3 className="wizard-subtitle">Private Key</h3>
          <p className="wizard-note">
            This is your private signing key. Store it securely and never share it.
          </p>
          <div className="wizard-key-item warning">
            <code className="wizard-key-value">{privateKey}</code>
            <button
              className="wizard-button secondary small"
              onClick={copyPrivateKey}
            >
              {privateKeyCopied ? "Copied!" : "Copy to Clipboard"}
            </button>
          </div>
        </div>
      )}

      <div className="wizard-next-steps">
        <h3 className="wizard-subtitle">Next Steps</h3>
        <ol className="wizard-steps-list">
          <li>Build your launcher: <code>npm run tauri build</code></li>
          <li>Set up your update server with the host-server tool</li>
          <li>Sign your manifests with the private key</li>
          <li>Distribute your branded launcher to players</li>
        </ol>
      </div>

      <div className="wizard-actions">
        <button className="wizard-button primary large" onClick={onComplete}>
          Finish
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
      <h2 className="wizard-title">Setup Failed</h2>
      <p className="wizard-description">
        An error occurred during setup. Please try again.
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
 * Validate branding configuration.
 */
function validateConfig(config: BrandConfig): Record<string, string> {
  const errors: Record<string, string> = {};

  if (!config.product.displayName.trim()) {
    errors.displayName = "Display name is required";
  }

  if (!config.product.serverName.trim()) {
    errors.serverName = "Server name is required";
  } else if (/\s/.test(config.product.serverName)) {
    errors.serverName = "Server name cannot contain spaces";
  }

  if (!config.updateUrl.trim()) {
    errors.updateUrl = "Update URL is required";
  } else if (
    !config.updateUrl.startsWith("http://") &&
    !config.updateUrl.startsWith("https://")
  ) {
    errors.updateUrl = "Update URL must start with http:// or https://";
  }

  if (config.migration?.autoMigrateOnFirstLaunch) {
    const detectPath = config.migration.autoDetectPath?.trim() || "";
    if (!detectPath) {
      errors.autoDetectPath =
        "Auto-detect path is required when auto-migration is enabled";
    }
  }

  return errors;
}

/**
 * Create default brand configuration.
 */
function createDefaultConfig(initial?: Partial<BrandConfig>): BrandConfig {
  return {
    product: {
      displayName: initial?.product?.displayName || "",
      serverName: initial?.product?.serverName || "",
      description: initial?.product?.description || "",
      supportEmail: initial?.product?.supportEmail || "",
      website: initial?.product?.website || "",
      discord: initial?.product?.discord || "",
    },
    updateUrl: initial?.updateUrl || "",
    publicKey: initial?.publicKey || "",
    ui: initial?.ui || {
      colors: {
        primary: "#1a1a2e",
        secondary: "#e94560",
        background: "#16213e",
        text: "#ffffff",
      },
      showPatchNotes: true,
    },
    migration: initial?.migration || {
      autoDetectPath: "",
      autoMigrateOnFirstLaunch: false,
    },
    brandVersion: initial?.brandVersion || "1.0",
  };
}

/**
 * Main SetupWizard component.
 */
export function SetupWizard({ onComplete, initialConfig }: SetupWizardProps) {
  const [currentStep, setCurrentStep] = useState<SetupWizardStep>("welcome");
  const [config, setConfig] = useState<BrandConfig>(() =>
    createDefaultConfig(initialConfig)
  );
  const [privateKey, setPrivateKey] = useState("");
  const [isGenerating, setIsGenerating] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [validationErrors, setValidationErrors] = useState<Record<string, string>>({});

  // Define wizard steps for the indicator
  const steps: { id: SetupWizardStep; label: string }[] = [
    { id: "welcome", label: "Welcome" },
    { id: "branding", label: "Branding" },
    { id: "keypair", label: "Security" },
    { id: "review", label: "Review" },
    { id: "complete", label: "Done" },
  ];

  // Filter out error step from indicator
  const visibleSteps = steps.filter((s) => s.id !== "error");

  const handleConfigChange = useCallback((newConfig: BrandConfig) => {
    setConfig(newConfig);
    setValidationErrors(validateConfig(newConfig));
  }, []);

  const goToStep = (step: SetupWizardStep) => {
    setCurrentStep(step);
    setErrorMessage(null);
  };

  const handleGenerateKeypair = async () => {
    setIsGenerating(true);
    setErrorMessage(null);
    try {
      const result = await invoke<KeypairResult>("generate_keypair");
      setConfig((prev) => ({ ...prev, publicKey: result.publicKey }));
      setPrivateKey(result.privateKey);
    } catch (error) {
      setErrorMessage(
        error instanceof Error ? error.message : "Failed to generate keypair"
      );
    } finally {
      setIsGenerating(false);
    }
  };

  const handleImportPublicKey = (key: string) => {
    setConfig((prev) => ({ ...prev, publicKey: key }));
    setPrivateKey(""); // Clear private key when importing
  };

  const handleSaveConfig = async () => {
    setIsSaving(true);
    setErrorMessage(null);
    try {
      await invoke("save_brand_config", { config });
      goToStep("complete");
    } catch (error) {
      setErrorMessage(
        error instanceof Error ? error.message : "Failed to save configuration"
      );
      goToStep("error");
    } finally {
      setIsSaving(false);
    }
  };

  const handleComplete = () => {
    if (onComplete) {
      onComplete(config);
    }
  };

  const handleRetry = () => {
    goToStep("welcome");
  };

  const nextStep = () => {
    const stepOrder: SetupWizardStep[] = [
      "welcome",
      "branding",
      "keypair",
      "review",
      "complete",
    ];
    const currentIndex = stepOrder.indexOf(currentStep);
    if (currentIndex < stepOrder.length - 1) {
      goToStep(stepOrder[currentIndex + 1]);
    }
  };

  const prevStep = () => {
    const stepOrder: SetupWizardStep[] = [
      "welcome",
      "branding",
      "keypair",
      "review",
      "complete",
    ];
    const currentIndex = stepOrder.indexOf(currentStep);
    if (currentIndex > 0) {
      goToStep(stepOrder[currentIndex - 1]);
    }
  };

  return (
    <div className="setup-wizard">
      <div className="wizard-container">
        {/* Step indicator (hidden on error) */}
        {currentStep !== "error" && (
          <StepIndicator currentStep={currentStep} steps={visibleSteps} />
        )}

        {/* Render current step */}
        {currentStep === "welcome" && <WelcomeStep onNext={nextStep} />}

        {currentStep === "branding" && (
          <BrandingStep
            config={config}
            onChange={handleConfigChange}
            errors={validationErrors}
            onNext={nextStep}
            onPrev={prevStep}
          />
        )}

        {currentStep === "keypair" && (
          <KeypairStep
            publicKey={config.publicKey}
            privateKey={privateKey}
            isGenerating={isGenerating}
            error={errorMessage}
            onGenerate={handleGenerateKeypair}
            onImportPublicKey={handleImportPublicKey}
            onNext={nextStep}
            onPrev={prevStep}
          />
        )}

        {currentStep === "review" && (
          <ReviewStep
            config={config}
            privateKey={privateKey}
            onSave={handleSaveConfig}
            onPrev={prevStep}
            isSaving={isSaving}
          />
        )}

        {currentStep === "complete" && (
          <CompleteStep
            config={config}
            privateKey={privateKey}
            onComplete={handleComplete}
          />
        )}

        {currentStep === "error" && (
          <ErrorStep errorMessage={errorMessage} onRetry={handleRetry} />
        )}
      </div>
    </div>
  );
}
