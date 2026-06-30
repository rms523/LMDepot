import { useEffect, useState } from "react";
import {
  formatBytes,
  formatDate,
  getModel,
  startBackup,
  startDelete,
  startOffload,
  startRestore,
  startSync,
  reverseOffload,
} from "../api/client";
import { ActionButtonWithHint } from "../components/ActionButtonWithHint";
import { SizeBadge, SourceBadge, StatusBadge } from "../components/Badges";
import type { BackupDrive, DeleteScope, ModelWithBackups } from "../types";

interface Props {
  model: ModelWithBackups;
  drives: BackupDrive[];
  onBack: () => void;
  onModelUpdated?: (model: ModelWithBackups | null) => void;
}

export function ModelDetail({ model, drives, onBack, onModelUpdated }: Props) {
  const [detail, setDetail] = useState(model);
  const [selectedDrive, setSelectedDrive] = useState(
    drives.find((d) => d.is_default)?.id ?? drives[0]?.id ?? ""
  );
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState<DeleteScope | null>(null);

  useEffect(() => {
    setDetail(model);
  }, [model]);

  const mountedDrives = drives.filter((d) => d.is_mounted);
  const sourcePresent = detail.source_present;
  const isOffloaded = detail.is_offloaded;
  const hasBackupOnDrive =
    !!selectedDrive &&
    detail.backups.some((b) => b.drive_id === selectedDrive && b.status !== "missing");
  const hasLocalCopy = sourcePresent && !isOffloaded;
  const canRestore = !isOffloaded && !sourcePresent && hasBackupOnDrive;
  const canReverseOffload = isOffloaded && hasBackupOnDrive;

  const runAction = async (
    action: () => Promise<string>,
    label: string,
    requireDrive = true
  ) => {
    if (requireDrive && !selectedDrive) {
      setError("Select a backup drive first");
      return;
    }
    setBusy(true);
    setError(null);
    setMessage(`${label} started...`);
    try {
      const jobId = await action();
      setMessage(`${label} job started (${jobId.slice(0, 8)}...). Check Jobs tab for progress.`);
      const refreshDetail = async () => {
        const updated = await getModel(detail.id);
        if (updated) {
          setDetail(updated);
          onModelUpdated?.(updated);
        }
      };
      await refreshDetail();
      window.setTimeout(refreshDetail, 2000);
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
    const needsDrive = showDeleteConfirm !== "source_only";
    runAction(
      () =>
        startDelete(
          detail.id,
          showDeleteConfirm,
          needsDrive ? selectedDrive || undefined : undefined
        ),
      "Delete",
      needsDrive
    );
  };

  return (
    <div className="page">
      <button className="back-btn" onClick={onBack}>
        ← Back to models
      </button>

      <div className="detail-header">
        <h2>{detail.display_name}</h2>
        <SourceBadge source={detail.source} />
        {isOffloaded && <StatusBadge status="offloaded" />}
        {!sourcePresent && !isOffloaded && <StatusBadge status="source_missing" />}
      </div>

      {isOffloaded && (
        <div className="notice-banner offloaded-banner">
          This model is <strong>offloaded</strong> — local files live on the backup drive and a
          symlink remains at <code>{detail.primary_path}</code>. Use{" "}
          <strong>Reverse offload</strong> to copy files back locally.
        </div>
      )}

      {!sourcePresent && !isOffloaded && (
        <div className="notice-banner">
          Local copy removed. Files remain on your backup drive — use{" "}
          <strong>Restore from backup</strong> to copy them back to{" "}
          <code>{detail.primary_path}</code>.
        </div>
      )}

      <div className="detail-meta">
        <div>
          <strong>Path:</strong> <code>{detail.primary_path}</code>
        </div>
        <div>
          <strong>Size:</strong> {formatBytes(detail.total_bytes)} ({detail.file_count} files)
        </div>
        <div>
          <strong>Last scanned:</strong> {formatDate(detail.scanned_at)}
        </div>
        {detail.revision && (
          <div>
            <strong>Revision:</strong> {detail.revision}
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
          {detail.backups.length === 0 ? (
            <StatusBadge status="not_backed_up" />
          ) : (
            detail.backups.map((b) => (
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
            disabled={busy || !selectedDrive || !hasLocalCopy}
            onClick={() => runAction(() => startBackup(detail.id, selectedDrive), "Backup")}
          >
            Backup
          </button>
          <button
            disabled={busy || !selectedDrive || !hasLocalCopy}
            onClick={() => runAction(() => startSync(detail.id, selectedDrive), "Sync")}
          >
            Sync to backup
          </button>
          <button
            disabled={busy || !selectedDrive || !canRestore}
            onClick={() =>
              runAction(() => startRestore(detail.id, selectedDrive), "Restore")
            }
          >
            Restore from backup
          </button>
          <ActionButtonWithHint
            label="Offload (move + symlink)"
            hint="Copies this model to the backup drive, deletes the local files, and leaves a symlink at the original path so your provider app still finds the model."
            disabled={busy || !selectedDrive || !hasLocalCopy || isOffloaded}
            onClick={() => runAction(() => startOffload(detail.id, selectedDrive), "Offload")}
          />
          <ActionButtonWithHint
            label="Reverse offload"
            hint="Removes the symlink and copies the model files back from the backup drive to the original local folder."
            disabled={busy || !selectedDrive || !canReverseOffload}
            onClick={() =>
              runAction(() => reverseOffload(detail.id, selectedDrive), "Reverse offload")
            }
          />
        </div>
      </section>

      <section className="section danger-section">
        <h3>Delete</h3>
        <div className="action-grid">
          <button
            className="danger"
            disabled={busy || !hasLocalCopy}
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
              Delete <strong>{detail.display_name}</strong> (
              {showDeleteConfirm.replace("_", " ")})? This cannot be undone.
              {showDeleteConfirm === "source_only" && detail.backups.length > 0 && (
                <>
                  {" "}
                  The model will stay in this list so you can restore it from backup.
                </>
              )}
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
        <h3>Files ({detail.files.length})</h3>
        <div className="file-list">
          {detail.files.slice(0, 50).map((f) => (
            <div key={f.relative_path} className="file-row">
              <span>{f.relative_path}</span>
              <SizeBadge bytes={f.size} />
            </div>
          ))}
          {detail.files.length > 50 && (
            <div className="muted">...and {detail.files.length - 50} more files</div>
          )}
        </div>
      </section>
    </div>
  );
}
