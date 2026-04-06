/**
 * LaunchOptions Component
 *
 * Per-slot launch configuration: account credentials, character selection,
 * and auto-login settings for multi-client support.
 */

import { useState, useEffect, useCallback } from "react";
import { getLaunchOptions, saveLaunchOptions } from "../lib/api";
import type { LaunchSlotConfig } from "../lib/types";
import "./LaunchOptions.css";

interface LaunchOptionsProps {
  onBack?: () => void;
}

const EMPTY_SLOT: LaunchSlotConfig = {
  username: "",
  password: "",
  characterName: "",
  autoLogin: false,
  serverChoice: null,
};

export function LaunchOptions({ onBack }: LaunchOptionsProps) {
  const [slots, setSlots] = useState<LaunchSlotConfig[]>([
    { ...EMPTY_SLOT },
    { ...EMPTY_SLOT },
    { ...EMPTY_SLOT },
  ]);
  const [saving, setSaving] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    getLaunchOptions()
      .then((loaded) => {
        // Pad to 3 slots
        while (loaded.length < 3) loaded.push({ ...EMPTY_SLOT });
        setSlots(loaded);
      })
      .catch(() => {});
  }, []);

  const updateSlot = useCallback(
    (index: number, field: keyof LaunchSlotConfig, value: string | boolean) => {
      setSlots((prev) => {
        const next = [...prev];
        next[index] = { ...next[index], [field]: value };
        return next;
      });
      setDirty(true);
      setMessage(null);
    },
    []
  );

  const handleSave = useCallback(async () => {
    setSaving(true);
    setMessage(null);
    try {
      await saveLaunchOptions(slots);
      setMessage("Saved");
      setDirty(false);
      setTimeout(() => setMessage(null), 2000);
    } catch (e) {
      setMessage(
        `Failed to save: ${e instanceof Error ? e.message : String(e)}`
      );
    } finally {
      setSaving(false);
    }
  }, [slots]);

  const handleClear = useCallback(
    (index: number) => {
      setSlots((prev) => {
        const next = [...prev];
        next[index] = { ...EMPTY_SLOT };
        return next;
      });
      setDirty(true);
      setMessage(null);
    },
    []
  );

  return (
    <div className="launch-options">
      <div className="launch-options-header">
        {onBack && (
          <button className="launch-options-back" onClick={onBack}>
            Back
          </button>
        )}
        <h2 className="launch-options-title">Launch Options</h2>
        <p className="launch-options-subtitle">
          Configure account credentials and auto-login per client slot.
          Leave fields empty to enter them manually in CUO.
        </p>
      </div>

      <div className="launch-options-slots">
        {slots.map((slot, i) => (
          <div key={i} className="slot-card">
            <div className="slot-header">
              <h3 className="slot-title">Client {i + 1}</h3>
              <button
                className="slot-clear"
                onClick={() => handleClear(i)}
                title="Clear this slot"
              >
                Clear
              </button>
            </div>

            <div className="slot-fields">
              <div className="slot-field">
                <label className="slot-label" htmlFor={`username-${i}`}>
                  Account
                </label>
                <input
                  id={`username-${i}`}
                  className="slot-input"
                  type="text"
                  placeholder="Username"
                  value={slot.username}
                  onChange={(e) => updateSlot(i, "username", e.target.value)}
                  autoComplete="off"
                />
              </div>

              <div className="slot-field">
                <label className="slot-label" htmlFor={`password-${i}`}>
                  Password
                </label>
                <input
                  id={`password-${i}`}
                  className="slot-input"
                  type="password"
                  placeholder="Password"
                  value={slot.password}
                  onChange={(e) => updateSlot(i, "password", e.target.value)}
                  autoComplete="off"
                />
              </div>

              <div className="slot-field">
                <label className="slot-label" htmlFor={`charname-${i}`}>
                  Character
                </label>
                <input
                  id={`charname-${i}`}
                  className="slot-input"
                  type="text"
                  placeholder="Character name (optional)"
                  value={slot.characterName}
                  onChange={(e) =>
                    updateSlot(i, "characterName", e.target.value)
                  }
                  autoComplete="off"
                />
              </div>

              <div className="slot-field slot-field-toggle">
                <label className="slot-label" htmlFor={`autologin-${i}`}>
                  Auto-login
                </label>
                <label className="slot-toggle">
                  <input
                    type="checkbox"
                    id={`autologin-${i}`}
                    checked={slot.autoLogin}
                    onChange={(e) =>
                      updateSlot(i, "autoLogin", e.target.checked)
                    }
                  />
                  <span className="slot-toggle-slider" />
                </label>
              </div>
            </div>
          </div>
        ))}
      </div>

      <div className="launch-options-footer">
        <button
          className="launch-options-save"
          onClick={handleSave}
          disabled={saving || !dirty}
        >
          {saving ? "Saving..." : "Save"}
        </button>
        {message && (
          <span
            className={`launch-options-message ${message === "Saved" ? "success" : "error"}`}
          >
            {message}
          </span>
        )}
      </div>
    </div>
  );
}
