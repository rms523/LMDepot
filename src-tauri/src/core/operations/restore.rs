use super::{new_job, run_copy_with_job, JobContext};
use crate::core::copy_engine;
use crate::core::drive_monitor::ensure_drive_mounted;
use crate::error::{AppError, AppResult};
use std::path::Path;
use uuid::Uuid;

pub fn run(
    ctx: &JobContext,
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
    let mut job = new_job("restore", model_id, drive_id);
    ctx.db.create_job(&job)?;

    let model = model_with.model;
    let result = (|| -> AppResult<()> {
        ctx.check_cancelled()?;
        let source = Path::new(backup_path);
        let dest = Path::new(&restore_target);
        std::fs::create_dir_all(dest)?;

        run_copy_with_job(ctx, &mut job, source, dest, &model.files)?;

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

    run(ctx, model_id, drive_id, Some(model.primary_path.clone()))
}
