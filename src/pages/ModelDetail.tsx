import { useState } from "react";
import {
  formatBytes,
  formatDate,
  startBackup,
  startDelete,
  startOffload,
  startRestore,
  startSync,
  reverseOffload,
} from "../api/client";
import { SizeBadge, SourceBadge, StatusBadge } from "../components/Badges";
import type { BackupDrive, DeleteScope, ModelWithBackups } from "../types";

interface Props {
  model: ModelWithBackups;
  drives: BackupDrive[];
  onBack: () => void;
}

export function ModelDetail({ model, drives, onBack }: Props) {
  const [selectedDrive, setSelectedDrive] = useState(
    drives.find((d) => d.is_default)?.id ?? drives[0]?.id ?? ""
  );
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState<DeleteScope | null>(null);

  const mountedDrives = drives.filter((d) => d.is_mounted);

  const runAction = async (action: () => Promise<string>, label: string) => {
    if (!selectedDrive && label !== "delete") {
      setError("Select a backup drive first");
      return;
    }
    setBusy(true);
    setError(null);
    setMessage(`${label} started...`);
    try {
      const jobId = await action();
      setMessage(`${label} job started (${jobId.slice(0, 8)}...). Check Jobs tab for progress.`);
    } catch (e) {
      setError(String(e));
      setMessage(null);
    } finally {
      setBusy(false);
      setShowDeleteConfirm(null);
    }
  };

  const confirmDelete = (scope: DeleteScope) => {
    setShowDeleteConfirm(scope);
  };

  const executeDelete = () => {
    if (!showDeleteConfirm) return;
    runAction(
      () => startDelete(model.id, showDeleteConfirm, selectedDrive || undefined),
      "Delete"
    );
  };

  return (
    <div className="page">
      <button className="back-btn" onClick={onBack}>
        ← Back to models
      </button>

      <div className="detail-header">
        <h2>{model.display_name}</h2>
        <SourceBadge source={model.source} />
      </div>

      <div className="detail-meta">
        <div>
          <strong>Path:</strong> <code>{model.primary_path}</code>
        </div>
        <div>
          <strong>Size:</strong> {formatBytes(model.total_bytes)} ({model.file_count} files)
        </div>
        <div>
          <strong>Last scanned:</strong> {formatDate(model.scanned_at)}
        </div>
        {model.revision && (
          <div>
            <strong>Revision:</strong> {model.revision}
          </div>
        )}
      </div>

      <section className="section">
        <h3>Backup drives</h3>
        {mountedDrives.length === 0 ? (
          <p className="muted">No mounted backup drives. Add one in the Backup Drives tab.</p>
        ) : (
          <select
            value={selectedDrive}
            onChange={(e) => setSelectedDrive(e.target.value)}
            disabled={busy}
          >
            {mountedDrives.map((d) => (
              <option key={d.id} value={d.id}>
                {d.label} ({d.root_path})
              </option>
            ))}
          </select>
        )}

        <div className="backup-status-list">
          {model.backups.length === 0 ? (
            <StatusBadge status="not_backed_up" />
          ) : (
            model.backups.map((b) => (
              <div key={b.drive_id} className="backup-row">
                <strong>{b.drive_label}</strong> — <StatusBadge status={b.status} />
                {b.last_synced_at && (
                  <span className="muted"> synced {formatDate(b.last_synced_at)}</span>
                )}
              </div>
            ))
          )}
        </div>
      </section>

      <section className="section">
        <h3>Actions</h3>
        <div className="action-grid">
          <button
            disabled={busy || !selectedDrive}
            onClick={() => runAction(() => startBackup(model.id, selectedDrive), "Backup")}
          >
            Backup
          </button>
          <button
            disabled={busy || !selectedDrive}
            onClick={() => runAction(() => startSync(model.id, selectedDrive), "Sync")}
          >
            Sync to backup
          </button>
          <button
            disabled={busy || !selectedDrive}
            onClick={() =>
              runAction(() => startRestore(model.id, selectedDrive), "Restore")
            }
          >
            Restore from backup
          </button>
          <button
            disabled={busy || !selectedDrive}
            onClick={() => runAction(() => startOffload(model.id, selectedDrive), "Offload")}
          >
            Offload (move + symlink)
          </button>
          <button
            disabled={busy || !selectedDrive}
            onClick={() =>
              runAction(() => reverseOffload(model.id, selectedDrive), "Reverse offload")
            }
          >
            Reverse offload
          </button>
        </div>
      </section>

      <section className="section danger-section">
        <h3>Delete</h3>
        <div className="action-grid">
          <button
            className="danger"
            disabled={busy}
            onClick={() => confirmDelete("source_only")}
          >
            Delete from source
          </button>
          <button
            className="danger"
            disabled={busy || !selectedDrive}
            onClick={() => confirmDelete("backup_only")}
          >
            Delete from backup
          </button>
          <button
            className="danger"
            disabled={busy || !selectedDrive}
            onClick={() => confirmDelete("both")}
          >
            Delete from both
          </button>
        </div>
      </section>

      {showDeleteConfirm && (
        <div className="modal-overlay">
          <div className="modal">
            <h3>Confirm delete</h3>
            <p>
              Delete <strong>{model.display_name}</strong> (
              {showDeleteConfirm.replace("_", " ")})? This cannot be undone.
            </p>
            <div className="modal-actions">
              <button onClick={() => setShowDeleteConfirm(null)}>Cancel</button>
              <button className="danger" onClick={executeDelete} disabled={busy}>
                Confirm delete
              </button>
            </div>
          </div>
        </div>
      )}

      {message && <div className="success">{message}</div>}
      {error && <div className="error">{error}</div>}

      <section className="section">
        <h3>Files ({model.files.length})</h3>
        <div className="file-list">
          {model.files.slice(0, 50).map((f) => (
            <div key={f.relative_path} className="file-row">
              <span>{f.relative_path}</span>
              <SizeBadge bytes={f.size} />
            </div>
          ))}
          {model.files.length > 50 && (
            <div className="muted">...and {model.files.length - 50} more files</div>
          )}
        </div>
      </section>
    </div>
  );
}
