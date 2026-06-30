import { useEffect, useState } from "react";
import { checkRunningApps, getSettings, saveSettings } from "../api/client";
import type { AppSettings, RunningAppsCheck } from "../types";

const PATH_FIELDS: {
  id: keyof AppSettings;
  label: string;
  placeholder: string;
}[] = [
  {
    id: "lmstudio_path_override",
    label: "LM Studio",
    placeholder: "~/.lmstudio",
  },
  {
    id: "hf_cache_path_override",
    label: "Hugging Face cache",
    placeholder: "~/.cache/huggingface/hub",
  },
  {
    id: "omlx_path_override",
    label: "oMLX models",
    placeholder: "~/.omlx/models",
  },
  {
    id: "ollama_models_override",
    label: "Ollama models",
    placeholder: "~/.ollama/models",
  },
  {
    id: "jan_data_override",
    label: "Jan data folder",
    placeholder: "~/Library/Application Support/jan",
  },
];

const RUNNING_APPS: {
  key: keyof RunningAppsCheck;
  label: string;
}[] = [
  { key: "lmstudio_running", label: "LM Studio" },
  { key: "huggingface_running", label: "Hugging Face" },
  { key: "omlx_running", label: "oMLX" },
  { key: "ollama_running", label: "Ollama" },
  { key: "jan_running", label: "Jan" },
];

export function SettingsPage() {
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [apps, setApps] = useState<RunningAppsCheck | null>(null);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    getSettings().then(setSettings).catch((e) => setError(String(e)));
    checkRunningApps().then(setApps).catch(() => {});
  }, []);

  const handleSave = async () => {
    if (!settings) return;
    setSaving(true);
    try {
      await saveSettings(settings);
      setError(null);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const updatePath = (id: (typeof PATH_FIELDS)[number]["id"], value: string) => {
    if (!settings) return;
    setSettings({ ...settings, [id]: value || null });
  };

  if (!settings) return <div className="loading">Loading settings...</div>;

  return (
    <div className="page settings-page">
      <header className="settings-header">
        <div>
          <h2>Settings</h2>
          <p className="muted settings-lead">
            Configure where models are discovered and how backups behave.
          </p>
        </div>
        <button type="button" onClick={handleSave} disabled={saving}>
          {saving ? "Saving…" : "Save settings"}
        </button>
      </header>

      {error && <div className="error">{error}</div>}
      {saved && <div className="success">Settings saved.</div>}

      <div className="settings-grid">
        <section className="settings-card">
          <div className="settings-card-head">
            <h3>Source paths</h3>
            <p className="muted">
              Leave blank to use each provider&apos;s default location. Set a path only if you
              moved model storage elsewhere.
            </p>
          </div>
          <div className="settings-path-list">
            {PATH_FIELDS.map((field) => (
              <div className="form-field" key={field.id}>
                <label htmlFor={`settings-${field.id}`}>{field.label}</label>
                <input
                  id={`settings-${field.id}`}
                  type="text"
                  value={(settings[field.id] as string | null | undefined) ?? ""}
                  onChange={(e) => updatePath(field.id, e.target.value)}
                  placeholder={field.placeholder}
                  spellCheck={false}
                />
              </div>
            ))}
          </div>
        </section>

        <div className="settings-side">
          <section className="settings-card">
            <div className="settings-card-head">
              <h3>Backup options</h3>
            </div>
            <div className="settings-options">
              <label className="settings-option">
                <input
                  type="checkbox"
                  checked={settings.verify_hashes}
                  onChange={(e) =>
                    setSettings({ ...settings, verify_hashes: e.target.checked })
                  }
                />
                <span>
                  <strong>Verify backup hashes</strong>
                  <span className="muted settings-option-desc">
                    Compute SHA-256 when writing manifests (slower, more thorough).
                  </span>
                </span>
              </label>
              <label className="settings-option">
                <input
                  type="checkbox"
                  checked={settings.warn_if_app_running}
                  onChange={(e) =>
                    setSettings({ ...settings, warn_if_app_running: e.target.checked })
                  }
                />
                <span>
                  <strong>Block destructive ops while apps run</strong>
                  <span className="muted settings-option-desc">
                    Prevent delete and offload when a provider app is still open.
                  </span>
                </span>
              </label>
            </div>
          </section>

          {apps && (
            <section className="settings-card">
              <div className="settings-card-head">
                <h3>Running apps</h3>
                <p className="muted">Detected provider processes on this machine.</p>
              </div>
              <ul className="app-status-list">
                {RUNNING_APPS.map(({ key, label }) => {
                  const running = apps[key];
                  return (
                    <li key={key}>
                      <span>{label}</span>
                      <span
                        className={`app-status-pill ${running ? "running" : "idle"}`}
                        aria-label={running ? "Running" : "Not running"}
                      >
                        {running ? "Running" : "Not running"}
                      </span>
                    </li>
                  );
                })}
              </ul>
            </section>
          )}
        </div>
      </div>
    </div>
  );
}
