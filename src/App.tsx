import { useState } from "react";
import { Layout } from "./components/Layout";
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
  const [phase, setPhase] = useState<AppPhase>("Ready");
  const [statusMessage, setStatusMessage] = useState<string>("");

  const handlePlay = () => {
    setPhase("GameRunning");
    setStatusMessage("Launching game...");
    // Simulate launching
    setTimeout(() => {
      setStatusMessage("Game is running");
    }, 1000);
  };

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
