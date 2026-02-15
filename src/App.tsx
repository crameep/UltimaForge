import { useState, useEffect } from "react";
import { Layout } from "./components/Layout";
import { InstallWizard } from "./components/InstallWizard";
import { UpdateProgress, useUpdate } from "./components/UpdateProgress";
import { LaunchButton } from "./components/LaunchButton";
import { PatchNotes } from "./components/PatchNotes";
import { Settings } from "./components/Settings";
import { checkNeedsInstall } from "./hooks/useInstall";
import "./App.css";

type AppPhase =
  | "Initializing"
  | "NeedsInstall"
  | "Installing"
  | "CheckingUpdates"
  | "UpdateAvailable"
  | "Updating"
  | "Ready"
  | "GameRunning"
  | "Error";

/** Current view within the application. */
type AppView = "home" | "settings";

function App() {
  const [phase, setPhase] = useState<AppPhase>("Initializing");
  const [statusMessage, setStatusMessage] = useState<string>("");
  const [currentView, setCurrentView] = useState<AppView>("home");

  // Update state management
  const [updateState, updateActions] = useUpdate();

  // Navigation handlers
  const navigateToSettings = () => setCurrentView("settings");
  const navigateToHome = () => setCurrentView("home");

  // Check installation status on mount
  useEffect(() => {
    const checkInstallation = async () => {
      try {
        const status = await checkNeedsInstall();
        if (status.needs_install || !status.install_complete) {
          setPhase("NeedsInstall");
          setStatusMessage("Installation required");
        } else {
          // Installation complete, check for updates
          setPhase("CheckingUpdates");
          setStatusMessage("Checking for updates...");
          await updateActions.checkForUpdates();
          setPhase("Ready");
          setStatusMessage("");
        }
      } catch (error) {
        // If the check fails (e.g., in dev mode without backend),
        // default to showing the wizard for testing
        setPhase("NeedsInstall");
        setStatusMessage("Ready for installation");
      }
    };

    checkInstallation();
  }, []);

  // Sync update state with app phase
  useEffect(() => {
    if (updateState.isUpdating) {
      setPhase("Updating");
      setStatusMessage("Updating...");
    } else if (updateState.isComplete) {
      setPhase("Ready");
      setStatusMessage("Update complete!");
    } else if (updateState.updateAvailable) {
      setPhase("UpdateAvailable");
      setStatusMessage("Update available");
    } else if (updateState.isChecking) {
      setPhase("CheckingUpdates");
      setStatusMessage("Checking for updates...");
    }
  }, [updateState.isUpdating, updateState.isComplete, updateState.updateAvailable, updateState.isChecking]);

  const handleUpdateRequest = () => {
    // Start the update process
    updateActions.startUpdate();
  };

  const handleLaunchSuccess = (pid: number | null, shouldClose: boolean) => {
    setPhase("GameRunning");
    setStatusMessage(pid ? `Game running (PID: ${pid})` : "Game is running");
    // Handle shouldClose if needed (e.g., close launcher window)
  };

  const handleLaunchError = (error: string) => {
    setStatusMessage(`Launch failed: ${error}`);
  };

  const handleGameStateChange = (isRunning: boolean) => {
    if (isRunning) {
      setPhase("GameRunning");
      setStatusMessage("Game is running");
    } else {
      setPhase("Ready");
      setStatusMessage("");
    }
  };

  const handleInstallComplete = () => {
    // After installation, check for updates
    setPhase("CheckingUpdates");
    setStatusMessage("Checking for updates...");
    updateActions.checkForUpdates().then(() => {
      if (!updateState.updateAvailable) {
        setPhase("Ready");
        setStatusMessage("Installation complete!");
      }
    });
  };

  const handleUpdateComplete = () => {
    setPhase("Ready");
    setStatusMessage("Update complete!");
  };

  const handleUpdateDismiss = () => {
    setPhase("Ready");
    setStatusMessage("");
  };

  // Show install wizard when installation is needed
  if (phase === "NeedsInstall" || phase === "Installing") {
    return (
      <Layout
        phase={phase}
        statusMessage={statusMessage}
        version="v0.1.0"
      >
        <InstallWizard
          serverName="UltimaForge"
          onComplete={handleInstallComplete}
        />
      </Layout>
    );
  }

  // Show loading state while initializing
  if (phase === "Initializing") {
    return (
      <Layout
        phase={phase}
        statusMessage="Checking installation..."
        version="v0.1.0"
      >
        <div className="main-content">
          <div className="hero-section">
            <h1 className="hero-title">UltimaForge</h1>
            <p className="hero-subtitle">Loading...</p>
          </div>
        </div>
      </Layout>
    );
  }

  // Show update progress when updating
  if (phase === "Updating" || updateState.isUpdating) {
    return (
      <Layout
        phase={phase}
        statusMessage={statusMessage}
        version="v0.1.0"
      >
        <div className="main-content">
          <UpdateProgress
            onComplete={handleUpdateComplete}
            onDismiss={handleUpdateDismiss}
          />
        </div>
      </Layout>
    );
  }

  // Show checking for updates state
  if (phase === "CheckingUpdates" && updateState.isChecking) {
    return (
      <Layout
        phase={phase}
        statusMessage="Checking for updates..."
        version="v0.1.0"
      >
        <div className="main-content">
          <div className="hero-section">
            <h1 className="hero-title">UltimaForge</h1>
            <p className="hero-subtitle">Checking for updates...</p>
          </div>
        </div>
      </Layout>
    );
  }

  // Show settings view
  if (currentView === "settings") {
    return (
      <Layout
        phase={phase}
        statusMessage={statusMessage}
        version="v0.1.0"
        onHomeClick={navigateToHome}
        onSettingsClick={navigateToSettings}
      >
        <Settings onBack={navigateToHome} />
      </Layout>
    );
  }

  // Main application view (Ready state)
  return (
    <Layout
      phase={phase}
      statusMessage={statusMessage}
      version="v0.1.0"
      onHomeClick={navigateToHome}
      onSettingsClick={navigateToSettings}
    >
      <div className="main-content">
        <div className="hero-section">
          <h1 className="hero-title">Welcome to UltimaForge</h1>
          <p className="hero-subtitle">Your adventure awaits</p>
        </div>

        {/* Show update banner if update is available */}
        {updateState.updateAvailable && updateState.checkResult && (
          <UpdateProgress
            showBanner={true}
            checkResult={updateState.checkResult}
            onComplete={handleUpdateComplete}
            onDismiss={handleUpdateDismiss}
          />
        )}

        {/* Show patch notes when available */}
        {updateState.checkResult?.patch_notes_url && (
          <PatchNotes
            patchNotesUrl={updateState.checkResult.patch_notes_url}
            version={updateState.checkResult.server_version}
            defaultCollapsed={!updateState.updateAvailable}
          />
        )}

        <div className="action-section">
          <LaunchButton
            disabled={phase === "Installing" || phase === "Updating" || phase === "CheckingUpdates"}
            updateAvailable={updateState.updateAvailable}
            onUpdateRequest={handleUpdateRequest}
            onLaunchSuccess={handleLaunchSuccess}
            onLaunchError={handleLaunchError}
            onGameStateChange={handleGameStateChange}
          />

          {phase === "UpdateAvailable" && !updateState.checkResult && (
            <p className="update-notice">
              An update is available. Click Play to update and launch.
            </p>
          )}
        </div>

        <div className="status-section">
          <p className="status-text">
            Status: <span className="status-value">{phase}</span>
          </p>
        </div>
      </div>
    </Layout>
  );
}

export default App;
