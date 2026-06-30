use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFileRecord {
    pub relative_path: String,
    pub size: u64,
    pub modified_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecord {
    pub id: String,
    pub display_name: String,
    pub source: String,
    pub primary_path: String,
    pub total_bytes: u64,
    pub file_count: u32,
    pub scanned_at: String,
    pub revision: Option<String>,
    #[serde(default)]
    pub files: Vec<ModelFileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupDrive {
    pub id: String,
    pub label: String,
    pub root_path: String,
    pub volume_id: Option<String>,
    pub is_default: bool,
    pub last_seen_at: Option<String>,
    pub is_mounted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelBackupStatus {
    pub drive_id: String,
    pub drive_label: String,
    pub status: String,
    pub backup_path: Option<String>,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelWithBackups {
    #[serde(flatten)]
    pub model: ModelRecord,
    pub backups: Vec<ModelBackupStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardStats {
    pub total_models: u32,
    pub total_bytes: u64,
    pub backed_up_count: u32,
    pub backup_coverage_pct: f64,
    pub drive_count: u32,
    pub mounted_drives: u32,
    pub lmstudio_bytes: u64,
    pub huggingface_bytes: u64,
    pub omlx_bytes: u64,
    pub ollama_bytes: u64,
    pub jan_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
    Backup,
    Sync,
    Restore,
    Delete,
    Offload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub id: String,
    pub job_type: String,
    pub status: String,
    pub model_id: Option<String>,
    pub drive_id: Option<String>,
    pub progress_pct: f64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub current_file: Option<String>,
    pub message: Option<String>,
    pub created_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgressEvent {
    pub job_id: String,
    pub job_type: Option<String>,
    pub model_id: Option<String>,
    pub progress_pct: f64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub current_file: Option<String>,
    pub message: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeleteScope {
    SourceOnly,
    BackupOnly,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub lmstudio_path_override: Option<String>,
    pub hf_cache_path_override: Option<String>,
    pub omlx_path_override: Option<String>,
    pub ollama_models_override: Option<String>,
    pub jan_data_override: Option<String>,
    pub verify_hashes: bool,
    pub warn_if_app_running: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            lmstudio_path_override: None,
            hf_cache_path_override: None,
            omlx_path_override: None,
            ollama_models_override: None,
            jan_data_override: None,
            verify_hashes: false,
            warn_if_app_running: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningAppsCheck {
    pub lmstudio_running: bool,
    pub huggingface_running: bool,
    pub omlx_running: bool,
    pub ollama_running: bool,
    pub jan_running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub model_id: String,
    pub display_name: String,
    pub source: String,
    pub source_path: String,
    pub backup_version: u32,
    pub created_at: String,
    pub files: Vec<ManifestFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFileEntry {
    pub relative_path: String,
    pub size: u64,
    pub modified_at: i64,
    pub sha256: Option<String>,
}
