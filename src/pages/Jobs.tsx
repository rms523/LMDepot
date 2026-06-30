import { useCallback, useEffect, useState } from "react";
import {
  cancelJob,
  formatBytes,
  formatDate,
  listJobs,
  onJobProgress,
} from "../api/client";
import { ProgressBar, StatusBadge } from "../components/Badges";
import type { JobProgressEvent, JobRecord } from "../types";

export function JobsPage() {
  const [jobs, setJobs] = useState<JobRecord[]>([]);
  const [liveProgress, setLiveProgress] = useState<Record<string, JobProgressEvent>>({});
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setJobs(await listJobs());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    load();
    const interval = setInterval(load, 3000);
    return () => clearInterval(interval);
  }, [load]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onJobProgress((event) => {
      setLiveProgress((prev) => ({ ...prev, [event.job_id]: event }));
      if (["completed", "failed", "cancelled"].includes(event.status)) {
        load();
      }
    }).then((fn) => {
      unlisten = fn;
    });
    return () => unlisten?.();
  }, [load]);

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

            return (
              <div key={job.id} className="job-card">
                <div className="job-header">
                  <span className="job-type">{job.job_type}</span>
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
                {job.message && <div className="job-message">{job.message}</div>}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
