use super::{new_job, run_copy_with_job, JobContext};
use crate::core::copy_engine;
use crate::core::drive_monitor::ensure_drive_mounted;
use crate::error::{AppError, AppResult};
use std::path::Path;
use uuid::Uuid;

/// Hugging Face Hub tools resolve snapshots via `refs/main` (or other ref files).
fn ensure_hf_snapshot_refs(dest: &Path) {
    let snapshots_dir = dest.parent();
    let repo_dir = snapshots_dir.and_then(|p| p.parent());
    let Some(snapshots_dir) = snapshots_dir else {
        return;
    };
    if snapshots_dir.file_name().and_then(|n| n.to_str()) != Some("snapshots") {
        return;
    }
    let Some(repo_dir) = repo_dir else {
        return;
    };
    if !repo_dir
        .file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with("models--"))
        .unwrap_or(false)
    {
        return;
    }

    let revision = dest
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.is_empty());
    let Some(revision) = revision else {
        return;
    };

    let refs_main = repo_dir.join("refs").join("main");
    if refs_main.exists() {
        return;
    }
    if std::fs::create_dir_all(refs_main.parent().unwrap()).is_err() {
        return;
    }
    let _ = std::fs::write(&refs_main, format!("{revision}\n"));
}

pub fn run(
    ctx: &JobContext,
    job_id: &str,
    model_id: &str,
    drive_id: &str,
    target_path: Option<String>,
) -> AppResult<()> {
    let model_with = ctx
        .db
        .get_model(model_id)?
        .ok_or_else(|| AppError::msg("Model not found"))?;

    let backup_entry = model_with
        .backups
        .iter()
        .find(|b| b.drive_id == drive_id)
        .ok_or_else(|| AppError::msg("No backup found on this drive"))?;

    let backup_path = backup_entry
        .backup_path
        .as_ref()
        .ok_or_else(|| AppError::msg("Backup path missing"))?;

    let drive = ctx
        .db
        .get_backup_drive(drive_id)?
        .ok_or_else(|| AppError::msg("Drive not found"))?;
    ensure_drive_mounted(&drive)?;

    let restore_target = target_path.unwrap_or_else(|| model_with.model.primary_path.clone());
    let mut job = new_job(job_id, "restore", model_id, drive_id);
    ctx.db.create_job(&job)?;

    let model = model_with.model;
    let result = (|| -> AppResult<()> {
        ctx.check_cancelled()?;
        let source = Path::new(backup_path);
        let dest = Path::new(&restore_target);
        std::fs::create_dir_all(dest)?;

        run_copy_with_job(ctx, &mut job, source, dest, &model.files)?;
        ensure_hf_snapshot_refs(dest);

        ctx.db.upsert_model_backup(
            &Uuid::new_v4().to_string(),
            model_id,
            drive_id,
            backup_path,
            "backed_up",
        )?;
        Ok(())
    })();

    match result {
        Ok(()) => ctx.finish_job(&mut job, true, "Restore completed")?,
        Err(e) => {
            let msg = e.to_string();
            ctx.finish_job(&mut job, false, &msg)?;
            return Err(e);
        }
    }
    Ok(())
}

pub fn restore_offload(
    ctx: &JobContext,
    job_id: &str,
    model_id: &str,
    drive_id: &str,
) -> AppResult<()> {
    let model = ctx
        .db
        .get_model(model_id)?
        .ok_or_else(|| AppError::msg("Model not found"))?
        .model;

    let source_path = Path::new(&model.primary_path);
    if !copy_engine::is_symlink(source_path) {
        return Err(AppError::msg("Model is not offloaded (not a symlink)"));
    }

    run(
        ctx,
        job_id,
        model_id,
        drive_id,
        Some(model.primary_path.clone()),
    )
}
