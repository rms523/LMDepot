use super::{backup, sync, JobContext};
use crate::error::{AppError, AppResult};
use crate::types::JobRecord;
use chrono::Utc;

pub fn backup_all(ctx: &JobContext, drive_id: &str) -> AppResult<()> {
    let models = ctx.db.list_models(None)?;
    if models.is_empty() {
        return Err(AppError::msg("No models to backup"));
    }

    let total_bytes: u64 = models.iter().map(|m| m.model.total_bytes).sum();
    let mut job = JobRecord {
        id: uuid::Uuid::new_v4().to_string(),
        job_type: "backup_all".to_string(),
        status: "running".to_string(),
        model_id: None,
        drive_id: Some(drive_id.to_string()),
        progress_pct: 0.0,
        bytes_done: 0,
        bytes_total: total_bytes,
        current_file: None,
        message: Some(format!("Backing up {} models...", models.len())),
        created_at: Utc::now().to_rfc3339(),
        finished_at: None,
    };
    ctx.db.create_job(&job)?;

    let mut completed = 0u32;
    let mut bytes_before = 0u64;
    let mut errors: Vec<String> = Vec::new();

    for (index, entry) in models.iter().enumerate() {
        let model = &entry.model;
        job.message = Some(format!(
            "Backing up {}/{}: {}",
            index + 1,
            models.len(),
            model.display_name
        ));
        ctx.db.update_job(&job)?;
        ctx.emit_progress(&job, "running");

        match backup::execute_backup(ctx, &mut job, &model.id, drive_id) {
            Ok(()) => {
                completed += 1;
                bytes_before += model.total_bytes;
                job.bytes_done = bytes_before;
                job.progress_pct = if total_bytes > 0 {
                    (bytes_before as f64 / total_bytes as f64) * 100.0
                } else {
                    ((index + 1) as f64 / models.len() as f64) * 100.0
                };
                ctx.db.update_job(&job)?;
            }
            Err(e) => errors.push(format!("{}: {}", model.display_name, e)),
        }
    }

    let msg = if errors.is_empty() {
        format!("Backed up {completed} models")
    } else if completed > 0 {
        format!(
            "Backed up {completed}/{} models. {} failed.",
            models.len(),
            errors.len()
        )
    } else {
        format!("All backups failed: {}", errors.first().unwrap_or(&String::new()))
    };

    ctx.finish_job(&mut job, errors.len() < models.len(), &msg)?;
    if completed == 0 && !errors.is_empty() {
        return Err(AppError::msg(msg));
    }
    Ok(())
}

pub fn sync_all(ctx: &JobContext, drive_id: &str) -> AppResult<()> {
    let models = ctx.db.list_models(None)?;
    if models.is_empty() {
        return Err(AppError::msg("No models to sync"));
    }

    let total_bytes: u64 = models.iter().map(|m| m.model.total_bytes).sum();
    let mut job = JobRecord {
        id: uuid::Uuid::new_v4().to_string(),
        job_type: "sync_all".to_string(),
        status: "running".to_string(),
        model_id: None,
        drive_id: Some(drive_id.to_string()),
        progress_pct: 0.0,
        bytes_done: 0,
        bytes_total: total_bytes,
        current_file: None,
        message: Some(format!("Syncing {} models...", models.len())),
        created_at: Utc::now().to_rfc3339(),
        finished_at: None,
    };
    ctx.db.create_job(&job)?;

    let mut completed = 0u32;
    let mut bytes_before = 0u64;
    let mut errors: Vec<String> = Vec::new();

    for (index, entry) in models.iter().enumerate() {
        let model = &entry.model;
        job.message = Some(format!(
            "Syncing {}/{}: {}",
            index + 1,
            models.len(),
            model.display_name
        ));
        ctx.db.update_job(&job)?;
        ctx.emit_progress(&job, "running");

        match sync::execute_sync(ctx, &mut job, &model.id, drive_id) {
            Ok(()) => {
                completed += 1;
                bytes_before += model.total_bytes;
                job.bytes_done = bytes_before;
                job.progress_pct = if total_bytes > 0 {
                    (bytes_before as f64 / total_bytes as f64) * 100.0
                } else {
                    ((index + 1) as f64 / models.len() as f64) * 100.0
                };
                ctx.db.update_job(&job)?;
            }
            Err(e) => errors.push(format!("{}: {}", model.display_name, e)),
        }
    }

    let msg = if errors.is_empty() {
        format!("Synced {completed} models")
    } else if completed > 0 {
        format!(
            "Synced {completed}/{} models. {} failed.",
            models.len(),
            errors.len()
        )
    } else {
        format!("All syncs failed: {}", errors.first().unwrap_or(&String::new()))
    };

    ctx.finish_job(&mut job, errors.len() < models.len(), &msg)?;
    if completed == 0 && !errors.is_empty() {
        return Err(AppError::msg(msg));
    }
    Ok(())
}
