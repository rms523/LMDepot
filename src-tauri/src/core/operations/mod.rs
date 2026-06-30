use crate::core::copy_engine::{self, CopyProgress};
use crate::core::drive_monitor::{backup_layout_path, ensure_drive_mounted};
use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::types::{BackupManifest, JobProgressEvent, JobRecord, ManifestFileEntry, ModelRecord};
use chrono::Utc;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

pub mod backup;
pub mod batch;
pub mod delete;
pub mod offload;
pub mod restore;
pub mod sync;

pub struct JobContext {
    pub db: Arc<Database>,
    pub app: AppHandle,
    pub cancelled: Arc<AtomicBool>,
}

impl JobContext {
    pub fn emit_progress(&self, job: &JobRecord, status: &str) {
        let event = JobProgressEvent {
            job_id: job.id.clone(),
            job_type: Some(job.job_type.clone()),
            model_id: job.model_id.clone(),
            progress_pct: job.progress_pct,
            bytes_done: job.bytes_done,
            bytes_total: job.bytes_total,
            current_file: job.current_file.clone(),
            message: job.message.clone(),
            status: status.to_string(),
        };
        let _ = self.app.emit("job-progress", &event);
    }

    pub fn update_job_progress(
        &self,
        job: &mut JobRecord,
        bytes_done: u64,
        bytes_total: u64,
        current_file: Option<String>,
        message: Option<String>,
    ) -> AppResult<()> {
        Self::apply_progress(job, bytes_done, bytes_total, current_file, message);
        self.db.update_job(job)?;
        self.emit_progress(job, &job.status);
        Ok(())
    }

    /// Emit UI events and persist to SQLite at most every `min_interval`.
    pub fn report_copy_progress(
        &self,
        job: &mut JobRecord,
        bytes_done: u64,
        bytes_total: u64,
        current_file: Option<String>,
        message: Option<String>,
        last_emit: &mut Instant,
        min_interval: Duration,
    ) -> AppResult<()> {
        Self::apply_progress(
            job,
            bytes_done,
            bytes_total,
            current_file,
            message.clone(),
        );

        let at_end = bytes_total > 0 && bytes_done >= bytes_total;
        if at_end || last_emit.elapsed() >= min_interval {
            self.emit_progress(job, &job.status);
            self.db.update_job(job)?;
            *last_emit = Instant::now();
        }
        Ok(())
    }

    fn apply_progress(
        job: &mut JobRecord,
        bytes_done: u64,
        bytes_total: u64,
        current_file: Option<String>,
        message: Option<String>,
    ) {
        job.bytes_done = bytes_done;
        job.bytes_total = bytes_total;
        job.current_file = current_file;
        if message.is_some() {
            job.message = message;
        }
        job.progress_pct = if bytes_total > 0 {
            (bytes_done as f64 / bytes_total as f64) * 100.0
        } else {
            0.0
        };
    }

    pub fn finish_job(&self, job: &mut JobRecord, success: bool, message: &str) -> AppResult<()> {
        job.status = if self.cancelled.load(Ordering::Relaxed) {
            "cancelled".to_string()
        } else if success {
            "completed".to_string()
        } else {
            "failed".to_string()
        };
        job.message = Some(message.to_string());
        job.finished_at = Some(Utc::now().to_rfc3339());
        job.progress_pct = if success { 100.0 } else { job.progress_pct };
        self.db.update_job(job)?;
        self.emit_progress(job, &job.status);
        Ok(())
    }

    pub fn check_cancelled(&self) -> AppResult<()> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(AppError::msg("Job cancelled"));
        }
        Ok(())
    }

    /// Fail fast when the target backup drive is unplugged mid-job.
    pub fn ensure_drive_available(&self, drive_id: Option<&str>) -> AppResult<()> {
        let Some(drive_id) = drive_id else {
            return Ok(());
        };
        let drive = self
            .db
            .get_backup_drive(drive_id)?
            .ok_or_else(|| AppError::msg("Backup drive not found"))?;
        ensure_drive_mounted(&drive)
    }

    pub fn reconcile_orphan_job(&self, job_id: &str, message: &str) -> AppResult<()> {
        if let Some(mut job) = self.db.get_job(job_id)? {
            if job.status == "running" {
                job.message = Some(message.to_string());
                self.finish_job(&mut job, false, message)?;
            }
        }
        Ok(())
    }
}

pub fn write_manifest(
    backup_dir: &Path,
    model: &ModelRecord,
    verify_hashes: bool,
) -> AppResult<BackupManifest> {
    let mut manifest_files = Vec::new();
    for f in &model.files {
        let mut entry = ManifestFileEntry {
            relative_path: f.relative_path.clone(),
            size: f.size,
            modified_at: f.modified_at,
            sha256: None,
        };
        if verify_hashes {
            let path = backup_dir.join(&f.relative_path);
            if path.exists() {
                entry.sha256 = Some(copy_engine::hash_file(&path)?);
            }
        }
        manifest_files.push(entry);
    }

    let manifest = BackupManifest {
        model_id: model.id.clone(),
        display_name: model.display_name.clone(),
        source: model.source.clone(),
        source_path: model.primary_path.clone(),
        backup_version: 1,
        created_at: Utc::now().to_rfc3339(),
        files: manifest_files,
    };

    let manifest_path = backup_dir.join("model.manifest.json");
    let json = serde_json::to_string_pretty(&manifest)?;
    std::fs::write(&manifest_path, json)?;
    Ok(manifest)
}

pub fn read_manifest(backup_dir: &Path) -> AppResult<BackupManifest> {
    let path = backup_dir.join("model.manifest.json");
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn files_needing_sync(
    model: &ModelRecord,
    manifest: &BackupManifest,
) -> Vec<crate::types::ModelFileRecord> {
    let mut needs = Vec::new();
    for src in &model.files {
        let existing = manifest.files.iter().find(|f| f.relative_path == src.relative_path);
        match existing {
            None => needs.push(src.clone()),
            Some(m) if m.size != src.size || m.modified_at != src.modified_at => {
                needs.push(src.clone())
            }
            _ => {}
        }
    }
    needs
}

pub fn run_copy_with_job(
    ctx: &JobContext,
    job: &mut JobRecord,
    source_root: &Path,
    dest_root: &Path,
    files: &[crate::types::ModelFileRecord],
) -> AppResult<u64> {
    let total = copy_engine::compute_total_bytes(files);
    ctx.update_job_progress(job, 0, total, None, Some("Copying files...".to_string()))?;

    let drive_id = job.drive_id.clone();
    let mut last_emit = Instant::now();
    let emit_interval = Duration::from_millis(250);

    let bytes = copy_engine::copy_model_files(
        source_root,
        dest_root,
        files,
        || {
            ctx.check_cancelled()?;
            ctx.ensure_drive_available(drive_id.as_deref())?;
            Ok(())
        },
        |p: CopyProgress| {
            let _ = ctx.report_copy_progress(
                job,
                p.bytes_done,
                p.bytes_total,
                Some(p.current_file),
                Some("Copying files...".to_string()),
                &mut last_emit,
                emit_interval,
            );
        },
    )?;

    // Ensure final state is persisted
    ctx.update_job_progress(
        job,
        bytes,
        total,
        job.current_file.clone(),
        Some("Copying files...".to_string()),
    )?;

    Ok(bytes)
}

pub fn prepare_backup_path(
    db: &Database,
    drive_id: &str,
    model: &ModelRecord,
) -> AppResult<(crate::types::BackupDrive, PathBuf)> {
    let drive = db
        .get_backup_drive(drive_id)?
        .ok_or_else(|| AppError::msg("Backup drive not found"))?;
    ensure_drive_mounted(&drive)?;
    let backup_path = backup_layout_path(Path::new(&drive.root_path), model);
    Ok((drive, backup_path))
}

pub fn new_job(job_id: &str, job_type: &str, model_id: &str, drive_id: &str) -> JobRecord {
    JobRecord {
        id: job_id.to_string(),
        job_type: job_type.to_string(),
        status: "running".to_string(),
        model_id: Some(model_id.to_string()),
        drive_id: Some(drive_id.to_string()),
        progress_pct: 0.0,
        bytes_done: 0,
        bytes_total: 0,
        current_file: None,
        message: Some("Starting...".to_string()),
        created_at: Utc::now().to_rfc3339(),
        finished_at: None,
    }
}
