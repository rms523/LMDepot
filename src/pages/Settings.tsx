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
          Override default paths if you relocated model storage for any provider.
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
          Hugging Face cache override
          <input
            value={settings.hf_cache_path_override ?? ""}
            onChange={(e) =>
              setSettings({ ...settings, hf_cache_path_override: e.target.value || null })
            }
            placeholder="~/.cache/huggingface/hub"
          />
        </label>
        <label>
          oMLX model directory override
          <input
            value={settings.omlx_path_override ?? ""}
            onChange={(e) =>
              setSettings({ ...settings, omlx_path_override: e.target.value || null })
            }
            placeholder="~/.omlx/models"
          />
        </label>
        <label>
          Ollama models directory override
          <input
            value={settings.ollama_models_override ?? ""}
            onChange={(e) =>
              setSettings({ ...settings, ollama_models_override: e.target.value || null })
            }
            placeholder="~/.ollama/models"
          />
        </label>
        <label>
          Jan data folder override
          <input
            value={settings.jan_data_override ?? ""}
            onChange={(e) =>
              setSettings({ ...settings, jan_data_override: e.target.value || null })
            }
            placeholder="~/Library/Application Support/jan"
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
          Block destructive operations when a model provider app is running
        </label>
      </section>

      {apps && (
        <section className="section">
          <h3>Running apps</h3>
          <div>
            LM Studio: {apps.lmstudio_running ? "Running" : "Not running"}
          </div>
          <div>
            Hugging Face: {apps.huggingface_running ? "Running" : "Not running"}
          </div>
          <div>
            oMLX: {apps.omlx_running ? "Running" : "Not running"}
          </div>
          <div>
            Ollama: {apps.ollama_running ? "Running" : "Not running"}
          </div>
          <div>
            Jan: {apps.jan_running ? "Running" : "Not running"}
          </div>
        </section>
      )}

      <button onClick={handleSave}>Save settings</button>
    </div>
  );
}
