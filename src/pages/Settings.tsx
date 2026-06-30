import { useEffect, useState } from "react";
import { checkRunningApps, getSettings, saveSettings } from "../api/client";
import type { AppSettings, RunningAppsCheck } from "../types";

export function SettingsPage() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [apps, setApps] = useState<RunningAppsCheck | null>(null);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getSettings().then(setSettings).catch((e) => setError(String(e)));
    checkRunningApps().then(setApps).catch(() => {});
  }, []);

  const handleSave = async () => {
    if (!settings) return;
    try {
      await saveSettings(settings);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(String(e));
    }
  };

  if (!settings) return <div className="loading">Loading settings...</div>;

  return (
    <div className="page">
      <h2>Settings</h2>

      {error && <div className="error">{error}</div>}
      {saved && <div className="success">Settings saved.</div>}

      <section className="section form-section">
        <h3>Source paths</h3>
        <p className="muted">
          Override default paths if you relocated LM Studio or Hugging Face cache.
        </p>
        <label>
          LM Studio home override
          <input
            value={settings.lmstudio_path_override ?? ""}
            onChange={(e) =>
              setSettings({ ...settings, lmstudio_path_override: e.target.value || null })
            }
            placeholder="~/.lmstudio or custom path"
          />
        </label>
        <label>
          Hugging Face cache override (Unsloth)
          <input
            value={settings.hf_cache_path_override ?? ""}
            onChange={(e) =>
              setSettings({ ...settings, hf_cache_path_override: e.target.value || null })
            }
            placeholder="~/.cache/huggingface/hub"
          />
        </label>
      </section>

      <section className="section form-section">
        <h3>Backup options</h3>
        <label className="checkbox-label">
          <input
            type="checkbox"
            checked={settings.verify_hashes}
            onChange={(e) => setSettings({ ...settings, verify_hashes: e.target.checked })}
          />
          Compute SHA-256 hashes when writing backup manifests (slower)
        </label>
        <label className="checkbox-label">
          <input
            type="checkbox"
            checked={settings.warn_if_app_running}
            onChange={(e) =>
              setSettings({ ...settings, warn_if_app_running: e.target.checked })
            }
          />
          Block destructive operations when LM Studio or Unsloth is running
        </label>
      </section>

      {apps && (
        <section className="section">
          <h3>Running apps</h3>
          <div>
            LM Studio: {apps.lmstudio_running ? "Running" : "Not running"}
          </div>
          <div>
            Unsloth: {apps.unsloth_running ? "Running" : "Not running"}
          </div>
        </section>
      )}

      <button onClick={handleSave}>Save settings</button>
    </div>
  );
}
