/**
 * CuoControls Component
 *
 * Server and assistant selection dropdowns for ClassicUO.
 * Hidden entirely if no CuoConfig is available (non-CUO servers).
 */

import "./CuoControls.css";
import type { AssistantKind, CuoConfig, ServerChoice } from "../lib/types";

interface CuoControlsProps {
  config: CuoConfig;
  selectedServer: ServerChoice;
  selectedAssistant: AssistantKind;
  onServerChange: (server: ServerChoice) => void;
  onAssistantChange: (assistant: AssistantKind) => void;
  disabled?: boolean;
}

const ASSISTANT_LABELS: Record<AssistantKind, string> = {
  razor_enhanced: "Razor Enhanced",
  razor: "Razor",
  none: "None",
};

export function CuoControls({
  config,
  selectedServer,
  selectedAssistant,
  onServerChange,
  onAssistantChange,
  disabled,
}: CuoControlsProps) {
  const showServerDropdown = false;
  const showAssistantDropdown = config.available_assistants.length > 1;

  return (
    <div className="cuo-controls">
      {showServerDropdown && (
        <div className="cuo-control-row">
          <label className="cuo-control-label">Server</label>
          <select
            className="cuo-control-select"
            value={selectedServer}
            disabled={disabled}
            onChange={(e) => onServerChange(e.target.value as ServerChoice)}
          >
            <option value="live">{config.live_server.label}</option>
            {config.test_server && (
              <option value="test">{config.test_server.label}</option>
            )}
          </select>
        </div>
      )}

      {showAssistantDropdown ? (
        <div className="cuo-control-row">
          <select
            className="cuo-control-select"
            value={selectedAssistant}
            disabled={disabled}
            onChange={(e) => onAssistantChange(e.target.value as AssistantKind)}
          >
            {config.available_assistants.map((a) => (
              <option key={a} value={a}>
                {ASSISTANT_LABELS[a]}
              </option>
            ))}
          </select>
        </div>
      ) : (
        config.available_assistants.length === 1 && (
          <div className="cuo-control-row">
            <span className="cuo-control-value">
              {ASSISTANT_LABELS[config.available_assistants[0]]}
            </span>
          </div>
        )
      )}
    </div>
  );
}
