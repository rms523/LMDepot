import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import {
  formatDate,
  listBackupDrives,
  listJobs,
  listModels,
  rescanModels,
  startBackupAll,
  startSyncAll,
} from "../api/client";
import { ActionButtonWithHint } from "../components/ActionButtonWithHint";
import { SizeBadge, SourceBadge, StatusBadge } from "../components/Badges";
import { getAllLiveProgress, subscribeJobProgress } from "../jobProgress";
import { getActiveModelIds, hasRunningModelJobs, jobLabelForModel } from "../modelJobs";
import type { BackupDrive, JobRecord, ModelWithBackups } from "../types";
import { ModelDetail } from "./ModelDetail";

function useLiveProgress() {
  return useSyncExternalStore(
    subscribeJobProgress,
    getAllLiveProgress,
    getAllLiveProgress
  );
}

export function ModelsPage() {
  const [models, setModels] = useState<ModelWithBackups[]>([]);
  const [drives, setDrives] = useState<BackupDrive[]>([]);
  const [jobs, setJobs] = useState<JobRecord[]>([]);
  const [filter, setFilter] = useState<string>("all");
  const [search, setSearch] = useState("");
  const [selectedDrive, setSelectedDrive] = useState("");
  const [loading, setLoading] = useState(true);
  const [scanning, setScanning] = useState(false);
  const [bulkBusy, setBulkBusy] = useState(false);
  const [selected, setSelected] = useState<ModelWithBackups | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const liveProgress = useLiveProgress();
  const prevRunningCount = useRef(0);
  const handledCompletions = useRef(new Set<string>());

  const mountedDrives = useMemo(() => drives.filter((d) => d.is_mounted), [drives]);
  const activeModelIds = useMemo(
    () => getActiveModelIds(jobs, liveProgress),
    [jobs, liveProgress]
  );
  const jobsRunning = hasRunningModelJobs(jobs);

  const refreshModels = useCallback(async () => {
    try {
      const m = await listModels(filter === "all" ? undefined : filter);
      setModels(m);
    } catch (e) {
      setError(String(e));
    }
  }, [filter]);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [m, d, j] = await Promise.all([
        listModels(filter === "all" ? undefined : filter),
        listBackupDrives(),
        listJobs(50),
      ]);
      setModels(m);
      setDrives(d);
      setJobs(j);
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

  // Poll jobs while work is active
  useEffect(() => {
    if (!jobsRunning) return;
    const pollJobs = async () => {
      try {
        setJobs(await listJobs(50));
      } catch {
        /* ignore transient poll errors */
      }
    };
    pollJobs();
    const interval = setInterval(pollJobs, 1000);
    return () => clearInterval(interval);
  }, [jobsRunning]);

  // Refresh model backup status while jobs run (silent, no loading spinner)
  useEffect(() => {
    if (!jobsRunning) return;
    refreshModels();
    const interval = setInterval(refreshModels, 2000);
    return () => clearInterval(interval);
  }, [jobsRunning, refreshModels]);

  // Reload when jobs finish
  useEffect(() => {
    const running = jobs.filter((j) => j.status === "running").length;
    if (prevRunningCount.current > 0 && running === 0) {
      load();
    }
    prevRunningCount.current = running;
  }, [jobs, load]);

  // Reload once when a job completes (live event arrives before DB poll)
  useEffect(() => {
    for (const [jobId, event] of Object.entries(liveProgress)) {
      if (!["completed", "failed", "cancelled"].includes(event.status)) {
        continue;
      }
      if (handledCompletions.current.has(jobId)) {
        continue;
      }
      handledCompletions.current.add(jobId);
      refreshModels();
      listJobs(50).then(setJobs).catch(() => {});
    }
  }, [liveProgress, refreshModels]);

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
      setJobs(await listJobs(50));
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
        onModelUpdated={(updated) => {
          if (!updated) return;
          setSelected(updated);
          setModels((prev) => prev.map((m) => (m.id === updated.id ? updated : m)));
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
            <option value="omlx">oMLX</option>
            <option value="ollama">Ollama</option>
            <option value="jan">Jan</option>
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
          <ActionButtonWithHint
            label="Sync all"
            buttonClassName="secondary"
            hint="For each model in this list, copies new or changed files from your local folders to the selected backup drive. Does not scan backup drives or import models that exist only on external storage."
            disabled={bulkBusy || !selectedDrive || filtered.length === 0}
            onClick={() => runBulk("sync")}
          />
          <ActionButtonWithHint
            label={scanning ? "Scanning..." : "Rescan"}
            hint="Scans local provider folders only (LM Studio, Hugging Face, oMLX, Ollama, Jan) and refreshes the model list. Does not scan backup drives."
            disabled={scanning}
            onClick={handleRescan}
          />
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
            {filtered.map((m) => {
              const inProgress = activeModelIds.has(m.id);
              const progressLabel = jobLabelForModel(m.id, jobs, liveProgress);

              return (
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
                    {inProgress ? (
                      <div className="backup-line">
                        <StatusBadge status="in_progress" />
                        {progressLabel && (
                          <span className="muted progress-hint"> {progressLabel}</span>
                        )}
                      </div>
                    ) : m.is_offloaded ? (
                      <div className="backup-line">
                        <StatusBadge status="offloaded" />
                        {m.backups.map((b) => (
                          <span key={b.drive_id} className="muted">
                            {" "}
                            · {b.drive_label}
                          </span>
                        ))}
                      </div>
                    ) : !m.source_present ? (
                      <div className="backup-line">
                        <StatusBadge status="source_missing" />
                        {m.backups.length > 0 && (
                          <span className="muted"> — restore from backup</span>
                        )}
                      </div>
                    ) : m.backups.length === 0 ? (
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
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
}
