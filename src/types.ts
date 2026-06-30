export interface ModelFileRecord {
  relative_path: string;
  size: number;
  modified_at: number;
}

export interface ModelRecord {
  id: string;
  display_name: string;
  source: string;
  primary_path: string;
  total_bytes: number;
  file_count: number;
  scanned_at: string;
  revision?: string | null;
  files: ModelFileRecord[];
}

export interface BackupDrive {
  id: string;
  label: string;
  root_path: string;
  volume_id?: string | null;
  is_default: boolean;
  last_seen_at?: string | null;
  is_mounted: boolean;
}

export interface ModelBackupStatus {
  drive_id: string;
  drive_label: string;
  status: string;
  backup_path?: string | null;
  last_synced_at?: string | null;
}

export interface ModelWithBackups {
  id: string;
  display_name: string;
  source: string;
  primary_path: string;
  total_bytes: number;
  file_count: number;
  scanned_at: string;
  revision?: string | null;
  files: ModelFileRecord[];
  backups: ModelBackupStatus[];
}

export interface DashboardStats {
  total_models: number;
  total_bytes: number;
  backed_up_count: number;
  backup_coverage_pct: number;
  drive_count: number;
  mounted_drives: number;
  lmstudio_bytes: number;
  huggingface_bytes: number;
  omlx_bytes: number;
  ollama_bytes: number;
  jan_bytes: number;
}

export interface JobRecord {
  id: string;
  job_type: string;
  status: string;
  model_id?: string | null;
  drive_id?: string | null;
  progress_pct: number;
  bytes_done: number;
  bytes_total: number;
  current_file?: string | null;
  message?: string | null;
  created_at: string;
  finished_at?: string | null;
}

export interface JobProgressEvent {
  job_id: string;
  job_type?: string | null;
  model_id?: string | null;
  progress_pct: number;
  bytes_done: number;
  bytes_total: number;
  current_file?: string | null;
  message?: string | null;
  status: string;
}

export type DeleteScope = "source_only" | "backup_only" | "both";

export interface AppSettings {
  lmstudio_path_override?: string | null;
  hf_cache_path_override?: string | null;
  omlx_path_override?: string | null;
  ollama_models_override?: string | null;
  jan_data_override?: string | null;
  verify_hashes: boolean;
  warn_if_app_running: boolean;
}

export interface RunningAppsCheck {
  lmstudio_running: boolean;
  huggingface_running: boolean;
  omlx_running: boolean;
  ollama_running: boolean;
  jan_running: boolean;
}
