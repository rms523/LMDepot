import type { JobProgressEvent, JobRecord } from "./types";

const MODEL_JOB_TYPES = new Set([
  "backup",
  "sync",
  "backup_all",
  "sync_all",
  "restore",
  "restore_all",
  "offload",
  "reverse_offload",
]);

/** Model IDs with an active backup/sync/restore/offload job. */
export function getActiveModelIds(
  jobs: JobRecord[],
  live: Record<string, JobProgressEvent>
): Set<string> {
  const ids = new Set<string>();

  for (const job of jobs) {
    if (job.status !== "running" || !MODEL_JOB_TYPES.has(job.job_type)) {
      continue;
    }
    if (job.model_id) {
      ids.add(job.model_id);
    }
    const liveEvent = live[job.id];
    if (liveEvent?.status === "running" && liveEvent.model_id) {
      ids.add(liveEvent.model_id);
    }
  }

  return ids;
}

export function hasRunningModelJobs(jobs: JobRecord[]): boolean {
  return jobs.some((j) => j.status === "running" && MODEL_JOB_TYPES.has(j.job_type));
}

export function jobLabelForModel(
  modelId: string,
  jobs: JobRecord[],
  live: Record<string, JobProgressEvent> = {}
): string | null {
  for (const job of jobs) {
    if (job.status !== "running" || !MODEL_JOB_TYPES.has(job.job_type)) {
      continue;
    }
    const activeModelId = live[job.id]?.model_id ?? job.model_id;
    if (activeModelId !== modelId) {
      continue;
    }
    if (job.job_type.includes("sync")) return "Syncing…";
    if (job.job_type.includes("restore")) return "Restoring…";
    if (job.job_type === "reverse_offload") return "Reversing offload…";
    if (job.job_type.includes("offload")) return "Offloading…";
    return "Backing up…";
  }
  return null;
}
