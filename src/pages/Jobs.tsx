import { useCallback, useEffect, useState, useSyncExternalStore } from "react";
import {
  cancelJob,
  formatBytes,
  formatDate,
  listJobs,
} from "../api/client";
import { ProgressBar, StatusBadge } from "../components/Badges";
import { getAllLiveProgress, subscribeJobProgress } from "../jobProgress";
import type { JobRecord } from "../types";

function useLiveProgress() {
  return useSyncExternalStore(
    subscribeJobProgress,
    getAllLiveProgress,
    getAllLiveProgress
  );
}

export function JobsPage() {
  const [jobs, setJobs] = useState<JobRecord[]>([]);
  const liveProgress = useLiveProgress();
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setJobs(await listJobs());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const hasRunning = jobs.some((j) => j.status === "running");

  useEffect(() => {
    load();
  }, [load]);

  // Poll DB as fallback; faster while jobs are active
  useEffect(() => {
    const ms = hasRunning ? 500 : 3000;
    const interval = setInterval(load, ms);
    return () => clearInterval(interval);
  }, [load, hasRunning]);

  // Refresh list when a job finishes (live event may arrive before DB write)
  useEffect(() => {
    const finished = Object.values(liveProgress).some((e) =>
      ["completed", "failed", "cancelled"].includes(e.status)
    );
    if (finished) {
      load();
    }
  }, [liveProgress, load]);

  const handleCancel = async (jobId: string) => {
    try {
      await cancelJob(jobId);
      await load();
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="page">
      <h2>Jobs</h2>
      <p className="muted">Background operations for backup, sync, restore, delete, and offload.</p>

      {error && <div className="error">{error}</div>}

      {jobs.length === 0 ? (
        <div className="empty">No jobs yet.</div>
      ) : (
        <div className="jobs-list">
          {jobs.map((job) => {
            const live = liveProgress[job.id];
            const pct = live?.progress_pct ?? job.progress_pct;
            const status = live?.status ?? job.status;
            const currentFile = live?.current_file ?? job.current_file;
            const bytesDone = live?.bytes_done ?? job.bytes_done;
            const bytesTotal = live?.bytes_total ?? job.bytes_total;
            const message = live?.message ?? job.message;

            return (
              <div key={job.id} className="job-card">
                <div className="job-header">
                  <span className="job-type">{job.job_type.replace(/_/g, " ")}</span>
                  <StatusBadge status={status} />
                  {status === "running" && (
                    <button className="small" onClick={() => handleCancel(job.id)}>
                      Cancel
                    </button>
                  )}
                </div>
                <div className="job-meta muted">
                  {formatDate(job.created_at)}
                  {job.model_id && <> · model {job.model_id.slice(0, 20)}...</>}
                </div>
                {status === "running" && (
                  <>
                    <ProgressBar pct={pct} />
                    <div className="job-bytes">
                      {formatBytes(bytesDone)} / {formatBytes(bytesTotal)}
                    </div>
                    {currentFile && <div className="job-file">{currentFile}</div>}
                  </>
                )}
                {message && <div className="job-message">{message}</div>}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
