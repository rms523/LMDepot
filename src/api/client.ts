import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type {
  AppSettings,
  BackupDrive,
  DashboardStats,
  DeleteScope,
  JobProgressEvent,
  JobRecord,
  ModelWithBackups,
  RunningAppsCheck,
} from "../types";

export async function getDashboardStats(): Promise<DashboardStats> {
  return invoke("get_dashboard_stats");
}

export async function listModels(source?: string): Promise<ModelWithBackups[]> {
  return invoke("list_models", { source: source ?? null });
}

export async function getModel(modelId: string): Promise<ModelWithBackups | null> {
  return invoke("get_model", { modelId });
}

export async function rescanModels() {
  return invoke("rescan_models");
}

export async function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}

export async function saveSettings(settings: AppSettings): Promise<void> {
  return invoke("save_settings", { settings });
}

export async function checkRunningApps(): Promise<RunningAppsCheck> {
  return invoke("check_running_apps_cmd");
}

export async function listBackupDrives(): Promise<BackupDrive[]> {
  return invoke("list_backup_drives");
}

export async function addBackupDrive(
  label: string,
  rootPath: string,
  isDefault: boolean
): Promise<BackupDrive> {
  return invoke("add_backup_drive", { label, rootPath, isDefault });
}

export async function removeBackupDrive(driveId: string): Promise<void> {
  return invoke("remove_backup_drive", { driveId });
}

export async function listJobs(limit = 50): Promise<JobRecord[]> {
  return invoke("list_jobs", { limit });
}

export async function getJob(jobId: string): Promise<JobRecord | null> {
  return invoke("get_job", { jobId });
}

export async function startBackup(modelId: string, driveId: string): Promise<string> {
  return invoke("start_backup", { modelId, driveId });
}

export async function startSync(modelId: string, driveId: string): Promise<string> {
  return invoke("start_sync", { modelId, driveId });
}

export async function startBackupAll(driveId: string): Promise<string> {
  return invoke("start_backup_all", { driveId });
}

export async function startSyncAll(driveId: string): Promise<string> {
  return invoke("start_sync_all", { driveId });
}

export async function startRestore(
  modelId: string,
  driveId: string,
  targetPath?: string
): Promise<string> {
  return invoke("start_restore", { modelId, driveId, targetPath: targetPath ?? null });
}

export async function startDelete(
  modelId: string,
  scope: DeleteScope,
  driveId?: string
): Promise<string> {
  return invoke("start_delete", { modelId, driveId: driveId ?? null, scope });
}

export async function startOffload(modelId: string, driveId: string): Promise<string> {
  return invoke("start_offload", { modelId, driveId });
}

export async function reverseOffload(modelId: string, driveId: string): Promise<string> {
  return invoke("reverse_offload", { modelId, driveId });
}

export async function cancelJob(jobId: string): Promise<void> {
  return invoke("cancel_job", { jobId });
}

export function onJobProgress(callback: (event: JobProgressEvent) => void) {
  return listen<JobProgressEvent>("job-progress", (e) => callback(e.payload));
}

export function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
}

export function formatDate(iso: string | null | undefined): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString();
}
