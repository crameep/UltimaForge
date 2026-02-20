/**
 * CuoControls Component
 *
 * Server and assistant selection dropdowns for ClassicUO.
 * Hidden entirely if no CuoConfig is available (non-CUO servers).
 */

import "./CuoControls.css";
import type { AssistantKind, CuoConfig } from "../lib/types";

interface CuoControlsProps {
  config: CuoConfig;
  selectedAssistant: AssistantKind;
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
  selectedAssistant,
  onAssistantChange,
  disabled,
}: CuoControlsProps) {
  const showAssistantDropdown = config.available_assistants.length > 1;

  return (
    <div className="cuo-controls">
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
