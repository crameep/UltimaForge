import { useState, useEffect } from "react";
import { Layout } from "./components/Layout";
import { InstallWizard } from "./components/InstallWizard";
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

function App() {
  const [phase, setPhase] = useState<AppPhase>("Initializing");
  const [statusMessage, setStatusMessage] = useState<string>("");

  // Check installation status on mount
  useEffect(() => {
    const checkInstallation = async () => {
      try {
        const status = await checkNeedsInstall();
        if (status.needs_install || !status.install_complete) {
          setPhase("NeedsInstall");
          setStatusMessage("Installation required");
        } else {
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

  const handlePlay = () => {
    setPhase("GameRunning");
    setStatusMessage("Launching game...");
    // Simulate launching
    setTimeout(() => {
      setStatusMessage("Game is running");
    }, 1000);
  };

  const handleInstallComplete = () => {
    setPhase("Ready");
    setStatusMessage("Installation complete!");
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

  // Main application view (Ready state)
  return (
    <Layout phase={phase} statusMessage={statusMessage} version="v0.1.0">
      <div className="main-content">
        <div className="hero-section">
          <h1 className="hero-title">Welcome to UltimaForge</h1>
          <p className="hero-subtitle">Your adventure awaits</p>
        </div>

        <div className="action-section">
          <button
            className="play-button"
            onClick={handlePlay}
            disabled={phase === "Installing" || phase === "Updating"}
          >
            {phase === "GameRunning" ? "Playing..." : "Play"}
          </button>

          {phase === "UpdateAvailable" && (
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
