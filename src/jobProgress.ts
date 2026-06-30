import { listen } from "@tauri-apps/api/event";
import type { JobProgressEvent } from "./types";

const live: Record<string, JobProgressEvent> = {};
const subscribers = new Set<() => void>();

export function getLiveJobProgress(jobId: string): JobProgressEvent | undefined {
  return live[jobId];
}

export function getAllLiveProgress(): Record<string, JobProgressEvent> {
  return { ...live };
}

export function subscribeJobProgress(cb: () => void): () => void {
  subscribers.add(cb);
  return () => subscribers.delete(cb);
}

function notify() {
  subscribers.forEach((cb) => cb());
}

let listenerStarted = false;

/** Call once at app startup to receive job-progress events globally. */
export function startJobProgressListener(): Promise<() => void> {
  if (listenerStarted) {
    return Promise.resolve(() => {});
  }
  listenerStarted = true;

  return listen<JobProgressEvent>("job-progress", (event) => {
    live[event.payload.job_id] = event.payload;
    notify();
  });
}

export function clearLiveProgress(jobId: string) {
  delete live[jobId];
}
