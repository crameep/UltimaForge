import { useState } from "react";
import "./App.css";

function App() {
  const [status, setStatus] = useState<string>("Ready");

  return (
    <main className="container">
      <h1>UltimaForge</h1>
      <p className="subtitle">Self-Hosted UO Launcher</p>

      <div className="status-section">
        <p>Status: <span className="status">{status}</span></p>
      </div>

      <div className="button-section">
        <button
          className="play-button"
          onClick={() => setStatus("Launching...")}
        >
          Play
        </button>
      </div>
    </main>
  );
}

export default App;
