import { listen } from "@tauri-apps/api/event";
import type { JobProgressEvent } from "./types";

/** Mutable store — updated in place on each event. */
const live: Record<string, JobProgressEvent> = {};

/**
 * Immutable snapshot for useSyncExternalStore.
 * Must keep a stable reference between updates; only replace on notify().
 */
let snapshot: Record<string, JobProgressEvent> = live;

const subscribers = new Set<() => void>();

export function getLiveJobProgress(jobId: string): JobProgressEvent | undefined {
  return snapshot[jobId];
}

export function getAllLiveProgress(): Record<string, JobProgressEvent> {
  return snapshot;
}

export function subscribeJobProgress(cb: () => void): () => void {
  subscribers.add(cb);
  return () => subscribers.delete(cb);
}

function notify() {
  snapshot = { ...live };
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
  if (jobId in live) {
    delete live[jobId];
    notify();
  }
}
