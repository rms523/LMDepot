import { useCallback, useEffect, useMemo, useState } from "react";
import {
  formatDate,
  listBackupDrives,
  listModels,
  rescanModels,
  startBackupAll,
  startSyncAll,
} from "../api/client";
import { SizeBadge, SourceBadge, StatusBadge } from "../components/Badges";
import type { BackupDrive, ModelWithBackups } from "../types";
import { ModelDetail } from "./ModelDetail";

export function ModelsPage() {
  const [models, setModels] = useState<ModelWithBackups[]>([]);
  const [drives, setDrives] = useState<BackupDrive[]>([]);
  const [filter, setFilter] = useState<string>("all");
  const [search, setSearch] = useState("");
  const [selectedDrive, setSelectedDrive] = useState("");
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [bulkBusy, setBulkBusy] = useState(false);
  const [selected, setSelected] = useState<ModelWithBackups | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const mountedDrives = useMemo(() => drives.filter((d) => d.is_mounted), [drives]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [m, d] = await Promise.all([
        listModels(filter === "all" ? undefined : filter),
        listBackupDrives(),
      ]);
      setModels(m);
      setDrives(d);
      const defaultDrive =
        d.find((drive) => drive.is_default && drive.is_mounted) ??
        d.find((drive) => drive.is_mounted);
      setSelectedDrive((prev) => {
        if (prev && d.some((drive) => drive.id === prev && drive.is_mounted)) return prev;
        return defaultDrive?.id ?? "";
      });
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, [filter]);

  useEffect(() => {
    load();
  }, [load]);

  const handleRescan = async () => {
    setScanning(true);
    try {
      await rescanModels();
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  };

  const runBulk = async (action: "backup" | "sync") => {
    if (!selectedDrive) {
      setError("Select a backup drive first");
      return;
    }
    setBulkBusy(true);
    setError(null);
    setSuccess(null);
    try {
      const jobId =
        action === "backup"
          ? await startBackupAll(selectedDrive)
          : await startSyncAll(selectedDrive);
      setSuccess(
        `${action === "backup" ? "Backup" : "Sync"} all started — check Jobs tab (${jobId.slice(0, 8)}...)`
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setBulkBusy(false);
    }
  };

  const filtered = models.filter((m) =>
    m.display_name.toLowerCase().includes(search.toLowerCase())
  );

  if (selected) {
    return (
      <ModelDetail
        model={selected}
        drives={drives}
        onBack={() => {
          setSelected(null);
          load();
        }}
      />
    );
  }

  return (
    <div className="page">
      <div className="page-header">
        <h2>Models</h2>
        <div className="toolbar">
          <input
            type="search"
            placeholder="Search models..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
          <select value={filter} onChange={(e) => setFilter(e.target.value)}>
            <option value="all">All sources</option>
            <option value="lmstudio">LM Studio</option>
            <option value="huggingface">Hugging Face</option>
          </select>
          <select
            value={selectedDrive}
            onChange={(e) => setSelectedDrive(e.target.value)}
            disabled={mountedDrives.length === 0}
            title="Target backup drive"
          >
            {mountedDrives.length === 0 ? (
              <option value="">No drives mounted</option>
            ) : (
              mountedDrives.map((d) => (
                <option key={d.id} value={d.id}>
                  {d.label}
                </option>
              ))
            )}
          </select>
          <button
            onClick={() => runBulk("backup")}
            disabled={bulkBusy || !selectedDrive || filtered.length === 0}
          >
            Backup all
          </button>
          <button
            className="secondary"
            onClick={() => runBulk("sync")}
            disabled={bulkBusy || !selectedDrive || filtered.length === 0}
          >
            Sync all
          </button>
          <button onClick={handleRescan} disabled={scanning}>
            {scanning ? "Scanning..." : "Rescan"}
          </button>
        </div>
      </div>

      {error && <div className="error">{error}</div>}
      {success && <div className="success">{success}</div>}
      {loading ? (
        <div className="loading">Loading models...</div>
      ) : filtered.length === 0 ? (
        <div className="empty">
          No models found. Click Rescan to discover models from LM Studio and Unsloth.
        </div>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Source</th>
              <th>Size</th>
              <th>Files</th>
              <th>Backup status</th>
              <th>Scanned</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((m) => (
              <tr key={m.id} onClick={() => setSelected(m)} className="clickable">
                <td className="name-cell">{m.display_name}</td>
                <td>
                  <SourceBadge source={m.source} />
                </td>
                <td>
                  <SizeBadge bytes={m.total_bytes} />
                </td>
                <td>{m.file_count}</td>
                <td>
                  {m.backups.length === 0 ? (
                    <StatusBadge status="not_backed_up" />
                  ) : (
                    m.backups.map((b) => (
                      <div key={b.drive_id} className="backup-line">
                        {b.drive_label}: <StatusBadge status={b.status} />
                      </div>
                    ))
                  )}
                </td>
                <td className="muted">{formatDate(m.scanned_at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
