import { useCallback, useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  addBackupDrive,
  formatDate,
  importFromBackupDrive,
  listBackupDrives,
  removeBackupDrive,
  startBackupAll,
  startRestoreAll,
  startSyncAll,
} from "../api/client";
import { ActionButtonWithHint } from "../components/ActionButtonWithHint";
import { StatusBadge } from "../components/Badges";
import type { BackupDrive } from "../types";

export function DrivesPage() {
  const [drives, setDrives] = useState<BackupDrive[]>([]);
  const [label, setLabel] = useState("");
  const [rootPath, setRootPath] = useState("");
  const [isDefault, setIsDefault] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [bulkBusy, setBulkBusy] = useState(false);
  const [importBusy, setImportBusy] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      setDrives(await listBackupDrives());
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const pickFolder = async () => {
    const selected = await open({ directory: true, multiple: false });
    if (selected) {
      setRootPath(typeof selected === "string" ? selected : selected);
    }
  };

  const handleAdd = async () => {
    if (!label.trim() || !rootPath.trim()) {
      setError("Label and path are required");
      return;
    }
    try {
      await addBackupDrive(label.trim(), rootPath.trim(), isDefault);
      setLabel("");
      setRootPath("");
      setIsDefault(false);
      setError(null);
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  const handleRemove = async (id: string) => {
    if (!confirm("Remove this backup drive registration? Backup files on disk are not deleted.")) {
      return;
    }
    try {
      await removeBackupDrive(id);
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  const runBulk = async (action: "backup" | "sync" | "restore", drive: BackupDrive) => {
    if (!drive.is_mounted) {
      setError(`Drive "${drive.label}" is not mounted`);
      return;
    }
    setBulkBusy(true);
    setError(null);
    setSuccess(null);
    try {
      const jobId =
        action === "backup"
          ? await startBackupAll(drive.id)
          : action === "sync"
            ? await startSyncAll(drive.id)
            : await startRestoreAll(drive.id);
      const label =
        action === "backup" ? "Backup" : action === "sync" ? "Sync" : "Restore";
      setSuccess(
        `${label} all started — check Jobs tab (${jobId.slice(0, 8)}...)`
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBulkBusy(false);
    }
  };

  const runImport = async (drive: BackupDrive) => {
    if (!drive.is_mounted) {
      setError(`Drive "${drive.label}" is not mounted`);
      return;
    }
    setImportBusy(true);
    setError(null);
    setSuccess(null);
    try {
      const result = await importFromBackupDrive(drive.id);
      let message = result.message;
      if (result.error_count > 0) {
        message += ` (${result.error_count} error(s))`;
      }
      setSuccess(message);
    } catch (e) {
      setError(String(e));
    } finally {
      setImportBusy(false);
    }
  };

  return (
    <div className="page">
      <h2>Backup Drives</h2>
      <p className="muted">
        Register external drives or folders where model backups are stored. On a new computer,
        add your drive and use <strong>Import from drive</strong> to discover backups, then{" "}
        <strong>Restore all</strong> to copy them into local LM Studio / Hugging Face folders.
      </p>

      <section className="section">
        <h3>Add drive</h3>
        <div className="drive-form">
          <div className="form-field">
            <label htmlFor="drive-label">Label</label>
            <input
              id="drive-label"
              type="text"
              value={label}
              onChange={(e) => setLabel(e.target.value)}
              placeholder="e.g. Samsung T7"
            />
          </div>

          <div className="form-field">
            <label htmlFor="drive-path">Root path</label>
            <div className="path-row">
              <input
                id="drive-path"
                type="text"
                value={rootPath}
                onChange={(e) => setRootPath(e.target.value)}
                placeholder="/Volumes/MyDrive/LMDepot"
              />
              <button type="button" className="secondary" onClick={pickFolder}>
                Browse
              </button>
            </div>
          </div>

          <div className="form-field form-field-inline">
            <label className="checkbox-label">
              <input
                type="checkbox"
                checked={isDefault}
                onChange={(e) => setIsDefault(e.target.checked)}
              />
              Set as default drive
            </label>
          </div>

          <div className="form-field form-field-submit">
            <button type="button" onClick={handleAdd}>
              Add drive
            </button>
          </div>
        </div>
      </section>

      {error && <div className="error">{error}</div>}
      {success && <div className="success">{success}</div>}

      <section className="section">
        <h3>Registered drives</h3>
        {loading ? (
          <div className="loading">Loading...</div>
        ) : drives.length === 0 ? (
          <div className="empty">No backup drives registered yet.</div>
        ) : (
          <table className="data-table">
            <thead>
              <tr>
                <th>Label</th>
                <th>Path</th>
                <th>Status</th>
                <th>Default</th>
                <th>Last seen</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {drives.map((d) => (
                <tr key={d.id}>
                  <td>{d.label}</td>
                  <td>
                    <code className="path-code">{d.root_path}</code>
                  </td>
                  <td>
                    <StatusBadge status={d.is_mounted ? "backed_up" : "missing"} />
                  </td>
                  <td>{d.is_default ? "Yes" : "—"}</td>
                  <td className="muted">{formatDate(d.last_seen_at)}</td>
                  <td>
                    <div className="row-actions">
                      <ActionButtonWithHint
                        label="Import"
                        buttonClassName="small"
                        hint="Scans this drive for model.manifest.json files and registers backups in the Models list (for new machines or drives created on another computer)."
                        disabled={!d.is_mounted || importBusy || bulkBusy}
                        onClick={() => runImport(d)}
                      />
                      <button
                        className="small"
                        disabled={!d.is_mounted || bulkBusy || importBusy}
                        onClick={() => runBulk("restore", d)}
                        title="Restore all imported models from this drive to local folders"
                      >
                        Restore all
                      </button>
                      <button
                        className="small"
                        disabled={!d.is_mounted || bulkBusy || importBusy}
                        onClick={() => runBulk("backup", d)}
                      >
                        Backup all
                      </button>
                      <button
                        className="small secondary"
                        disabled={!d.is_mounted || bulkBusy || importBusy}
                        onClick={() => runBulk("sync", d)}
                      >
                        Sync all
                      </button>
                      <button className="danger small" onClick={() => handleRemove(d.id)}>
                        Remove
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
